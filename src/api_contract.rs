//! Transport-neutral Agent v2 contract and operation registry.
use crate::{build::BuildPolicy, config::ConfigOverrides};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

pub const API_VERSION: u32 = 2;
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Capabilities,
    Catalog,
    ConfigShow,
    ConfigCheck,
    Doctor,
    Plan,
    Build,
    ReportRead,
}
impl Kind {
    pub const ALL: [Self; 8] = [
        Self::Capabilities,
        Self::Catalog,
        Self::ConfigShow,
        Self::ConfigCheck,
        Self::Doctor,
        Self::Plan,
        Self::Build,
        Self::ReportRead,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::Catalog => "catalog",
            Self::ConfigShow => "config_show",
            Self::ConfigCheck => "config_check",
            Self::Doctor => "doctor",
            Self::Plan => "plan",
            Self::Build => "build",
            Self::ReportRead => "report_read",
        }
    }
    pub fn persists_report(self) -> bool {
        !matches!(self, Self::Capabilities | Self::ReportRead)
    }
    pub fn schema(self) -> Value {
        let mut props = serde_json::Map::new();
        if !matches!(self, Self::Capabilities | Self::ReportRead) {
            props.insert(
                "options".into(),
                options_schema(matches!(self, Self::Build)),
            );
            props.insert("verbose".into(), json!({"type":"boolean","default":false}));
        }
        if matches!(self, Self::Catalog | Self::ReportRead) {
            props.insert(
                "offset".into(),
                json!({"type":"integer","minimum":0,"maximum":usize::MAX,"default":0}),
            );
            props.insert("limit".into(),json!({"type":"integer","minimum":1,"maximum":crate::agent_queries::MAX_PAGE_SIZE,"default":DEFAULT_PAGE_SIZE}));
        }
        if matches!(self, Self::Catalog) {
            props.insert("query".into(),json!({"type":"string","maxLength":256,"description":"Substring in path/group/ID. Filters catalog display, never the build scope."}));
        }
        if matches!(self, Self::ReportRead) {
            props.insert("path".into(),json!({"type":"string","minLength":1,"description":"Exact complete report path returned by an earlier operation."}));
            props.insert("section".into(),json!({"type":"string","enum":crate::agent_queries::REPORT_SECTIONS,"default":"summary"}));
        }
        if matches!(self, Self::Build) {
            props.insert("policy".into(),json!({"type":"object","additionalProperties":false,"properties":{
                "force":{"type":"boolean","default":false,"description":"Replace protected modified non-Go or stale outputs after backup; never enable automatically to silence a conflict."},
                "adopt_existing":{"type":"boolean","default":false,"description":"Adopt protected existing non-Go outputs after backup; pb.go already uses backup-then-replace."}
            }}));
        }
        let mut schema = json!({"type":"object","additionalProperties":false,"properties":props});
        if matches!(self, Self::Build) {
            schema["required"] = json!(["options"]);
        }
        if matches!(self, Self::ReportRead) {
            schema["required"] = json!(["path"]);
        }
        schema
    }
    fn description(self) -> &'static str {
        match self {
            Self::Capabilities=>"Discover Agent v2 operations, schemas and side effects without project setup or report writes.",
            Self::Catalog=>"Page protocol metadata using the actual build filters. No generation or SVN update; saves a local report.",
            Self::ConfigShow=>"Show effective paths/configuration and per-field provenance; saves a local report.",
            Self::ConfigCheck=>"Validate configuration without compiling or updating SVN; saves a local report.",
            Self::Doctor=>"Check tools, paths, output layout and permissions. May prepare toolchain cache and probe permissions; no protocol generation or SVN update.",
            Self::Plan=>"Preview selected Go and full shared-document scope without generation or SVN update. Saves a local report; not a frozen execution plan.",
            Self::Build=>"Build using normal SVN, snapshot, backup and transactional output policies. Requires explicit absolute options.repo_root. Check output state even on failure before retrying.",
            Self::ReportRead=>"Read an immutable section/page of a complete report without writing or rotating logs. Original run status is in result.source.",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    api_version: u32,
    request_id: Option<String>,
    operation: Kind,
    params: Value,
}
#[derive(Debug)]
pub struct Request {
    pub request_id: Option<String>,
    pub operation: Operation,
}
#[derive(Debug)]
pub enum Operation {
    Capabilities,
    Catalog(CatalogParams),
    ConfigShow(ProjectParams),
    ConfigCheck(ProjectParams),
    Doctor(ProjectParams),
    Plan(ProjectParams),
    Build(BuildParams),
    ReportRead(ReportParams),
}
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectParams {
    pub options: ConfigOverrides,
    pub verbose: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogParams {
    #[serde(default)]
    pub options: ConfigOverrides,
    pub query: Option<String>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "page_size")]
    pub limit: usize,
    #[serde(default)]
    pub verbose: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildParams {
    pub options: ConfigOverrides,
    #[serde(default)]
    pub policy: BuildPolicy,
    #[serde(default)]
    pub verbose: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportParams {
    pub path: PathBuf,
    #[serde(default = "summary")]
    pub section: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "page_size")]
    pub limit: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
