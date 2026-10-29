//! Versioned, bounded process interface. Full evidence remains in the normal report.
use crate::{failure::Failure, report::Report};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub use crate::api_contract::API_VERSION;
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
    // Keep both paths verbatim for tempfile's native Windows rename operation.
    let native_output = std::fs::canonicalize(&output)?;
    let mut staged = tempfile::NamedTempFile::new_in(&native_output)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    staged
        .persist_noclobber(native_output.join("SKILL.md"))
        .with_context(|| {
            format!(
                "[E_CONFIG] export {} (existing files are never overwritten)",
                target.display()
            )
        })?;
    report.result(json!({"file":target,"sha256":crate::util::hash(bytes)}));
    report.info("skill", target.display().to_string());
    Ok(())
}

#[derive(Serialize)]
struct Response<'a> {
    api: &'static str,
    api_version: u32,
    request_id: Option<&'a str>,
    operation: Option<&'a str>,
    tool: ToolIdentity,
    run: RunIdentity<'a>,
    status: &'static str,
    exit_code: i32,
    context: &'a Value,
    result: Value,
    output: OutputState,
    error: Option<Value>,
    diagnostics: DiagnosticPage,
    warnings: WarningCount,
    report: ReportStorage<'a>,
}
#[derive(Serialize)]
struct ToolIdentity {
    name: &'static str,
    version: &'static str,
}
#[derive(Serialize)]
struct RunIdentity<'a> {
    id: &'a str,
    duration_seconds: f64,
}
#[derive(Serialize)]
struct OutputState {
    state: &'static str,
    changed: Option<bool>,
    recovered_previous_transaction: bool,
}
#[derive(Serialize)]
struct DiagnosticPage {
    items: Vec<Value>,
    total: usize,
    truncated: bool,
}
#[derive(Serialize)]
struct WarningCount {
    count: usize,
}
#[derive(Serialize)]
struct ReportStorage<'a> {
    path: Option<&'a PathBuf>,
    error: Option<String>,
}

