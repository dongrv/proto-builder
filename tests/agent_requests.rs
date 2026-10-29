use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

fn fixture(root: &Path) {
    for group in ["cmd", "act7001", "act7002"] {
        fs::create_dir_all(root.join("proto").join(group)).unwrap();
        fs::write(
            root.join("proto").join(group).join("example.proto"),
            format!(
                "syntax=\"proto3\";\npackage {group};\noption go_package=\"example/protocol/{group}\";\nmessage Request {{string name=1;}}\n"
            ),
        )
        .unwrap();
    }
    fs::write(
        root.join("proto-builder.toml"),
        "config_version=1\nserver_path='server'\ncalc_path='calc'\ndocs=false\nprotocol_config=false\ncopy_to_calc=false\n[svn]\nupdate=false\n[update]\nenabled=false\n",
    )
    .unwrap();
}

fn run(root: &Path, args: &[&str], input: Option<&[u8]>) -> (Output, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proto-builder"));
    command
        .current_dir(root)
        .env("PROTO_BUILDER_HOME", root.join("user-data"))
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in [
        "ENV_CENTER_PROJECT",
        "ENV_PROJECT",
        "ENV_CAL_CENTER_PROJECT",
        "ENV_PROTOBUF",
        "ENV_DEVELOPER_NAME",
        "PROTO_BUILDER_PROJECT_ROOT",
        "PROTO_BUILDER_PROTO_PATH",
        "PROTO_BUILDER_SERVER_PATH",
        "PROTO_BUILDER_CALC_PATH",
        "PROTO_BUILDER_JOBS",
        "PROTO_BUILDER_LANG",
        "PROTO_BUILDER_SVN_UPDATE",
        "PROTO_BUILDER_ACTIVITY_IDS",
        "PROTO_BUILDER_MACHINE_IDS",
        "PROTO_BUILDER_EXCLUDES",
        "PROTO_BUILDER_TOOLCHAIN_MODE",
        "PROTO_BUILDER_UPDATE_CHANNEL",
    ] {
        command.env_remove(name);
    }
    let mut child = command.spawn().unwrap();
    if let Some(input) = input {
        // Oversized input can be rejected before the producer finishes writing.
        if let Err(error) = child.stdin.take().unwrap().write_all(input) {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
    }
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
    (output, value)
}

fn request(root: &Path, value: Value) -> (Output, Value) {
    run(
        root,
        &["agent", "run", "--request", "-"],
        Some(&serde_json::to_vec(&value).unwrap()),
    )
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, directory: &Path, found: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if !directory.is_dir() {
            return;
        }
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(base, &path, found);
            } else {
                found.insert(
                    path.strip_prefix(base).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

#[test]
fn capabilities_need_no_project_and_do_not_create_reports() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // Independent discovery must remain usable even beside a broken project config.
    fs::write(root.join("proto-builder.toml"), "invalid = [").unwrap();
    let (output, direct) = run(root, &["agent", "capabilities"], None);
    assert!(output.status.success(), "{direct}");
    assert_eq!(direct["operation"], "capabilities");
    for operation in ["catalog", "plan", "build", "report_read", "config_show"] {
        assert!(direct["result"].to_string().contains(operation), "{direct}");
    }
    let (output, requested) = request(
        root,
        json!({"api_version":2,"request_id":"discover-董","operation":"capabilities","params":{}}),
    );
    assert!(output.status.success(), "{requested}");
    assert_eq!(requested["request_id"], "discover-董");
    assert_eq!(requested["operation"], "capabilities");
    assert_eq!(requested["result"], direct["result"]);
    assert!(!root.join("user-data/logs").exists());
}

#[test]
fn file_and_stdin_requests_match_and_return_correlation_fields() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let body = json!({"api_version":2,"request_id":"plan-7001","operation":"plan","params":{"options":{"repo_root":root,"activity_ids":["7001"]}}});
    let filename = root.join("请求 with spaces.json");
    fs::write(&filename, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    let (output, from_file) = run(
        root,
        &["agent", "run", "--request", filename.to_str().unwrap()],
        None,
    );
    assert!(output.status.success(), "{from_file}");
    let (output, from_stdin) = request(root, body);
    assert!(output.status.success(), "{from_stdin}");
    assert_eq!(from_file["result"], from_stdin["result"]);
    assert_eq!(from_file["result"]["counts"]["selected"], 2);
    assert_eq!(from_stdin["operation"], "plan");
    assert_eq!(from_stdin["request_id"], "plan-7001");
    assert_eq!(from_stdin["output"]["state"], "not_written");
    assert!(!root.join("server").exists());
}

#[test]
fn invalid_requests_never_fall_back_to_a_build() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let invalid = [
        json!({"schema_version":1,"operation":"plan","options":{"repo_root":root}}),
        json!({"api_version":1,"operation":"plan","params":{}}),
        json!({"api_version":3,"operation":"plan","params":{}}),
        json!({"api_version":2,"operation":"buidl","params":{}}),
        json!({"api_version":2,"operation":"plan"}),
        json!({"api_version":2,"operation":"plan","params":null}),
        json!({"api_version":2,"operation":"plan","params":[]}),
        json!({"api_version":2,"operation":"plan","params":{},"unknown":true}),
        json!({"api_version":2,"operation":"plan","options":{},"params":{}}),
        json!({"api_version":2,"operation":"plan","params":{"options":{"unknown":true}}}),
        json!({"api_version":2,"operation":"plan","params":{"options":{"docs":"false"}}}),
        json!({"api_version":2,"operation":"plan","params":{"options":{"activity_ids":[7001]}}}),
        json!({"api_version":2,"operation":"plan","params":{"options":null}}),
        json!({"api_version":2,"operation":"plan","params":{"options":{"docs":null}}}),
        json!({"api_version":2,"operation":"plan","params":{},"request_id":"x".repeat(129)}),
        json!({"api_version":2,"operation":"plan","params":{"query":"7001"}}),
        json!({"api_version":2,"operation":"plan","params":{"offset":0}}),
        json!({"api_version":2,"operation":"catalog","params":{"path":"x.json"}}),
        json!({"api_version":2,"operation":"report_read","params":{"report_path":"x.json"}}),
        json!({"api_version":2,"operation":"report_read","params":{"query":"7001"}}),
        json!({"api_version":2,"operation":"build","params":{"options":{"repo_root":root,"force":true}}}),
        json!({"api_version":2,"operation":"plan","params":{"options":{"repo_root":root},"policy":{"force":true}}}),
        json!({"api_version":2,"operation":"capabilities","params":{"options":{}}}),
    ];
    for body in invalid {
        let (output, value) = request(root, body.clone());
        assert_eq!(
            output.status.code(),
            Some(2),
            "request={body}\nresponse={value}"
        );
        assert_eq!(value["error"]["code"], "E_ARGUMENT");
        assert_eq!(value["output"]["changed"], false);
    }
    for input in [b"".as_slice(), b"{", b"{} {}"] {
        let (output, value) = run(root, &["agent", "run", "--request", "-"], Some(input));
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert_eq!(value["error"]["code"], "E_ARGUMENT");
    }
    let (output, value) = run(root, &["agent", "run", "--request", "missing.json"], None);
    assert_eq!(output.status.code(), Some(2), "{value}");
    let mut oversized =
        serde_json::to_vec(&json!({"api_version":2,"operation":"capabilities","params":{}}))
            .unwrap();
    oversized.resize(1024 * 1024 + 1, b' ');
    let (output, value) = run(root, &["agent", "run", "--request", "-"], Some(&oversized));
    assert_eq!(output.status.code(), Some(2), "{value}");
    assert!(!root.join("server").exists());
}

#[test]
fn empty_arrays_clear_configuration_and_false_overrides_true() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    fs::write(root.join("proto-builder.toml"), "config_version=1\nserver_path='server'\ncalc_path='calc'\nactivity_ids=['7001']\nmachine_ids=['5001']\nexcludes=['act7002']\ndocs=true\nprotocol_config=true\ncopy_to_calc=true\n[svn]\nupdate=true\n[update]\nenabled=false\n").unwrap();
    let (_, inherited) = request(
        root,
        json!({"api_version":2,"operation":"config_show","params":{"options":{"repo_root":root}}}),
    );
    assert_eq!(
        inherited["result"]["settings"]["activity_ids"],
        json!(["7001"])
    );
    assert_eq!(
        inherited["result"]["settings"]["machine_ids"],
        json!(["5001"])
    );
    assert_eq!(
        inherited["result"]["settings"]["excludes"],
        json!(["act7002"])
    );
    let options = json!({"repo_root":root,"activity_ids":[],"machine_ids":[],"excludes":[],"docs":false,"protocol_config":false,"copy_to_calc":false,"svn_update":false});
    let (output, shown) = request(
        root,
        json!({"api_version":2,"operation":"config_show","params":{"options":options}}),
    );
    assert!(output.status.success(), "{shown}");
    for key in ["activity_ids", "machine_ids", "excludes"] {
        assert_eq!(shown["result"]["settings"][key], json!([]));
    }
    for key in ["docs", "protocol_config", "copy_to_calc"] {
        assert_eq!(shown["result"]["settings"][key], false);
    }
    assert_eq!(shown["result"]["settings"]["svn"]["update"], false);
    let (output, planned) = request(
        root,
        json!({"api_version":2,"operation":"plan","params":{"options":options}}),
    );
    assert!(output.status.success(), "{planned}");
    assert_eq!(planned["result"]["counts"]["selected"], 3);
    assert_eq!(planned["result"]["full_build"], true);
    assert_eq!(planned["context"]["policy"]["svn_update"], false);
    assert!(!root.join("server").exists());
}

