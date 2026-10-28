use crate::{config::Settings, process, report::Reporter, util};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct Snapshot {
    pub url: String,
    pub revision: u64,
    pub working_copy_root: PathBuf,
}
#[derive(Debug, Deserialize)]
struct Info {
    entry: Entry,
}
#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(rename = "@revision")]
    revision: u64,
    url: String,
    #[serde(rename = "wc-info")]
    wc_info: WcInfo,
}
#[derive(Debug, Deserialize)]
struct WcInfo {
    #[serde(rename = "wcroot-abspath")]
    root: PathBuf,
}
#[derive(Debug, Deserialize)]
struct Status {
    #[serde(default)]
    target: Vec<Target>,
}
#[derive(Debug, Deserialize)]
struct Target {
    #[serde(default)]
    entry: Vec<StatusEntry>,
}
#[derive(Debug, Deserialize)]
struct StatusEntry {
    #[serde(rename = "@path")]
    path: String,
    #[serde(rename = "wc-status")]
    status: WcStatus,
}
#[derive(Debug, Deserialize)]
struct WcStatus {
    #[serde(rename = "@item")]
    item: String,
    #[serde(rename = "@props")]
    props: Option<String>,
    #[serde(rename = "@tree-conflicted")]
    tree: Option<String>,
}
pub fn snapshot(root: &Path, s: &Settings, report: &Reporter) -> Result<Snapshot> {
    let output = process::run(
        Command::new(&s.svn.executable)
            .args(["info", "--xml", "--non-interactive"])
            .arg(root),
        "svn-info",
        s.timeout_seconds,
        report,
    )
    .context("[E_SVN] svn info failed")?;
    let info: Info =
        quick_xml::de::from_str(&output.stdout).context("[E_SVN] parse svn info --xml")?;
    Ok(Snapshot {
        url: info.entry.url,
        revision: info.entry.revision,
        working_copy_root: info.entry.wc_info.root,
    })
}
pub fn conflicts(root: &Path, s: &Settings, report: &Reporter) -> Result<Vec<String>> {
    let output = process::run(
        Command::new(&s.svn.executable)
            .args(["status", "--xml", "--non-interactive"])
            .arg(root),
        "svn-status",
        s.timeout_seconds,
        report,
    )
    .context("[E_SVN] svn status failed")?;
    parse_conflicts(&output.stdout)
}
fn parse_conflicts(text: &str) -> Result<Vec<String>> {
    let status: Status = quick_xml::de::from_str(text).context("[E_SVN] parse svn status --xml")?;
    Ok(status
        .target
        .into_iter()
        .flat_map(|t| t.entry)
        .filter(|e| {
            e.status.item == "conflicted"
                || e.status.props.as_deref() == Some("conflicted")
                || e.status.tree.as_deref() == Some("true")
        })
        .map(|e| e.path)
        .collect())
}
pub fn update(root: &Path, s: &Settings, report: &Reporter) -> Result<serde_json::Value> {
    if !s.svn.update {
        return Ok(serde_json::json!({"updated":false}));
    }
    let before = snapshot(root, s, report)?;
    let initial = conflicts(root, s, report)?;
    if !initial.is_empty() {
        bail!(
            "[E_SVN_CONFLICT] existing SVN conflicts / 已有 SVN 冲突:\n{}",
            initial.join("\n")
        );
    }
    let paths = if s.svn.update_paths.is_empty() {
        vec![root.to_owned()]
    } else {
        s.svn
            .update_paths
            .iter()
            .map(|p| util::absolute(root, p))
            .collect()
    };
    for path in &paths {
        util::no_symlinks(path)?;
        if !path.starts_with(root) || !path.exists() {
            bail!(
                "[E_SVN_SCOPE] update path must exist inside project root: {}",
                path.display()
            );
        }
    }
    let before_paths = paths
        .iter()
        .map(|path| snapshot(path, s, report))
        .collect::<Result<Vec<_>>>()?;
    report.info(
        "svn",
        format!(
            "{}: {}",
            report.text("更新项目范围", "Updating project scope"),
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    let output = process::run(
        Command::new(&s.svn.executable)
            .args(["update", "--non-interactive", "--accept", "postpone"])
            .args(&paths),
        "svn-update",
        s.timeout_seconds,
        report,
    )
    .context("[E_SVN_UPDATE] SVN update failed / SVN 更新失败，已中止生成")?;
    let problems = conflicts(root, s, report)?;
    if !problems.is_empty() {
        bail!("[E_SVN_CONFLICT] SVN update produced conflicts; resolve manually / 更新存在冲突，请手动解决:\n{}",problems.join("\n"));
    }
    let after = snapshot(root, s, report)?;
    let mut changes = vec![];
    for (path, previous) in paths.iter().zip(before_paths) {
        let current = snapshot(path, s, report)?;
        report.info(
            "svn",
            format!(
                "{}: r{} → r{}",
                path.display(),
                previous.revision,
                current.revision
            ),
        );
        changes.push(serde_json::json!({"path":path,"before":previous,"after":current}));
    }
    Ok(
        serde_json::json!({"updated":true,"paths":paths,"before":before,"after":after,"path_revisions":changes,"output":output.stdout}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_property_text_and_tree_conflicts() {
        let text = r#"<status><target path="."><entry path="a"><wc-status item="conflicted" props="none" /></entry><entry path="b"><wc-status item="modified" props="conflicted" /></entry><entry path="c"><wc-status item="normal" tree-conflicted="true" /></entry><entry path="d"><wc-status item="modified" /></entry></target></status>"#;
        assert_eq!(parse_conflicts(text).unwrap(), vec!["a", "b", "c"]);
    }
}
