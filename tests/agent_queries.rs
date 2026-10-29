use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn fixture(root: &Path) {
    for (group, package, metadata) in [
        ("cmd", "cmd", ""),
        ("act7001", "act7001", ""),
        ("custom", "example", "// activity-id: 7002\n"),
        ("rpc5001", "rpc5001", ""),
    ] {
        fs::create_dir_all(root.join("proto").join(group)).unwrap();
        fs::write(
            root.join("proto").join(group).join("example.proto"),
            format!("syntax=\"proto3\";\npackage {package};\n{metadata}option go_package=\"example/protocol/{group}\";\nmessage Request {{string name=1;}}\n"),
        ).unwrap();
    }
    fs::write(root.join("proto-builder.toml"), "config_version=1\nserver_path='server'\ncalc_path='calc'\ndocs=true\nprotocol_config=false\ncopy_to_calc=false\n[svn]\nupdate=false\n[update]\nenabled=false\n").unwrap();
}

fn run(root: &Path, operation: &str, params: Value) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proto-builder"));
    command
        .current_dir(root)
        .env("PROTO_BUILDER_HOME", root.join("user-data"));
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name.starts_with("ENV_")
            || (name.starts_with("PROTO_BUILDER_") && name != "PROTO_BUILDER_HOME")
        {
            command.env_remove(key);
        }
    }
    let mut child = command
        .args(["agent", "run", "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let body = json!({"api_version":2,"operation":operation,"params":params});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&body).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(value["api"], "proto-builder.agent");
    assert_eq!(value["api_version"], 2);
    assert_eq!(
        value["exit_code"].as_i64(),
        output.status.code().map(i64::from)
    );
    assert!(!output.stdout.contains(&0x1b));
    value
}

#[test]
fn catalog_pages_share_scan_metadata_and_query_does_not_change_selection() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let first = run(
        root,
        "catalog",
        json!({"options":{"repo_root":root},"limit":2}),
    );
    assert_eq!(first["status"], "success", "{first}");
    let result = &first["result"];
    assert_eq!(result["total"], 4);
    assert_eq!(result["items"].as_array().unwrap().len(), 2);
    assert_eq!(result["next_offset"], 2);
    assert_eq!(result["documentation_proto_files"], 3);
    assert_eq!(
        result["classification_counts"],
        json!({"shared":1,"activity":2,"machine":1,"combined":0})
    );
    assert_eq!(
        result["facets"]["activity_ids"]["items"],
        json!(["7001", "7002"])
    );
    assert_eq!(result["facets"]["machine_ids"]["items"], json!(["5001"]));
    let second = run(
        root,
        "catalog",
        json!({"options":{"repo_root":root},"offset":2,"limit":2}),
    );
    assert_eq!(
        second["result"]["items"][0]["relative"],
        "custom/example.proto"
    );
    assert_eq!(second["result"]["items"][0]["activity_id"], "7002");
    assert!(second["result"]["next_offset"].is_null());
    let filtered = run(
        root,
        "catalog",
        json!({"options":{"repo_root":root,"activity_ids":["7002"]},"query":"CUSTOM"}),
    );
    assert_eq!(filtered["result"]["total"], 1);
    assert_eq!(filtered["result"]["selected_total"], 2);
    assert_eq!(filtered["result"]["documentation_proto_files"], 3);
    assert_eq!(filtered["result"]["query_scope"], "catalog_display_only");
    let empty = run(
        root,
        "catalog",
        json!({"options":{"repo_root":root},"query":"does-not-exist"}),
    );
    assert_eq!(empty["status"], "success");
    assert_eq!(empty["result"]["total"], 0);
    assert_eq!(empty["result"]["selected_total"], 4);
    assert!(empty["result"]["next_offset"].is_null());
    let beyond = run(
        root,
        "catalog",
        json!({"options":{"repo_root":root},"offset":usize::MAX}),
    );
    assert_eq!(beyond["result"]["items"], json!([]));
    assert!(beyond["result"]["next_offset"].is_null());
    assert!(!root.join("server").exists());
    assert!(!root.join("proto/protocol.html").exists());
}

