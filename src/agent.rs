//! Versioned, bounded process interface. Full evidence remains in the normal report.
use crate::report::{Diagnostic, Report};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::BTreeSet, fmt, path::PathBuf};

pub const SCHEMA_VERSION: u32 = 1;
const INLINE_LIMIT: usize = 20;

pub fn export_skill(
    output: &std::path::Path,
    report: &crate::report::Reporter,
) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;
    let output = crate::util::absolute(&std::env::current_dir()?, output);
    let target = output.join("SKILL.md");
    crate::util::no_symlinks(&target)?;
    std::fs::create_dir_all(&output)
        .with_context(|| format!("[E_CONFIG] create {}", output.display()))?;
    let bytes = include_bytes!("../.agents/skills/proto-builder/SKILL.md");
    let mut staged = tempfile::NamedTempFile::new_in(&output)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    staged.persist_noclobber(&target).with_context(|| {
        format!(
            "[E_CONFIG] export {} (existing files are never overwritten)",
            target.display()
        )
    })?;
    report.result(json!({"file":target,"sha256":crate::util::hash(bytes)}));
    report.info("skill", target.display().to_string());
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct Conflict {
    pub file: PathBuf,
    pub kind: &'static str,
}
#[derive(Debug)]
pub struct OutputConflict(pub Vec<Conflict>);
impl fmt::Display for OutputConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[E_OUTPUT_CONFLICT] output transaction aborted; no generated files changed / 写回已中止:")?;
        for item in &self.0 {
            let label = match item.kind {
                "unowned_output" => "unowned / 尚未接管 (review --adopt-existing)",
                "modified_stale_output" => {
                    "modified stale file / 已修改的过期产物 (review --force)"
                }
                _ => "modified / 用户已修改 (review --force)",
            };
            writeln!(f, "{label}: {}", item.file.display())?;
        }
        Ok(())
    }
}
impl std::error::Error for OutputConflict {}

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub code: String,
    pub kind: &'static str,
    pub retryable: bool,
    pub suggested_action: &'static str,
    pub files: Vec<String>,
    pub conflicts: Vec<Conflict>,
}
impl Failure {
    pub fn from_error(error: &anyhow::Error, diagnostics: &[Diagnostic]) -> Self {
        let text = format!("{error:#}");
        let re = regex::Regex::new(r"\[(E_[A-Z_]+)\]").expect("constant regex");
        let codes = re
            .captures_iter(&text)
            .map(|c| c[1].to_owned())
            .collect::<Vec<_>>();
        let code = codes
            .iter()
            .find(|c| c.as_str() == "E_CANCELLED")
            .or_else(|| codes.iter().find(|c| c.as_str() != "E_BUILD"))
            .or_else(|| codes.first())
            .cloned()
            .unwrap_or_else(|| "E_EXECUTION".into());
        let conflicts = error
            .downcast_ref::<OutputConflict>()
            .map(|c| c.0.clone())
            .unwrap_or_default();
        let (kind, retryable, suggested_action) = match code.as_str() {
            "E_OUTPUT_CONFLICT" => {
                let kinds = conflicts.iter().map(|c| c.kind).collect::<BTreeSet<_>>();
                (
                    if kinds.len() == 1 {
                        *kinds.first().unwrap()
                    } else {
                        "output_conflict"
                    },
                    false,
                    "review_existing_outputs",
                )
            }
            "E_CANCELLED" => ("cancelled", false, "resume_if_requested"),
            "E_LOCKED" => ("target_busy", true, "retry_after_current_build"),
            "E_CONFIG_CHANGED" | "E_INPUT_CHANGED" => {
                ("inputs_changed", true, "reload_configuration_and_plan")
            }
            "E_ARGUMENT" | "E_FILTER" => ("invalid_arguments", false, "correct_arguments"),
            "E_NO_MATCH" => ("no_matching_protocols", false, "review_filters"),
            "E_SVN_CONFLICT" => ("svn_conflict", false, "resolve_svn_conflicts"),
            "E_OUTPUT_SCOPE" | "E_PATH" => ("invalid_paths", false, "correct_paths"),
            "E_PROTOCOL" | "E_IMPORT" | "E_INPUT" | "E_ID" => {
                ("invalid_protocols", false, "correct_protocol_sources")
            }
            "E_TIMEOUT" => ("process_timeout", false, "inspect_timeout_and_process"),
            "E_REPORT" => (
                "report_storage",
                false,
                "check_report_storage_and_output_state",
            ),
            "E_RECOVERY" | "E_COMMIT" => (
                "output_transaction",
                false,
                "inspect_output_state_and_recovery",
            ),
            "E_PROCESS" if !diagnostics.is_empty() => {
                ("invalid_protocols", false, "correct_protocol_sources")
            }
            "E_PROCESS" => ("subprocess_failed", false, "inspect_process_diagnostics"),
            v if v.starts_with("E_SVN") => ("svn_failed", false, "inspect_svn_failure"),
            v if v.starts_with("E_CONFIG") => {
                ("invalid_configuration", false, "correct_configuration")
            }
            v if v.starts_with("E_TOOLCHAIN") => ("toolchain_failed", false, "check_toolchain"),
            "E_INTEGRITY" | "E_SIGNATURE" | "E_TRUST_KEY" => {
                ("integrity_failed", false, "verify_official_release")
            }
            _ => ("execution_failed", false, "inspect_full_report"),
        };
        let files = diagnostics
            .iter()
            .map(|d| d.file.clone())
            .chain(conflicts.iter().map(|c| c.file.display().to_string()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            code,
            kind,
            retryable,
            suggested_action,
            files,
            conflicts,
        }
    }
}

fn bounded(text: &str) -> String {
    let mut chars = text.chars();
    let result: String = chars.by_ref().take(1200).collect();
    if chars.next().is_some() {
        format!("{result}…")
    } else {
        result
    }
}
fn summary(report: &Report) -> Value {
    let r = &report.result;
    match report.command.as_str() {
        "build" if r.get("changes").is_some() => {
            let c = &r["changes"];
            json!({"proto_files":r["proto_files"],"documentation_proto_files":r["documentation_proto_files"],"groups":r["groups"],"commands":r["commands"],
                "changes":{"added":c["added"].as_array().map(Vec::len),"modified":c["modified"].as_array().map(Vec::len),"removed":c["removed"].as_array().map(Vec::len),"unchanged":c["unchanged"].as_array().map(Vec::len),"adopted":c["adopted"].as_array().map(Vec::len)},"backup":c["backup"]})
        }
        "plan" | "build" if r.get("scanned").is_some() => {
            json!({"scanned":r["scanned"],"selected":r["selected"].as_array().map(Vec::len),"documentation_proto_files":r["documentation_inputs"].as_array().map(Vec::len),"groups":r["groups"].as_object().map(serde_json::Map::len),"excluded":r["excluded"].as_array().map(Vec::len),"filtered":r["filtered"].as_array().map(Vec::len),"import_only":r["import_only"].as_array().map(Vec::len),"full_build":r["full_build"]})
        }
        "licenses" => {
            json!({"licenses":r.as_object().map(|items|items.iter().map(|(name,text)|json!({"name":name,"characters":text.as_str().map(str::len)})).collect::<Vec<_>>())})
        }
        "compat" => {
            json!({"equal_files":r["equal_files"],"difference_count":r["differences"].as_array().map(Vec::len)})
        }
        _ => r.clone(),
    }
}
pub fn envelope(report: &Report, storage: &anyhow::Result<PathBuf>) -> Value {
    let storage_failure = storage
        .as_ref()
        .err()
        .map(|e| anyhow::anyhow!("[E_REPORT] {e:#}"));
    let failure = report.failure.clone().or_else(|| {
        storage_failure
            .as_ref()
            .map(|e| Failure::from_error(e, &[]))
    });
    let exit_code = if report.exit_code != 0 {
        report.exit_code
    } else if storage.is_err() {
        1
    } else {
        0
    };
    let status = match failure.as_ref().map(|f| f.kind) {
        None => "success",
        Some("cancelled") => "cancelled",
        Some(
            "unowned_output"
            | "modified_output"
            | "modified_stale_output"
            | "output_conflict"
            | "target_busy"
            | "svn_conflict"
            | "invalid_paths",
        ) => "blocked",
        _ => "failed",
    };
    let message = report
        .error
        .as_deref()
        .map(str::to_owned)
        .or_else(|| storage_failure.as_ref().map(|e| format!("{e:#}")));
    let error = failure.as_ref().map(|f| json!({"code":f.code,"kind":f.kind,"message":message.as_deref().map(bounded),"retryable":f.retryable,"suggested_action":f.suggested_action,
        "files":f.files.iter().take(INLINE_LIMIT).collect::<Vec<_>>(),"file_count":f.files.len(),"conflicts":f.conflicts.iter().take(INLINE_LIMIT).collect::<Vec<_>>(),"conflict_count":f.conflicts.len()}));
    let diagnostics = report.diagnostics.iter().take(INLINE_LIMIT).map(|d| json!({"file":d.file,"line":d.line,"column":d.column,"message":bounded(&d.message)})).collect::<Vec<_>>();
    json!({"schema_version":SCHEMA_VERSION,"interface":"agent","tool":report.tool,"version":report.version,"run_id":report.run_id,"command":report.command,"status":status,"exit_code":exit_code,
        "duration_seconds":report.duration_seconds,"project_root":report.context["project_root"],"proto_root":report.context["proto_root"],"server_root":report.context["server_root"],"calc_root":report.context["calc_root"],
        "selection":report.context["selection"],"policy":report.context["policy"],"svn":if report.context["svn"].is_object() {json!({"updated":report.context["svn"]["updated"],"before_revision":report.context["svn"]["before"]["revision"],"after_revision":report.context["svn"]["after"]["revision"],"updated_path_count":report.context["svn"]["paths"].as_array().map(Vec::len)})}else{Value::Null},
        "output_state":report.output_state,"generated_files_changed":report.generated_files_changed,"recovered_previous_transaction":report.recovered_previous_transaction,"result":summary(report),"error":error,"diagnostics":diagnostics,"diagnostic_count":report.diagnostics.len(),
        "warning_count":report.events.iter().filter(|e|e.level=="warning").count(),"report_path":storage.as_ref().ok(),"report_error":storage_failure.as_ref().map(|e|bounded(&format!("{e:#}")))})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_build_and_svn_context_do_not_hide_cancellation_or_timeout() {
        let cancelled = anyhow::anyhow!("[E_SVN] info failed: [E_CANCELLED] cancelled");
        let failure = Failure::from_error(&cancelled, &[]);
        assert_eq!(failure.code, "E_CANCELLED");
        assert_eq!(failure.kind, "cancelled");
        let failure = Failure::from_error(
            &anyhow::anyhow!("[E_BUILD] generation failed\n[E_TIMEOUT] protoc exceeded timeout"),
            &[],
        );
        assert_eq!(failure.code, "E_TIMEOUT");
        assert_eq!(failure.kind, "process_timeout");
        let failure = Failure::from_error(
            &anyhow::anyhow!("[E_LOCKED] another build owns the target"),
            &[],
        );
        assert!(failure.retryable);
        assert_eq!(failure.suggested_action, "retry_after_current_build");
    }
    #[test]
    fn previous_transaction_recovery_never_claims_unchanged_outputs() {
        let report = crate::report::Reporter::for_agent("build", false, "en-US");
        report.outputs("recovering", None);
        report.outputs("recovered", None);
        report.outputs("committed", Some(false));
        let value = report.agent_output(&Ok("unused-report.json".into()));
        assert_eq!(value["recovered_previous_transaction"], true);
        assert!(value["generated_files_changed"].is_null());
    }
}
