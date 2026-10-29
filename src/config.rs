use crate::{
    cli::Options,
    provenance::{self, Origins},
    setup, util,
};
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
    pub developer_name: String,
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
            developer_name: String::new(),
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
#[derive(Debug, Clone)]
pub struct Loaded {
    pub settings: Settings,
    pub root: PathBuf,
    pub sources: Vec<PathBuf>,
    pub project_file: Option<PathBuf>,
    pub origins: Origins,
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
/// Transport-independent explicit configuration overrides.
/// None inherits a value; Some(vec![]) explicitly clears a filter.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigOverrides {
    pub config: Option<PathBuf>,
    pub repo_root: Option<PathBuf>,
    pub proto_path: Option<PathBuf>,
    pub server_path: Option<PathBuf>,
    pub calc_path: Option<PathBuf>,
    pub developer_name: Option<String>,
    pub activity_ids: Option<Vec<String>>,
    pub machine_ids: Option<Vec<String>>,
    pub excludes: Option<Vec<String>>,
    pub assign_id: Option<u64>,
    pub docs: Option<bool>,
    pub protocol_config: Option<bool>,
    pub copy_to_calc: Option<bool>,
    pub svn_update: Option<bool>,
    pub jobs: Option<usize>,
    pub timeout_seconds: Option<u64>,
    pub lang: Option<String>,
}
impl From<&Options> for ConfigOverrides {
    fn from(options: &Options) -> Self {
        fn filter(values: &[String]) -> Option<Vec<String>> {
            (!values.is_empty()).then(|| values.iter().filter(|s| !s.is_empty()).cloned().collect())
        }
        Self {
            config: options.config.clone(),
            repo_root: options.repo_root.clone(),
            proto_path: options.proto_path.clone(),
            server_path: options.server_path.clone(),
            calc_path: options.calc_path.clone(),
            developer_name: options.developer_name.clone(),
            activity_ids: filter(&options.activity_ids),
            machine_ids: filter(&options.machine_ids),
            excludes: filter(&options.excludes),
            assign_id: options.assign_id,
            docs: if options.no_docs {
                Some(false)
            } else {
                options.docs
            },
            protocol_config: if options.no_config {
                Some(false)
            } else {
                options.protocol_config
            },
            copy_to_calc: if options.no_copy {
                Some(false)
            } else {
                options.copy_to_calc
            },
            svn_update: if options.no_svn_update {
                Some(false)
            } else {
                options.svn_update
            },
            jobs: options.jobs,
            timeout_seconds: options.timeout_seconds,
            lang: options.lang.clone(),
        }
    }
}
#[derive(Debug, Clone)]
enum OverrideSource {
    Cli(BTreeMap<&'static str, &'static str>),
    Request,
}
impl OverrideSource {
    fn cli(options: &Options) -> Self {
        let mut names = BTreeMap::new();
        for (field, active, flag) in [
            ("docs", options.no_docs, "--no-docs"),
            ("protocol_config", options.no_config, "--no-config"),
            ("copy_to_calc", options.no_copy, "--no-copy"),
            ("svn.update", options.no_svn_update, "--no-svn-update"),
        ] {
            if active {
                names.insert(field, flag);
            }
        }
        Self::Cli(names)
    }
    fn origin(&self, field: &str, cli_name: &str) -> (&'static str, String) {
        match self {
            Self::Cli(names) => ("cli", names.get(field).copied().unwrap_or(cli_name).into()),
            Self::Request => (
                "request",
                format!(
                    "params.options.{}",
                    if field == "svn.update" {
                        "svn_update"
                    } else {
                        field
                    }
                ),
            ),
        }
    }
}