#[test]
fn request_build_requires_an_explicit_absolute_project_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    for options in [json!({}), json!({"repo_root":"."}), json!({"repo_root":""})] {
        let (output, value) = request(
            root,
            json!({"api_version":2,"operation":"build","params":{"options":options}}),
        );
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert_eq!(value["error"]["code"], "E_ARGUMENT");
        assert_eq!(value["output"]["state"], "not_written");
    }
    assert!(!root.join("server").exists());
}

#[test]
fn request_build_matches_human_cli_bytes_and_repeat_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let legacy = root.join("legacy");
    let requested = root.join("requested");
    fixture(&legacy);
    fixture(&requested);
    let output = Command::new(env!("CARGO_BIN_EXE_proto-builder"))
        .current_dir(&legacy)
        .env_clear()
        .envs(std::env::vars_os().filter(|(key, _)| {
            let key = key.to_string_lossy();
            !key.starts_with("ENV_") && !key.starts_with("PROTO_BUILDER_")
        }))
        .env("PROTO_BUILDER_HOME", legacy.join("user-data"))
        .args([
            "build",
            "--json",
            "--non-interactive",
            "--activity-id",
            "7001",
        ])
        .output()
        .unwrap();
    let old: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{old}");
    assert_eq!(old["schema_version"], 1);
    let body = json!({"api_version":2,"request_id":"build-7001","operation":"build","params":{"options":{"repo_root":requested,"activity_ids":["7001"]}}});
    let (output, built) = request(&requested, body.clone());
    assert!(output.status.success(), "{built}");
    assert_eq!(built["request_id"], "build-7001");
    assert_eq!(built["operation"], "build");
    assert_eq!(built["output"]["state"], "committed");
    let expected = files(&legacy.join("server/protocol"));
    assert_eq!(expected.len(), 2);
    assert_eq!(files(&requested.join("server/protocol")), expected);
    let (output, repeated) = request(&requested, body);
    assert!(output.status.success(), "{repeated}");
    assert_eq!(repeated["output"]["changed"], false);
    assert_eq!(repeated["result"]["changes"]["unchanged"], 2);
    assert_eq!(files(&requested.join("server/protocol")), expected);
}

