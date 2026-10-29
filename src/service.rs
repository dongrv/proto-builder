//! Application operations shared by machine transports, independent of CLI parsing.
use crate::{
    agent_queries,
    api_contract::Operation,
    build::{self, BuildPolicy},
    config::{self, Loaded},
    report::Reporter,
};
use anyhow::Result;
use serde_json::json;

/// Record effective values once, after all configuration layers have been applied.
pub fn configure_report(loaded: &Loaded, policy: &BuildPolicy, report: &Reporter) {
    report.configure(&loaded.settings.lang, loaded.settings.log_retention);
    report.context(json!({
        "project_root":loaded.root,
        "proto_root":loaded.settings.proto(&loaded.root),
        "server_root":loaded.settings.server(&loaded.root),
        "calc_root":loaded.settings.calc(&loaded.root),
        "configuration_sources":loaded.sources,
        "selection":{
            "activity_ids":loaded.settings.activity_ids,
            "machine_ids":loaded.settings.machine_ids,
            "assign_id":loaded.settings.assign_id,
            "excludes":loaded.settings.excludes
        },
        "policy":{
            "svn_update":loaded.settings.svn.update,
            "docs":loaded.settings.docs,
            "protocol_config":loaded.settings.protocol_config,
            "copy_to_calc":loaded.settings.copy_to_calc,
            "protobuf_overwrite":"generated_with_backup",
            "force":policy.force,
            "adopt_existing":policy.adopt_existing,
            "toolchain_mode":loaded.settings.toolchain.mode
        }
    }));
}

pub fn execute(operation: &Operation, report: &Reporter) -> Result<()> {
    let default_policy = BuildPolicy::default();
    let (options, policy) = match operation {
        Operation::Capabilities => {
            report.result(crate::api_contract::capabilities());
            return Ok(());
        }
        Operation::ReportRead(params) => {
            report.result(agent_queries::read_report(
                &params.path,
                &params.section,
                params.offset,
                params.limit,
            )?);
            return Ok(());
        }
        Operation::Catalog(params) => (&params.options, &default_policy),
        Operation::Build(params) => (&params.options, &params.policy),
        Operation::ConfigShow(params)
        | Operation::ConfigCheck(params)
        | Operation::Doctor(params)
        | Operation::Plan(params) => (&params.options, &default_policy),
    };
    let loaded = config::load_overrides(options)?;
    configure_report(&loaded, policy, report);
    match operation {
        Operation::ConfigShow(_) => {
            report.result(json!({"settings":loaded.settings,"sources":loaded.sources,"project_root":loaded.root,"explanation":config::explain(&loaded)?}));
            Ok(())
        }
        Operation::ConfigCheck(_) => {
            config::validate(&loaded.settings)?;
            report.result(json!({"valid":true}));
            Ok(())
        }
        Operation::Catalog(params) => {
            report.result(agent_queries::catalog(
                &loaded,
                params.query.as_deref(),
                params.offset,
                params.limit,
            )?);
            Ok(())
        }
        Operation::Doctor(_) => build::doctor(&loaded, report),
        Operation::Plan(_) => build::plan(&loaded, report),
        Operation::Build(_) => build::run(&loaded, policy, report),
        Operation::Capabilities | Operation::ReportRead(_) => {
            unreachable!("independent operations handled above")
        }
    }
}
