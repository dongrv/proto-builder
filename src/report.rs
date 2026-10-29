use crate::{agent, failure, util};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Instant,
};

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub file: String,
    pub line: usize,
    pub column: Option<usize>,
    pub message: String,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub tool: &'static str,
    pub version: &'static str,
    pub run_id: String,
    pub command: String,
    pub status: String,
    pub exit_code: i32,
    pub phase: String,
    pub duration_seconds: f64,
    pub context: Value,
    pub result: Value,
    pub events: Vec<Event>,
    pub diagnostics: Vec<Diagnostic>,
    pub error: Option<String>,
    pub failure: Option<failure::Failure>,
    pub output_state: &'static str,
    pub generated_files_changed: Option<bool>,
    pub recovered_previous_transaction: bool,
}
#[derive(Debug, Serialize)]
pub struct Event {
    pub elapsed_seconds: f64,
    pub phase: String,
    pub level: &'static str,
    pub message: String,
}
#[derive(Clone)]
pub struct Reporter {
    report: Arc<Mutex<Report>>,
    start: Instant,
    pub json: bool,
    pub verbose: bool,
    pub automation: bool,
    print_json: bool,
    english: Arc<AtomicBool>,
    retention: Arc<AtomicUsize>,
    last_progress_ms: Arc<AtomicU64>,
}
impl Reporter {
    pub fn new(command: &str, json: bool, verbose: bool, lang: &str) -> Self {
        Self {
            report: Arc::new(Mutex::new(Report {
                schema_version: 1,
                tool: "proto-builder",
                version: env!("CARGO_PKG_VERSION"),
                run_id: util::run_id(),
                command: command.into(),
                status: "running".into(),
                exit_code: 0,
                phase: "startup".into(),
                duration_seconds: 0.0,
                context: Value::Null,
                result: Value::Null,
                events: vec![],
                diagnostics: vec![],
                error: None,
                failure: None,
                output_state: "not_written",
                generated_files_changed: Some(false),
                recovered_previous_transaction: false,
            })),
            start: Instant::now(),
            json,
            verbose,
            automation: false,
            print_json: json,
            english: Arc::new(AtomicBool::new(lang == "en-US")),
            retention: Arc::new(AtomicUsize::new(20)),
            last_progress_ms: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn for_automation(command: &str, verbose: bool, lang: &str) -> Self {
        let mut report = Self::new(command, true, verbose, lang);
        report.automation = true;
        report.print_json = false;
        report
    }
    pub fn response(&self, storage: Option<&Result<PathBuf>>) -> Value {
        let report = self.report.lock().unwrap_or_else(|e| e.into_inner());
        redact_value(agent::envelope(&report, storage))
    }
    pub fn outputs(&self, state: &'static str, changed: Option<bool>) {
        let mut report = self.report.lock().unwrap_or_else(|e| e.into_inner());
        if state == "recovering" {
            report.recovered_previous_transaction = true;
        }
        report.output_state = state;
        report.generated_files_changed = if report.recovered_previous_transaction {
            None
        } else {
            changed
        };
    }
    pub fn for_web(command: &str, lang: &str, retention: usize) -> Self {
        let mut report = Self::new(command, true, false, lang);
        report.print_json = false;
        report.configure(lang, retention);
        report
    }
    pub fn snapshot(&self, after: usize) -> Result<Value> {
        let report = self.report.lock().unwrap_or_else(|e| e.into_inner());
        let count = report.events.len();
        // Serialize only new events: polling must not repeatedly copy the complete log.
        Ok(serde_json::json!({
            "schema_version":report.schema_version,"tool":report.tool,"version":report.version,
            "run_id":report.run_id,"command":report.command,"status":report.status,
            "exit_code":report.exit_code,"phase":report.phase,
            "duration_seconds":if report.status == "running" {self.start.elapsed().as_secs_f64()} else {report.duration_seconds},
            "context":report.context,"result":report.result,"error":report.error,
            "failure":report.failure,"output_state":report.output_state,
            "generated_files_changed":report.generated_files_changed,
            "recovered_previous_transaction":report.recovered_previous_transaction,
            "diagnostics":report.diagnostics,"events":&report.events[after.min(count)..],"event_count":count
        }))
    }
    pub fn text<'a>(&self, zh: &'a str, en: &'a str) -> &'a str {
        if self.english.load(Ordering::Relaxed) {
            en
        } else {
            zh
        }
    }
    pub fn configure(&self, lang: &str, retention: usize) {
        self.english.store(lang == "en-US", Ordering::Relaxed);
        self.retention.store(retention, Ordering::Relaxed);
    }
    pub fn emit(&self, phase: &str, level: &'static str, message: impl Into<String>) {
        let message = redact(&message.into());
        let progress_visible = !self.automation
            || self.verbose
            || level != "info"
            || (phase != "select"
                && (phase != "compile"
                    || self
                        .last_progress_ms
                        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
                            let now = self.start.elapsed().as_millis() as u64;
                            (now.saturating_sub(last) >= 250).then_some(now)
                        })
                        .is_ok()));
        if progress_visible && (!self.json || self.automation) && (level != "debug" || self.verbose)
        {
            if !self.automation
                && io::stderr().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
            {
                let color = match level {
                    "error" => 31,
                    "warning" => 33,
                    "debug" => 90,
                    _ => 32,
                };
                eprintln!("\x1b[{color}m{phase:>10}\x1b[0m  {message}");
            } else {
                eprintln!("{phase:>10}  {message}");
            }
        }
        let mut report = self.report.lock().unwrap_or_else(|e| e.into_inner());
        report.phase = phase.into();
        report.events.push(Event {
            elapsed_seconds: self.start.elapsed().as_secs_f64(),
            phase: phase.into(),
            level,
            message,
        });
    }
    pub fn info(&self, phase: &str, message: impl Into<String>) {
        self.emit(phase, "info", message);
    }
    pub fn detail(&self, phase: &str, message: impl Into<String>) {
        self.emit(phase, "debug", message);
    }
    pub fn context(&self, value: Value) {
        let mut report = self.report.lock().unwrap_or_else(|e| e.into_inner());
        let value = redact_value(value);
        if let (Some(existing), Some(patch)) = (report.context.as_object_mut(), value.as_object()) {
            existing.extend(patch.clone());
        } else {
            report.context = value;
        }
    }
    pub fn result(&self, value: Value) {
        self.report.lock().unwrap_or_else(|e| e.into_inner()).result = redact_value(value);
    }
    fn finalize(&self, outcome: &Result<()>) {
        let mut report = self.report.lock().unwrap_or_else(|e| e.into_inner());
        report.duration_seconds = self.start.elapsed().as_secs_f64();
        report.status = if outcome.is_ok() { "success" } else { "failed" }.into();
        report.exit_code = exit_code(outcome);
        if !self.json {
            eprintln!(
                "{:>10}  {} | {:.3}s",
                if outcome.is_ok() {
                    "finished"
                } else {
                    "aborted"
                },
                self.text(
                    if outcome.is_ok() {
                        "完成"
                    } else {
                        "失败，中止执行"
                    },
                    if outcome.is_ok() {
                        "Completed"
                    } else {
                        "Failed; aborted"
                    }
                ),
                report.duration_seconds
            );
        }
        if let Err(error) = outcome {
            let message = redact(&format!("{error:#}"));
            report.diagnostics = diagnostics(&message);
            if let Some(proto) = report.context["proto_root"].as_str().map(PathBuf::from) {
                for diagnostic in &mut report.diagnostics {
                    if Path::new(&diagnostic.file).is_relative() {
                        diagnostic.file = proto.join(&diagnostic.file).display().to_string();
                    }
                }
            }
            report.failure = Some(failure::Failure::from_error(error, &report.diagnostics));
            report.error = Some(message);
        }
    }
    /// Read-only discovery/report queries must not rotate away the report being read.
    pub fn finish_in_memory(&self, outcome: &Result<()>) -> Value {
        self.finalize(outcome);
        self.response(None)
    }
    /// Finalize a complete internal report without persisting or rendering it.
    pub fn finish_snapshot(&self, outcome: &Result<()>) -> Result<Value> {
        self.finalize(outcome);
        self.snapshot(0)
    }
    pub fn finish(&self, outcome: &Result<()>, explicit: Option<&Path>) -> Result<PathBuf> {
        self.finalize(outcome);
        let report = self.report.lock().unwrap_or_else(|e| e.into_inner());
        let dir = util::data_dir()?.join("logs");
        let path = dir.join(format!("{}.json", report.run_id));
        fs::create_dir_all(&dir)?;
        util::write_json(&path, &*report)?;
        let log = report
            .events
            .iter()
            .map(|e| {
                format!(
                    "{:.3} [{}] [{}] {}\n",
                    e.elapsed_seconds, e.phase, e.level, e.message
                )
            })
            .collect::<String>();
        util::atomic_write(
            &path.with_extension("log"),
            format!("{log}{}", report.error.as_deref().unwrap_or("")).as_bytes(),
        )?;
        if let Some(explicit) = explicit {
            util::write_json(explicit, &*report)?;
        }
        if self.json && self.print_json {
            println!("{}", serde_json::to_string_pretty(&*report)?);
        }
        drop(report);
        rotate(&dir, self.retention.load(Ordering::Relaxed).max(1))?;
        Ok(path)
    }
}
pub fn diagnostics(text: &str) -> Vec<Diagnostic> {
    let re =
        regex::Regex::new(r"(?m)^(?:\[[A-Z_]+\]\s*)?([^\r\n]*?\.proto):(\d+)(?::(\d+))?:\s*(.*)$")
            .expect("constant regex");
    re.captures_iter(text)
        .filter_map(|c| {
            Some(Diagnostic {
                file: c[1].to_owned(),
                line: c[2].parse().ok()?,
                column: c.get(3).and_then(|v| v.as_str().parse().ok()),
                message: c[4].to_owned(),
            })
        })
        .collect()
}
fn redact(text: &str) -> String {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"(?i)(https?://)[^/\s:@]+:[^/\s@]+@").expect("constant regex")
    });
    re.replace_all(text, "${1}<redacted>@").into_owned()
}
fn redact_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_value).collect()),
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .map(|(k, v)| {
                    let hidden = ["password", "token", "secret", "authorization"]
                        .contains(&k.to_lowercase().as_str());
                    (
                        k,
                        if hidden {
                            Value::String("<redacted>".into())
                        } else {
                            redact_value(v)
                        },
                    )
                })
                .collect(),
        ),
        other => other,
    }
}
pub fn exit_code(outcome: &Result<()>) -> i32 {
    let Err(error) = outcome else {
        return 0;
    };
    let text = format!("{error:#}");
    for (marker, code) in [
        ("E_CANCELLED", 130),
        ("E_CONFIG", 2),
        ("E_ARGUMENT", 2),
        ("E_NO_MATCH", 3),
        ("E_SVN", 4),
        ("E_TOOLCHAIN", 5),
        ("E_INTEGRITY", 6),
        ("E_SIGNATURE", 6),
        ("E_TRUST_KEY", 6),
        ("E_OUTPUT_CONFLICT", 7),
        ("E_LOCKED", 8),
        ("E_COMPAT", 9),
    ] {
        if text.contains(marker) {
            return code;
        }
    }
    1
}
fn rotate(dir: &Path, keep: usize) -> Result<()> {
    let mut reports = fs::read_dir(dir)?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|x| x.path().extension().is_some_and(|x| x == "json"))
        .collect::<Vec<_>>();
    reports.sort_by_key(|e| e.file_name());
    let mut total = reports
        .iter()
        .map(|r| {
            r.metadata().map(|m| m.len()).unwrap_or(0)
                + fs::metadata(r.path().with_extension("log"))
                    .map(|m| m.len())
                    .unwrap_or(0)
        })
        .sum::<u64>();
    let remove = reports.len().saturating_sub(keep);
    for (index, item) in reports.into_iter().enumerate() {
        if index >= remove && total <= 100 * 1024 * 1024 {
            break;
        }
        total = total.saturating_sub(
            item.metadata().map(|m| m.len()).unwrap_or(0)
                + fs::metadata(item.path().with_extension("log"))
                    .map(|m| m.len())
                    .unwrap_or(0),
        );
        remove_if_present(&item.path()).context("rotate report")?;
        let log = item.path().with_extension("log");
        remove_if_present(&log)?;
    }
    Ok(())
}
fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finish_snapshot_keeps_the_complete_internal_contract() {
        let report = Reporter::new("report", true, false, "en-US");
        report.info("read", "Loaded the requested report");
        report.result(serde_json::json!({"items":[1,2,3]}));
        let value = report.finish_snapshot(&Ok(())).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["status"], "success");
        assert_eq!(value["exit_code"], 0);
        assert_eq!(value["events"].as_array().unwrap().len(), 1);
        assert_eq!(value["result"]["items"], serde_json::json!([1, 2, 3]));
        assert!(value.get("api_version").is_none());
        assert!(value.get("report").is_none());
    }

    #[test]
    fn web_snapshot_preserves_failure_and_uncertain_recovery_state() {
        let report = Reporter::for_web("build", "en-US", 20);
        report.info("recovery", "Restoring an interrupted build");
        report.outputs("recovering", None);
        report.outputs("recovered", None);
        let error = anyhow::anyhow!("[E_OUTPUT_SCOPE] overlapping output roots");
        {
            let mut stored = report.report.lock().unwrap();
            stored.status = "failed".into();
            stored.failure = Some(failure::Failure::from_error(&error, &[]));
            stored.error = Some(error.to_string());
        }
        let snapshot = report.snapshot(1).unwrap();
        assert_eq!(snapshot["failure"]["kind"], "invalid_paths");
        assert_eq!(snapshot["failure"]["suggested_action"], "correct_paths");
        assert_eq!(snapshot["output_state"], "recovered");
        assert!(snapshot["generated_files_changed"].is_null());
        assert_eq!(snapshot["recovered_previous_transaction"], true);
        assert_eq!(snapshot["event_count"], 1);
        assert!(snapshot["events"].as_array().unwrap().is_empty());
    }
    #[test]
    fn web_snapshot_distinguishes_committed_outputs_from_unwritten_outputs() {
        let report = Reporter::for_web("build", "en-US", 20);
        assert_eq!(report.snapshot(0).unwrap()["output_state"], "not_written");
        report.outputs("committed", Some(true));
        let snapshot = report.snapshot(0).unwrap();
        assert_eq!(snapshot["output_state"], "committed");
        assert_eq!(snapshot["generated_files_changed"], true);
        assert_eq!(snapshot["recovered_previous_transaction"], false);
    }
    #[test]
    fn diagnostics_accept_windows_drive_and_column() {
        let d = diagnostics("D:/路径/sample.proto:12:4: invalid field\ncmd/a.proto:2: bad enum\n[E_PROTOCOL] D:/project/proto/a.proto:8: duplicate protocol ID");
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].line, 12);
        assert_eq!(d[0].column, Some(4));
        assert_eq!(d[2].file, "D:/project/proto/a.proto");
    }
    #[test]
    fn credentials_are_redacted_in_json_values() {
        let value = redact_value(
            serde_json::json!({"proxy":"https://alice:password@host:1234","token":"private"}),
        );
        assert_eq!(value["proxy"], "https://<redacted>@host:1234");
        assert_eq!(value["token"], "<redacted>");
    }
}
