use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn run(root: &Path, args: &[&str]) -> Output {
    run_with_env(root, args, &[])
}
fn run_with_env(root: &Path, args: &[&str], environment: &[(&str, String)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_proto-builder"))
        .current_dir(root)
        .env("PROTO_BUILDER_HOME", root.join("user-data"))
        .env_remove("ENV_CENTER_PROJECT")
        .env_remove("ENV_PROJECT")
        .env_remove("ENV_CAL_CENTER_PROJECT")
        .env_remove("ENV_PROTOBUF")
        .env_remove("ENV_DEVELOPER_NAME")
        .envs(environment.iter().map(|(key, value)| (*key, value)))
        .args(args)
        .args(["--json", "--non-interactive"])
        .output()
        .expect("run binary")
}
fn report(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
fn fixture(root: &Path) {
    fs::create_dir_all(root.join("proto/cmd")).unwrap();
    fs::create_dir_all(root.join("proto/rpc")).unwrap();
    fs::write(root.join("proto/cmd/common.proto"),"syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\n// SEQ[100,200]\nmessage HelloReq {string name=1;}\nmessage HelloResp {string value=1;}\n").unwrap();
    fs::write(root.join("proto/rpc/echo.proto"),"syntax=\"proto3\";\npackage rpc;\noption go_package=\"example/protocol/rpc\";\nimport \"cmd/common.proto\";\nservice Echo {rpc Call(cmd.HelloReq) returns (cmd.HelloResp);}\n").unwrap();
    fs::write(root.join("proto-builder.toml"),"config_version=1\nserver_path='server'\ncalc_path='calc'\n[svn]\nupdate=false\n[update]\nenabled=false\n").unwrap();
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, out);
            } else {
                out.insert(path.clone(), fs::read(path).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, &mut files);
    files
}
#[test]
fn disjoint_activity_machine_filters_explain_existing_ids_and_remedy() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    for folder in ["act7001", "slot5001"] {
        fs::create_dir_all(dir.path().join("proto").join(folder)).unwrap();
        fs::write(
            dir.path().join("proto").join(folder).join("a.proto"),
            format!("syntax=\"proto3\"; package {folder};"),
        )
        .unwrap();
    }
    let args = ["plan", "--activity-id", "7001", "--machine-id", "5001"];
    let out = run(dir.path(), &args);
    assert_eq!(out.status.code(), Some(3));
    let value = report(&out);
    let error = value["error"].as_str().unwrap();
    assert!(error.contains("E_NO_MATCH"));
    assert!(error.contains("activity matches 1 files, machine matches 1 files"));
    assert!(error.contains("intersection is empty"));
    assert!(error.contains("--machine-id="));
    let out = run(
        dir.path(),
        &["plan", "--activity-id", "7001", "--machine-id="],
    );
    assert!(out.status.success());
    assert!(report(&out)["result"]["selected"]
        .as_array()
        .unwrap()
        .iter()
        .any(|input| input["relative"] == "act7001/a.proto"));
    assert!(!dir.path().join("server").exists());
    assert!(!dir.path().join("calc").exists());
}

