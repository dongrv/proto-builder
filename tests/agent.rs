use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

fn fixture(root: &Path) {
    fs::create_dir_all(root.join("proto/cmd")).unwrap();
    fs::write(root.join("proto/cmd/example.proto"),"syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\nmessage Request {string name=1;}\n").unwrap();
    fs::write(root.join("proto-builder.toml"),"config_version=1\nserver_path='server'\ncalc_path='calc'\ndocs=false\nprotocol_config=false\ncopy_to_calc=false\n[svn]\nupdate=false\n[update]\nenabled=false\n").unwrap();
}
fn run(root: &Path, args: &[&str], environment: &[(&str, String)]) -> (Output, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proto-builder"));
    command
        .current_dir(root)
        .stdin(Stdio::null())
        .env("PROTO_BUILDER_HOME", root.join("user-data"));
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
    let output = command
        .envs(environment.iter().map(|(k, v)| (*k, v)))
        .args(args)
        .arg("--agent")
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(value["interface"], "agent");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(
        value["exit_code"].as_i64(),
        output.status.code().map(i64::from)
    );
    assert!(!output.stdout.contains(&0x1b));
    (output, value)
}
fn full(value: &Value) -> Value {
    serde_json::from_slice(
        &fs::read(value["report_path"].as_str().expect("saved full report")).unwrap(),
    )
    .unwrap()
}

#[test]
fn parsing_and_noninteractive_errors_always_return_one_agent_object() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    for args in [
        vec![],
        vec!["buidl"],
        vec!["build", "--jobs", "NaN"],
        vec!["plan", "--interactive"],
        vec!["web"],
        vec!["plan", "--unknown-option"],
    ] {
        let (output, value) = run(root, &args, &[]);
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert_eq!(value["error"]["code"], "E_ARGUMENT");
        assert_eq!(value["generated_files_changed"], false);
        assert!(!root.join("server").exists());
    }
    for args in [vec!["--help"], vec!["--version"], vec!["version"]] {
        let (output, value) = run(root, &args, &[]);
        assert!(output.status.success(), "{value}");
        assert_eq!(value["status"], "success");
        assert!(value["error"].is_null());
    }
}

#[test]
fn provenance_records_equal_value_overrides_and_resolved_paths() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let server = root.join("team-server");
    let calc = root.join("team-calc");
    fs::create_dir(&server).unwrap();
    fs::create_dir(&calc).unwrap();
    fs::create_dir(root.join("user-data")).unwrap();
    fs::write(
        root.join("user-data/config.toml"),
        "server_path='user-server'\njobs=2\n",
    )
    .unwrap();
    let environment = [
        ("ENV_PROJECT", server.display().to_string()),
        ("ENV_PROTOBUF", root.display().to_string()),
        ("ENV_CAL_CENTER_PROJECT", calc.display().to_string()),
        ("ENV_DEVELOPER_NAME", "董荣旺".into()),
        ("PROTO_BUILDER_SERVER_PATH", server.display().to_string()),
    ];
    let (output, value) = run(
        root,
        &[
            "config",
            "show",
            "--explain",
            "--repo-root",
            root.to_str().unwrap(),
            "--server-path",
            server.to_str().unwrap(),
            "--no-docs",
        ],
        &environment,
    );
    assert!(output.status.success(), "{value}");
    let fields = &value["result"]["explanation"]["fields"];
    assert_eq!(
        fields["server_path"]["resolved_path"],
        server.to_str().unwrap()
    );
    assert_eq!(fields["server_path"]["directory_exists"], true);
    assert_eq!(fields["server_path"]["source"]["kind"], "cli");
    let history = fields["server_path"]["history"].as_array().unwrap();
    assert_eq!(
        history
            .iter()
            .map(|v| v["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "default",
            "user_config",
            "project_config",
            "environment",
            "environment",
            "cli"
        ]
    );
    assert_eq!(history[3]["name"], "ENV_PROJECT");
    assert_eq!(history[4]["name"], "PROTO_BUILDER_SERVER_PATH");
    assert_eq!(history[4]["value"], history[5]["value"]);
    assert_eq!(fields["project_root"]["source"]["name"], "--repo-root");
    assert_eq!(fields["proto_path"]["source"]["name"], "ENV_PROTOBUF");
    assert_eq!(fields["jobs"]["source"]["kind"], "user_config");
    assert_eq!(fields["docs"]["source"]["name"], "--no-docs");
    assert!(!root.join("user-data/local-setup.json").exists());
}

