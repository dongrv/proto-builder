use crate::{
    agent::{Conflict, OutputConflict},
    report::Reporter,
    util,
};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub struct Locks {
    _files: Vec<fs::File>,
}
impl Locks {
    pub fn acquire(paths: &[PathBuf]) -> Result<Self> {
        let base = util::data_dir()?.join("locks");
        fs::create_dir_all(&base)?;
        let identities = paths
            .iter()
            .map(|p| util::path_key(p))
            .collect::<BTreeSet<_>>();
        let mut files = vec![];
        for identity in identities {
            let path = base.join(format!("{}.lock", util::hash(identity.as_bytes())));
            let file = fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)?;
            file.try_lock_exclusive().with_context(|| {
                format!("[E_LOCKED] another build is using {identity}; retry after it finishes")
            })?;
            files.push(file);
        }
        Ok(Self { _files: files })
    }
}
#[derive(Clone, Debug)]
pub struct Output {
    pub source: PathBuf,
    pub target: PathBuf,
    pub scope: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Owned {
    pub target: PathBuf,
    pub sha256: String,
    pub scope: String,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub files: BTreeMap<String, Owned>,
}
#[derive(Debug, Serialize, Default)]
pub struct Changes {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
    pub adopted: Vec<PathBuf>,
    pub backup: Option<PathBuf>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    phase: String,
    operations: Vec<Operation>,
    backup_dir: PathBuf,
}
#[derive(Debug, Serialize, Deserialize)]
struct Operation {
    target: PathBuf,
    backup: Option<PathBuf>,
}
pub fn read_manifest(state: &Path) -> Result<Manifest> {
    let path = state.join("manifest.json");
    if !path.is_file() {
        return Ok(Manifest {
            schema_version: 1,
            ..Default::default()
        });
    }
    let manifest: Manifest = serde_json::from_slice(&fs::read(&path)?)
        .with_context(|| format!("[E_MANIFEST] {}", path.display()))?;
    if manifest.schema_version != 1 {
        bail!("[E_MANIFEST] unsupported schema_version");
    }
    for (key, item) in &manifest.files {
        if !item.target.is_absolute() || *key != util::path_key(&item.target) {
            bail!("[E_MANIFEST] invalid output identity: {key}");
        }
    }
    Ok(manifest)
}
pub fn recover(state: &Path, report: &Reporter) -> Result<()> {
    let path = state.join("journal.json");
    if !path.is_file() {
        return Ok(());
    }
    let journal: Journal = serde_json::from_slice(&fs::read(&path)?)
        .context("[E_RECOVERY] invalid transaction journal")?;
    if journal.phase == "prepared" {
        report.outputs("recovering", None);
        report.emit(
            "recovery",
            "warning",
            report.text(
                "恢复上次中断的文件写回",
                "Recovering interrupted output transaction",
            ),
        );
        rollback(&journal)?;
        report.outputs("recovered", None);
    } else if journal.phase != "committed" {
        bail!("[E_RECOVERY] invalid journal phase");
    }
    fs::remove_file(path)?;
    Ok(())
}
fn rollback(journal: &Journal) -> Result<()> {
    let mut failures = vec![];
    for op in journal.operations.iter().rev() {
        let result = (|| -> Result<()> {
            if let Some(backup) = &op.backup {
                util::atomic_write(&op.target, &fs::read(backup)?)?;
            } else if op.target.exists() {
                fs::remove_file(&op.target)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            failures.push(format!("{}: {error:#}", op.target.display()));
        }
    }
    if !failures.is_empty() {
        bail!(
            "[E_RECOVERY] rollback incomplete; keep {} and retry:\n{}",
            journal.backup_dir.display(),
            failures.join("\n")
        );
    }
    Ok(())
}
pub struct Policy<'a> {
    pub force: bool,
    pub adopt_existing: bool,
    pub scopes: &'a BTreeSet<String>,
    pub full_go: bool,
    pub full_calc: bool,
    pub roots: &'a [PathBuf],
}
pub fn commit(
    state: &Path,
    outputs: &[Output],
    policy: &Policy<'_>,
    report: &Reporter,
) -> Result<Changes> {
    commit_impl(state, outputs, policy, report, None)
}
fn commit_impl(
    state: &Path,
    outputs: &[Output],
    policy: &Policy<'_>,
    report: &Reporter,
    fail_after: Option<usize>,
) -> Result<Changes> {
    let old = read_manifest(state)?;
    let mut next = old.clone();
    let mut changes = Changes::default();
    let mut writes = vec![];
    let mut removes = vec![];
    let mut seen = BTreeSet::new();
    let mut conflicts = vec![];
    for output in outputs {
        if !output.target.is_absolute()
            || !policy.roots.iter().any(|p| output.target.starts_with(p))
        {
            bail!(
                "[E_OUTPUT_SCOPE] {} is outside configured output roots",
                output.target.display()
            );
        }
        util::no_symlinks(&output.target)?;
        let key = util::path_key(&output.target);
        let generated = util::file_hash(&output.source)?;
        if !seen.insert(key.clone()) {
            bail!(
                "[E_OUTPUT_COLLISION] duplicate output {}",
                output.target.display()
            );
        }
        let existing = if output.target.is_file() {
            Some(util::file_hash(&output.target)?)
        } else if output.target.exists() {
            bail!("[E_OUTPUT_TYPE] expected file: {}", output.target.display());
        } else {
            None
        };
        match &existing {
            None => {
                changes.added.push(output.target.clone());
                writes.push(output);
            }
            Some(current) if *current == generated => changes.unchanged.push(output.target.clone()),
            Some(current) => {
                match old.files.get(&key) {
                    Some(previous) if *current == previous.sha256 || policy.force => {}
                    Some(_) => {
                        conflicts.push(Conflict {
                            file: output.target.clone(),
                            kind: "modified_output",
                        });
                    }
                    None if policy.adopt_existing => changes.adopted.push(output.target.clone()),
                    None => {
                        conflicts.push(Conflict {
                            file: output.target.clone(),
                            kind: "unowned_output",
                        });
                    }
                }
                changes.modified.push(output.target.clone());
                writes.push(output);
            }
        }
        next.files.insert(
            key,
            Owned {
                target: output.target.clone(),
                sha256: generated,
                scope: output.scope.clone(),
            },
        );
    }
    for (key, item) in &old.files {
        if seen.contains(key)
            || !(policy.scopes.contains(&item.scope)
                || (policy.full_go && item.scope.starts_with("go:"))
                || (policy.full_calc && item.scope.starts_with("calc:")))
        {
            continue;
        }
        if !policy
            .roots
            .iter()
            .any(|root| item.target.starts_with(root))
        {
            bail!(
                "[E_MANIFEST] stale output outside configured roots: {}",
                item.target.display()
            );
        }
        util::no_symlinks(&item.target)?;
        if item.target.is_file() {
            if util::file_hash(&item.target)? != item.sha256 && !policy.force {
                conflicts.push(Conflict {
                    file: item.target.clone(),
                    kind: "modified_stale_output",
                });
            }
            removes.push(item.target.clone());
            changes.removed.push(item.target.clone());
        }
        next.files.remove(key);
    }
    if !conflicts.is_empty() {
        return Err(OutputConflict(conflicts).into());
    }
    util::check_cancelled()?;
    fs::create_dir_all(state)?;
    let backup = state.join("backups").join(util::run_id());
    fs::create_dir_all(&backup)?;
    let mut journal = Journal {
        phase: "prepared".into(),
        operations: vec![],
        backup_dir: backup.clone(),
    };
    let manifest_path = state.join("manifest.json");
    for (index, target) in writes
        .iter()
        .map(|o| &o.target)
        .chain(removes.iter())
        .chain(std::iter::once(&manifest_path))
        .enumerate()
    {
        let snapshot = if target.is_file() {
            let path = backup.join(format!("{index:06}.bak"));
            fs::copy(target, &path).with_context(|| format!("backup {}", target.display()))?;
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)?
                .sync_all()?;
            Some(path)
        } else {
            None
        };
        journal.operations.push(Operation {
            target: target.clone(),
            backup: snapshot,
        });
    }
    util::write_json(&backup.join("index.json"), &journal)?;
    util::write_json(&state.join("journal.json"), &journal)?;
    report.outputs("writing", None);
    let apply = (|| -> Result<()> {
        for (index, output) in writes.iter().enumerate() {
            util::check_cancelled()?;
            util::atomic_write(&output.target, &fs::read(&output.source)?)?;
            if fail_after == Some(index + 1) {
                bail!("injected transaction failure");
            }
        }
        for path in &removes {
            util::check_cancelled()?;
            fs::remove_file(path).with_context(|| format!("remove stale {}", path.display()))?;
        }
        util::write_json(&manifest_path, &next)?;
        journal.phase = "committed".into();
        util::write_json(&state.join("journal.json"), &journal)?;
        Ok(())
    })();
    if let Err(error) = apply {
        if let Err(recovery) = rollback(&journal) {
            report.outputs("recovery_required", None);
            return Err(recovery.context(format!("[E_COMMIT] {error:#}")));
        }
        report.outputs("rolled_back", Some(false));
        fs::remove_file(state.join("journal.json"))?;
        return Err(error.context("[E_COMMIT] restored original outputs / 已回滚原有产物"));
    }
    report.outputs(
        "committed",
        Some(
            !changes.added.is_empty()
                || !changes.modified.is_empty()
                || !changes.removed.is_empty(),
        ),
    );
    fs::remove_file(state.join("journal.json"))?;
    changes.backup = Some(backup);
    report.info(
        "commit",
        format!(
            "{} {} | {} {} | {} {} | {} {} | {} {}",
            report.text("新增", "added"),
            changes.added.len(),
            report.text("修改", "modified"),
            changes.modified.len(),
            report.text("清理", "removed"),
            changes.removed.len(),
            report.text("未变", "unchanged"),
            changes.unchanged.len(),
            report.text("接管", "adopted"),
            changes.adopted.len()
        ),
    );
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rollback_restores_all_files_and_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let stage = root.join("staged");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("a"), "new a").unwrap();
        fs::write(stage.join("b"), "new b").unwrap();
        fs::write(root.join("a"), "old a").unwrap();
        let outputs = vec![
            Output {
                source: stage.join("a"),
                target: root.join("a"),
                scope: "go:a".into(),
            },
            Output {
                source: stage.join("b"),
                target: root.join("b"),
                scope: "go:a".into(),
            },
        ];
        let report = Reporter::new("test", true, false, "en-US");
        let roots = vec![root.clone()];
        let scopes = BTreeSet::new();
        let policy = Policy {
            force: false,
            adopt_existing: true,
            scopes: &scopes,
            full_go: false,
            full_calc: false,
            roots: &roots,
        };
        assert!(commit_impl(&root.join("state"), &outputs, &policy, &report, Some(2)).is_err());
        assert_eq!(fs::read_to_string(root.join("a")).unwrap(), "old a");
        assert!(!root.join("b").exists());
        assert!(!root.join("state/manifest.json").exists());
        let result = report.agent_output(&Ok(root.join("unused-report.json")));
        assert_eq!(result["output_state"], "rolled_back");
        assert_eq!(result["generated_files_changed"], false);
    }
    #[test]
    fn edits_require_force_unowned_files_require_adoption_and_partial_preserves_other_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_owned();
        let source = root.join("source");
        fs::write(&source, "v1").unwrap();
        let out = Output {
            source: source.clone(),
            target: root.join("out/a"),
            scope: "go:act1".into(),
        };
        let state = root.join("state");
        let roots = vec![root.join("out")];
        let scopes = BTreeSet::from(["go:act1".into()]);
        let mut policy = Policy {
            force: false,
            adopt_existing: false,
            scopes: &scopes,
            full_go: false,
            full_calc: false,
            roots: &roots,
        };
        let report = Reporter::new("test", true, false, "en-US");
        commit(&state, std::slice::from_ref(&out), &policy, &report).unwrap();
        fs::write(&out.target, "user").unwrap();
        fs::write(&source, "v2").unwrap();
        assert!(commit(&state, std::slice::from_ref(&out), &policy, &report).is_err());
        policy.force = true;
        let result = commit(&state, std::slice::from_ref(&out), &policy, &report).unwrap();
        assert!(result.backup.unwrap().is_dir());
        assert_eq!(fs::read_to_string(&out.target).unwrap(), "v2");
        let other = BTreeSet::from(["go:act2".into()]);
        policy.scopes = &other;
        commit(&state, &[], &policy, &report).unwrap();
        assert!(out.target.is_file());
    }
}
