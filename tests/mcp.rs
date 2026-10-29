use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

struct Mcp {
    child: Child,
    input: Option<ChildStdin>,
    output: Receiver<Value>,
}

impl Mcp {
    fn start(root: &Path) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_proto-builder"));
        command
            .arg("mcp")
            .current_dir(root)
            .env("PROTO_BUILDER_HOME", root.join("user-data"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (key, _) in std::env::vars_os() {
            let name = key.to_string_lossy();
            if name.starts_with("PROTO_BUILDER_") && name != "PROTO_BUILDER_HOME"
                || [
                    "ENV_CENTER_PROJECT",
                    "ENV_PROJECT",
                    "ENV_CAL_CENTER_PROJECT",
                    "ENV_PROTOBUF",
                    "ENV_DEVELOPER_NAME",
                ]
                .contains(&name.as_ref())
            {
                command.env_remove(key);
            }
        }
        let mut child = command.spawn().unwrap();
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, output) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = line.expect("read MCP stdout");
                let value: Value = serde_json::from_str(&line)
                    .unwrap_or_else(|e| panic!("Non-protocol stdout: {line}: {e}"));
                assert_eq!(value["jsonrpc"], "2.0");
                if tx.send(value).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            output,
        }
    }

    fn write(&mut self, value: Value) {
        self.raw(&format!("{value}\n"));
    }

    fn raw(&mut self, text: &str) {
        let input = self.input.as_mut().unwrap();
        input.write_all(text.as_bytes()).unwrap();
        input.flush().unwrap();
    }

    fn read(&self) -> Value {
        self.output
            .recv_timeout(Duration::from_secs(45))
            .expect("MCP response deadline")
    }

    fn initialize(&mut self, version: &str) -> Value {
        self.write(json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"proto-builder-integration-test","version":"1"}}}));
        let response = self.read();
        assert!(response.get("error").is_none(), "{response}");
        self.write(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        response
    }

    fn call(&mut self, id: i64, tool: &str, arguments: Value) -> Value {
        self.write(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":format!("proto_builder_{tool}"),"arguments":arguments}}));
        let response = self.read();
        assert_eq!(response["id"], id, "{response}");
        response
    }

    fn finish(&mut self) {
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "MCP exit: {status}");
                return;
            }
            assert!(Instant::now() < deadline, "MCP failed to shut down");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn fixture(root: &Path, count: usize) {
    fs::create_dir_all(root.join("proto/cmd")).unwrap();
    for index in 0..count {
        fs::write(root.join(format!("proto/cmd/file{index}.proto")), format!("syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\nmessage Message{index} {{string value=1;}}\n")).unwrap();
    }
    fs::write(root.join("proto-builder.toml"), "config_version=1\nserver_path='server'\ncalc_path='calc'\ndocs=false\nprotocol_config=false\ncopy_to_calc=false\n[svn]\nupdate=false\n[update]\nenabled=false\n").unwrap();
}

