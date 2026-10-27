use crate::{cli::Options, util};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub config_version: u32,
    pub project_root: PathBuf,
    pub proto_path: PathBuf,
    pub server_path: PathBuf,
    pub calc_path: PathBuf,
    pub assign_id: u64,
    pub activity_ids: Vec<String>,
    pub machine_ids: Vec<String>,
    pub excludes: Vec<String>,
    pub jobs: usize,
    pub docs: bool,
    pub protocol_config: bool,
    pub copy_to_calc: bool,
    pub lang: String,
    pub timeout_seconds: u64,
    pub log_retention: usize,
    pub svn: Svn,
    pub toolchain: Toolchain,
    pub update: Update,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Svn {
    pub update: bool,
    pub update_paths: Vec<PathBuf>,
    pub executable: PathBuf,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Toolchain {
    pub mode: String,
    pub paths: BTreeMap<String, PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Update {
    pub enabled: bool,
    pub interval_hours: u64,
    pub channel: String,
    pub mirror: Option<String>,
    pub proxy: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            config_version: 1,
            project_root: PathBuf::from("."),
            proto_path: PathBuf::from("proto"),
            server_path: PathBuf::from("../../../server/branches/beta"),
            calc_path: PathBuf::from("../../../cal_center/branches/beta"),
            assign_id: 0,
            activity_ids: vec![],
            machine_ids: vec![],
            excludes: vec![],
            jobs: 0,
            docs: true,
            protocol_config: true,
            copy_to_calc: true,
            lang: system_language(),
            timeout_seconds: 120,
            log_retention: 20,
            svn: Svn::default(),
            toolchain: Toolchain::default(),
            update: Update::default(),
        }
    }
}
fn system_language() -> String {
    if let Ok(language) = env::var("LANG") {
        if language.to_lowercase().starts_with("en") {
            return "en-US".into();
        }
    }
    #[cfg(windows)]
    {
        if unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() } & 0x03ff == 0x09
        {
            return "en-US".into();
        }
    }
    "zh-CN".into()
}
impl Default for Svn {
    fn default() -> Self {
        Self {
            update: true,
            update_paths: vec![],
            executable: PathBuf::from("svn"),
        }
    }
}
impl Default for Toolchain {
    fn default() -> Self {
        Self {
            mode: "embedded".into(),
            paths: BTreeMap::new(),
        }
    }
}
impl Default for Update {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_hours: 24,
            channel: "stable".into(),
            mirror: None,
            proxy: None,
        }
    }
}
#[derive(Debug)]
pub struct Loaded {
    pub settings: Settings,
    pub root: PathBuf,
    pub sources: Vec<PathBuf>,
    pub project_file: Option<PathBuf>,
}
impl Settings {
    pub fn proto(&self, root: &Path) -> PathBuf {
        util::absolute(root, &self.proto_path)
    }
    pub fn server(&self, root: &Path) -> PathBuf {
        util::absolute(root, &self.server_path)
    }
    pub fn calc(&self, root: &Path) -> PathBuf {
        util::absolute(root, &self.calc_path)
    }
    pub fn workers(&self) -> usize {
        let cpu = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        if self.jobs == 0 {
            cpu.saturating_sub(1).max(1)
        } else {
            self.jobs
        }
    }
}
pub fn load(options: &Options) -> Result<Loaded> {
    load_from(options, &env::current_dir()?, Some(&util::user_config()?))
}
pub fn load_from(options: &Options, cwd: &Path, user: Option<&Path>) -> Result<Loaded> {
    let mut value = toml::Value::try_from(Settings::default())?;
    let mut sources = vec![];
    if let Some(user) = user.filter(|p| p.is_file()) {
        merge_file(&mut value, user)?;
        sources.push(user.to_owned());
    }
    let search = options
        .repo_root
        .as_deref()
        .map(|p| util::absolute(cwd, p))
        .unwrap_or_else(|| cwd.to_owned());
    let explicit = options.config.as_ref().map(|p| util::absolute(cwd, p));
    let project_file = explicit.clone().or_else(|| {
        search
            .ancestors()
            .map(|p| p.join("proto-builder.toml"))
            .find(|p| p.is_file())
    });
    if let Some(path) = &project_file {
        merge_file(&mut value, path)?;
        sources.push(path.clone());
    }
    let mut settings: Settings = value
        .try_into()
        .context("[E_CONFIG] invalid configuration")?;
    let base = project_file
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or(cwd);
    let config_root = if settings.project_root != Path::new(".") || project_file.is_some() {
        Some(util::absolute(base, &settings.project_root))
    } else {
        None
    };
    apply_env(&mut settings)?;
    let env_root =
        env::var_os("PROTO_BUILDER_PROJECT_ROOT").map(|p| util::absolute(cwd, Path::new(&p)));
    let root = options
        .repo_root
        .as_deref()
        .map(|p| util::absolute(cwd, p))
        .or(env_root)
        .or(config_root)
        .or_else(|| {
            search
                .ancestors()
                .find(|p| p.join("proto").is_dir())
                .map(Path::to_path_buf)
        })
        .unwrap_or(search);
    if let Some(v) = &options.proto_path {
        settings.proto_path = v.clone();
    }
    if let Some(v) = &options.server_path {
        settings.server_path = v.clone();
    }
    if let Some(v) = &options.calc_path {
        settings.calc_path = v.clone();
    }
    if let Some(v) = options.assign_id {
        settings.assign_id = v;
    }
    if !options.activity_ids.is_empty() {
        settings.activity_ids = options.activity_ids.clone();
    }
    if !options.machine_ids.is_empty() {
        settings.machine_ids = options.machine_ids.clone();
    }
    if !options.excludes.is_empty() {
        settings.excludes = options.excludes.clone();
    }
    if let Some(v) = options.jobs {
        settings.jobs = v;
    }
    if let Some(v) = &options.lang {
        settings.lang = v.clone();
    }
    if let Some(value) = options.docs {
        settings.docs = value;
    }
    if let Some(value) = options.protocol_config {
        settings.protocol_config = value;
    }
    if let Some(value) = options.copy_to_calc {
        settings.copy_to_calc = value;
    }
    if let Some(value) = options.svn_update {
        settings.svn.update = value;
    }
    settings.activity_ids.retain(|v| !v.is_empty());
    settings.machine_ids.retain(|v| !v.is_empty());
    settings.excludes.retain(|v| !v.is_empty());
    if options.no_docs {
        settings.docs = false;
    }
    if options.no_config {
        settings.protocol_config = false;
    }
    if options.no_copy {
        settings.copy_to_calc = false;
    }
    if options.no_svn_update {
        settings.svn.update = false;
    }
    validate(&settings)?;
    Ok(Loaded {
        settings,
        root,
        sources,
        project_file,
    })
}
fn merge(target: &mut toml::Value, patch: toml::Value) {
    match (target, patch) {
        (toml::Value::Table(target), toml::Value::Table(patch)) => {
            for (k, v) in patch {
                if let Some(old) = target.get_mut(&k) {
                    merge(old, v)
                } else {
                    target.insert(k, v);
                }
            }
        }
        (target, patch) => *target = patch,
    }
}
fn merge_file(value: &mut toml::Value, path: &Path) -> Result<()> {
    let text =
        fs::read_to_string(path).with_context(|| format!("[E_CONFIG] read {}", path.display()))?;
    let _: Settings =
        toml::from_str(&text).with_context(|| format!("[E_CONFIG] {}", path.display()))?;
    let mut patch: toml::Value =
        toml::from_str(&text).with_context(|| format!("[E_CONFIG] {}", path.display()))?;
    if let Some(version) = patch.get("config_version").and_then(|v| v.as_integer()) {
        if !(0..=1).contains(&version) {
            bail!(
                "[E_CONFIG] {}: unsupported config_version={version}",
                path.display()
            );
        }
        if version == 0 {
            patch["config_version"] = toml::Value::Integer(1);
        }
    }
    merge(value, patch);
    Ok(())
}
fn apply_env(s: &mut Settings) -> Result<()> {
    for (name, target) in [
        ("PROTO_PATH", &mut s.proto_path),
        ("SERVER_PATH", &mut s.server_path),
        ("CALC_PATH", &mut s.calc_path),
    ] {
        if let Some(v) = env::var_os(format!("PROTO_BUILDER_{name}")) {
            *target = PathBuf::from(v);
        }
    }
    for (name, target) in [
        ("ACTIVITY_IDS", &mut s.activity_ids),
        ("MACHINE_IDS", &mut s.machine_ids),
        ("EXCLUDES", &mut s.excludes),
    ] {
        if let Ok(v) = env::var(format!("PROTO_BUILDER_{name}")) {
            *target = v
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect();
        }
    }
    if let Ok(v) = env::var("PROTO_BUILDER_JOBS") {
        s.jobs = v.parse().context("PROTO_BUILDER_JOBS must be an integer")?;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_LANG") {
        s.lang = v;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_SVN_UPDATE") {
        s.svn.update = v
            .parse()
            .context("PROTO_BUILDER_SVN_UPDATE must be true/false")?;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_TOOLCHAIN_MODE") {
        s.toolchain.mode = v;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_UPDATE_CHANNEL") {
        s.update.channel = v;
    }
    Ok(())
}
pub fn validate(s: &Settings) -> Result<()> {
    if s.config_version != 1 {
        bail!("[E_CONFIG] unsupported config_version {}", s.config_version);
    }
    if !["zh-CN", "en-US"].contains(&s.lang.as_str()) {
        bail!("[E_CONFIG] lang: expected zh-CN or en-US");
    }
    if !["embedded", "external"].contains(&s.toolchain.mode.as_str()) {
        bail!("[E_CONFIG] toolchain.mode: embedded or external required");
    }
    if !["stable", "beta"].contains(&s.update.channel.as_str()) {
        bail!("[E_CONFIG] update.channel: stable or beta required");
    }
    if s.timeout_seconds == 0 || s.jobs > 256 {
        bail!("[E_CONFIG] timeout_seconds must be > 0; jobs must be 0..256");
    }
    for id in s.activity_ids.iter().chain(&s.machine_ids) {
        if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) {
            bail!("[E_CONFIG] invalid numeric ID: {id:?}");
        }
    }
    Ok(())
}
pub fn save(path: &Path, settings: &Settings) -> Result<()> {
    if path.is_file() {
        let backup = path.with_extension("toml.bak");
        fs::copy(path, backup)?;
    }
    util::atomic_write(path, toml::to_string_pretty(settings)?.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_default_value_overrides_config_and_paths_anchor_to_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(
            root.join("custom.toml"),
            "config_version=1\nproto_path='other'\n[svn]\nupdate=false\n",
        )
        .unwrap();
        let opts = Options {
            config: Some(root.join("custom.toml")),
            proto_path: Some("proto".into()),
            ..Default::default()
        };
        let loaded = load_from(&opts, root, None).unwrap();
        assert_eq!(loaded.settings.proto_path, PathBuf::from("proto"));
        assert!(!loaded.settings.svn.update);
        assert_eq!(loaded.root, root);
    }
    #[test]
    fn unknown_keys_and_future_versions_are_errors() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("proto-builder.toml");
        for text in ["config_version=999", "prot_path='oops'"] {
            fs::write(&file, text).unwrap();
            assert!(load_from(&Options::default(), dir.path(), None).is_err());
        }
    }
}