#[test]
fn selective_go_build_preserves_complete_shared_documents() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    for id in [7001, 7002, 7003] {
        let folder = root.join(format!("proto/act{id}"));
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("a.proto"), format!("syntax=\"proto3\";\npackage act{id};\noption go_package=\"example/protocol/act{id}\";\nmessage Module{id} {{string value=1;}}\n")).unwrap();
    }
    let full = run(root, &["build"]);
    assert!(full.status.success(), "{}", report(&full));
    let docs = ["protocol.html", "protocol.md"];
    let original = docs.map(|name| fs::read(root.join("proto").join(name)).unwrap());
    let unselected = tree(&root.join("server/protocol/act7003"));
    for args in [
        vec!["build", "--activity-id", "7001,7002"],
        vec!["build", "--assign-id", "7001"],
    ] {
        let selected = run(root, &args);
        assert!(selected.status.success(), "{}", report(&selected));
        assert_eq!(report(&selected)["result"]["documentation_proto_files"], 4);
        for (index, name) in docs.iter().enumerate() {
            assert_eq!(
                fs::read(root.join("proto").join(name)).unwrap(),
                original[index]
            );
        }
        assert_eq!(tree(&root.join("server/protocol/act7003")), unselected);
    }
    // A change in an unselected module still belongs in the shared documents.
    let path = root.join("proto/act7003/a.proto");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("string value=1;", "string value=1; int32 added=2;"),
    )
    .unwrap();
    let selected = run(root, &["build", "--activity-id", "7001"]);
    assert!(selected.status.success(), "{}", report(&selected));
    for name in docs {
        let text = fs::read_to_string(root.join("proto").join(name)).unwrap();
        assert!(text.contains("Module7003"));
        assert!(text.contains("added"));
    }
    assert_eq!(tree(&root.join("server/protocol/act7003")), unselected);
    let before = docs.map(|name| fs::read(root.join("proto").join(name)).unwrap());
    let no_docs = run(root, &["build", "--activity-id", "7001", "--no-docs"]);
    assert!(no_docs.status.success(), "{}", report(&no_docs));
    for (index, name) in docs.iter().enumerate() {
        assert_eq!(
            fs::read(root.join("proto").join(name)).unwrap(),
            before[index]
        );
    }
    let excluded = run(
        root,
        &["build", "--activity-id", "7001", "--exclude", "act7003"],
    );
    assert!(excluded.status.success(), "{}", report(&excluded));
    for name in docs {
        assert!(!fs::read_to_string(root.join("proto").join(name))
            .unwrap()
            .contains("Module7003"));
    }
    let before = docs.map(|name| fs::read(root.join("proto").join(name)).unwrap());
    fs::write(path, "syntax=\"proto3\"; package act7003; INVALID").unwrap();
    let failed = run(root, &["build", "--activity-id", "7001"]);
    assert!(!failed.status.success());
    for (index, name) in docs.iter().enumerate() {
        assert_eq!(
            fs::read(root.join("proto").join(name)).unwrap(),
            before[index]
        );
    }
}

