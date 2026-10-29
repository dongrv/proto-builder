//! Read-only, bounded queries shared by command-line and agent transports.
use crate::{config::Loaded, scan, util};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs::File, io::Read, path::Path};

pub const MAX_PAGE_SIZE: usize = 500;
pub const MAX_REPORT_BYTES: u64 = 32 * 1024 * 1024;
const SUMMARY_EVIDENCE_LIMIT: usize = 20;
const SUMMARY_ERROR_CHARACTERS: usize = 1200;
pub const REPORT_SECTIONS: &[&str] = &[
    "summary",
    "events",
    "diagnostics",
    "selected",
    "excluded",
    "filtered",
    "outputs",
    "import_only",
    "documentation_inputs",
    "changes",
];

fn validate_page(limit: usize) -> Result<()> {
    if !(1..=MAX_PAGE_SIZE).contains(&limit) {
        bail!("[E_ARGUMENT] limit must be between 1 and {MAX_PAGE_SIZE}");
    }
    Ok(())
}

fn page(items: Vec<Value>, total: usize, offset: usize, limit: usize) -> Value {
    let end = offset.saturating_add(limit).min(total);
    let next = (end < total).then_some(end);
    json!({
        "items": items,
        "total": total,
        "offset": offset,
        "limit": limit,
        "next_offset": next,
    })
}

fn facet(values: BTreeSet<&str>) -> Value {
    let total = values.len();
    json!({
        "items": values.into_iter().take(MAX_PAGE_SIZE).collect::<Vec<_>>(),
        "total": total,
        "truncated": total > MAX_PAGE_SIZE,
    })
}

pub fn catalog(loaded: &Loaded, query: Option<&str>, offset: usize, limit: usize) -> Result<Value> {
    validate_page(limit)?;
    let query = query.unwrap_or_default().trim();
    if query.chars().count() > 256 {
        bail!("[E_ARGUMENT] catalog query must not exceed 256 characters");
    }
    let needle = query.to_lowercase();
    let plan = scan::make(&loaded.root, &loaded.settings)?;
    let matches = plan
        .selected
        .iter()
        .filter(|input| {
            [
                Some(input.relative.as_str()),
                Some(input.group.as_str()),
                input.activity_id.as_deref(),
                input.machine_id.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|text| text.to_lowercase().contains(&needle))
        })
        .collect::<Vec<_>>();
    let activities = matches
        .iter()
        .filter_map(|i| i.activity_id.as_deref())
        .collect();
    let machines = matches
        .iter()
        .filter_map(|i| i.machine_id.as_deref())
        .collect();
    let groups = matches.iter().map(|i| i.group.as_str()).collect();
    let mut shared = 0;
    let mut activity = 0;
    let mut machine = 0;
    let mut combined = 0;
    for input in &matches {
        match (&input.activity_id, &input.machine_id) {
            (None, None) => shared += 1,
            (Some(_), None) => activity += 1,
            (None, Some(_)) => machine += 1,
            (Some(_), Some(_)) => combined += 1,
        }
    }
    let total = matches.len();
    let end = offset.saturating_add(limit).min(total);
    let items = matches
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(json!({
        "items": items,
        "total": total,
        "offset": offset,
        "limit": limit,
        "next_offset": (end < total).then_some(end),
        "query": query,
        "query_scope": "catalog_display_only",
        "scanned": plan.scanned,
        "selected_total": plan.selected.len(),
        "documentation_proto_files": plan.documentation_inputs.len(),
        "excluded_total": plan.excluded.len(),
        "filtered_total": plan.filtered.len(),
        "import_only_total": plan.import_only.len(),
        "full_build": plan.full_build,
        "classification_counts": {"shared":shared,"activity":activity,"machine":machine,"combined":combined},
        "facets": {"activity_ids":facet(activities),"machine_ids":facet(machines),"groups":facet(groups)},
    }))
}

fn array_len(value: &Value) -> usize {
    value.as_array().map_or(0, Vec::len)
}