#[test]
fn plan_is_compact_and_never_updates_svn_or_writes_generated_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let config = root.join("proto-builder.toml");
    let before = fs::read(&config).unwrap();
    fs::write(
        &config,
        String::from_utf8(before)
            .unwrap()
            .replace("update=false", "update=true"),
    )
    .unwrap();
    let (output, value) = run(root, &["plan"], &[]);
    assert!(output.status.success(), "{value}");
    assert_eq!(value["result"]["scanned"], 1);
    assert_eq!(value["result"]["selected"], 1);
    assert_eq!(value["output_state"], "not_written");
    assert_eq!(
        full(&value)["result"]["selected"].as_array().unwrap().len(),
        1
    );
    assert!(!root.join("server").exists());
    assert!(!root.join("user-data/local-setup.json").exists());
    let (_, settings) = run(root, &["config", "show", "--explain"], &[]);
    assert_eq!(settings["result"]["settings"]["svn"]["update"], true);
}

#[test]
fn build_reports_real_changes_and_protects_modified_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let (output, first) = run(root, &["build"], &[]);
    assert!(output.status.success(), "{first}");
    assert_eq!(first["output_state"], "committed");
    assert_eq!(first["generated_files_changed"], true);
    assert_eq!(first["result"]["changes"]["added"], 1);
    assert!(first["events"].is_null());
    assert!(full(&first)["result"]["outputs"].is_array());
    assert!(String::from_utf8_lossy(&output.stderr).contains("commit"));
    let (_, repeat) = run(root, &["build"], &[]);
    assert_eq!(repeat["result"]["changes"]["unchanged"], 1);
    assert_eq!(repeat["generated_files_changed"], false);
    let target = root.join("server/protocol/cmd/example.pb.go");
    let mut edited = fs::read(&target).unwrap();
    edited.extend_from_slice(b"// developer edit\n");
    fs::write(&target, &edited).unwrap();
    let (output, blocked) = run(root, &["build"], &[]);
    assert_eq!(output.status.code(), Some(7), "{blocked}");
    assert_eq!(blocked["status"], "blocked");
    assert_eq!(blocked["error"]["kind"], "modified_output");
    assert_eq!(blocked["error"]["retryable"], false);
    assert_eq!(blocked["error"]["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        Path::new(blocked["error"]["files"][0].as_str().unwrap()),
        target
    );
    assert_eq!(blocked["generated_files_changed"], false);
    assert_eq!(fs::read(&target).unwrap(), edited);
    let (_, shown) = run(root, &["config", "show", "--explain"], &[]);
    assert_eq!(
        shown["result"]["explanation"]["fields"]["server_path"]["source"]["kind"],
        "local_setup"
    );
}

#[test]
fn conflicts_are_bounded_and_complete_evidence_is_retained() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    fs::create_dir_all(root.join("server/protocol/cmd")).unwrap();
    for index in 0..25 {
        fs::write(root.join(format!("proto/cmd/module{index}.proto")),format!("syntax=\"proto3\"; package cmd; option go_package=\"example/protocol/cmd\"; message Module{index} {{}}" )).unwrap();
        fs::write(
            root.join(format!("server/protocol/cmd/module{index}.pb.go")),
            "existing generated file",
        )
        .unwrap();
    }
    let (output, value) = run(root, &["build"], &[]);
    assert_eq!(output.status.code(), Some(7), "{value}");
    assert_eq!(value["error"]["kind"], "unowned_output");
    assert_eq!(value["error"]["conflict_count"], 25);
    assert_eq!(value["error"]["conflicts"].as_array().unwrap().len(), 20);
    assert_eq!(value["error"]["file_count"], 25);
    assert_eq!(
        full(&value)["failure"]["conflicts"]
            .as_array()
            .unwrap()
            .len(),
        25
    );
    assert_eq!(value["generated_files_changed"], false);
    assert!(!root.join("server/protocol/cmd/example.pb.go").exists());
    assert_eq!(
        fs::read_to_string(root.join("server/protocol/cmd/module24.pb.go")).unwrap(),
        "existing generated file"
    );
    assert!(!root.join("user-data/local-setup.json").exists());
}

#[test]
fn protocol_diagnostics_reference_the_original_source() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let source = root.join("proto/cmd/example.proto");
    fs::write(
        &source,
        "syntax=\"proto3\";\npackage cmd;\ninvalid syntax\n",
    )
    .unwrap();
    let (output, value) = run(root, &["build"], &[]);
    assert!(!output.status.success(), "{value}");
    assert_eq!(value["error"]["kind"], "invalid_protocols", "{value}");
    assert_eq!(
        Path::new(value["diagnostics"][0]["file"].as_str().unwrap()),
        source
    );
    assert_eq!(value["diagnostics"][0]["line"], 3);
    assert_eq!(value["generated_files_changed"], false);
    assert!(!root.join("server").exists());
}

