//! One application invocation pipeline for JSON CLI and MCP; no CLI command emulation.
use crate::{
    api_contract::{self, Operation, Request, API_VERSION, MAX_REQUEST_BYTES},
    cli::Options,
    report::Reporter,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{self, IsTerminal, Read},
    path::Path,
};

pub fn error_output(error: anyhow::Error) -> Value {
    Reporter::for_automation("request", false, "en-US").finish_in_memory(&Err(error))
}
pub fn invoke_value(value: Value) -> Value {
    let request_id = value["request_id"]
        .as_str()
        .filter(|s| s.chars().count() <= 128)
        .map(str::to_owned);
    let operation = value["operation"]
        .as_str()
        .filter(|s| s.len() <= 64)
        .map(str::to_owned);
    match api_contract::parse(value) {
        Ok(request) => invoke(request),
        Err(e) => {
            let mut output = error_output(e);
            output["request_id"] = json!(request_id);
            output["operation"] = json!(operation);
            output
        }
    }
}
fn invoke(request: Request) -> Value {
    let operation = request.operation;
    let kind = operation.kind();
    let language = operation
        .options()
        .and_then(|o| o.lang.as_deref())
        .unwrap_or("zh-CN");
    let report = Reporter::for_automation(kind.name(), operation.verbose(), language);
    report.context(json!({"request":{"api_version":API_VERSION,"request_id":request.request_id,"operation":kind.name()}}));
    let outcome = crate::service::execute(&operation, &report);
    if kind.persists_report() {
        let storage = report.finish(&outcome, None);
        report.response(Some(&storage))
    } else {
        report.finish_in_memory(&outcome)
    }
}
pub fn read_request(mut reader: impl Read) -> Value {
    let result = (|| -> Result<Value> {
        let mut bytes = Vec::new();
        reader
            .by_ref()
            .take((MAX_REQUEST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .context("[E_ARGUMENT] cannot read JSON request")?;
        if bytes.len() > MAX_REQUEST_BYTES {
            bail!("[E_ARGUMENT] JSON request exceeds 1 MiB");
        }
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        serde_json::from_slice(bytes).context("[E_ARGUMENT] invalid JSON request")
    })();
    match result {
        Ok(v) => invoke_value(v),
        Err(e) => error_output(e),
    }
}
pub fn run_file(path: &Path, options: &Options) -> Value {
    if let Err(e) = validate_transport_options(options) {
        return error_output(e);
    }
    if path == Path::new("-") {
        if io::stdin().is_terminal() {
            return error_output(anyhow::anyhow!(
                "[E_ARGUMENT] --request - requires piped UTF-8 JSON"
            ));
        }
        read_request(io::stdin().lock())
    } else {
        match File::open(path) {
            Ok(f) => read_request(f),
            Err(e) => error_output(anyhow::anyhow!(
                "[E_ARGUMENT] cannot open request {}: {e}",
                path.display()
            )),
        }
    }
}
pub fn run_capabilities(options: &Options) -> Value {
    if let Err(e) = validate_transport_options(options) {
        return error_output(e);
    }
    invoke(Request {
        request_id: None,
        operation: Operation::Capabilities,
    })
}
pub fn validate_transport_options(options: &Options) -> Result<()> {
    if options != &Options::default() {
        bail!("[E_ARGUMENT] agent and mcp do not accept global CLI settings; supply options inside the request params");
    }
    Ok(())
}
