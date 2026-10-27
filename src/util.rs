use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub static CANCELLED: AtomicBool = AtomicBool::new(false);
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);
pub fn check_cancelled() -> Result<()> {
    if CANCELLED.load(Ordering::Relaxed) {
        bail!("[E_CANCELLED] cancelled / 用户中断，已停止启动新任务");
    }
    Ok(())
}
pub fn run_id() -> String {
    format!("{}-{}", now_nanos(), std::process::id())
}
pub fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes.as_ref()))
}
pub fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).with_context(|| format!("read {}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(hex::encode(digest.finalize()))
}
pub fn path_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}
pub fn absolute(base: &Path, value: &Path) -> PathBuf {
    let joined = if value.is_absolute() {
        value.to_owned()
    } else {
        base.join(value)
    };
    let mut result = PathBuf::new();
    for item in joined.components() {
        match item {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(item.as_os_str()),
        }
    }
    result
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("file has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}
pub fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    atomic_write(path, &bytes)
}
pub fn files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    if !root.exists() {
        return Ok(result);
    }
    let mut entries = fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            bail!(
                "[E_PATH] symbolic link is not supported: {}",
                entry.path().display()
            );
        }
        if ty.is_dir() {
            result.extend(files(&entry.path())?);
        } else if ty.is_file() {
            result.push(entry.path());
        }
    }
    Ok(result)
}
pub fn no_symlinks(path: &Path) -> Result<()> {
    for parent in path.ancestors() {
        if let Ok(meta) = fs::symlink_metadata(parent) {
            if meta.file_type().is_symlink() {
                bail!("[E_PATH] refusing symbolic link: {}", parent.display());
            }
        }
    }
    Ok(())
}
pub fn data_dir() -> Result<PathBuf> {
    if let Some(value) = std::env::var_os("PROTO_BUILDER_HOME") {
        return Ok(absolute(&std::env::current_dir()?, Path::new(&value)));
    }
    Ok(dirs::data_local_dir()
        .context("cannot locate user data directory")?
        .join("ProtoBuilder"))
}
pub fn user_config() -> Result<PathBuf> {
    Ok(data_dir()?.join("config.toml"))
}
pub fn project_state(root: &Path, server: &Path, calc: &Path) -> Result<PathBuf> {
    let identity = format!(
        "{}\n{}\n{}",
        path_key(root),
        path_key(server),
        path_key(calc)
    );
    Ok(data_dir()?.join("projects").join(hash(identity)))
}
pub fn display_command(command: &std::process::Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|x| format!("{:?}", x.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}