#[test]
fn query_errors_are_structured_and_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    for limit in [0, 501] {
        let value = run(
            root,
            "catalog",
            json!({"options":{"repo_root":root},"limit":limit}),
        );
        assert_eq!(value["error"]["code"], "E_ARGUMENT", "{value}");
    }
    let long_query = "x".repeat(257);
    let value = run(
        root,
        "catalog",
        json!({"options":{"repo_root":root},"query":long_query}),
    );
    assert_eq!(value["error"]["code"], "E_ARGUMENT");
    let value = run(
        root,
        "catalog",
        json!({"options":{"repo_root":root,"activity_ids":["9999"]}}),
    );
    assert_eq!(value["error"]["code"], "E_NO_MATCH");
}

#[test]
fn full_report_sections_are_paged_without_rotating_source_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let plan = run(
        root,
        "plan",
        json!({"options":{"repo_root":root,"activity_ids":["7001"],"excludes":["rpc5001"]}}),
    );
    assert_eq!(plan["status"], "success", "{plan}");
    let path = plan["report"]["path"].as_str().unwrap();
    let before = fs::read(path).unwrap();
    let first = run(
        root,
        "report_read",
        json!({"path":path,"section":"selected","limit":1}),
    );
    assert_eq!(first["status"], "success", "{first}");
    assert_eq!(first["result"]["total"], 2);
    assert_eq!(first["result"]["next_offset"], 1);
    assert_eq!(first["result"]["source"]["run_id"], plan["run"]["id"]);
    assert_eq!(first["result"]["source"]["output_state"], "not_written");
    let fingerprint = first["result"]["source"]["sha256"].clone();
    for section in [
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
    ] {
        let value = run(
            root,
            "report_read",
            json!({"path":path,"section":section,"limit":1}),
        );
        assert_eq!(value["status"], "success", "{value}");
        assert_eq!(value["result"]["source"]["sha256"], fingerprint);
        assert!(value["result"]["items"].as_array().unwrap().len() <= 1);
        let empty = run(
            root,
            "report_read",
            json!({"path":path,"section":section,"offset":usize::MAX}),
        );
        assert_eq!(empty["result"]["total"], value["result"]["total"]);
        assert_eq!(empty["result"]["items"], json!([]));
        assert!(empty["result"]["next_offset"].is_null());
    }
    // Queries, including invalid parameters, cannot rotate their source report away.
    for _ in 0..22 {
        assert_eq!(
            run(root, "report_read", json!({"path":path}))["status"],
            "success"
        );
        let invalid = run(root, "report_read", json!({"path":path,"section":"typo"}));
        assert_eq!(invalid["error"]["code"], "E_ARGUMENT");
        assert!(invalid["report"]["path"].is_null());
    }
    assert_eq!(fs::read(path).unwrap(), before);
    let mut synthetic: Value = serde_json::from_slice(&before).unwrap();
    synthetic["command"] = json!("build");
    synthetic["output_state"] = json!("committed");
    synthetic["result"] = json!({"changes":{"added":["z.pb.go","a.pb.go"],"modified":["m.pb.go"],"removed":[],"unchanged":[],"adopted":["m.pb.go"],"backup":"backup"}});
    let changes_path = root.join("changes.json");
    fs::write(&changes_path, serde_json::to_vec(&synthetic).unwrap()).unwrap();
    let changes = run(
        root,
        "report_read",
        json!({"path":changes_path,"section":"changes","limit":3}),
    );
    assert_eq!(changes["result"]["total"], 4);
    assert_eq!(
        changes["result"]["items"],
        json!([{"kind":"added","file":"a.pb.go"},{"kind":"added","file":"z.pb.go"},{"kind":"modified","file":"m.pb.go"}])
    );
    assert_eq!(changes["result"]["next_offset"], 3);
    assert_eq!(changes["result"]["source"]["output_state"], "committed");
    assert_ne!(changes["result"]["source"]["sha256"], fingerprint);
    let second = run(
        root,
        "report_read",
        json!({"path":changes_path,"section":"changes","offset":1,"limit":2}),
    );
    assert_eq!(
        second["result"]["items"],
        json!([{"kind":"added","file":"z.pb.go"},{"kind":"modified","file":"m.pb.go"}])
    );
    let empty = run(
        root,
        "report_read",
        json!({"path":changes_path,"section":"changes","offset":usize::MAX}),
    );
    assert_eq!(empty["result"]["total"], 4);
    assert_eq!(empty["result"]["items"], json!([]));
    assert!(empty["result"]["next_offset"].is_null());
}

