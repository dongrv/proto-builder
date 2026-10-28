use reqwest::blocking::{Client, Response};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Web {
    child: Child,
    base: String,
    token: String,
    client: Client,
}
impl Web {
    fn start(root: &Path) -> Self {
        Self::start_with_env(root, &[])
    }
    fn start_with_env(root: &Path, environment: &[(&str, String)]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_proto-builder"))
            .args(["web", "--no-open", "--non-interactive"])
            .arg("--repo-root")
            .arg(root)
            .current_dir(root)
            .env("PROTO_BUILDER_HOME", root.join("user-data"))
            .env_remove("ENV_CENTER_PROJECT")
            .env_remove("ENV_PROJECT")
            .env_remove("ENV_CAL_CENTER_PROJECT")
            .env_remove("ENV_PROTOBUF")
            .env_remove("ENV_DEVELOPER_NAME")
            .envs(environment.iter().map(|(key, value)| (*key, value)))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let url = reqwest::Url::parse(line.trim()).expect("server launch URL");
        let token = url
            .fragment()
            .unwrap()
            .strip_prefix("token=")
            .unwrap()
            .to_owned();
        Self {
            child,
            base: format!("http://{}:{}", url.host_str().unwrap(), url.port().unwrap()),
            token,
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap(),
        }
    }
    fn get(&self, path: &str) -> Response {
        self.client
            .get(format!("{}{path}", self.base))
            .header("X-Proto-Token", &self.token)
            .send()
            .unwrap()
    }
    fn post(&self, path: &str, data: &Value) -> Response {
        self.client
            .post(format!("{}{path}", self.base))
            .header("X-Proto-Token", &self.token)
            .json(data)
            .send()
            .unwrap()
    }
    fn form(&self) -> Value {
        let data: Value = self.get("/api/state").json().unwrap();
        json!({"root":data["root"],"settings":data["settings"],"force":false,"adopt_existing":false})
    }
    fn start_job(&self, action: &str, form: &Value) -> String {
        let response = self.post("/api/jobs", &json!({"action":action,"form":form}));
        assert!(response.status().is_success());
        response.json::<Value>().unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn wait(&self, id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(40);
        loop {
            let report: Value = self.get(&format!("/api/jobs/{id}")).json().unwrap();
            if report["finished"] == true {
                return report;
            }
            assert!(Instant::now() < deadline, "job timed out: {report}");
            thread::sleep(Duration::from_millis(25));
        }
    }
    fn stop(&mut self) {
        assert!(self.post("/api/shutdown", &json!({})).status().is_success());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline, "server did not stop");
            thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Web {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn fixture(root: &Path) {
    fs::create_dir_all(root.join("proto/cmd")).unwrap();
    fs::create_dir_all(root.join("proto/act7034")).unwrap();
    fs::write(root.join("proto/cmd/common.proto"), "syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\n// SEQ[100,200]\nmessage HelloReq {string name=1;}\nmessage HelloResp {string value=1;}\n").unwrap();
    fs::write(root.join("proto/act7034/bonus.proto"), "syntax=\"proto3\";\npackage act7034;\noption go_package=\"example/protocol/act7034\";\n// SEQ[300,400]\nmessage BonusReq {}\n").unwrap();
    fs::write(root.join("proto-builder.toml"), "config_version=1\nserver_path='server'\ncalc_path='calc'\n[svn]\nupdate=false\n[update]\nenabled=false\n").unwrap();
}

#[test]
fn environment_alias_is_reported_with_its_actual_name_and_value() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let server = root.join("env-server");
    let calc = root.join("env-calc");
    fs::create_dir(&server).unwrap();
    fs::create_dir(&calc).unwrap();
    let environment = [
        ("ENV_PROJECT", server.display().to_string()),
        ("ENV_CAL_CENTER_PROJECT", calc.display().to_string()),
        ("ENV_PROTOBUF", root.display().to_string()),
        ("ENV_DEVELOPER_NAME", "董荣旺".into()),
    ];
    let mut web = Web::start_with_env(root, &environment);
    let state: Value = web.get("/api/state").json().unwrap();
    assert_eq!(state["setup"]["needs_confirmation"], false);
    assert_eq!(state["settings"]["server_path"], server.to_str().unwrap());
    assert_eq!(state["settings"]["calc_path"], calc.to_str().unwrap());
    assert_eq!(state["settings"]["developer_name"], "董荣旺");
    let found = state["setup"]["environment"].as_object().unwrap();
    assert_eq!(found.len(), 4);
    for (name, value) in environment {
        assert_eq!(found[name], value);
    }
    assert!(!found.contains_key("ENV_CENTER_PROJECT"));
    assert!(!root.join("user-data/local-setup.json").exists());
    web.stop();
}

#[test]
fn first_use_setup_survives_restart_only_after_success() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fixture(root);
    let mut web = Web::start(root);
    let state: Value = web.get("/api/state").json().unwrap();
    assert_eq!(state["setup"]["needs_confirmation"], true);
    assert_eq!(state["setup"]["issues"].as_array().unwrap().len(), 4);
    let mut form = web.form();
    form["settings"]["developer_name"] = "董荣旺".into();
    form["settings"]["server_path"] = "manual-server".into();
    form["settings"]["calc_path"] = "manual-calc".into();
    let plan = web.start_job("plan", &form);
    assert_eq!(web.wait(&plan)["status"], "success");
    let saved = root.join("user-data/local-setup.json");
    assert!(!saved.exists());
    let mut bad = form.clone();
    bad["settings"]["activity_ids"] = json!(["99999"]);
    let failed = web.start_job("build", &bad);
    assert_eq!(web.wait(&failed)["status"], "failed");
    assert!(!saved.exists());
    let job = web.start_job("build", &form);
    assert_eq!(web.wait(&job)["status"], "success");
    let state: Value = web.get("/api/state").json().unwrap();
    assert_eq!(state["setup"]["needs_confirmation"], false);
    assert_eq!(state["setup"]["saved"], true);
    web.stop();
    let mut restarted = Web::start(root);
    let state: Value = restarted.get("/api/state").json().unwrap();
    assert_eq!(state["setup"]["needs_confirmation"], false);
    assert_eq!(state["settings"]["developer_name"], "董荣旺");
    assert_eq!(
        state["settings"]["server_path"],
        root.join("manual-server").to_str().unwrap()
    );
    assert_eq!(
        state["settings"]["calc_path"],
        root.join("manual-calc").to_str().unwrap()
    );
    restarted.stop();
}

#[test]
fn local_session_authentication_assets_and_request_limits() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut web = Web::start(dir.path());
    assert_eq!(
        web.client
            .get(format!("{}/api/state", web.base))
            .send()
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        web.client
            .get(format!("{}/api/state", web.base))
            .header("X-Proto-Token", "invalid")
            .send()
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        web.client
            .get(format!("{}/api/state", web.base))
            .header("X-Proto-Token", &web.token)
            .header("Origin", "https://example.com")
            .send()
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        web.client
            .get(format!("{}/api/state", web.base))
            .header("X-Proto-Token", &web.token)
            .header("Host", "example.com")
            .send()
            .unwrap()
            .status(),
        403
    );
    let page = web.get("/");
    assert_eq!(page.status(), 200);
    assert!(page.headers()["Content-Security-Policy"]
        .to_str()
        .unwrap()
        .contains("frame-ancestors 'none'"));
    assert!(page.text().unwrap().contains("/app.js"));
    for path in ["/app.js", "/styles.css", "/favicon.svg"] {
        assert_eq!(web.get(path).status(), 200);
    }
    assert_eq!(web.get("/proto-builder.toml").status(), 404);
    let bad = web
        .client
        .post(format!("{}/api/jobs", web.base))
        .header("X-Proto-Token", &web.token)
        .header("Content-Type", "application/json")
        .body("{".to_owned())
        .send()
        .unwrap();
    assert_eq!(bad.status(), 400);
    let big = web
        .client
        .post(format!("{}/api/jobs", web.base))
        .header("X-Proto-Token", &web.token)
        .header("Content-Type", "application/json")
        .body(" ".repeat(256 * 1024 + 1))
        .send()
        .unwrap();
    assert_eq!(big.status(), 413);
    web.stop();
}