#[test]
fn negotiates_versions_lists_schemas_and_returns_compatible_tool_results() {
    for version in ["2025-11-25", "2025-06-18", "2024-11-05", "2099-01-01"] {
        let dir = tempfile::tempdir().unwrap();
        let mut server = Mcp::start(dir.path());
        let initialized = server.initialize(version);
        assert_eq!(
            initialized["result"]["protocolVersion"],
            if version == "2099-01-01" {
                "2025-11-25"
            } else {
                version
            }
        );
        server.write(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
        let listed = server.read();
        let tools = listed["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
        let build = tools
            .iter()
            .find(|tool| tool["name"] == "proto_builder_build")
            .unwrap();
        assert_eq!(build["inputSchema"]["type"], "object");
        assert_eq!(build["annotations"]["readOnlyHint"], false);
        assert!(!build["inputSchema"]["properties"]
            .as_object()
            .unwrap()
            .contains_key("operation"));
        let response = server.call(2, "capabilities", json!({}));
        assert_eq!(response["result"]["isError"], false, "{response}");
        let text: Value =
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(text["api"], "proto-builder.agent");
        assert_eq!(text["api_version"], 2);
        assert_eq!(text["request_id"], "2");
        assert_eq!(text["operation"], "capabilities");
        for tool in tools {
            if version == "2024-11-05" {
                assert!(tool.get("outputSchema").is_none());
            } else {
                assert_eq!(tool["outputSchema"], text["result"]["response_schema"]);
            }
            let properties = tool["inputSchema"]["properties"].as_object().unwrap();
            for reserved in ["operation", "api_version", "request_id"] {
                assert!(!properties.contains_key(reserved));
            }
        }
        assert!(build["inputSchema"]["properties"]["policy"]["properties"]["force"].is_object());
        assert!(text.get("interface").is_none());
        assert!(text.get("schema_version").is_none());
        if version == "2024-11-05" {
            assert!(response["result"].get("structuredContent").is_none());
        } else {
            assert_eq!(response["result"]["structuredContent"], text);
        }
        server.finish();
    }
}

#[test]
fn invalid_frames_notifications_and_duplicate_ids_never_execute_tools() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), 1);
    let mut server = Mcp::start(dir.path());
    server.write(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
    assert_eq!(server.read()["error"]["code"], -32002);
    server.initialize("2025-11-25");
    server.write(json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"proto_builder_build","arguments":{"options":{"repo_root":dir.path()}}}}));
    server.write(json!({"jsonrpc":"2.0","id":2,"method":"ping"}));
    assert_eq!(server.read()["id"], 2);
    assert!(!dir.path().join("server").exists());
    server.raw("not json\n");
    assert_eq!(server.read()["error"]["code"], -32700);
    for value in [
        json!([]),
        json!({"jsonrpc":"2.0","id":null,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":2.5,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":3,"method":"ping","params":[]}),
    ] {
        server.write(value);
        assert_eq!(server.read()["error"]["code"], -32600);
    }
    server.write(json!({"jsonrpc":"2.0","id":"界".repeat(129),"method":"ping"}));
    let oversized_id = server.read();
    assert_eq!(oversized_id["error"]["code"], -32600);
    assert!(oversized_id["id"].is_null());
    // The limit counts Unicode characters, not bytes, and rejection leaves the
    // session usable without retaining or reflecting an oversized identifier.
    let boundary_id = "界".repeat(128);
    server.write(json!({"jsonrpc":"2.0","id":boundary_id,"method":"ping"}));
    let boundary = server.read();
    assert_eq!(boundary["id"], boundary_id);
    assert_eq!(boundary["result"], json!({}));
    server.write(json!({"jsonrpc":"2.0","id":4,"method":"unknown"}));
    assert_eq!(server.read()["error"]["code"], -32601);
    assert_eq!(
        server.call(5, "not_available", json!({}))["error"]["code"],
        -32602
    );
    assert_eq!(
        server.call(6, "build", json!({"operation":"capabilities"}))["error"]["code"],
        -32602
    );
    assert_eq!(
        server.call(2, "build", json!({"options":{"repo_root":dir.path()}}))["error"]["code"],
        -32600
    );
    server.raw(&format!("{}\n", "x".repeat(1024 * 1024 + 1)));
    assert_eq!(server.read()["error"]["code"], -32600);
    for (id, invalid) in [
        (8, json!({"request_id":"forged"})),
        (9, json!({"api_version":2})),
    ] {
        assert_eq!(
            server.call(id, "capabilities", invalid)["error"]["code"],
            -32602
        );
    }
    server.write(json!({"jsonrpc":"2.0","id":7,"method":"ping"}));
    assert_eq!(server.read()["result"], json!({}));
    assert!(!dir.path().join("server").exists());
    server.finish();
}

#[test]
fn tools_share_agent_validation_planning_and_paged_reports() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), 2);
    let mut server = Mcp::start(dir.path());
    server.initialize("2025-11-25");
    let invalid = server.call(
        1,
        "plan",
        json!({"options":{"repo_root":dir.path(),"unknown":true}}),
    );
    assert_eq!(invalid["result"]["isError"], true, "{invalid}");
    assert_eq!(
        invalid["result"]["structuredContent"]["error"]["code"],
        "E_ARGUMENT"
    );
    let plan = server.call(2, "plan", json!({"options":{"repo_root":dir.path()}}));
    let planned = &plan["result"]["structuredContent"];
    assert_eq!(planned["status"], "success", "{plan}");
    assert_eq!(planned["request_id"], "2");
    assert_eq!(planned["result"]["counts"]["selected"], 2);
    assert_eq!(planned["output"]["state"], "not_written");
    let report = server.call(
        3,
        "report_read",
        json!({"path":planned["report"]["path"],"section":"selected","offset":0,"limit":1}),
    );
    let page = &report["result"]["structuredContent"]["result"];
    assert_eq!(page["total"], 2, "{report}");
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["next_offset"], 1);
    server.write(json!({"jsonrpc":"2.0","id":"named-call","method":"tools/call","params":{"name":"proto_builder_capabilities","arguments":{}}}));
    let named = server.read();
    assert_eq!(named["id"], "named-call");
    assert_eq!(
        named["result"]["structuredContent"]["request_id"],
        "named-call"
    );
    assert!(!dir.path().join("server").exists());
    server.finish();
}