#[test]
fn global_settings_cannot_silently_override_a_json_request() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let body = serde_json::to_vec(
        &json!({"api_version":2,"operation":"plan","params":{"options":{"repo_root":root}}}),
    )
    .unwrap();
    for extra in [
        vec!["--force"],
        vec!["--json"],
        vec!["--non-interactive"],
        vec!["--no-svn-update"],
        vec!["--activity-id", "7001"],
        vec!["--lang", "en-US"],
        vec!["--repo-root", root.to_str().unwrap()],
    ] {
        let mut args = vec!["agent", "run", "--request", "-"];
        args.extend(extra);
        let (output, value) = run(root, &args, Some(&body));
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert_eq!(value["error"]["code"], "E_ARGUMENT");
    }
}

#[test]
fn report_pages_are_complete_and_do_not_create_or_rotate_logs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let (output, planned) = request(
        root,
        json!({"api_version":2,"operation":"plan","params":{"options":{"repo_root":root}}}),
    );
    assert!(output.status.success(), "{planned}");
    let path = Path::new(planned["report"]["path"].as_str().unwrap());
    let original = fs::read(path).unwrap();
    let saved: Value = serde_json::from_slice(&original).unwrap();
    let expected = saved["events"].as_array().unwrap();
    assert!(!expected.is_empty());
    let before = files(&root.join("user-data/logs"));
    let mut found = Vec::new();
    for offset in 0..expected.len() {
        let (output, page) = request(
            root,
            json!({"api_version":2,"operation":"report_read","params":{"path":path,"section":"events","offset":offset,"limit":1}}),
        );
        assert!(output.status.success(), "{page}");
        assert_eq!(page["result"]["total"], expected.len());
        assert!(page["report"]["path"].is_null());
        found.extend(page["result"]["items"].as_array().unwrap().iter().cloned());
        if offset + 1 < expected.len() {
            assert_eq!(page["result"]["next_offset"], offset + 1);
        } else {
            assert!(page["result"]["next_offset"].is_null());
        }
    }
    assert_eq!(&found, expected);
    assert_eq!(fs::read(path).unwrap(), original);
    assert_eq!(files(&root.join("user-data/logs")), before);
}

#[test]
fn machine_entrypoints_reject_outer_settings_and_legacy_flags() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        vec![
            "--repo-root",
            dir.path().to_str().unwrap(),
            "agent",
            "capabilities",
        ],
        vec!["agent", "capabilities", "--json"],
        vec!["agent", "capabilities", "--non-interactive"],
    ] {
        let (output, value) = run(dir.path(), &args, None);
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert_eq!(value["error"]["code"], "E_ARGUMENT");
    }
    for args in [vec!["mcp", "--agent"], vec!["build", "--agent"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_proto-builder"))
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("--agent"));
    }
    let (output, value) = run(dir.path(), &["agent", "capabilities", "--agent"], None);
    assert_eq!(output.status.code(), Some(2), "{value}");
    assert_eq!(value["error"]["code"], "E_ARGUMENT");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("--agent"));
    assert_eq!(value["output"]["changed"], false);
    assert!(!dir.path().join("server").exists());
    for option in ["--repo-root", "--server-path", "--proto_path"] {
        let output = Command::new(env!("CARGO_BIN_EXE_proto-builder"))
            .args([option, "agent", "--help"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
        assert!(serde_json::from_slice::<Value>(&output.stdout).is_err());
    }
}
