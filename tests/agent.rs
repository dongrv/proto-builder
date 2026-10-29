use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

fn fixture(root: &Path) {
    fs::create_dir_all(root.join("proto/cmd")).unwrap();
    fs::write(root.join("proto/cmd/example.proto"),"syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\nmessage Request {string name=1;}\n").unwrap();
    fs::write(root.join("proto-builder.toml"),"config_version=1\nserver_path='server'\ncalc_path='calc'\ndocs=false\nprotocol_config=false\ncopy_to_calc=false\n[svn]\nupdate=false\n[update]\nenabled=false\n").unwrap();
}
fn command(root: &Path, environment: &[(&str, String)]) -> Command {
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
    command.envs(environment.iter().map(|(k, v)| (*k, v)));
    command
}
fn decode(output: Output) -> (Output, Value) {
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(
        value["exit_code"].as_i64(),
        output.status.code().map(i64::from)
    );
    assert!(!output.stdout.contains(&0x1b));
    (output, value)
}
fn run(
    root: &Path,
    operation: &str,
    params: Value,
    environment: &[(&str, String)],
) -> (Output, Value) {
    let request = json!({"api_version":2,"operation":operation,"params":params});
    let mut child = command(root, environment)
        .args(["agent", "run", "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    let (output, value) = decode(child.wait_with_output().unwrap());
    assert_eq!(value["api"], "proto-builder.agent");
    assert_eq!(value["api_version"], 2);
    (output, value)
}
fn project(root: &Path, operation: &str, environment: &[(&str, String)]) -> (Output, Value) {
    run(
        root,
        operation,
        json!({"options":{"repo_root":root}}),
        environment,
    )
}
fn human(root: &Path, args: &[&str], environment: &[(&str, String)]) -> (Output, Value) {
    let result = decode(
        command(root, environment)
            .args(args)
            .args(["--json", "--non-interactive"])
            .stdin(Stdio::null())
            .output()
            .unwrap(),
    );
    assert_eq!(result.1["schema_version"], 1);
    result
}
fn full(value: &Value) -> Value {
    serde_json::from_slice(
        &fs::read(value["report"]["path"].as_str().expect("saved full report")).unwrap(),
    )
    .unwrap()
}

#[test]
fn removed_agent_flag_and_human_help_are_not_machine_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    for args in [
        vec!["--agent"],
        vec!["build", "--agent"],
        vec!["plan", "--agent"],
        vec!["config", "show", "--agent"],
    ] {
        let output = command(root, &[])
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("--agent"));
        assert!(!root.join("server").exists());
    }
    for args in [vec!["--help"], vec!["--version"]] {
        let output = command(root, &[]).args(args).output().unwrap();
        assert!(output.status.success());
        assert!(serde_json::from_slice::<Value>(&output.stdout).is_err());
    }
    let (output, help) = decode(
        command(root, &[])
            .args(["agent", "--help"])
            .output()
            .unwrap(),
    );
    assert!(output.status.success());
    assert_eq!(help["api"], "proto-builder.agent");
    assert_eq!(help["api_version"], 2);
    assert_eq!(help["status"], "success");
    assert!(help["result"]["help"].as_str().unwrap().contains("Usage:"));
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
        "config_show",
        json!({"options":{"repo_root":root,"server_path":server,"docs":false}}),
        &environment,
    );
    assert!(output.status.success(), "{value}");
    let fields = &value["result"]["explanation"]["fields"];
    assert_eq!(
        fields["server_path"]["resolved_path"],
        server.to_str().unwrap()
    );
    assert_eq!(fields["server_path"]["directory_exists"], true);
    assert_eq!(fields["server_path"]["source"]["kind"], "request");
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
            "request"
        ]
    );
    assert_eq!(history[3]["name"], "ENV_PROJECT");
    assert_eq!(history[4]["name"], "PROTO_BUILDER_SERVER_PATH");
    assert_eq!(history[4]["value"], history[5]["value"]);
    assert_eq!(
        fields["project_root"]["source"]["name"],
        "params.options.repo_root"
    );
    assert_eq!(fields["proto_path"]["source"]["name"], "ENV_PROTOBUF");
    assert_eq!(fields["jobs"]["source"]["kind"], "user_config");
    assert_eq!(fields["docs"]["source"]["name"], "params.options.docs");
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
    let (output, value) = project(root, "plan", &[]);
    assert!(output.status.success(), "{value}");
    assert_eq!(value["result"]["counts"]["scanned"], 1);
    assert_eq!(value["result"]["counts"]["selected"], 1);
    assert_eq!(value["output"]["state"], "not_written");
    assert_eq!(
        full(&value)["result"]["selected"].as_array().unwrap().len(),
        1
    );
    assert!(!root.join("server").exists());
    assert!(!root.join("user-data/local-setup.json").exists());
    let (_, settings) = project(root, "config_show", &[]);
    assert_eq!(settings["result"]["settings"]["svn"]["update"], true);
}