#[test]
fn web_plan_catalog_directory_picker_and_config_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let root = dir.path();
    let mut web = Web::start(root);
    let mut form = web.form();
    let catalog: Value = web.post("/api/catalog", &form).json().unwrap();
    assert_eq!(catalog["activities"], json!(["7034"]));
    assert_eq!(catalog["proto_files"], 2);
    let mut url = reqwest::Url::parse(&format!("{}/api/directories", web.base)).unwrap();
    url.query_pairs_mut()
        .append_pair("path", root.join("proto").to_str().unwrap());
    let folders: Value = web
        .get(&format!("{}?{}", url.path(), url.query().unwrap()))
        .json()
        .unwrap();
    assert_eq!(folders["entries"].as_array().unwrap().len(), 2);
    // Preview must stay read-only even when automatic SVN update is enabled.
    form["settings"]["svn"]["update"] = true.into();
    form["settings"]["activity_ids"] = json!(["7034"]);
    form["force"] = true.into();
    let id = web.start_job("plan", &form);
    let report = web.wait(&id);
    assert_eq!(report["status"], "success", "{report}");
    assert_eq!(report["result"]["selected"].as_array().unwrap().len(), 2);
    assert!(!root.join("server").exists());
    assert!(!root.join("calc").exists());
    let incremental: Value = web
        .get(&format!("/api/jobs/{id}?after={}", report["event_count"]))
        .json()
        .unwrap();
    assert_eq!(incremental["events"], json!([]));
    let current: Value = web.get("/api/state").json().unwrap();
    assert_eq!(current["force"], true);
    assert_eq!(current["last_job"], id);
    let download = web.get(&format!("/api/report/{id}"));
    assert!(download.headers()["Content-Disposition"]
        .to_str()
        .unwrap()
        .contains(&id));
    assert_eq!(
        download.json::<Value>().unwrap()["events"],
        report["events"]
    );
    let config = root.join("proto-builder.toml");
    let original = fs::read(&config).unwrap();
    let preview: Value = web.post("/api/config/preview", &form).json().unwrap();
    assert_eq!(fs::read(&config).unwrap(), original);
    let save = json!({"form":form,"expected_hash":preview["expected_hash"]});
    fs::write(&config, b"# externally changed\n").unwrap();
    assert_eq!(web.post("/api/config/save", &save).status(), 409);
    assert_eq!(fs::read(&config).unwrap(), b"# externally changed\n");
    fs::write(&config, &original).unwrap();
    assert_eq!(web.post("/api/config/save", &save).status(), 200);
    assert_eq!(
        fs::read(config.with_extension("toml.bak")).unwrap(),
        original
    );
    let wrong_project = json!({"root":root.join("elsewhere"),"settings":form["settings"]});
    assert_eq!(
        web.post("/api/jobs", &json!({"action":"plan","form":wrong_project}))
            .status(),
        409
    );
    let other = tempfile::tempdir().unwrap();
    fixture(other.path());
    let opened: Value = web
        .post("/api/project", &json!({"root":other.path()}))
        .json()
        .unwrap();
    assert!(opened["last_job"].is_null());
    assert_eq!(opened["force"], false);
    web.stop();
}