#[test]
fn builds_report_committed_outputs_even_when_report_storage_fails() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root, 1);
    let mut server = Mcp::start(root);
    server.initialize("2025-11-25");
    let arguments =
        json!({"options":{"repo_root":root},"policy":{"force":false,"adopt_existing":false}});
    let first = server.call(1, "build", arguments.clone());
    let built = &first["result"]["structuredContent"];
    assert_eq!(first["result"]["isError"], false, "{first}");
    assert_eq!(built["output"]["state"], "committed");
    assert_eq!(built["output"]["changed"], true);
    assert_eq!(built["result"]["changes"]["added"], 1);
    let target = root.join("server/protocol/cmd/file0.pb.go");
    let original = fs::read(&target).unwrap();

    let repeat = server.call(2, "build", arguments.clone());
    let repeated = &repeat["result"]["structuredContent"];
    assert_eq!(repeat["result"]["isError"], false, "{repeat}");
    assert_eq!(repeated["output"]["state"], "committed");
    assert_eq!(repeated["output"]["changed"], false);
    assert_eq!(repeated["result"]["changes"]["unchanged"], 1);
    assert_eq!(fs::read(&target).unwrap(), original);

    // Only this isolated fixture is changed: block its log directory after a
    // successful build, then require a real output modification on the next call.
    let logs = root.join("user-data/logs");
    fs::rename(&logs, root.join("user-data/logs.saved")).unwrap();
    fs::write(&logs, "intentional report storage failure").unwrap();
    let source = root.join("proto/cmd/file0.proto");
    let changed = fs::read_to_string(&source)
        .unwrap()
        .replace("string value=1;", "string value=1; int32 counter=2;");
    fs::write(source, changed).unwrap();

    let failed_report = server.call(3, "build", arguments);
    assert!(failed_report.get("error").is_none(), "{failed_report}");
    assert_eq!(failed_report["result"]["isError"], true, "{failed_report}");
    let committed = &failed_report["result"]["structuredContent"];
    assert_eq!(committed["error"]["code"], "E_REPORT");
    assert_eq!(committed["output"]["state"], "committed");
    assert_eq!(committed["output"]["changed"], true);
    assert_eq!(committed["result"]["changes"]["modified"], 1);
    assert!(committed["report"]["path"].is_null());
    assert!(committed["report"]["error"].is_string());
    let actual = fs::read(&target).unwrap();
    assert_ne!(actual, original);
    assert!(String::from_utf8(actual).unwrap().contains("Counter"));
    server.finish();
}

#[test]
fn cancellation_targets_only_active_call_and_busy_calls_do_not_queue_builds() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), 100);
    let mut server = Mcp::start(dir.path());
    server.initialize("2025-11-25");
    let start = json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"proto_builder_build","arguments":{"options":{"repo_root":dir.path()}}}});
    let busy = json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"proto_builder_build","arguments":{"options":{"repo_root":dir.path()}}}});
    let cancel = json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":10,"reason":"integration test"}});
    let ping = json!({"jsonrpc":"2.0","id":12,"method":"ping"});
    server.raw(&format!("{start}\n{busy}\n{cancel}\n{ping}\n"));
    let replies = [server.read(), server.read(), server.read()];
    let cancelled = replies.iter().find(|v| v["id"] == 10).unwrap();
    let rejected = replies.iter().find(|v| v["id"] == 11).unwrap();
    assert_eq!(rejected["error"]["code"], -32000, "{rejected}");
    assert_eq!(
        cancelled["result"]["structuredContent"]["error"]["code"], "E_CANCELLED",
        "{cancelled}"
    );
    assert_eq!(
        cancelled["result"]["structuredContent"]["output"]["state"],
        "not_written"
    );
    server.write(cancel);
    let after = server.call(20, "plan", json!({"options":{"repo_root":dir.path()}}));
    assert_eq!(after["result"]["isError"], false, "{after}");
    assert!(!dir.path().join("server/protocol/cmd/file0.pb.go").exists());
    server.finish();
}

#[test]
fn eof_cancels_and_joins_active_tool_before_process_exit() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), 100);
    let mut server = Mcp::start(dir.path());
    server.initialize("2025-11-25");
    server.write(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"proto_builder_build","arguments":{"options":{"repo_root":dir.path()}}}}));
    server.input.take();
    let last = server.read();
    assert_eq!(
        last["result"]["structuredContent"]["error"]["code"], "E_CANCELLED",
        "{last}"
    );
    assert!(!dir.path().join("server/protocol/cmd/file0.pb.go").exists());
    server.finish();
}