#[test]
fn report_summary_bounds_conflicts_without_changing_or_rotating_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let logs = root.join("user-data/logs");
    fs::create_dir_all(&logs).unwrap();
    let path = logs.join("original.json");
    let files = (0..1500)
        .map(|index| format!("server/protocol/act{index}/example.pb.go"))
        .collect::<Vec<_>>();
    let conflicts = files
        .iter()
        .map(|file| json!({"file":file,"kind":"modified_stale_output"}))
        .collect::<Vec<_>>();
    let original_error = format!(
        "[E_OUTPUT_CONFLICT] {}",
        "尚未接管的协议文件\n".repeat(2000)
    );
    let full = json!({
        "schema_version":1,"tool":"proto-builder","version":"1.3.0-dev.1",
        "run_id":"original","command":"build","status":"failed","exit_code":7,
        "duration_seconds":1.25,"context":{},"result":{},"events":[],"diagnostics":[],
        "output_state":"not_written","generated_files_changed":false,"recovered_previous_transaction":false,
        "error":original_error,
        "failure":{"code":"E_OUTPUT_CONFLICT","kind":"modified_stale_output","retryable":false,"suggested_action":"review_existing_outputs","files":files,"conflicts":conflicts}
    });
    let bytes = serde_json::to_vec_pretty(&full).unwrap();
    let fingerprint = hex::encode(Sha256::digest(&bytes));
    fs::write(&path, &bytes).unwrap();
    fs::write(logs.join("original.log"), "retained original log").unwrap();

    // More than the normal retention count of queries must never rotate reports.
    for _ in 0..22 {
        let value = run(root, "report_read", json!({"path":path}));
        assert_eq!(value["status"], "success", "{value}");
        assert_eq!(value["result"]["source"]["status"], "failed");
        assert_eq!(value["result"]["source"]["output_state"], "not_written");
        assert_eq!(value["result"]["source"]["sha256"], fingerprint);
        assert!(value["report"]["path"].is_null());
        let summary = &value["result"]["items"][0];
        assert_eq!(summary["exit_code"], 7);
        let failure = &summary["failure"];
        assert_eq!(failure["code"], "E_OUTPUT_CONFLICT");
        assert_eq!(failure["kind"], "modified_stale_output");
        assert_eq!(failure["retryable"], false);
        assert_eq!(failure["suggested_action"], "review_existing_outputs");
        for field in ["files", "conflicts"] {
            assert_eq!(failure[field].as_array().unwrap().len(), 20);
            assert_eq!(failure[format!("{field}_total")], 1500);
            assert_eq!(failure[format!("{field}_truncated")], true);
            assert_eq!(failure[field][0], full["failure"][field][0]);
            assert_eq!(failure[field][19], full["failure"][field][19]);
        }
        let excerpt = summary["error"].as_str().unwrap();
        assert_eq!(excerpt.chars().count(), 1201);
        assert_eq!(
            excerpt,
            format!("{}…", original_error.chars().take(1200).collect::<String>())
        );
        assert_eq!(summary["error_truncated"], true);
    }
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        fs::read_to_string(logs.join("original.log")).unwrap(),
        "retained original log"
    );
    assert_eq!(fs::read_dir(&logs).unwrap().count(), 2);
}

#[test]
fn report_reader_rejects_wrong_formats_unknown_sections_and_oversized_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let plan = run(root, "plan", json!({"options":{"repo_root":root}}));
    let report_path = plan["report"]["path"].as_str().unwrap();
    let path = root.join("invalid.json");
    let mut unsupported: Value = serde_json::from_slice(&fs::read(report_path).unwrap()).unwrap();
    unsupported["schema_version"] = json!(2);
    for bytes in [
        b"{".to_vec(),
        serde_json::to_vec(&plan).unwrap(),
        serde_json::to_vec(&unsupported).unwrap(),
    ] {
        fs::write(&path, bytes).unwrap();
        let value = run(root, "report_read", json!({"path":path}));
        assert_eq!(value["error"]["code"], "E_ARGUMENT", "{value}");
    }
    fs::File::create(&path)
        .unwrap()
        .set_len(32 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        run(root, "report_read", json!({"path":path}))["error"]["code"],
        "E_ARGUMENT"
    );
    for params in [
        json!({"path":report_path,"section":"everything"}),
        json!({"path":report_path,"limit":0}),
    ] {
        assert_eq!(
            run(root, "report_read", params)["error"]["code"],
            "E_ARGUMENT"
        );
    }
}