#[test]
fn build_replaces_modified_go_by_default_and_preserves_backup() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let (output, first) = project(root, "build", &[]);
    assert!(output.status.success(), "{first}");
    assert_eq!(first["output"]["state"], "committed");
    assert_eq!(first["output"]["changed"], true);
    assert_eq!(first["result"]["changes"]["added"], 1);
    assert!(first["events"].is_null());
    assert!(full(&first)["result"]["outputs"].is_array());
    assert!(String::from_utf8_lossy(&output.stderr).contains("commit"));
    let (_, repeat) = project(root, "build", &[]);
    assert_eq!(repeat["result"]["changes"]["unchanged"], 1);
    assert_eq!(repeat["output"]["changed"], false);
    let target = root.join("server/protocol/cmd/example.pb.go");
    let generated = fs::read(&target).unwrap();
    let mut edited = fs::read(&target).unwrap();
    edited.extend_from_slice(b"// developer edit\n");
    fs::write(&target, &edited).unwrap();
    let (output, replaced) = project(root, "build", &[]);
    assert!(output.status.success(), "{replaced}");
    assert_eq!(replaced["output"]["changed"], true);
    assert_eq!(replaced["result"]["changes"]["modified"], 1);
    assert_eq!(
        replaced["context"]["policy"]["protobuf_overwrite"],
        "generated_with_backup"
    );
    assert_eq!(fs::read(&target).unwrap(), generated);
    let backup = Path::new(replaced["result"]["backup"].as_str().unwrap());
    let journal: Value =
        serde_json::from_slice(&fs::read(backup.join("index.json")).unwrap()).unwrap();
    let saved = journal["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|op| Path::new(op["target"].as_str().unwrap()) == target)
        .unwrap();
    assert_eq!(fs::read(saved["backup"].as_str().unwrap()).unwrap(), edited);
    let (_, shown) = project(root, "config_show", &[]);
    assert_eq!(
        shown["result"]["explanation"]["fields"]["server_path"]["source"]["kind"],
        "local_setup"
    );
}

#[test]
fn stale_conflicts_are_bounded_and_complete_evidence_is_retained() {
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
    let (output, adopted) = project(root, "build", &[]);
    assert!(output.status.success(), "{adopted}");
    assert_eq!(adopted["result"]["changes"]["adopted"], 25);
    for index in 0..25 {
        fs::remove_file(root.join(format!("proto/cmd/module{index}.proto"))).unwrap();
        fs::write(
            root.join(format!("server/protocol/cmd/module{index}.pb.go")),
            "edited stale output",
        )
        .unwrap();
    }
    let generated = fs::read(root.join("server/protocol/cmd/example.pb.go")).unwrap();
    let (output, value) = project(root, "build", &[]);
    assert_eq!(output.status.code(), Some(7), "{value}");
    assert_eq!(value["error"]["kind"], "modified_stale_output");
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
    assert_eq!(value["output"]["changed"], false);
    assert_eq!(
        fs::read(root.join("server/protocol/cmd/example.pb.go")).unwrap(),
        generated
    );
    assert_eq!(
        fs::read_to_string(root.join("server/protocol/cmd/module24.pb.go")).unwrap(),
        "edited stale output"
    );
}

