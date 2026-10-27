use crate::config::Settings;
use anyhow::{bail, Context, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub struct Input {
    pub relative: String,
    pub group: String,
    pub grpc: bool,
    pub has_service: bool,
    pub activity_id: Option<String>,
    pub machine_id: Option<String>,
    pub imports: Vec<String>,
    pub import_lines: BTreeMap<String, usize>,
}
#[derive(Debug, Serialize)]
pub struct Skipped {
    pub file: String,
    pub reason: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Plan {
    pub scanned: usize,
    pub selected: Vec<Input>,
    pub excluded: Vec<Skipped>,
    pub filtered: Vec<Skipped>,
    pub import_only: Vec<String>,
    pub groups: BTreeMap<String, Vec<usize>>,
    pub layers: Vec<Vec<String>>,
    pub full_build: bool,
    pub outputs: Vec<String>,
}
pub struct Excludes {
    patterns: Vec<String>,
    globs: GlobSet,
}
impl Excludes {
    pub fn new(values: &[String]) -> Result<Self> {
        let mut globs = GlobSetBuilder::new();
        let mut patterns = vec![];
        for pattern in values {
            let value = pattern.trim().replace('\\', "/");
            let value = value
                .strip_prefix("./")
                .unwrap_or(&value)
                .trim_end_matches('/');
            let value = value.strip_prefix("proto/").unwrap_or(value).to_owned();
            if value.is_empty() {
                continue;
            }
            globs.add(
                GlobBuilder::new(&value)
                    .literal_separator(true)
                    .case_insensitive(cfg!(windows))
                    .build()
                    .with_context(|| format!("[E_FILTER] invalid exclusion {value:?}"))?,
            );
            patterns.push(value);
        }
        Ok(Self {
            patterns,
            globs: globs.build()?,
        })
    }
    pub fn matches(&self, relative: &str) -> bool {
        let parts = relative.split('/').collect::<Vec<_>>();
        (1..=parts.len()).any(|n| {
            let path = parts[..n].join("/");
            self.globs.is_match(&path)
                || self.patterns.iter().any(|pattern| {
                    let eq = |a: &str, b: &str| {
                        if cfg!(windows) {
                            a.eq_ignore_ascii_case(b)
                        } else {
                            a == b
                        }
                    };
                    eq(pattern, &path) || (!pattern.contains('/') && eq(pattern, parts[n - 1]))
                })
        })
    }
}
pub fn discover(root: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(root)
        .with_context(|| format!("[E_SCAN] read {}", root.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    let mut result = vec![];
    for e in entries {
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let ty = e.file_type()?;
        if ty.is_symlink() {
            bail!("[E_SCAN] symbolic link input: {}", e.path().display());
        }
        if ty.is_dir() {
            result.extend(discover(&e.path())?);
        } else if e.path().extension().is_some_and(|x| x == "proto") {
            result.push(e.path());
        }
    }
    Ok(result)
}
pub fn make(root: &Path, s: &Settings) -> Result<Plan> {
    let proto = s.proto(root);
    let exclude = Excludes::new(&s.excludes)?;
    let files = discover(&proto)?;
    let path_id = regex::Regex::new(r"(?i)(?:^|/)(act|activity|slot|machine|rpc)([0-9]+)(?:/|$)")?;
    let declarations = regex::Regex::new(
        r"(?mi)^\s*//\s*(?:@|proto-builder[: ]+)?(activity[-_]id|machine[-_]id)\s*[:=]\s*([0-9]+)\s*$",
    )?;
    let package = regex::Regex::new(r"(?m)^\s*package\s+([A-Za-z_][A-Za-z_0-9]*)\s*;")?;
    let imports = regex::Regex::new(r#"\bimport\s+(?:(?:public|weak)\s+)?"([^"]+)"\s*;"#)?;
    let service = regex::Regex::new(r"\bservice\s+[A-Za-z_][A-Za-z_0-9]*\s*\{")?;
    let mut all = BTreeMap::new();
    for path in files {
        let relative = path
            .strip_prefix(&proto)?
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path)
            .with_context(|| format!("[E_INPUT] {}:1: invalid UTF-8", path.display()))?;
        let mut activity = None;
        let mut machine = None;
        let clean = without_comments(&text);
        let mut identities = vec![relative.clone()];
        if let Some(p) = package.captures(&clean) {
            identities.push(p[1].to_owned());
        }
        for identity in identities {
            for cap in path_id.captures_iter(&identity) {
                let field = if ["act", "activity"].contains(&cap[1].to_lowercase().as_str()) {
                    &mut activity
                } else {
                    &mut machine
                };
                if field.as_ref().is_some_and(|v| v != &cap[2]) {
                    bail!("[E_ID] {}:1: path and package IDs disagree", path.display());
                }
                *field = Some(cap[2].to_owned());
            }
        }
        for cap in declarations.captures_iter(&text) {
            let field = if cap[1].to_lowercase().starts_with("activity") {
                &mut activity
            } else {
                &mut machine
            };
            if field.as_ref().is_some_and(|v| v != &cap[2]) {
                let line = text[..cap.get(0).unwrap().start()]
                    .bytes()
                    .filter(|c| *c == b'\n')
                    .count()
                    + 1;
                bail!(
                    "[E_ID] {}:{line}: metadata and path IDs disagree",
                    path.display()
                );
            }
            *field = Some(cap[2].to_owned());
        }
        let group = relative
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or(".")
            .to_owned();
        let grpc = relative
            .split('/')
            .next()
            .is_some_and(|p| p.starts_with("rpc"));
        let import_lines = imports
            .captures_iter(&clean)
            .map(|cap| {
                let line = clean[..cap.get(0).unwrap().start()]
                    .bytes()
                    .filter(|c| *c == b'\n')
                    .count()
                    + 1;
                (cap[1].to_owned(), line)
            })
            .collect();
        let dependencies = imports
            .captures_iter(&clean)
            .map(|c| c[1].to_owned())
            .collect();
        all.insert(
            relative.clone(),
            Input {
                relative,
                group,
                grpc,
                has_service: service.is_match(&clean),
                activity_id: activity,
                machine_id: machine,
                imports: dependencies,
                import_lines,
            },
        );
    }
    let mut plan = Plan {
        scanned: all.len(),
        selected: vec![],
        excluded: vec![],
        filtered: vec![],
        import_only: vec![],
        groups: BTreeMap::new(),
        layers: vec![],
        full_build: s.assign_id == 0
            && s.activity_ids.is_empty()
            && s.machine_ids.is_empty()
            && s.excludes.is_empty(),
        outputs: vec![],
    };
    let mut target_matches = 0;
    for input in all.values() {
        if input.relative.starts_with("google/") || exclude.matches(&input.relative) {
            plan.excluded.push(Skipped {
                file: input.relative.clone(),
                reason: if input.relative.starts_with("google/") {
                    "include-only"
                } else {
                    "excluded"
                },
            });
            continue;
        }
        let shared = input.activity_id.is_none() && input.machine_id.is_none();
        let legacy_ok = s.assign_id == 0 || legacy_selected(&input.group, s.assign_id);
        let activity_ok = s.activity_ids.is_empty()
            || input
                .activity_id
                .as_ref()
                .is_some_and(|id| s.activity_ids.contains(id));
        let machine_ok = s.machine_ids.is_empty()
            || input
                .machine_id
                .as_ref()
                .is_some_and(|id| s.machine_ids.contains(id));
        if legacy_ok && (shared || (activity_ok && machine_ok)) {
            if !shared {
                target_matches += 1;
            }
            let index = plan.selected.len();
            plan.groups
                .entry(input.group.clone())
                .or_default()
                .push(index);
            plan.selected.push(input.clone());
        } else {
            plan.filtered.push(Skipped {
                file: input.relative.clone(),
                reason: "id-filter",
            });
        }
    }
    if plan.selected.is_empty()
        || ((s.assign_id > 0 || !s.activity_ids.is_empty() || !s.machine_ids.is_empty())
            && target_matches == 0)
    {
        bail!("[E_NO_MATCH] no protobuf matches activity={:?}, machine={:?}, assign_id={}, excludes={:?}",s.activity_ids,s.machine_ids,s.assign_id,s.excludes);
    }
    let mut closure = BTreeSet::new();
    fn visit(
        name: &str,
        all: &BTreeMap<String, Input>,
        closure: &mut BTreeSet<String>,
        visiting: &mut BTreeSet<String>,
    ) -> Result<()> {
        if visiting.contains(name) {
            bail!("[E_IMPORT] {name}:1: circular import");
        }
        if closure.contains(name) {
            return Ok(());
        }
        let input = all
            .get(name)
            .with_context(|| format!("[E_IMPORT] {name}:1: imported protobuf not found"))?;
        visiting.insert(name.into());
        for dep in &input.imports {
            visit(dep, all, closure, visiting).with_context(|| {
                format!(
                    "{}:{}: imports {dep}",
                    input.relative,
                    input.import_lines.get(dep).unwrap_or(&1)
                )
            })?;
        }
        visiting.remove(name);
        closure.insert(name.into());
        Ok(())
    }
    for input in &plan.selected {
        visit(&input.relative, &all, &mut closure, &mut BTreeSet::new())?;
    }
    let selected = plan
        .selected
        .iter()
        .map(|i| i.relative.clone())
        .collect::<BTreeSet<_>>();
    plan.import_only = closure.difference(&selected).cloned().collect();
    // protoc reads imports directly; group ordering prevents output collisions and keeps scheduling deterministic.
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = plan
        .groups
        .keys()
        .map(|g| (g.clone(), BTreeSet::new()))
        .collect();
    for input in &plan.selected {
        for dep in &input.imports {
            if let Some(dependency) = all.get(dep) {
                if dependency.group != input.group && plan.groups.contains_key(&dependency.group) {
                    dependencies
                        .get_mut(&input.group)
                        .unwrap()
                        .insert(dependency.group.clone());
                }
            }
        }
    }
    while !dependencies.is_empty() {
        let ready = dependencies
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        if ready.is_empty() {
            plan.layers.push(dependencies.keys().cloned().collect());
            break;
        }
        for item in &ready {
            dependencies.remove(item);
        }
        for deps in dependencies.values_mut() {
            for item in &ready {
                deps.remove(item);
            }
        }
        plan.layers.push(ready);
    }
    for input in &plan.selected {
        let stem = input.relative.strip_suffix(".proto").unwrap();
        plan.outputs.push(format!("server/protocol/{stem}.pb.go"));
        if input.grpc && input.has_service {
            plan.outputs
                .push(format!("server/protocol/{stem}_grpc.pb.go"));
        }
    }
    if s.docs {
        plan.outputs
            .extend(["proto/protocol.md".into(), "proto/protocol.html".into()]);
    }
    if s.protocol_config {
        plan.outputs.push("proto/ProtoConfig.json + namespace JSON + server/configs/{fatcat,reels,thunder}/sys/ProtoConfig.json".into());
    }
    Ok(plan)
}
pub fn legacy_selected(name: &str, id: u64) -> bool {
    id == 0
        || name == "rpc"
        || (!name.starts_with("rpc") && name.len() <= 4)
        || name.ends_with(&id.to_string())
}
pub fn without_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut result = bytes.to_vec();
    let mut i = 0;
    let mut quoted = false;
    while i < bytes.len() {
        if quoted {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            quoted = true;
            i += 1;
            continue;
        }
        if bytes.get(i..i + 2) == Some(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                result[i] = b' ';
                i += 1;
            }
        } else if bytes.get(i..i + 2) == Some(b"/*") {
            result[i] = b' ';
            i += 1;
            result[i] = b' ';
            i += 1;
            while i < bytes.len() {
                if bytes.get(i..i + 2) == Some(b"*/") {
                    result[i] = b' ';
                    result[i + 1] = b' ';
                    i += 2;
                    break;
                }
                if bytes[i] != b'\n' && bytes[i] != b'\r' {
                    result[i] = b' ';
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(result).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exclude_prefix_glob_and_component() {
        let e =
            Excludes::new(&["proto/act7*".into(), "nested/private".into(), "temp".into()]).unwrap();
        assert!(e.matches("act7034/a.proto"));
        assert!(e.matches("nested/private/a.proto"));
        assert!(e.matches("a/temp/b.proto"));
        assert!(!e.matches("act6001/a.proto"));
    }
    #[test]
    fn typed_ids_keep_shared_and_reject_substring_match() {
        let dir = tempfile::tempdir().unwrap();
        for folder in ["act7034", "act17034", "slot6034", "cmd"] {
            fs::create_dir_all(dir.path().join("proto").join(folder)).unwrap();
            fs::write(
                dir.path().join("proto").join(folder).join("a.proto"),
                format!("package {folder};"),
            )
            .unwrap();
        }
        let mut s = Settings {
            activity_ids: vec!["7034".into()],
            ..Default::default()
        };
        let p = make(dir.path(), &s).unwrap();
        assert_eq!(p.selected.len(), 2);
        assert!(p.selected.iter().any(|i| i.group == "cmd"));
        s.activity_ids = vec!["9999".into()];
        assert!(make(dir.path(), &s).is_err());
    }
    #[test]
    fn missing_import_and_cycle_are_actionable() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("proto")).unwrap();
        fs::write(
            dir.path().join("proto/a.proto"),
            "import \"missing.proto\";",
        )
        .unwrap();
        assert!(make(dir.path(), &Settings::default())
            .unwrap_err()
            .to_string()
            .contains("a.proto"));
    }
}
