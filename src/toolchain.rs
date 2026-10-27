use crate::{config::Settings, report::Reporter, util};
use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MANIFEST: &str = include_str!("../assets/windows-x86_64/manifest.json");
const ARCHIVES: &[(&str, &[u8])] = &[
    (
        "protoc.exe",
        include_bytes!("../assets/windows-x86_64/protoc.exe.gz"),
    ),
    (
        "protoc-gen-go.exe",
        include_bytes!("../assets/windows-x86_64/protoc-gen-go.exe.gz"),
    ),
    (
        "protoc-gen-go-grpc.exe",
        include_bytes!("../assets/windows-x86_64/protoc-gen-go-grpc.exe.gz"),
    ),
    (
        "protoc-gen-doc.exe",
        include_bytes!("../assets/windows-x86_64/protoc-gen-doc.exe.gz"),
    ),
];
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub platform: String,
    pub tools: Vec<Tool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub size: u64,
}
pub struct Tools {
    pub paths: BTreeMap<String, PathBuf>,
    pub provenance: serde_json::Value,
}
impl Tools {
    pub fn get(&self, name: &str) -> Result<&Path> {
        self.paths
            .get(name)
            .map(PathBuf::as_path)
            .with_context(|| format!("[E_TOOLCHAIN] missing {name}"))
    }
}
pub fn manifest() -> Result<Manifest> {
    serde_json::from_str(MANIFEST.trim_start_matches('\u{feff}')).context("embedded manifest")
}
pub fn resolve(root: &Path, s: &Settings, report: &Reporter) -> Result<Tools> {
    if s.toolchain.mode == "external" {
        report.emit(
            "toolchain",
            "warning",
            report.text(
                "已显式启用外部工具链；结果将记录路径、版本与哈希",
                "External toolchain enabled; paths, versions and hashes are recorded",
            ),
        );
        let mut paths = BTreeMap::new();
        let mut records = vec![];
        for name in [
            "protoc",
            "protoc-gen-go",
            "protoc-gen-go-grpc",
            "protoc-gen-doc",
        ] {
            if name == "protoc-gen-doc" && !s.docs {
                continue;
            }
            let configured = s.toolchain.paths.get(name).with_context(|| {
                format!("[E_TOOLCHAIN] explicit toolchain.paths.{name} required in external mode")
            })?;
            let path = util::absolute(root, configured);
            if !path.is_file() {
                bail!("[E_TOOLCHAIN] {} does not exist", path.display());
            }
            let version_output = crate::process::run(
                std::process::Command::new(&path).arg("--version"),
                "toolchain",
                s.timeout_seconds,
                report,
            )?;
            let version = format!("{}{}", version_output.stdout, version_output.stderr);
            records.push(serde_json::json!({"name":name,"path":path,"sha256":util::file_hash(&path)?,"version":version.trim()}));
            paths.insert(name.into(), path);
        }
        return Ok(Tools {
            paths,
            provenance: serde_json::json!({"source":"external","tools":records}),
        });
    }
    if !cfg!(all(windows, target_arch = "x86_64")) {
        bail!("[E_PLATFORM] embedded toolchain currently supports Windows x86_64 only; use explicit external mode on other platforms");
    }
    let spec = manifest()?;
    let identity = util::hash(MANIFEST);
    let base = util::data_dir()?.join("toolchains").join(&identity[..20]);
    fs::create_dir_all(&base)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(base.join("extract.lock"))?;
    lock.lock_exclusive().context("lock toolchain cache")?;
    let mut paths = BTreeMap::new();
    for tool in &spec.tools {
        let path = base.join(&tool.name);
        let valid = path.is_file() && util::file_hash(&path)? == tool.sha256;
        if !valid {
            let bytes = ARCHIVES
                .iter()
                .find(|(name, _)| *name == tool.name)
                .context("tool archive missing")?
                .1;
            let mut unpacked = Vec::with_capacity(tool.size as usize);
            GzDecoder::new(bytes)
                .take(tool.size + 1)
                .read_to_end(&mut unpacked)?;
            if unpacked.len() as u64 != tool.size || util::hash(&unpacked) != tool.sha256 {
                bail!(
                    "[E_INTEGRITY] embedded {} failed SHA256 verification",
                    tool.name
                );
            }
            util::atomic_write(&path, &unpacked)?;
            report.detail(
                "toolchain",
                format!("extract {} {}", tool.name, tool.version),
            );
        }
        paths.insert(tool.name.trim_end_matches(".exe").to_owned(), path);
    }
    util::write_json(&base.join("manifest.json"), &spec)?;
    Ok(Tools {
        paths,
        provenance: serde_json::json!({"source":"embedded","identity":identity,"manifest":spec}),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_archive_matches_pinned_manifest() {
        let spec = manifest().unwrap();
        for tool in spec.tools {
            let archive = ARCHIVES.iter().find(|(n, _)| *n == tool.name).unwrap().1;
            let mut bytes = vec![];
            GzDecoder::new(archive).read_to_end(&mut bytes).unwrap();
            assert_eq!(util::hash(&bytes), tool.sha256);
            assert_eq!(bytes.len() as u64, tool.size);
        }
    }
}