fn failure_summary(failure: &Value) -> Value {
    if !failure.is_object() {
        return Value::Null;
    }
    let files = failure["files"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let conflicts = failure["conflicts"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    json!({
        "code":failure["code"],
        "kind":failure["kind"],
        "retryable":failure["retryable"],
        "suggested_action":failure["suggested_action"],
        "files":&files[..files.len().min(SUMMARY_EVIDENCE_LIMIT)],
        "files_total":files.len(),
        "files_truncated":files.len() > SUMMARY_EVIDENCE_LIMIT,
        "conflicts":&conflicts[..conflicts.len().min(SUMMARY_EVIDENCE_LIMIT)],
        "conflicts_total":conflicts.len(),
        "conflicts_truncated":conflicts.len() > SUMMARY_EVIDENCE_LIMIT,
    })
}

fn error_summary(error: &Value) -> (Value, bool) {
    let Some(text) = error.as_str() else {
        return (Value::Null, false);
    };
    let mut chars = text.chars();
    let mut summary: String = chars.by_ref().take(SUMMARY_ERROR_CHARACTERS).collect();
    let truncated = chars.next().is_some();
    if truncated {
        summary.push('…');
    }
    (summary.into(), truncated)
}

fn report_summary(report: &Value) -> Value {
    let result = &report["result"];
    let context = &report["context"];
    let (error, error_truncated) = error_summary(&report["error"]);
    json!({
        "version": report["version"],
        "exit_code": report["exit_code"],
        "duration_seconds": report["duration_seconds"],
        "generated_files_changed": report["generated_files_changed"],
        "recovered_previous_transaction": report["recovered_previous_transaction"],
        "context": {
            "project_root": context["project_root"],
            "proto_root": context["proto_root"],
            "server_root": context["server_root"],
            "calc_root": context["calc_root"],
            "selection": context["selection"],
            "policy": context["policy"],
        },
        "counts": {
            "events": array_len(&report["events"]),
            "diagnostics": array_len(&report["diagnostics"]),
            "scanned": result["scanned"],
            "selected": result["selected"].as_array().map(Vec::len).map(Value::from).unwrap_or_else(|| result["proto_files"].clone()),
            "documentation_proto_files": result["documentation_inputs"].as_array().map(Vec::len).map(Value::from).unwrap_or_else(|| result["documentation_proto_files"].clone()),
            "groups": result["groups"].as_object().map(serde_json::Map::len).map(Value::from).unwrap_or_else(|| result["groups"].clone()),
            "commands": result["commands"],
            "excluded": result["excluded"].as_array().map(Vec::len),
            "filtered": result["filtered"].as_array().map(Vec::len),
            "outputs": result["outputs"].as_array().map(Vec::len),
            "import_only": result["import_only"].as_array().map(Vec::len),
            "added": array_len(&result["changes"]["added"]),
            "modified": array_len(&result["changes"]["modified"]),
            "removed": array_len(&result["changes"]["removed"]),
            "unchanged": array_len(&result["changes"]["unchanged"]),
            "adopted": array_len(&result["changes"]["adopted"]),
        },
        "failure": failure_summary(&report["failure"]),
        "error": error,
        "error_truncated": error_truncated,
        "backup": result["changes"]["backup"],
    })
}

/// Reports contain evidence, never commands. Reading a section cannot execute or write anything.
pub fn read_report(path: &Path, section: &str, offset: usize, limit: usize) -> Result<Value> {
    validate_page(limit)?;
    if !REPORT_SECTIONS.contains(&section) {
        bail!(
            "[E_ARGUMENT] unknown report section {section:?}; expected {}",
            REPORT_SECTIONS.join(", ")
        );
    }
    let file =
        File::open(path).with_context(|| format!("[E_ARGUMENT] open report {}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_REPORT_BYTES {
        bail!("[E_ARGUMENT] report must be a regular file no larger than 32 MiB");
    }
    // The bound also applies when a file grows between metadata() and read().
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_REPORT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        bail!("[E_ARGUMENT] report exceeds 32 MiB");
    }
    let report: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("[E_ARGUMENT] invalid JSON report {}", path.display()))?;
    if report["tool"] != "proto-builder"
        || report["schema_version"] != 1
        || report.get("interface").is_some()
        || !report["run_id"].is_string()
        || !report["command"].is_string()
        || !report["status"].is_string()
        || !report["events"].is_array()
        || !report["diagnostics"].is_array()
        || report.get("context").is_none()
        || report.get("result").is_none()
    {
        bail!("[E_ARGUMENT] expected a full proto-builder report with schema_version 1; agent envelopes are not full reports");
    }
    let (items, total, applicable) = match section {
        "summary" => (
            if offset == 0 {
                vec![report_summary(&report)]
            } else {
                vec![]
            },
            1,
            true,
        ),
        "changes" => {
            let changes = &report["result"]["changes"];
            let mut items = vec![];
            let mut total = 0;
            for kind in ["added", "modified", "removed", "unchanged", "adopted"] {
                if let Some(paths) = changes.get(kind) {
                    let mut paths = paths
                        .as_array()
                        .context("[E_ARGUMENT] report changes must contain file arrays")?
                        .iter()
                        .map(|path| {
                            path.as_str()
                                .context("[E_ARGUMENT] report change path must be a string")
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let start = offset.saturating_sub(total).min(paths.len());
                    let end = offset
                        .saturating_add(limit)
                        .saturating_sub(total)
                        .min(paths.len());
                    if start < end {
                        paths.sort_unstable();
                        items.extend(
                            paths[start..end]
                                .iter()
                                .map(|file| json!({"kind":kind,"file":file})),
                        );
                    }
                    total += paths.len();
                }
            }
            (items, total, changes.is_object())
        }
        _ => {
            let parent = if matches!(section, "events" | "diagnostics") {
                &report
            } else {
                &report["result"]
            };
            let applicable = parent.get(section).is_some();
            let items = match parent.get(section) {
                None => &[][..],
                Some(value) => value
                    .as_array()
                    .context("[E_ARGUMENT] report section must be an array")?
                    .as_slice(),
            };
            let total = items.len();
            let start = offset.min(total);
            let end = offset.saturating_add(limit).min(total);
            (items[start..end].to_vec(), total, applicable)
        }
    };
    let mut result = page(items, total, offset, limit);
    result["section"] = json!(section);
    result["applicable"] = applicable.into();
    result["source"] = json!({
        "path": util::absolute(&std::env::current_dir()?, path),
        "sha256": util::hash(&bytes),
        "run_id": report["run_id"],
        "command": report["command"],
        "status": report["status"],
        "output_state": report["output_state"],
    });
    Ok(result)
}
