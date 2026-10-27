use crate::{config::Settings, report::Reporter, util};
use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::PathBuf, time::Duration};

const REPOSITORY: &str = "dongrv/proto-builder";
const PUBLIC_KEY: &str = match option_env!("PROTO_BUILDER_UPDATE_PUBLIC_KEY") {
    Some(key) => key,
    None => include_str!("../assets/update-public-key.hex"),
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct ReleaseManifest {
    repository: String,
    version: String,
    channel: String,
    assets: Vec<SignedAsset>,
}
#[derive(Debug, Serialize, Deserialize)]
struct SignedAsset {
    platform: String,
    name: String,
    sha256: String,
    size: u64,
}
#[derive(Serialize, Deserialize)]
struct CheckCache {
    checked_at: u64,
    release: Option<Release>,
}
#[derive(Serialize, Deserialize)]
struct Rollback {
    target: PathBuf,
    backup: PathBuf,
    sha256: String,
}
pub fn trust_configured() -> bool {
    hex::decode(PUBLIC_KEY.trim()).is_ok_and(|v| v.len() == 32)
}
pub fn public_key() -> &'static str {
    PUBLIC_KEY.trim()
}
fn client(s: &Settings) -> Result<Client> {
    let mut builder = Client::builder()
        .user_agent(concat!("proto-builder/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(12))
        .https_only(true);
    if let Some(proxy) = &s.update.proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    builder.build().context("[E_UPDATE] HTTP client")
}
fn read_limited(response: reqwest::blocking::Response, limit: u64) -> Result<Vec<u8>> {
    let response = response
        .error_for_status()
        .context("[E_UPDATE] HTTP status")?;
    if response.content_length().is_some_and(|n| n > limit) {
        bail!("[E_UPDATE] response exceeds size limit");
    }
    let mut bytes = vec![];
    response.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("[E_UPDATE] download exceeds size limit");
    }
    Ok(bytes)
}
fn version(tag: &str) -> Result<Version> {
    Version::parse(tag.trim_start_matches('v')).context("[E_UPDATE] invalid release version")
}
fn select(releases: Vec<Release>, channel: &str) -> Option<Release> {
    releases
        .into_iter()
        .filter(|r| !r.draft && (channel == "beta" || !r.prerelease))
        .filter_map(|r| {
            version(&r.tag_name)
                .ok()
                .filter(|v| channel == "beta" || v.pre.is_empty())
                .map(|v| (v, r))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, r)| r)
}
pub fn check(s: &Settings, report: &Reporter) -> Result<Option<Release>> {
    let client = client(s)?;
    let selected = if s.update.mirror.is_some() {
        discover_signed_channel(&client, s)?
    } else {
        match discover_api(&client, s) {
            Ok(release) => release,
            Err(error) => {
                report.detail(
                    "update",
                    format!("GitHub API unavailable, trying signed channel manifest: {error:#}"),
                );
                discover_signed_channel(&client,s).with_context(||format!("[E_UPDATE] GitHub API unavailable ({error:#}); signed channel fallback also failed"))?
            }
        }
    };
    show_release(s, report, selected)
}
fn discover_api(client: &Client, s: &Settings) -> Result<Option<Release>> {
    let mut request = client
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases?per_page=100"
        ))
        .header("Accept", "application/vnd.github+json");
    if let Ok(token) = std::env::var("PROTO_BUILDER_GITHUB_TOKEN") {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .context("[E_UPDATE] GitHub releases request")?;
    let bytes = read_limited(response, 4 * 1024 * 1024)?;
    let releases: Vec<Release> = serde_json::from_slice(&bytes)?;
    Ok(select(releases, &s.update.channel))
}
fn discover_signed_channel(client: &Client, s: &Settings) -> Result<Option<Release>> {
    let base = if let Some(mirror) = &s.update.mirror {
        format!(
            "{}/{}",
            mirror.trim_end_matches('/'),
            if s.update.channel == "stable" {
                "latest"
            } else {
                "beta"
            }
        )
    } else if s.update.channel == "stable" {
        format!("https://github.com/{REPOSITORY}/releases/latest/download")
    } else {
        bail!("[E_UPDATE] beta discovery requires GitHub API access or an explicit signed mirror");
    };
    let bytes = read_limited(client.get(format!("{base}/release.json")).send()?, 65536)?;
    let signature = read_limited(client.get(format!("{base}/release.json.sig")).send()?, 1024)?;
    let manifest = verify_manifest(&bytes, &signature, public_key())?;
    if manifest.repository != REPOSITORY || manifest.channel != s.update.channel {
        bail!("[E_SIGNATURE] channel manifest belongs to another repository or channel");
    }
    let tag = format!("v{}", manifest.version);
    let mut names = manifest
        .assets
        .iter()
        .map(|a| a.name.clone())
        .collect::<Vec<_>>();
    names.extend(["release.json".into(), "release.json.sig".into()]);
    let assets = names
        .into_iter()
        .map(|name| Asset {
            browser_download_url: format!(
                "https://github.com/{REPOSITORY}/releases/download/{tag}/{name}"
            ),
            name,
        })
        .collect();
    Ok(Some(Release {
        tag_name: tag,
        draft: false,
        prerelease: manifest.channel == "beta",
        published_at: String::new(),
        body: String::new(),
        assets,
    }))
}
fn show_release(
    s: &Settings,
    report: &Reporter,
    selected: Option<Release>,
) -> Result<Option<Release>> {
    if let Some(release) = &selected {
        if version(&release.tag_name)? > Version::parse(env!("CARGO_PKG_VERSION"))? {
            report.info(
                "update",
                format!(
                    "{} {} ({})\n{}\nproto-builder self-update apply",
                    report.text("发现新版本", "New version"),
                    release.tag_name,
                    release.published_at,
                    release.body.chars().take(1600).collect::<String>()
                ),
            );
        } else {
            report.info(
                "update",
                report.text("已是当前渠道最新版本", "Already current for this channel"),
            );
        }
    } else {
        report.info(
            "update",
            report.text(
                "官方仓库尚无该渠道发布版本",
                "No published release for this channel",
            ),
        );
    }
    util::write_json(
        &util::data_dir()?.join(format!("update-check-{}.json", s.update.channel)),
        &CheckCache {
            checked_at: util::now_secs(),
            release: selected.clone(),
        },
    )?;
    Ok(selected)
}
pub fn background_check(s: &Settings, report: &Reporter) {
    if !s.update.enabled {
        return;
    }
    let last = util::data_dir()
        .ok()
        .and_then(|p| fs::read(p.join(format!("update-check-{}.json", s.update.channel))).ok())
        .and_then(|v| serde_json::from_slice::<CheckCache>(&v).ok());
    if last.is_some_and(|v| {
        util::now_secs().saturating_sub(v.checked_at) < s.update.interval_hours.saturating_mul(3600)
    }) {
        return;
    }
    if let Err(error) = check(s, report) {
        report.detail(
            "update",
            format!("version check skipped / 更新检查失败，不影响本地功能: {error:#}"),
        );
        if let Ok(base) = util::data_dir() {
            let _ = util::write_json(
                &base.join(format!("update-check-{}.json", s.update.channel)),
                &CheckCache {
                    checked_at: util::now_secs(),
                    release: None,
                },
            );
        }
    }
}
fn asset_url(release: &Release, name: &str, s: &Settings) -> Result<String> {
    if let Some(mirror) = &s.update.mirror {
        if !mirror.starts_with("https://") {
            bail!("[E_UPDATE] mirror must use HTTPS");
        }
        return Ok(format!(
            "{}/{}/{}",
            mirror.trim_end_matches('/'),
            release.tag_name,
            name
        ));
    }
    let url = &release
        .assets
        .iter()
        .find(|a| a.name == name)
        .with_context(|| format!("[E_UPDATE] release asset missing: {name}"))?
        .browser_download_url;
    let prefix = format!(
        "https://github.com/{REPOSITORY}/releases/download/{}/",
        release.tag_name
    );
    if !url.starts_with(&prefix) {
        bail!("[E_UPDATE] asset URL is outside official release");
    }
    Ok(url.clone())
}
fn verify_manifest(bytes: &[u8], signature: &[u8], public_key: &str) -> Result<ReleaseManifest> {
    let key: [u8; 32] = hex::decode(public_key.trim())
        .context("[E_TRUST_KEY] public key must be hex")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("[E_TRUST_KEY] expected 32-byte Ed25519 key"))?;
    let raw = hex::decode(std::str::from_utf8(signature)?.trim())
        .context("[E_SIGNATURE] invalid signature encoding")?;
    let signature =
        Signature::from_slice(&raw).context("[E_SIGNATURE] invalid Ed25519 signature")?;
    VerifyingKey::from_bytes(&key)?
        .verify_strict(bytes, &signature)
        .context("[E_SIGNATURE] release signature verification failed")?;
    serde_json::from_slice(bytes).context("[E_UPDATE] signed manifest JSON")
}
pub fn apply(s: &Settings, report: &Reporter) -> Result<()> {
    let key = public_key();
    if !trust_configured() {
        bail!("[E_TRUST_KEY] invalid embedded official release public key");
    }
    let target = std::env::current_exe()?;
    let _lock = crate::transaction::Locks::acquire(std::slice::from_ref(&target))?;
    let release = check(s, report)?.context("[E_UPDATE] no release available")?;
    if version(&release.tag_name)? <= Version::parse(env!("CARGO_PKG_VERSION"))? {
        return Ok(());
    }
    let client = client(s)?;
    let manifest_bytes = read_limited(
        client.get(asset_url(&release, "release.json", s)?).send()?,
        65536,
    )?;
    let signature = read_limited(
        client
            .get(asset_url(&release, "release.json.sig", s)?)
            .send()?,
        1024,
    )?;
    let manifest = verify_manifest(&manifest_bytes, &signature, key)?;
    if manifest.repository != REPOSITORY
        || version(&manifest.version)? != version(&release.tag_name)?
        || manifest.channel != if release.prerelease { "beta" } else { "stable" }
    {
        bail!("[E_SIGNATURE] signed repository/version/channel does not match selected release");
    }
    let asset = manifest
        .assets
        .iter()
        .find(|a| a.platform == "windows-x86_64")
        .context("[E_UPDATE] Windows x86_64 asset missing")?;
    if !cfg!(all(windows, target_arch = "x86_64")) {
        bail!("[E_PLATFORM] automatic update supports Windows x86_64 only");
    }
    if asset.size > 128 * 1024 * 1024
        || asset.name.contains(['/', '\\'])
        || !asset.name.ends_with(".exe")
    {
        bail!("[E_UPDATE] invalid signed asset metadata");
    }
    let bytes = read_limited(
        client.get(asset_url(&release, &asset.name, s)?).send()?,
        asset.size,
    )?;
    if bytes.len() as u64 != asset.size || util::hash(&bytes) != asset.sha256 {
        bail!("[E_INTEGRITY] downloaded binary failed size/SHA256 verification");
    }
    let dir = util::data_dir()?.join("updates").join(util::run_id());
    fs::create_dir_all(&dir)?;
    let downloaded = dir.join(&asset.name);
    util::atomic_write(&downloaded, &bytes)?;
    // Test the signed replacement before touching the installed executable.
    let verification = crate::process::run(
        std::process::Command::new(&downloaded).args(["version", "--json", "--non-interactive"]),
        "update-test",
        30,
        report,
    )?;
    let value: serde_json::Value = serde_json::from_str(&verification.stdout)
        .context("[E_UPDATE] replacement version output")?;
    if value["version"].as_str().map(version).transpose()?.as_ref()
        != Some(&version(&manifest.version)?)
    {
        bail!("[E_UPDATE] replacement reported an unexpected version");
    }
    if value["result"]["platform"] != "windows"
        || value["result"]["architecture"] != "x86_64"
        || value["result"]["update_public_key"] != public_key()
    {
        bail!(
            "[E_UPDATE] replacement platform or release trust key differs from this installation"
        );
    }
    let backup = dir.join("previous.exe");
    fs::copy(&target, &backup)?;
    util::write_json(
        &util::data_dir()?.join("rollback.json"),
        &Rollback {
            target: target.clone(),
            backup: backup.clone(),
            sha256: util::file_hash(&backup)?,
        },
    )?;
    if let Err(error) = self_replace::self_replace(&downloaded) {
        if !target.is_file() || util::file_hash(&target)? != util::file_hash(&backup)? {
            util::atomic_write(&target, &fs::read(&backup)?).with_context(|| {
                format!(
                    "[E_UPDATE] replacement and automatic restoration failed; original backup: {}",
                    backup.display()
                )
            })?;
        }
        return Err(error).context("[E_UPDATE] native replacement failed; original restored");
    }
    report.info(
        "update",
        format!(
            "{} {} | rollback: proto-builder self-update rollback",
            report.text("已更新到", "Updated to"),
            manifest.version
        ),
    );
    report.result(serde_json::json!({"version":manifest.version,"verified_signature":true,"sha256":asset.sha256,"backup":backup}));
    Ok(())
}
pub fn rollback(report: &Reporter) -> Result<()> {
    let file = util::data_dir()?.join("rollback.json");
    let record: Rollback = serde_json::from_slice(
        &fs::read(&file).context("[E_UPDATE] no rollback backup available")?,
    )?;
    let target = std::env::current_exe()?;
    let _lock = crate::transaction::Locks::acquire(std::slice::from_ref(&target))?;
    if util::path_key(&target) != util::path_key(&record.target)
        || util::file_hash(&record.backup)? != record.sha256
    {
        bail!("[E_INTEGRITY] rollback backup does not match this installation");
    }
    self_replace::self_replace(&record.backup)?;
    fs::remove_file(file)?;
    report.info(
        "rollback",
        report.text("已恢复更新前版本", "Restored previous executable"),
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[test]
    fn tampered_signed_manifest_is_rejected() {
        let key = SigningKey::from_bytes(&[42; 32]);
        let bytes=br#"{"repository":"dongrv/proto-builder","version":"1.0.1","channel":"stable","assets":[]}"#;
        let sig = hex::encode(key.sign(bytes).to_bytes());
        let public = hex::encode(key.verifying_key().to_bytes());
        assert!(verify_manifest(bytes, sig.as_bytes(), &public).is_ok());
        let mut corrupted = bytes.to_vec();
        corrupted[5] = b'X';
        assert!(verify_manifest(&corrupted, sig.as_bytes(), &public).is_err());
    }
    #[test]
    fn stable_excludes_prereleases() {
        let releases = vec![
            Release {
                tag_name: "v2.0.0-beta.1".into(),
                draft: false,
                prerelease: true,
                published_at: String::new(),
                body: String::new(),
                assets: vec![],
            },
            Release {
                tag_name: "v1.1.0".into(),
                draft: false,
                prerelease: false,
                published_at: String::new(),
                body: String::new(),
                assets: vec![],
            },
        ];
        assert_eq!(
            select(releases.clone(), "stable").unwrap().tag_name,
            "v1.1.0"
        );
        assert_eq!(select(releases, "beta").unwrap().tag_name, "v2.0.0-beta.1");
    }
}