#[test]
fn other_outputs_still_require_explicit_adoption_and_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    fs::write(
        root.join("proto-builder.toml"),
        fs::read_to_string(root.join("proto-builder.toml"))
            .unwrap()
            .replace("docs=false", "docs=true"),
    )
    .unwrap();
    let html = root.join("proto/protocol.html");
    fs::write(&html, "existing documentation").unwrap();
    let (output, blocked) = project(root, "build", &[]);
    assert_eq!(output.status.code(), Some(7), "{blocked}");
    assert_eq!(blocked["error"]["kind"], "unowned_output");
    assert_eq!(fs::read_to_string(&html).unwrap(), "existing documentation");
    assert!(!root.join("server/protocol/cmd/example.pb.go").exists());
    let (output, adopted) = run(
        root,
        "build",
        json!({"options":{"repo_root":root},"policy":{"adopt_existing":true}}),
        &[],
    );
    assert!(output.status.success(), "{adopted}");
    fs::write(&html, "manually edited documentation").unwrap();
    let (output, blocked) = project(root, "build", &[]);
    assert_eq!(output.status.code(), Some(7), "{blocked}");
    assert_eq!(blocked["error"]["kind"], "modified_output");
    assert_eq!(
        fs::read_to_string(&html).unwrap(),
        "manually edited documentation"
    );
    let (output, replaced) = run(
        root,
        "build",
        json!({"options":{"repo_root":root},"policy":{"force":true}}),
        &[],
    );
    assert!(output.status.success(), "{replaced}");
    assert_ne!(
        fs::read_to_string(&html).unwrap(),
        "manually edited documentation"
    );
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
    let (output, value) = project(root, "build", &[]);
    assert!(!output.status.success(), "{value}");
    assert_eq!(value["error"]["kind"], "invalid_protocols", "{value}");
    assert_eq!(
        Path::new(value["diagnostics"]["items"][0]["file"].as_str().unwrap()),
        source
    );
    assert_eq!(value["diagnostics"]["items"][0]["line"], 3);
    assert_eq!(value["output"]["changed"], false);
    assert!(!root.join("server").exists());
}

#[test]
fn report_storage_failure_still_returns_json_and_does_not_hide_committed_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    fs::create_dir_all(root.join("user-data")).unwrap();
    fs::write(root.join("user-data/logs"), "not a directory").unwrap();
    let (output, value) = project(root, "build", &[]);
    assert_eq!(output.status.code(), Some(1), "{value}");
    assert_eq!(value["error"]["code"], "E_REPORT");
    assert_eq!(value["output"]["state"], "committed");
    assert_eq!(value["output"]["changed"], true);
    assert!(value["report"]["path"].is_null());
    assert!(value["report"]["error"].is_string());
    assert!(root.join("server/protocol/cmd/example.pb.go").is_file());
    let (_, readonly) = project(root, "config_show", &[]);
    assert_eq!(readonly["error"]["code"], "E_REPORT");
    assert_eq!(readonly["output"]["changed"], false);
}

#[test]
fn skill_export_works_without_project_setup_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (output, value) = human(root, &["skill", "--output", "skills/proto-builder"], &[]);
    assert!(output.status.success(), "{value}");
    let target = root.join("skills/proto-builder/SKILL.md");
    assert_eq!(
        fs::read(&target).unwrap(),
        include_bytes!("../.agents/skills/proto-builder/SKILL.md")
    );
    fs::write(&target, "local skill customization").unwrap();
    let (output, blocked) = human(root, &["skill", "--output", "skills/proto-builder"], &[]);
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
    let (output, value) = project(root, "build", &[]);
    assert_eq!(output.status.code(), Some(4), "{value}");
    assert_eq!(value["error"]["kind"], "svn_failed");
    assert_eq!(value["context"]["policy"]["svn_update"], true);
    assert_eq!(value["output"]["changed"], false);
    assert!(!root.join("server").exists());
    let (_, shown) = project(root, "config_show", &[]);
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
    let (output, first) = project(root, "build", &environment);
    assert!(output.status.success(), "{first}");
    assert_eq!(first["output"]["state"], "committed");
    assert_eq!(first["output"]["changed"], true);
    let target = root.join("server/protocol/cmd/example.pb.go");
    let initial = fs::read(&target).unwrap();
    let (output, repeat) = project(root, "build", &environment);
    assert!(output.status.success(), "{repeat}");
    assert_eq!(repeat["output"]["changed"], false);
    assert_eq!(fs::read(&target).unwrap(), initial);
    assert_eq!(full(&repeat)["output_state"], "committed");

    let destination = data.join("skills/proto-builder");
    let args = ["skill", "--output", destination.to_str().unwrap()];
    let (output, exported) = human(root, &args, &environment);
    assert!(output.status.success(), "{exported}");
    let skill = destination.join("SKILL.md");
    assert_eq!(
        fs::read(&skill).unwrap(),
        include_bytes!("../.agents/skills/proto-builder/SKILL.md")
    );
    fs::write(&skill, "local customization").unwrap();
    let (output, blocked) = human(root, &args, &environment);
    assert_eq!(output.status.code(), Some(2), "{blocked}");
    assert_eq!(fs::read_to_string(skill).unwrap(), "local customization");
}