fn page_size() -> usize {
    DEFAULT_PAGE_SIZE
}
fn summary() -> String {
    "summary".into()
}
impl Operation {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Capabilities => Kind::Capabilities,
            Self::Catalog(_) => Kind::Catalog,
            Self::ConfigShow(_) => Kind::ConfigShow,
            Self::ConfigCheck(_) => Kind::ConfigCheck,
            Self::Doctor(_) => Kind::Doctor,
            Self::Plan(_) => Kind::Plan,
            Self::Build(_) => Kind::Build,
            Self::ReportRead(_) => Kind::ReportRead,
        }
    }
    pub fn options(&self) -> Option<&ConfigOverrides> {
        match self {
            Self::Capabilities | Self::ReportRead(_) => None,
            Self::Catalog(p) => Some(&p.options),
            Self::Build(p) => Some(&p.options),
            Self::ConfigShow(p) | Self::ConfigCheck(p) | Self::Doctor(p) | Self::Plan(p) => {
                Some(&p.options)
            }
        }
    }
    pub fn verbose(&self) -> bool {
        match self {
            Self::Capabilities | Self::ReportRead(_) => false,
            Self::Catalog(p) => p.verbose,
            Self::Build(p) => p.verbose,
            Self::ConfigShow(p) | Self::ConfigCheck(p) | Self::Doctor(p) | Self::Plan(p) => {
                p.verbose
            }
        }
    }
    fn validate(&self) -> Result<()> {
        if let Some(o) = self.options() {
            for path in [
                &o.config,
                &o.repo_root,
                &o.proto_path,
                &o.server_path,
                &o.calc_path,
            ]
            .into_iter()
            .flatten()
            {
                if path.as_os_str().is_empty() {
                    bail!("[E_ARGUMENT] paths must not be empty");
                }
            }
            if o.developer_name
                .as_ref()
                .is_some_and(|v| v.trim().is_empty() || v.chars().any(char::is_control))
            {
                bail!("[E_ARGUMENT] developer_name must be nonempty without control characters");
            }
            for values in [&o.activity_ids, &o.machine_ids, &o.excludes]
                .into_iter()
                .flatten()
            {
                if values.len() > 1000 || values.iter().any(|v| v.trim().is_empty()) {
                    bail!("[E_ARGUMENT] filters require at most 1000 nonempty strings; [] explicitly clears a filter");
                }
            }
            for ids in [&o.activity_ids, &o.machine_ids].into_iter().flatten() {
                if ids.iter().any(|v| !v.bytes().all(|c| c.is_ascii_digit())) {
                    bail!("[E_ARGUMENT] activity_ids and machine_ids must contain numeric strings");
                }
            }
            if o.lang
                .as_ref()
                .is_some_and(|v| !["zh-CN", "en-US"].contains(&v.as_str()))
                || o.jobs.is_some_and(|v| v > 256)
                || o.timeout_seconds == Some(0)
            {
                bail!("[E_ARGUMENT] invalid lang, jobs (0..256), or timeout_seconds (>0)");
            }
        }
        let limit = match self {
            Self::Catalog(p) => Some(p.limit),
            Self::ReportRead(p) => Some(p.limit),
            _ => None,
        };
        if limit.is_some_and(|v| v == 0 || v > crate::agent_queries::MAX_PAGE_SIZE) {
            bail!("[E_ARGUMENT] limit must be between 1 and 500");
        }
        match self {
            Self::Build(p) if !p.options.repo_root.as_ref().is_some_and(|v|v.is_absolute())=>bail!("[E_ARGUMENT] build requires params.options.repo_root as an explicit absolute protocol project path"),
            Self::Catalog(p) if p.query.as_ref().is_some_and(|v|v.chars().count()>256)=>bail!("[E_ARGUMENT] query exceeds 256 characters"),
            Self::ReportRead(p) if p.path.as_os_str().is_empty() || !crate::agent_queries::REPORT_SECTIONS.contains(&p.section.as_str())=>bail!("[E_ARGUMENT] invalid report path or section"),
            _=>Ok(())
        }
    }
}
fn reject_null(value: &Value) -> Result<()> {
    match value {
        Value::Null => {
            bail!("[E_ARGUMENT] null is not accepted; omit optional fields to inherit defaults")
        }
        Value::Array(v) => {
            for item in v {
                reject_null(item)?;
            }
        }
        Value::Object(v) => {
            for item in v.values() {
                reject_null(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn parse(value: Value) -> Result<Request> {
    reject_null(&value)?;
    let wire:WireRequest=serde_json::from_value(value).context("[E_ARGUMENT] expected Agent v2 request: api_version, operation, params, optional request_id")?;
    if wire.api_version != API_VERSION {
        bail!(
            "[E_ARGUMENT] unsupported api_version {}; supported: {API_VERSION}",
            wire.api_version
        );
    }
    if wire
        .request_id
        .as_ref()
        .is_some_and(|v| v.chars().count() > 128)
    {
        bail!("[E_ARGUMENT] request_id exceeds 128 characters");
    }
    if !wire.params.is_object() {
        bail!("[E_ARGUMENT] params must be an object");
    }
    let p = wire.params;
    let operation = match wire.operation {
        Kind::Capabilities => {
            serde_json::from_value::<Empty>(p)
                .context("[E_ARGUMENT] capabilities params must be empty")?;
            Operation::Capabilities
        }
        Kind::Catalog => Operation::Catalog(
            serde_json::from_value(p).context("[E_ARGUMENT] invalid catalog params")?,
        ),
        Kind::ConfigShow => Operation::ConfigShow(
            serde_json::from_value(p).context("[E_ARGUMENT] invalid config_show params")?,
        ),
        Kind::ConfigCheck => Operation::ConfigCheck(
            serde_json::from_value(p).context("[E_ARGUMENT] invalid config_check params")?,
        ),
        Kind::Doctor => Operation::Doctor(
            serde_json::from_value(p).context("[E_ARGUMENT] invalid doctor params")?,
        ),
        Kind::Plan => {
            Operation::Plan(serde_json::from_value(p).context("[E_ARGUMENT] invalid plan params")?)
        }
        Kind::Build => Operation::Build(
            serde_json::from_value(p).context("[E_ARGUMENT] invalid build params")?,
        ),
        Kind::ReportRead => Operation::ReportRead(
            serde_json::from_value(p).context("[E_ARGUMENT] invalid report_read params")?,
        ),
    };
    operation.validate()?;
    Ok(Request {
        request_id: wire.request_id,
        operation,
    })
}
fn options_schema(build: bool) -> Value {
    let mut p = serde_json::Map::new();
    for name in [
        "config",
        "repo_root",
        "proto_path",
        "server_path",
        "calc_path",
        "developer_name",
    ] {
        p.insert(name.into(), json!({"type":"string","minLength":1}));
    }
    p["repo_root"]["description"]=json!("Protocol project root. Build requires an explicit absolute path; output-relative paths resolve against it.");
    for name in ["activity_ids", "machine_ids", "excludes"] {
        let item = if name == "excludes" {
            json!({"type":"string","minLength":1})
        } else {
            json!({"type":"string","pattern":"^[0-9]+$"})
        };
        p.insert(name.into(),json!({"type":"array","items":item,"maxItems":1000,"description":"Omit to inherit; [] clears. Activity/machine constraints intersect; public protocols retained."}));
    }
    for name in ["docs", "protocol_config", "copy_to_calc", "svn_update"] {
        p.insert(name.into(), json!({"type":"boolean"}));
    }
    p["svn_update"]["description"]=json!("Inherit normal configuration (default true); plan/catalog never update SVN. Do not automatically disable.");
    p.insert(
        "assign_id".into(),
        json!({"type":"integer","minimum":0,"maximum":u64::MAX}),
    );
    p.insert(
        "jobs".into(),
        json!({"type":"integer","minimum":0,"maximum":256}),
    );
    p.insert(
        "timeout_seconds".into(),
        json!({"type":"integer","minimum":1,"maximum":u64::MAX}),
    );
    p.insert(
        "lang".into(),
        json!({"type":"string","enum":["zh-CN","en-US"]}),
    );
    let mut schema = json!({"type":"object","additionalProperties":false,"properties":p});
    if build {
        schema["required"] = json!(["repo_root"]);
    }
    schema
}
pub fn capabilities() -> Value {
    let operations:Vec<Value>=Kind::ALL.iter().map(|kind|json!({
        "name":kind.name(),"description":kind.description(),"input_schema":kind.schema(),
        "read_only":!matches!(kind,Kind::Build|Kind::Doctor),"destructive":matches!(kind,Kind::Build),"idempotent":!matches!(kind,Kind::Build),"open_world":matches!(kind,Kind::Build|Kind::Doctor),
        "persists_report":kind.persists_report()
    })).collect();
    let names: Vec<_> = Kind::ALL.iter().map(|k| k.name()).collect();
    json!({
        "api_version":API_VERSION,"operations":operations,"transports":["json_request","mcp_stdio"],
        "request_envelope_schema":{"type":"object","additionalProperties":false,"required":["api_version","operation","params"],"properties":{
            "api_version":{"const":API_VERSION},"request_id":{"type":"string","maxLength":128},"operation":{"type":"string","enum":names},"params":{"type":"object","description":"Validate against the selected operation.input_schema."}
        }},
        "response_schema":crate::agent::response_schema(),
        "limits":{"request_bytes":MAX_REQUEST_BYTES,"page_default":DEFAULT_PAGE_SIZE,"page_max":crate::agent_queries::MAX_PAGE_SIZE,"report_bytes":crate::agent_queries::MAX_REPORT_BYTES},
        "policy":{"selection_combination":"intersection","public_protocols":"retained","shared_documentation":"full_non_rpc_respecting_excludes","protobuf_overwrite":"generated_with_backup","svn_update":"inherit_configuration_default_true","request_id":"correlation_only_not_deduplication","plan":"read_only_scope_not_frozen_execution","mcp_concurrency":1},
        "workflow":["config_show","catalog","plan","build","report_read"]
    })
}
