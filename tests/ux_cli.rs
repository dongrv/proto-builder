use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proto-builder"));
    command.current_dir(root);
    for (name, _) in std::env::vars_os() {
        let name_text = name.to_string_lossy();
        if name_text.starts_with("PROTO_BUILDER_")
            || [
                "ENV_PROJECT",
                "ENV_CENTER_PROJECT",
                "ENV_CAL_CENTER_PROJECT",
                "ENV_PROTOBUF",
                "ENV_DEVELOPER_NAME",
            ]
            .contains(&name_text.as_ref())
        {
            command.env_remove(name);
        }
    }
    command.env("PROTO_BUILDER_HOME", root.join("user-data"));
    command
}

#[test]
fn help_explains_common_paths_generation_and_entry_points() {
    let dir = tempfile::tempdir().unwrap();
    let output = command(dir.path()).arg("--help").output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Project paths",
        "Selection",
        "Generation",
        "Execution",
        "Output",
        "--no-protocol-config",
        "no-config",
        "still read TOML",
        "proto-builder web",
        "--activity-id 7001,7002",
        "0 = automatic",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    let output = command(dir.path())
        .args(["config", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Show effective configuration"));
    assert!(text.contains("Validate configuration"));
}

#[test]
fn explicit_protocol_config_switch_and_legacy_alias_keep_reading_toml() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("proto-builder.toml"), "config_version=1\nserver_path='configured-server'\ncalc_path='configured-calc'\nprotocol_config=true\n[update]\nenabled=false\n").unwrap();
    for flag in ["--no-protocol-config", "--no-config"] {
        let output = command(dir.path())
            .args(["config", "show", flag, "--json", "--non-interactive"])
            .output()
            .unwrap();
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(output.status.success(), "{value}");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["status"], "success");
        assert!(value["failure"].is_null());
        assert_eq!(value["result"]["settings"]["protocol_config"], false);
        assert_eq!(
            value["result"]["settings"]["server_path"],
            "configured-server"
        );
        assert_eq!(value["result"]["settings"]["calc_path"], "configured-calc");
    }
}