#[test]
#[cfg(all(windows, target_arch = "x86_64"))]
fn web_build_errors_keep_outputs_and_cancel_releases_the_session() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let root = dir.path();
    let mut web = Web::start(root);
    let form = web.form();
    let first = web.start_job("build", &form);
    let success = web.wait(&first);
    assert_eq!(success["status"], "success", "{success}");
    let go = root.join("server/protocol/cmd/common.pb.go");
    let original = fs::read(&go).unwrap();
    let input = root.join("proto/cmd/common.proto");
    fs::write(&input, "syntax=\"proto3\";\npackage cmd;\noption go_package=\"example/protocol/cmd\";\nmessage HelloReq { invalid_syntax }\n").unwrap();
    let failed = web.start_job("build", &form);
    let error = web.wait(&failed);
    assert_eq!(error["status"], "failed", "{error}");
    assert_eq!(
        error["diagnostics"][0]["file"].as_str().map(Path::new),
        Some(input.as_path()),
        "{error}"
    );
    assert_eq!(error["diagnostics"][0]["line"], 4);
    assert_eq!(fs::read(&go).unwrap(), original);
    fixture(root);
    for i in 0..80 {
        let path = root.join(format!("proto/group{i}"));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("item.proto"), format!("syntax=\"proto3\"; package group{i}; option go_package=\"example/protocol/group{i}\"; message Item {{}}\n")).unwrap();
    }
    let mut long_form = form.clone();
    long_form["settings"]["jobs"] = 1.into();
    let cancelled = web.start_job("build", &long_form);
    assert_eq!(web.post("/api/config/preview", &long_form).status(), 409);
    assert_eq!(
        web.post(&format!("/api/cancel/{cancelled}"), &json!({}))
            .status(),
        200
    );
    let report = web.wait(&cancelled);
    assert_eq!(report["exit_code"], 130, "{report}");
    assert_eq!(fs::read(&go).unwrap(), original);
    assert!(!root.join("server/protocol/group0/item.pb.go").exists());
    assert_eq!(web.get("/api/state").status(), 200);
    let plan = web.start_job("plan", &form);
    assert_eq!(web.wait(&plan)["status"], "success");
    web.stop();
}