/// The same response contract is advertised to JSON callers and MCP clients.
pub fn response_schema() -> Value {
    let output = json!({
        "type":"object","required":["state","changed","recovered_previous_transaction"],"additionalProperties":false,
        "properties":{
            "state":{"type":"string","enum":["not_written","writing","committed","rolled_back","recovering","recovered","recovery_required"]},
            "changed":{"type":["boolean","null"]},"recovered_previous_transaction":{"type":"boolean"}
        }
    });
    let error = json!({
        "type":"object","required":["code","kind","message","retryable","suggested_action","files","file_count","conflicts","conflict_count"],"additionalProperties":false,
        "properties":{
            "code":{"type":"string"},"kind":{"type":"string"},"message":{"type":["string","null"],"maxLength":1201},
            "retryable":{"type":"boolean"},"suggested_action":{"type":"string"},
            "files":{"type":"array","maxItems":INLINE_LIMIT,"items":{"type":"string"}},"file_count":{"type":"integer","minimum":0},
            "conflicts":{"type":"array","maxItems":INLINE_LIMIT,"items":{"type":"object","required":["file","kind"],"additionalProperties":false,"properties":{"file":{"type":"string"},"kind":{"type":"string"}}}},
            "conflict_count":{"type":"integer","minimum":0}
        }
    });
    let diagnostic = json!({
        "type":"object","required":["file","line","column","message"],"additionalProperties":false,
        "properties":{
            "file":{"type":"string"},"line":{"type":"integer","minimum":0},
            "column":{"type":["integer","null"],"minimum":0},"message":{"type":"string","maxLength":1201}
        }
    });
    let diagnostics = json!({
        "type":"object","required":["items","total","truncated"],"additionalProperties":false,
        "properties":{"items":{"type":"array","maxItems":INLINE_LIMIT,"items":diagnostic},"total":{"type":"integer","minimum":0},"truncated":{"type":"boolean"}}
    });
    json!({
        "type":"object",
        "required":["api","api_version","request_id","operation","tool","run","status","exit_code","context","result","output","error","diagnostics","warnings","report"],
        "additionalProperties":false,
        "properties":{
            "api":{"type":"string","const":"proto-builder.agent"},
            "api_version":{"type":"integer","const":API_VERSION},
            "request_id":{"type":["string","null"],"maxLength":128},
            "operation":{"type":["string","null"]},
            "tool":{"type":"object","required":["name","version"],"additionalProperties":false,"properties":{"name":{"type":"string","const":"proto-builder"},"version":{"type":"string"}}},
            "run":{"type":"object","required":["id","duration_seconds"],"additionalProperties":false,"properties":{"id":{"type":"string"},"duration_seconds":{"type":"number","minimum":0}}},
            "status":{"type":"string","enum":["success","blocked","failed","cancelled"]},
            "exit_code":{"type":"integer","minimum":0},
            "context":{"type":["object","null"]},
            "result":{},
            "output":output,
            "error":{"anyOf":[{"type":"null"},error]},
            "diagnostics":diagnostics,
            "warnings":{"type":"object","required":["count"],"additionalProperties":false,"properties":{"count":{"type":"integer","minimum":0}}},
            "report":{"type":"object","required":["path","error"],"additionalProperties":false,"properties":{"path":{"type":["string","null"]},"error":{"type":["string","null"],"maxLength":1201}}}
        }
    })
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
            json!({"counts":{"proto_files":r["proto_files"],"documentation_proto_files":r["documentation_proto_files"],"groups":r["groups"],"commands":r["commands"]},
                "changes":{"added":c["added"].as_array().map(Vec::len),"modified":c["modified"].as_array().map(Vec::len),"removed":c["removed"].as_array().map(Vec::len),"unchanged":c["unchanged"].as_array().map(Vec::len),"adopted":c["adopted"].as_array().map(Vec::len)},"backup":c["backup"]})
        }
        "plan" | "build" if r.get("scanned").is_some() => {
            json!({"counts":{"scanned":r["scanned"],"selected":r["selected"].as_array().map(Vec::len),"documentation_proto_files":r["documentation_inputs"].as_array().map(Vec::len),"groups":r["groups"].as_object().map(serde_json::Map::len),"excluded":r["excluded"].as_array().map(Vec::len),"filtered":r["filtered"].as_array().map(Vec::len),"import_only":r["import_only"].as_array().map(Vec::len)},"full_build":r["full_build"]})
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
pub fn envelope(report: &Report, storage: Option<&anyhow::Result<PathBuf>>) -> Value {
    let storage_failure = storage
        .and_then(|result| result.as_ref().err())
        .map(|e| anyhow::anyhow!("[E_REPORT] {e:#}"));
    let failure = report.failure.clone().or_else(|| {
        storage_failure
            .as_ref()
            .map(|e| Failure::from_error(e, &[]))
    });
    let exit_code = if report.exit_code != 0 {
        report.exit_code
    } else if storage.is_some_and(Result::is_err) {
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
    let value = Response {
        api: "proto-builder.agent",
        api_version: API_VERSION,
        request_id: report.context["request"]["request_id"].as_str(),
        operation: report.context["request"]["operation"].as_str(),
        tool: ToolIdentity {
            name: report.tool,
            version: report.version,
        },
        run: RunIdentity {
            id: &report.run_id,
            duration_seconds: report.duration_seconds,
        },
        status,
        exit_code,
        context: &report.context,
        result: summary(report),
        output: OutputState {
            state: report.output_state,
            changed: report.generated_files_changed,
            recovered_previous_transaction: report.recovered_previous_transaction,
        },
        error,
        diagnostics: DiagnosticPage {
            items: diagnostics,
            total: report.diagnostics.len(),
            truncated: report.diagnostics.len() > INLINE_LIMIT,
        },
        warnings: WarningCount {
            count: report
                .events
                .iter()
                .filter(|e| e.level == "warning")
                .count(),
        },
        report: ReportStorage {
            path: storage.and_then(|result| result.as_ref().ok()),
            error: storage_failure.as_ref().map(|e| bounded(&format!("{e:#}"))),
        },
    };
    serde_json::to_value(value).expect("response fields are serializable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_bounds_diagnostics_while_internal_report_preserves_evidence() {
        let report = crate::report::Reporter::for_automation("plan", false, "en-US");
        report.context(json!({"request":{"request_id":"check-42","operation":"plan"}}));
        let messages = (0..25)
            .map(|index| format!("file{index}.proto:12:4: {}", "界".repeat(1300)))
            .collect::<Vec<_>>()
            .join("\n");
        let outcome = Err(anyhow::anyhow!("[E_BUILD] invalid source\n{messages}"));
        let value = report.finish_in_memory(&outcome);
        assert_eq!(value["api"], "proto-builder.agent");
        assert_eq!(value["api_version"], 2);
        assert_eq!(value["request_id"], "check-42");
        assert_eq!(value["operation"], "plan");
        assert_eq!(value["diagnostics"]["items"].as_array().unwrap().len(), 20);
        assert_eq!(value["diagnostics"]["total"], 25);
        assert_eq!(value["diagnostics"]["truncated"], true);
        assert_eq!(
            value["diagnostics"]["items"][0]["message"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            1201
        );
        assert_eq!(value["error"]["files"].as_array().unwrap().len(), 20);
        assert_eq!(value["error"]["file_count"], 25);
        assert!(value["report"]["path"].is_null());
        assert!(value["report"]["error"].is_null());
        for legacy in [
            "schema_version",
            "interface",
            "project_root",
            "output_state",
            "generated_files_changed",
            "report_path",
            "report_error",
        ] {
            assert!(value.get(legacy).is_none(), "legacy field {legacy}");
        }
        let full = report.snapshot(0).unwrap();
        assert_eq!(full["schema_version"], 1);
        assert_eq!(full["diagnostics"].as_array().unwrap().len(), 25);
        assert_eq!(
            full["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            1300
        );
    }

    #[test]
    fn report_storage_failure_preserves_original_failure_and_committed_outputs() {
        let report = crate::report::Reporter::for_automation("build", false, "en-US");
        report.outputs("committed", Some(true));
        let successful = report.finish_in_memory(&Ok(()));
        assert_eq!(successful["status"], "success");
        assert!(successful["report"]["error"].is_null());
        let storage = Err(anyhow::anyhow!("permission denied"));
        let failed = report.response(Some(&storage));
        assert_eq!(failed["status"], "failed");
        assert_eq!(failed["exit_code"], 1);
        assert_eq!(failed["error"]["code"], "E_REPORT");
        assert_eq!(failed["output"]["state"], "committed");
        assert_eq!(failed["output"]["changed"], true);
        assert!(failed["report"]["path"].is_null());
        assert!(failed["report"]["error"]
            .as_str()
            .unwrap()
            .contains("permission denied"));

        let blocked = crate::report::Reporter::for_automation("build", false, "en-US");
        blocked.finish_in_memory(&Err(anyhow::anyhow!(
            "[E_LOCKED] another build owns the output"
        )));
        let value = blocked.response(Some(&storage));
        assert_eq!(value["status"], "blocked");
        assert_eq!(value["exit_code"], 8);
        assert_eq!(value["error"]["code"], "E_LOCKED");
        assert_eq!(value["output"]["state"], "not_written");
        assert!(value["report"]["error"].is_string());
    }

    #[test]
    fn previous_transaction_recovery_never_claims_unchanged_outputs() {
        let report = crate::report::Reporter::for_automation("build", false, "en-US");
        report.outputs("recovering", None);
        report.outputs("recovered", None);
        report.outputs("committed", Some(false));
        let value = report.response(Some(&Ok("unused-report.json".into())));
        assert_eq!(value["output"]["recovered_previous_transaction"], true);
        assert!(value["output"]["changed"].is_null());
    }
}
