use crate::{config::Loaded, util};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, fs, path::PathBuf};

pub const TEAM_VARIABLES: [&str; 4] = [
    "ENV_CENTER_PROJECT",
    "ENV_CAL_CENTER_PROJECT",
    "ENV_PROTOBUF",
    "ENV_DEVELOPER_NAME",
];

#[derive(Default)]
pub struct Team {
    pub server: Option<PathBuf>,
    pub calc: Option<PathBuf>,
    pub root: Option<PathBuf>,
    pub proto: Option<PathBuf>,
    pub developer: Option<String>,
    pub detected: Vec<String>,
    pub issues: Vec<String>,
    pub invalid: bool,
}
impl Team {
    pub fn read() -> Self {
        Self::from_values(&TEAM_VARIABLES.map(|key| env::var(key).ok()))
    }
    pub fn from_values(values: &[Option<String>; 4]) -> Self {
        let mut team = Self::default();
        for (index, key) in TEAM_VARIABLES.iter().enumerate() {
            let Some(value) = values[index]
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
            else {
                team.issues.push(format!("{key} 未设置 / not set"));
                continue;
            };
            if index == 3 {
                if value.chars().any(char::is_control) {
                    team.invalid = true;
                    team.issues.push(format!(
                        "{key}: 名字包含控制字符，已忽略 / control characters in name; ignored"
                    ));
                    continue;
                }
                team.developer = Some(value.into());
                team.detected.push((*key).into());
                continue;
            }
            let path = PathBuf::from(value);
            if !path.is_absolute() || !path.is_dir() {
                team.invalid = true;
                team.issues.push(format!("{key}: 必须是存在的绝对目录，已忽略 / expected an existing absolute directory; ignored: {value}"));
                continue;
            }
            match index {
                0 => team.server = Some(path),
                1 => team.calc = Some(path),
                _ => {
                    if path.join("proto").is_dir() {
                        team.proto = Some(path.join("proto"));
                        team.root = Some(path);
                    } else if path.file_name().is_some_and(|n| n == "proto") {
                        team.root = path.parent().map(PathBuf::from);
                        team.proto = Some(path);
                    } else {
                        team.invalid = true;
                        team.issues.push(format!("{key}: 未找到 proto 子目录，已忽略 / proto subdirectory not found; ignored: {value}"));
                        continue;
                    }
                }
            }
            team.detected.push((*key).into());
        }
        team
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub root: PathBuf,
    pub proto: PathBuf,
    pub server: PathBuf,
    pub calc: PathBuf,
    pub developer_name: String,
}
impl Profile {
    pub fn current(loaded: &Loaded) -> Self {
        Self {
            root: loaded.root.clone(),
            proto: loaded.settings.proto(&loaded.root),
            server: loaded.settings.server(&loaded.root),
            calc: loaded.settings.calc(&loaded.root),
            developer_name: loaded.settings.developer_name.clone(),
        }
    }
    fn matches(&self, other: &Self) -> bool {
        [&self.root, &self.proto, &self.server, &self.calc]
            .iter()
            .zip([&other.root, &other.proto, &other.server, &other.calc])
            .all(|(a, b)| util::path_key(a) == util::path_key(b))
            && self.developer_name == other.developer_name
    }
}
#[derive(Default, Serialize, Deserialize)]
pub struct Profiles {
    pub last_root: Option<PathBuf>,
    pub projects: BTreeMap<String, Profile>,
}
impl Profiles {
    pub fn read() -> Result<Self> {
        let path = util::data_dir()?.join("local-setup.json");
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .with_context(|| format!("[E_CONFIG] read saved local setup: {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("[E_CONFIG] read {}", path.display())),
        }
    }
}
#[derive(Serialize)]
pub struct Status {
    pub needs_confirmation: bool,
    pub saved: bool,
    pub detected: Vec<String>,
    pub issues: Vec<String>,
    pub paths: Profile,
}
pub fn status(loaded: &Loaded) -> Result<Status> {
    let mut team = Team::read();
    let paths = Profile::current(loaded);
    let saved = Profiles::read()?
        .projects
        .get(&util::path_key(&loaded.root))
        .is_some_and(|p| p.matches(&paths));
    if util::path_key(&paths.server) == util::path_key(&paths.calc) {
        team.invalid = true;
        team.issues.push("业务服与计算服目录相同，请核对 ENV_CAL_CENTER_PROJECT / Server and calculation server directories are identical".into());
    }
    Ok(Status {
        needs_confirmation: team.invalid || (!saved && !team.issues.is_empty()),
        saved,
        detected: team.detected,
        issues: team.issues,
        paths,
    })
}
pub fn remember(loaded: &Loaded) -> Result<()> {
    let path = util::data_dir()?.join("local-setup.json");
    // Serialize updates across independent projects, which may build concurrently.
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("lock"))?;
    fs2::FileExt::lock_exclusive(&lock)?;
    let mut profiles = Profiles::read()?;
    profiles.last_root = Some(loaded.root.clone());
    profiles
        .projects
        .insert(util::path_key(&loaded.root), Profile::current(loaded));
    util::write_json(&path, &profiles)
}

pub fn notify(loaded: &Loaded, report: &crate::report::Reporter) -> Result<()> {
    let setup = status(loaded)?;
    if setup.needs_confirmation {
        report.emit("setup", "warning", format!(
            "请确认项目路径，首次编译成功后将记住本机设置 / Confirm paths; remembered after a successful build.\n{}\nproto={}\nserver={}\ncalc={}\ndeveloper={}",
            setup.issues.join("; "), setup.paths.proto.display(), setup.paths.server.display(), setup.paths.calc.display(), setup.paths.developer_name
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn team_paths_names_and_proto_root_are_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for name in ["proto", "server", "calc"] {
            fs::create_dir(root.join(name)).unwrap();
        }
        let mut values = [
            Some(root.join("server").display().to_string()),
            Some(root.join("calc").display().to_string()),
            Some(root.display().to_string()),
            Some(" 董荣旺 ".into()),
        ];
        let team = Team::from_values(&values);
        assert!(team.issues.is_empty());
        assert_eq!(team.detected.len(), 4);
        assert_eq!(team.root.as_deref(), Some(root));
        assert_eq!(team.proto, Some(root.join("proto")));
        assert_eq!(team.developer.as_deref(), Some("董荣旺"));
        values[2] = Some(root.join("proto").display().to_string());
        assert_eq!(Team::from_values(&values).root.as_deref(), Some(root));
    }
    #[test]
    fn missing_empty_relative_and_invalid_team_values_remain_actionable() {
        let dir = tempfile::tempdir().unwrap();
        let values = [
            Some("relative/server".into()),
            Some(dir.path().join("missing").display().to_string()),
            Some("  ".into()),
            Some("bad\nname".into()),
        ];
        let team = Team::from_values(&values);
        assert_eq!(team.issues.len(), 4);
        assert!(team.detected.is_empty());
        assert!(
            team.root.is_none()
                && team.server.is_none()
                && team.calc.is_none()
                && team.developer.is_none()
        );
    }
}
