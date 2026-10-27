use crate::{report::Reporter, util};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
struct Installation {
    directory: PathBuf,
    path_added: bool,
    system: bool,
}
fn record(system: bool) -> Result<PathBuf> {
    if system {
        return Ok(PathBuf::from(
            std::env::var_os("ProgramData").context("ProgramData not found")?,
        )
        .join("ProtoBuilder/installation.json"));
    }
    Ok(util::data_dir()?.join("installation.json"))
}
pub fn install(system: bool, directory: Option<&Path>, report: &Reporter) -> Result<()> {
    if !cfg!(windows) {
        bail!("[E_PLATFORM] installer currently supports Windows x86_64 only");
    }
    let target = if let Some(dir) = directory {
        util::absolute(&std::env::current_dir()?, dir)
    } else if system {
        PathBuf::from(std::env::var_os("ProgramFiles").context("ProgramFiles not found")?)
            .join("ProtoBuilder/bin")
    } else {
        util::data_dir()?.join("bin")
    };
    let _lock = crate::transaction::Locks::acquire(&[
        util::data_dir()?.join("installation"),
        target.clone(),
    ])?;
    let previous = if record(system)?.is_file() {
        Some(serde_json::from_slice::<Installation>(&fs::read(record(
            system,
        )?)?)?)
    } else {
        None
    };
    if previous
        .as_ref()
        .is_some_and(|p| util::path_key(&p.directory) != util::path_key(&target))
    {
        bail!("[E_INSTALL] another directory is registered; uninstall that installation first");
    }
    util::no_symlinks(&target)?;
    fs::create_dir_all(&target).with_context(|| {
        format!(
            "[E_INSTALL] cannot create {}; system installs require an elevated terminal",
            target.display()
        )
    })?;
    let source = std::env::current_exe()?;
    let executable = target.join("proto-builder.exe");
    let replacing = util::path_key(&source) != util::path_key(&executable);
    let original = if replacing && executable.is_file() {
        Some(fs::read(&executable)?)
    } else {
        None
    };
    if replacing {
        util::atomic_write(&executable, &fs::read(&source)?)?;
    }
    let mut added_now = false;
    let registration = (|| -> Result<bool> {
        added_now = register_path(&target, system, true)?;
        let path_added = added_now || previous.is_some_and(|p| p.path_added);
        util::write_json(
            &record(system)?,
            &Installation {
                directory: target.clone(),
                path_added,
                system,
            },
        )?;
        Ok(path_added)
    })();
    let path_added = match registration {
        Ok(added) => added,
        Err(error) => {
            if added_now {
                register_path(&target, system, false)
                    .context("[E_INSTALL] failed to undo PATH registration")?;
            }
            if replacing {
                if let Some(bytes) = original {
                    util::atomic_write(&executable, &bytes)
                        .context("[E_INSTALL] failed to restore previous executable")?;
                } else {
                    fs::remove_file(&executable)?;
                }
            }
            return Err(error).context("[E_INSTALL] registration failed; executable restored");
        }
    };
    report.info(
        "install",
        format!(
            "{} {}",
            report.text("已安装到", "Installed to"),
            target.display()
        ),
    );
    report.info(
        "PATH",
        report.text(
            "用户 PATH 已注册；请新开终端执行 proto-builder doctor",
            "PATH registered; open a new terminal and run proto-builder doctor",
        ),
    );
    report.result(serde_json::json!({"directory":target,"path_added":path_added,"system":system}));
    Ok(())
}
pub fn uninstall(system: bool, purge: bool, report: &Reporter) -> Result<()> {
    let path = record(system)?;
    if !path.is_file() {
        bail!("[E_INSTALL] installation record not found; portable executables can be deleted directly");
    }
    let installation: Installation = serde_json::from_slice(&fs::read(&path)?)?;
    if installation.system != system {
        bail!("[E_INSTALL] installation scope mismatch; use the matching --system option");
    }
    let exe = installation.directory.join("proto-builder.exe");
    let current = std::env::current_exe()?;
    let _lock = crate::transaction::Locks::acquire(&[
        installation.directory.clone(),
        util::data_dir()?.join("installation"),
    ])?;
    if installation.path_added {
        register_path(&installation.directory, system, false)?;
    }
    if util::path_key(&current) == util::path_key(&exe) {
        self_replace::self_delete().context("schedule native executable deletion")?;
    } else if exe.is_file() {
        fs::remove_file(&exe)?;
    }
    fs::remove_file(path)?;
    if purge {
        // Remove caches and preferences; retain ownership manifests, backups, logs and release keys.
        let base = util::data_dir()?;
        for name in [
            "toolchains",
            "config.toml",
            "config.toml.bak",
            "update-check-stable.json",
            "update-check-beta.json",
            "recent-project.json",
        ] {
            let path = base.join(name);
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else if path.is_file() {
                fs::remove_file(path)?;
            }
        }
    }
    report.info(
        "uninstall",
        report.text(
            "已卸载；项目配置和生成代码保留",
            "Uninstalled; project configuration and generated code retained",
        ),
    );
    Ok(())
}
#[cfg(windows)]
fn register_path(directory: &Path, system: bool, add: bool) -> Result<bool> {
    use winreg::{enums::*, RegKey};
    let hive = RegKey::predef(if system {
        HKEY_LOCAL_MACHINE
    } else {
        HKEY_CURRENT_USER
    });
    let key_name = if system {
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment"
    } else {
        "Environment"
    };
    let key = hive
        .open_subkey_with_flags(key_name, KEY_READ | KEY_WRITE)
        .context("[E_PATH] open Windows environment registry")?;
    let original = match key.get_raw_value("Path") {
        Ok(value) => Some(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).context("[E_PATH] read existing PATH"),
    };
    let existing: String = if original.is_some() {
        key.get_value("Path")
            .context("[E_PATH] decode existing PATH")?
    } else {
        String::new()
    };
    let needle = directory.to_string_lossy().to_string();
    let contains = existing.split(';').any(|x| {
        x.trim_matches('"')
            .trim_end_matches('\\')
            .eq_ignore_ascii_case(needle.trim_end_matches('\\'))
    });
    if add && contains {
        return Ok(false);
    }
    let next = if add {
        if existing.is_empty() {
            needle.clone()
        } else {
            format!("{existing};{needle}")
        }
    } else {
        existing
            .split(';')
            .filter(|x| {
                !x.trim_matches('"')
                    .trim_end_matches('\\')
                    .eq_ignore_ascii_case(needle.trim_end_matches('\\'))
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    let bytes = next
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    key.set_raw_value(
        "Path",
        &winreg::RegValue {
            vtype: original.map(|value| value.vtype).unwrap_or(REG_EXPAND_SZ),
            bytes,
        },
    )?;
    let message = "Environment\0".encode_utf16().collect::<Vec<_>>();
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW(
            windows_sys::Win32::UI::WindowsAndMessaging::HWND_BROADCAST,
            windows_sys::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE,
            0,
            message.as_ptr() as isize,
            windows_sys::Win32::UI::WindowsAndMessaging::SMTO_ABORTIFHUNG,
            1000,
            std::ptr::null_mut(),
        );
    }
    Ok(add)
}
#[cfg(not(windows))]
fn register_path(_: &Path, _: bool, _: bool) -> Result<bool> {
    bail!("Windows PATH registration is not supported on this platform")
}