#[test]
fn team_environment_and_successful_setup_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let launch = dir.path();
    let project = launch.join("协议 项目");
    fixture(&project);
    let server = project.join("团队业务服");
    let calc = project.join("团队计算服");
    fs::create_dir(&server).unwrap();
    fs::create_dir(&calc).unwrap();
    let environment = [
        ("ENV_PROTOBUF", project.display().to_string()),
        ("ENV_CENTER_PROJECT", server.display().to_string()),
        ("ENV_CAL_CENTER_PROJECT", calc.display().to_string()),
        ("ENV_DEVELOPER_NAME", "董荣旺".into()),
    ];
    let shown = run_with_env(launch, &["config", "show"], &environment);
    assert!(shown.status.success(), "{}", report(&shown));
    let shown = report(&shown);
    assert_eq!(shown["result"]["project_root"], project.to_str().unwrap());
    assert_eq!(
        shown["result"]["settings"]["server_path"],
        server.to_str().unwrap()
    );
    assert_eq!(
        shown["result"]["settings"]["calc_path"],
        calc.to_str().unwrap()
    );
    assert_eq!(shown["result"]["settings"]["developer_name"], "董荣旺");
    let mut aliased = environment.clone();
    aliased[1].0 = "ENV_PROJECT";
    let alias_result = run_with_env(launch, &["config", "show"], &aliased);
    assert!(alias_result.status.success(), "{}", report(&alias_result));
    assert_eq!(report(&alias_result)["result"], shown["result"]);
    let overridden = run_with_env(
        launch,
        &[
            "config",
            "show",
            "--server-path",
            "manual",
            "--developer-name",
            "手动名字",
        ],
        &environment,
    );
    assert_eq!(
        report(&overridden)["result"]["settings"]["server_path"],
        "manual"
    );
    assert_eq!(
        report(&overridden)["result"]["settings"]["developer_name"],
        "手动名字"
    );
    assert!(run_with_env(launch, &["plan"], &environment)
        .status
        .success());
    let saved = launch.join("user-data/local-setup.json");
    assert!(!saved.exists());
    assert!(
        !run_with_env(launch, &["build", "--activity-id", "99999"], &environment)
            .status
            .success()
    );
    assert!(!saved.exists());
    let success = run_with_env(launch, &["build"], &environment);
    assert!(success.status.success(), "{}", report(&success));
    assert!(saved.is_file());
    let resumed = report(&run(launch, &["config", "show"]));
    assert_eq!(resumed["result"]["project_root"], project.to_str().unwrap());
    assert_eq!(
        resumed["result"]["settings"]["server_path"],
        server.to_str().unwrap()
    );
    assert_eq!(resumed["result"]["settings"]["developer_name"], "董荣旺");
    let plan = run(launch, &["plan"]);
    assert!(plan.status.success(), "{}", report(&plan));
    assert!(!report(&plan)["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["phase"] == "setup" && e["level"] == "warning"));
    let new_server = project.join("新的业务服");
    let invalid = run_with_env(
        launch,
        &["plan"],
        &[("ENV_CENTER_PROJECT", "relative/invalid".into())],
    );
    assert!(invalid.status.success());
    assert!(report(&invalid)["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["phase"] == "setup" && e["level"] == "warning"));
    fs::create_dir(&new_server).unwrap();
    let mut changed = environment.clone();
    changed[1].1 = new_server.display().to_string();
    let changed = run_with_env(launch, &["config", "show"], &changed);
    assert_eq!(
        report(&changed)["result"]["settings"]["server_path"],
        new_server.to_str().unwrap()
    );
}

#[test]
fn remembered_paths_are_scoped_to_each_project() {
    let dir = tempfile::tempdir().unwrap();
    let launch = dir.path();
    for name in ["one", "two"] {
        let project = launch.join(name);
        fixture(&project);
        let result = run(
            launch,
            &[
                "build",
                "--repo-root",
                project.to_str().unwrap(),
                "--server-path",
                "custom-server",
                "--calc-path",
                "custom-calc",
            ],
        );
        assert!(result.status.success(), "{}", report(&result));
    }
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(launch.join("user-data/local-setup.json")).unwrap())
            .unwrap();
    assert_eq!(saved["projects"].as_object().unwrap().len(), 2);
    let one = run(
        launch,
        &[
            "config",
            "show",
            "--repo-root",
            launch.join("one").to_str().unwrap(),
        ],
    );
    assert_eq!(
        Path::new(
            report(&one)["result"]["settings"]["server_path"]
                .as_str()
                .unwrap()
        ),
        launch.join("one").join("custom-server")
    );
    let other = launch.join("other");
    fixture(&other);
    let other = run(
        launch,
        &["config", "show", "--repo-root", other.to_str().unwrap()],
    );
    assert_eq!(
        report(&other)["result"]["settings"]["server_path"],
        "server"
    );
}

#[test]
fn plan_is_read_only_even_with_default_svn_update() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    fs::write(
        dir.path().join("proto-builder.toml"),
        "config_version=1\nserver_path='server'\ncalc_path='calc'\n[update]\nenabled=false\n",
    )
    .unwrap();
    let before = tree(&dir.path().join("proto"));
    let out = run(dir.path(), &["plan"]);
    assert!(out.status.success(), "{}", report(&out));
    assert_eq!(
        report(&out)["result"]["selected"].as_array().unwrap().len(),
        2
    );
    assert_eq!(before, tree(&dir.path().join("proto")));
    assert!(!dir.path().join("server").exists());
}
#[test]
fn unknown_command_is_rejected_and_never_builds() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let out = run(dir.path(), &["buidl"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!dir.path().join("server").exists());
}
#[test]
fn protocol_id_error_identifies_original_file_and_leaves_outputs_untouched() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let duplicate = dir.path().join("proto/cmd/duplicate.proto");
    fs::write(&duplicate, "syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\n// SEQ[100,200]\nmessage OtherReq {}\n").unwrap();
    let output = run(dir.path(), &["build", "--no-docs", "--no-copy"]);
    let value = report(&output);
    assert_eq!(output.status.code(), Some(1), "{value}");
    assert!(value["error"].as_str().unwrap().contains("E_PROTOCOL"));
    let diagnostic = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| {
            d["file"]
                .as_str()
                .is_some_and(|file| Path::new(file) == duplicate)
        })
        .unwrap_or_else(|| panic!("missing original path: {value}"));
    assert_eq!(diagnostic["line"], 5);
    assert!(!dir.path().join("server").exists());
    assert!(!dir.path().join("calc").exists());
    assert!(!dir.path().join("proto/ProtoConfig.json").exists());
}
#[test]
fn unknown_config_key_and_missing_match_fail_before_any_output() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let out = run(dir.path(), &["build", "--activity-id", "9999"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(!dir.path().join("server").exists());
    fs::write(
        dir.path().join("proto-builder.toml"),
        "config_version=1\nprot_path='oops'",
    )
    .unwrap();
    let out = run(dir.path(), &["config", "check"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(report(&out)["error"]
        .as_str()
        .unwrap()
        .contains("prot_path"));
}
#[test]
#[cfg(all(windows, target_arch = "x86_64"))]
fn native_build_replaces_go_backs_up_and_cleans_only_owned_stale_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let first = run(root, &["build", "--jobs", "2"]);
    assert!(first.status.success(), "{}", report(&first));
    let go = root.join("server/protocol/cmd/common.pb.go");
    let original = fs::read(&go).unwrap();
    let rpc = root.join("server/protocol/rpc/echo_grpc.pb.go");
    assert!(rpc.is_file());
    assert!(root.join("calc/protocol/rpc/echo_grpc.pb.go").is_file());
    let repeated = run(root, &["build", "--jobs", "1"]);
    assert!(repeated.status.success(), "{}", report(&repeated));
    assert_eq!(
        report(&repeated)["result"]["changes"]["modified"],
        serde_json::json!([])
    );
    let mut edited = original.clone();
    edited.extend_from_slice(b"// user edit\n");
    fs::write(&go, &edited).unwrap();
    let input = root.join("proto/cmd/common.proto");
    let text = fs::read_to_string(&input)
        .unwrap()
        .replace("string name=1;", "string name=1;int32 age=2;");
    fs::write(&input, text).unwrap();
    let grpc_original = fs::read(&rpc).unwrap();
    fs::write(&rpc, b"edited grpc output").unwrap();
    let forced = run(root, &["build"]);
    assert!(forced.status.success(), "{}", report(&forced));
    let backup = report(&forced)["result"]["changes"]["backup"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(Path::new(&backup).join("index.json").is_file());
    assert_ne!(fs::read(&go).unwrap(), edited);
    assert_eq!(fs::read(&rpc).unwrap(), grpc_original);
    let journal: serde_json::Value =
        serde_json::from_slice(&fs::read(Path::new(&backup).join("index.json")).unwrap()).unwrap();
    for (target, bytes) in [
        (&go, edited.as_slice()),
        (&rpc, b"edited grpc output".as_slice()),
    ] {
        let saved = journal["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|op| Path::new(op["target"].as_str().unwrap()) == target)
            .unwrap();
        assert_eq!(fs::read(saved["backup"].as_str().unwrap()).unwrap(), bytes);
    }
    let server_before = tree(&root.join("server"));
    let calc_before = tree(&root.join("calc"));
    fs::write(&input, "syntax=\"proto3\";\npackage cmd;\ninvalid syntax\n").unwrap();
    let failure = run(root, &["build"]);
    assert!(!failure.status.success());
    assert!(
        !report(&failure)["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{}",
        report(&failure)
    );
    assert_eq!(server_before, tree(&root.join("server")));
    assert_eq!(calc_before, tree(&root.join("calc")));
    fs::write(&input,"syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\n// SEQ[100,200]\nmessage HelloReq {}\nmessage HelloResp {}\n").unwrap();
    fs::remove_file(root.join("proto/rpc/echo.proto")).unwrap();
    fs::write(root.join("server/protocol/notes.txt"), "user-owned").unwrap();
    let no_copy = run(root, &["build", "--no-copy"]);
    assert!(no_copy.status.success(), "{}", report(&no_copy));
    assert!(!rpc.exists());
    assert!(root.join("calc/protocol/rpc/echo_grpc.pb.go").exists());
    let cleaned = run(root, &["build"]);
    assert!(cleaned.status.success(), "{}", report(&cleaned));
    assert!(!root.join("calc/protocol/rpc/echo_grpc.pb.go").exists());
    assert_eq!(
        fs::read_to_string(root.join("server/protocol/notes.txt")).unwrap(),
        "user-owned"
    );
}
#[test]
fn real_svn_update_is_scoped_and_text_conflicts_abort_before_generation() {
    if Command::new("svnadmin")
        .arg("--version")
        .arg("--quiet")
        .output()
        .is_err()
    {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let repo = root.join("repo");
    let checked = |command: &mut Command| {
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    checked(Command::new("svnadmin").arg("create").arg(&repo));
    let url = reqwest::Url::from_directory_path(&repo)
        .unwrap()
        .to_string();
    let working = root.join("wc");
    let remote = root.join("other");
    checked(
        Command::new("svn")
            .args(["checkout", "--quiet"])
            .arg(&url)
            .arg(&working),
    );
    fixture(&working);
    fs::write(working.join("proto-builder.toml"),"config_version=1\nserver_path='server'\ncalc_path='calc'\n[svn]\nupdate=true\nupdate_paths=['proto']\n[update]\nenabled=false\n").unwrap();
    checked(
        Command::new("svn")
            .arg("add")
            .arg(working.join("proto"))
            .arg(working.join("proto-builder.toml")),
    );
    checked(
        Command::new("svn")
            .arg("commit")
            .arg(&working)
            .args(["-m", "fixture"]),
    );
    checked(
        Command::new("svn")
            .args(["checkout", "--quiet"])
            .arg(&url)
            .arg(&remote),
    );
    fs::write(remote.join("proto/cmd/remote.proto"),"syntax=\"proto3\";package remote;option go_package=\"example/protocol/remote\";message New {}\n").unwrap();
    checked(
        Command::new("svn")
            .arg("add")
            .arg(remote.join("proto/cmd/remote.proto")),
    );
    checked(
        Command::new("svn")
            .arg("commit")
            .arg(&remote)
            .args(["-m", "remote change"]),
    );
    let first = run(&working, &["build"]);
    assert!(first.status.success(), "{}", report(&first));
    assert!(working.join("proto/cmd/remote.proto").is_file());
    let local = working.join("proto/cmd/common.proto");
    let other = remote.join("proto/cmd/common.proto");
    fs::write(
        &local,
        fs::read_to_string(&local)
            .unwrap()
            .replace("string name=1;", "string name=1;int32 local=2;"),
    )
    .unwrap();
    fs::write(
        &other,
        fs::read_to_string(&other)
            .unwrap()
            .replace("string name=1;", "string name=1;int32 remote=2;"),
    )
    .unwrap();
    checked(
        Command::new("svn")
            .arg("commit")
            .arg(&remote)
            .args(["-m", "conflicting change"]),
    );
    let before = tree(&working.join("server"));
    let out = run(&working, &["build"]);
    assert_eq!(out.status.code(), Some(4), "{}", report(&out));
    assert!(report(&out)["error"]
        .as_str()
        .unwrap()
        .contains("E_SVN_CONFLICT"));
    assert_eq!(before, tree(&working.join("server")));
}