#[test]
fn report_storage_failure_still_returns_json_and_does_not_hide_committed_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    fs::write(root.join("blocked"), "not a directory").unwrap();
    let (output, value) = run(
        root,
        &["build", "--report-json", "blocked/report.json"],
        &[],
    );
    assert_eq!(output.status.code(), Some(1), "{value}");
    assert_eq!(value["error"]["code"], "E_REPORT");
    assert_eq!(value["output_state"], "committed");
    assert_eq!(value["generated_files_changed"], true);
    assert!(value["report_path"].is_null());
    assert!(value["report_error"].is_string());
    assert!(root.join("server/protocol/cmd/example.pb.go").is_file());
    let (_, readonly) = run(
        root,
        &["version", "--report-json", "blocked/report.json"],
        &[],
    );
    assert_eq!(readonly["error"]["code"], "E_REPORT");
    assert_eq!(readonly["generated_files_changed"], false);
}

#[test]
fn skill_export_works_without_project_setup_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (output, value) = run(root, &["skill", "--output", "skills/proto-builder"], &[]);
    assert!(output.status.success(), "{value}");
    let target = root.join("skills/proto-builder/SKILL.md");
    assert_eq!(
        fs::read(&target).unwrap(),
        include_bytes!("../.agents/skills/proto-builder/SKILL.md")
    );
    fs::write(&target, "local skill customization").unwrap();
    let (output, blocked) = run(root, &["skill", "--output", "skills/proto-builder"], &[]);
    assert_eq!(output.status.code(), Some(2), "{blocked}");
    assert_eq!(
        fs::read_to_string(target).unwrap(),
        "local skill customization"
    );
}

#[test]
fn svn_failure_is_classified_without_disabling_the_configured_update() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let config = root.join("proto-builder.toml");
    fs::write(
        &config,
        fs::read_to_string(&config).unwrap().replace(
            "[svn]\nupdate=false",
            "[svn]\nupdate=true\nexecutable='no-such-svn-for-agent-test'",
        ),
    )
    .unwrap();
    let (output, value) = run(root, &["build"], &[]);
    assert_eq!(output.status.code(), Some(4), "{value}");
    assert_eq!(value["error"]["kind"], "svn_failed");
    assert_eq!(value["policy"]["svn_update"], true);
    assert_eq!(value["generated_files_changed"], false);
    assert!(!root.join("server").exists());
    let (_, shown) = run(root, &["config", "show"], &[]);
    assert_eq!(shown["result"]["settings"]["svn"]["update"], true);
}

#[cfg(windows)]
#[test]
fn long_windows_data_and_skill_paths_support_atomic_replacement() {
    use std::os::windows::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let mut data = root.join("deep-user-data");
    while data.as_os_str().encode_wide().count() < 300 {
        data.push("long-windows-directory-for-agent");
    }
    let environment = [("PROTO_BUILDER_HOME", data.display().to_string())];
    let (output, first) = run(root, &["build"], &environment);
    assert!(output.status.success(), "{first}");
    assert_eq!(first["output_state"], "committed");
    assert_eq!(first["generated_files_changed"], true);
    let target = root.join("server/protocol/cmd/example.pb.go");
    let initial = fs::read(&target).unwrap();
    let (output, repeat) = run(root, &["build"], &environment);
    assert!(output.status.success(), "{repeat}");
    assert_eq!(repeat["generated_files_changed"], false);
    assert_eq!(fs::read(&target).unwrap(), initial);
    assert_eq!(full(&repeat)["output_state"], "committed");

    let destination = data.join("skills/proto-builder");
    let args = ["skill", "--output", destination.to_str().unwrap()];
    let (output, exported) = run(root, &args, &environment);
    assert!(output.status.success(), "{exported}");
    let skill = destination.join("SKILL.md");
    assert_eq!(
        fs::read(&skill).unwrap(),
        include_bytes!("../.agents/skills/proto-builder/SKILL.md")
    );
    fs::write(&skill, "local customization").unwrap();
    let (output, blocked) = run(root, &args, &environment);
    assert_eq!(output.status.code(), Some(2), "{blocked}");
    assert_eq!(fs::read_to_string(skill).unwrap(), "local customization");
}