pub fn load(options: &Options) -> Result<Loaded> {
    load_from(options, &env::current_dir()?, Some(&util::user_config()?))
}
pub fn load_from(options: &Options, cwd: &Path, user: Option<&Path>) -> Result<Loaded> {
    load_with_team(options, cwd, user, &setup::Team::read())
}
fn load_with_team(
    options: &Options,
    cwd: &Path,
    user: Option<&Path>,
    team: &setup::Team,
) -> Result<Loaded> {
    load_config(
        &ConfigOverrides::from(options),
        &OverrideSource::cli(options),
        cwd,
        user,
        team,
    )
}
pub fn load_overrides(options: &ConfigOverrides) -> Result<Loaded> {
    load_config(
        options,
        &OverrideSource::Request,
        &env::current_dir()?,
        Some(&util::user_config()?),
        &setup::Team::read(),
    )
}
fn load_config(
    options: &ConfigOverrides,
    source: &OverrideSource,
    cwd: &Path,
    user: Option<&Path>,
    team: &setup::Team,
) -> Result<Loaded> {
    let profiles = setup::Profiles::read()?;
    let mut value = toml::Value::try_from(Settings::default())?;
    let mut origins = Origins::default();
    origins.layer(&serde_json::to_value(&value)?, "default", "built-in");
    let mut sources = vec![];
    if let Some(user) = user.filter(|p| p.is_file()) {
        merge_file(&mut value, user, &mut origins, "user_config")?;
        sources.push(user.to_owned());
    }
    let search = options
        .repo_root
        .as_deref()
        .map(|p| util::absolute(cwd, p))
        .or_else(|| {
            env::var_os("PROTO_BUILDER_PROJECT_ROOT").map(|p| util::absolute(cwd, Path::new(&p)))
        })
        .or_else(|| team.root.clone())
        .or_else(|| {
            cwd.ancestors()
                .find(|p| p.join("proto").is_dir() || p.join("proto-builder.toml").is_file())
                .map(Path::to_path_buf)
        })
        .or_else(|| profiles.last_root.clone().filter(|p| p.is_dir()))
        .unwrap_or_else(|| cwd.to_owned());
    let explicit = options.config.as_ref().map(|p| util::absolute(cwd, p));
    let project_file = explicit.clone().or_else(|| {
        search
            .ancestors()
            .map(|p| p.join("proto-builder.toml"))
            .find(|p| p.is_file())
    });
    if let Some(path) = &project_file {
        merge_file(
            &mut value,
            path,
            &mut origins,
            if explicit.is_some() {
                "explicit_config"
            } else {
                "project_config"
            },
        )?;
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
    let env_root =
        env::var_os("PROTO_BUILDER_PROJECT_ROOT").map(|p| util::absolute(cwd, Path::new(&p)));
    let root_source = if options.repo_root.is_some() {
        let (kind, name) = source.origin("repo_root", "--repo-root");
        (kind.to_owned(), name)
    } else if env_root.is_some() {
        ("environment".into(), "PROTO_BUILDER_PROJECT_ROOT".into())
    } else if options.config.is_none() && team.root.is_some() {
        ("environment".into(), "ENV_PROTOBUF".into())
    } else if config_root.is_some() {
        let origin = origins.0.get("project_root").and_then(|items| items.last());
        if let Some(origin) = origin.filter(|o| o.kind != "default") {
            (origin.kind.clone(), origin.name.clone())
        } else if let Some(file) = &project_file {
            (
                "project_config_directory".into(),
                file.display().to_string(),
            )
        } else {
            ("working_directory".into(), cwd.display().to_string())
        }
    } else if options.repo_root.is_none()
        && team.root.is_none()
        && !cwd
            .ancestors()
            .any(|p| p.join("proto").is_dir() || p.join("proto-builder.toml").is_file())
        && profiles
            .last_root
            .as_ref()
            .is_some_and(|r| r.is_dir() && util::path_key(r) == util::path_key(&search))
    {
        (
            "local_setup".into(),
            util::data_dir()?
                .join("local-setup.json")
                .display()
                .to_string(),
        )
    } else {
        ("working_directory".into(), cwd.display().to_string())
    };
    let root = options
        .repo_root
        .as_deref()
        .map(|p| util::absolute(cwd, p))
        .or(env_root)
        .or_else(|| {
            if options.config.is_none() {
                team.root.clone()
            } else {
                None
            }
        })
        .or(config_root)
        .or_else(|| {
            search
                .ancestors()
                .find(|p| p.join("proto").is_dir())
                .map(Path::to_path_buf)
        })
        .unwrap_or(search);
    origins.record(
        "project_root",
        serde_json::to_value(&root)?,
        &root_source.0,
        &root_source.1,
    );
    if options.config.is_none() {
        if let Some(saved) = profiles.projects.get(&util::path_key(&root)) {
            settings.proto_path = saved.proto.clone();
            settings.server_path = saved.server.clone();
            settings.calc_path = saved.calc.clone();
            settings.developer_name = saved.developer_name.clone();
            let name = util::data_dir()?
                .join("local-setup.json")
                .display()
                .to_string();
            for (key, value) in [
                ("proto_path", serde_json::to_value(&saved.proto)?),
                ("server_path", serde_json::to_value(&saved.server)?),
                ("calc_path", serde_json::to_value(&saved.calc)?),
                (
                    "developer_name",
                    serde_json::to_value(&saved.developer_name)?,
                ),
            ] {
                origins.record(key, value, "local_setup", &name);
            }
        }
    }
    if let Some(v) = &team.server {
        settings.server_path = v.clone();
        let name = if team.environment.contains_key("ENV_PROJECT") {
            "ENV_PROJECT"
        } else {
            "ENV_CENTER_PROJECT"
        };
        origins.record("server_path", serde_json::to_value(v)?, "environment", name);
    }
    if let Some(v) = &team.calc {
        settings.calc_path = v.clone();
        origins.record(
            "calc_path",
            serde_json::to_value(v)?,
            "environment",
            "ENV_CAL_CENTER_PROJECT",
        );
    }
    if let Some(v) = &team.developer {
        settings.developer_name = v.clone();
        origins.record(
            "developer_name",
            serde_json::to_value(v)?,
            "environment",
            "ENV_DEVELOPER_NAME",
        );
    }
    if let Some(v) = &team.proto {
        if team
            .root
            .as_ref()
            .is_some_and(|r| util::path_key(&root) == util::path_key(r))
            && options.config.is_none()
        {
            settings.proto_path = v.clone();
            origins.record(
                "proto_path",
                serde_json::to_value(v)?,
                "environment",
                "ENV_PROTOBUF",
            );
        }
    }

    apply_env(&mut settings, &mut origins)?;
    if let Some(v) = &options.proto_path {
        settings.proto_path = v.clone();
    }
    if let Some(v) = &options.server_path {
        settings.server_path = v.clone();
    }
    if let Some(v) = &options.calc_path {
        settings.calc_path = v.clone();
    }
    if let Some(v) = &options.developer_name {
        settings.developer_name = v.trim().to_owned();
    }
    if let Some(v) = options.assign_id {
        settings.assign_id = v;
    }
    if let Some(values) = &options.activity_ids {
        settings.activity_ids = values.clone();
    }
    if let Some(values) = &options.machine_ids {
        settings.machine_ids = values.clone();
    }
    if let Some(values) = &options.excludes {
        settings.excludes = values.clone();
    }
    if let Some(v) = options.jobs {
        settings.jobs = v;
    }
    if let Some(v) = options.timeout_seconds {
        settings.timeout_seconds = v;
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
    validate(&settings)?;
    let final_values = provenance::flatten(&serde_json::to_value(&settings)?);
    for (key, present, cli_name) in [
        ("proto_path", options.proto_path.is_some(), "--proto-path"),
        (
            "server_path",
            options.server_path.is_some(),
            "--server-path",
        ),
        ("calc_path", options.calc_path.is_some(), "--calc-path"),
        (
            "developer_name",
            options.developer_name.is_some(),
            "--developer-name",
        ),
        ("assign_id", options.assign_id.is_some(), "--assign-id"),
        (
            "activity_ids",
            options.activity_ids.is_some(),
            "--activity-id",
        ),
        ("machine_ids", options.machine_ids.is_some(), "--machine-id"),
        ("excludes", options.excludes.is_some(), "--exclude"),
        ("jobs", options.jobs.is_some(), "--jobs"),
        (
            "timeout_seconds",
            options.timeout_seconds.is_some(),
            "--timeout-seconds",
        ),
        ("lang", options.lang.is_some(), "--lang"),
        ("docs", options.docs.is_some(), "--docs"),
        (
            "protocol_config",
            options.protocol_config.is_some(),
            "--protocol-config",
        ),
        (
            "copy_to_calc",
            options.copy_to_calc.is_some(),
            "--copy-to-calc",
        ),
        ("svn.update", options.svn_update.is_some(), "--svn-update"),
    ] {
        if present {
            let (kind, name) = source.origin(key, cli_name);
            origins.record(key, final_values[key].clone(), kind, &name);
        }
    }
    Ok(Loaded {
        settings,
        root,
        sources,
        project_file,
        origins,
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
fn merge_file(
    value: &mut toml::Value,
    path: &Path,
    origins: &mut Origins,
    kind: &str,
) -> Result<()> {
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
    origins.layer(
        &serde_json::to_value(&patch)?,
        kind,
        &path.display().to_string(),
    );
    merge(value, patch);
    Ok(())
}
fn apply_env(s: &mut Settings, origins: &mut Origins) -> Result<()> {
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
        s.jobs = v
            .parse()
            .context("[E_CONFIG] PROTO_BUILDER_JOBS must be an integer")?;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_LANG") {
        s.lang = v;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_SVN_UPDATE") {
        s.svn.update = v
            .parse()
            .context("[E_CONFIG] PROTO_BUILDER_SVN_UPDATE must be true/false")?;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_TOOLCHAIN_MODE") {
        s.toolchain.mode = v;
    }
    if let Ok(v) = env::var("PROTO_BUILDER_UPDATE_CHANNEL") {
        s.update.channel = v;
    }
    let values = provenance::flatten(&serde_json::to_value(&*s)?);
    for (name, key) in [
        ("PROTO_PATH", "proto_path"),
        ("SERVER_PATH", "server_path"),
        ("CALC_PATH", "calc_path"),
        ("ACTIVITY_IDS", "activity_ids"),
        ("MACHINE_IDS", "machine_ids"),
        ("EXCLUDES", "excludes"),
        ("JOBS", "jobs"),
        ("LANG", "lang"),
        ("SVN_UPDATE", "svn.update"),
        ("TOOLCHAIN_MODE", "toolchain.mode"),
        ("UPDATE_CHANNEL", "update.channel"),
    ] {
        let name = format!("PROTO_BUILDER_{name}");
        let present = if key.ends_with("_path") {
            env::var_os(&name).is_some()
        } else {
            env::var(&name).is_ok()
        };
        if present {
            origins.record(key, values[key].clone(), "environment", &name);
        }
    }
    Ok(())
}

pub fn explain(loaded: &Loaded) -> Result<serde_json::Value> {
    let mut fields = serde_json::Map::new();
    for (key, value) in provenance::flatten(&serde_json::to_value(&loaded.settings)?) {
        let history = loaded.origins.0.get(&key).cloned().unwrap_or_default();
        let mut field =
            serde_json::json!({"value":value,"source":history.last(),"history":history});
        let resolved = match key.as_str() {
            "project_root" => Some(loaded.root.clone()),
            "proto_path" => Some(loaded.settings.proto(&loaded.root)),
            "server_path" => Some(loaded.settings.server(&loaded.root)),
            "calc_path" => Some(loaded.settings.calc(&loaded.root)),
            _ => None,
        };
        if let Some(path) = resolved {
            field["resolved_path"] = serde_json::to_value(&path)?;
            field["directory_exists"] = path.is_dir().into();
        }
        fields.insert(key, field);
    }
    Ok(serde_json::json!({"fields":fields,"team_environment":setup::status(loaded)?}))
}
pub fn validate(s: &Settings) -> Result<()> {
    if s.developer_name.chars().any(char::is_control) {
        bail!("[E_CONFIG] developer_name must not contain control characters");
    }
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
            timeout_seconds: Some(180),
            ..Default::default()
        };
        let loaded = load_with_team(&opts, root, None, &setup::Team::default()).unwrap();
        assert_eq!(loaded.settings.proto_path, PathBuf::from("proto"));
        assert!(!loaded.settings.svn.update);
        assert_eq!(loaded.settings.timeout_seconds, 180);
        assert_eq!(loaded.root, root);
    }
    #[test]
    fn native_overrides_clear_filters_and_record_request_provenance() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        fs::write(&file, "config_version=1\nactivity_ids=['7001']\nmachine_ids=['5001']\nexcludes=['draft']\ndocs=true\n[svn]\nupdate=true\n").unwrap();
        let overrides = ConfigOverrides {
            config: Some(file),
            repo_root: Some(dir.path().into()),
            activity_ids: Some(vec![]),
            machine_ids: Some(vec![]),
            excludes: Some(vec![]),
            docs: Some(false),
            svn_update: Some(false),
            ..Default::default()
        };
        let loaded = load_config(
            &overrides,
            &OverrideSource::Request,
            dir.path(),
            None,
            &setup::Team::default(),
        )
        .unwrap();
        assert!(loaded.settings.activity_ids.is_empty());
        assert!(loaded.settings.machine_ids.is_empty());
        assert!(loaded.settings.excludes.is_empty());
        assert!(!loaded.settings.docs);
        assert!(!loaded.settings.svn.update);
        for (key, field) in [
            ("project_root", "repo_root"),
            ("activity_ids", "activity_ids"),
            ("machine_ids", "machine_ids"),
            ("excludes", "excludes"),
            ("docs", "docs"),
            ("svn.update", "svn_update"),
        ] {
            let origin = loaded.origins.0[key].last().unwrap();
            assert_eq!(origin.kind, "request");
            assert_eq!(origin.name, format!("params.options.{field}"));
        }
        assert_eq!(
            loaded.origins.0["activity_ids"].last().unwrap().value,
            serde_json::json!([])
        );
    }
    #[test]
    fn cli_negative_flags_and_explicit_empty_filters_keep_their_sources() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        fs::write(
            &file,
            "config_version=1\nactivity_ids=['7001']\nexcludes=['draft']\n",
        )
        .unwrap();
        let options = Options {
            config: Some(file),
            repo_root: Some(dir.path().into()),
            activity_ids: vec![String::new()],
            excludes: vec![String::new()],
            docs: Some(true),
            protocol_config: Some(true),
            copy_to_calc: Some(true),
            svn_update: Some(true),
            no_docs: true,
            no_config: true,
            no_copy: true,
            no_svn_update: true,
            ..Default::default()
        };
        let loaded = load_with_team(&options, dir.path(), None, &setup::Team::default()).unwrap();
        assert!(loaded.settings.activity_ids.is_empty());
        assert!(loaded.settings.excludes.is_empty());
        for (key, flag) in [
            ("docs", "--no-docs"),
            ("protocol_config", "--no-config"),
            ("copy_to_calc", "--no-copy"),
            ("svn.update", "--no-svn-update"),
        ] {
            let origin = loaded.origins.0[key].last().unwrap();
            assert_eq!(origin.kind, "cli");
            assert_eq!(origin.name, flag);
            assert_eq!(origin.value, false);
        }
    }
    #[test]
    fn unknown_keys_and_future_versions_are_errors() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("proto-builder.toml");
        for text in ["config_version=999", "prot_path='oops'"] {
            fs::write(&file, text).unwrap();
            assert!(load_with_team(
                &Options::default(),
                dir.path(),
                None,
                &setup::Team::default()
            )
            .is_err());
        }
    }
}
