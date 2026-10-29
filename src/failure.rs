//! Shared domain failures for transactions, reports and every user interface.
use crate::report::Diagnostic;
use serde::Serialize;
use std::{collections::BTreeSet, fmt, path::PathBuf};

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
}
