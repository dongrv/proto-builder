#![cfg(all(windows, target_arch = "x86_64"))]

use reqwest::blocking::{Client, Response};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn fixture(root: &Path) {
    for group in ["cmd", "act7001"] {
        fs::create_dir_all(root.join("proto").join(group)).unwrap();
        let name = if group == "cmd" {
            "example.proto"
        } else {
            "bonus.proto"
        };
        fs::write(root.join("proto").join(group).join(name), format!("syntax=\"proto3\";\npackage {group};\noption go_package=\"example/protocol/{group}\";\nmessage Request {{ string name=1; }}\n")).unwrap();
    }
    fs::write(root.join("proto-builder.toml"), "config_version=1\nserver_path='server'\ncalc_path='calc'\ndocs=false\nprotocol_config=false\ncopy_to_calc=false\n[svn]\nupdate=false\n[update]\nenabled=false\n").unwrap();
}
fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proto-builder"));
    command
        .current_dir(root)
        .stdin(Stdio::null())
        .env("PROTO_BUILDER_HOME", root.join("user-data"));
    for (name, _) in std::env::vars_os() {
        let text = name.to_string_lossy();
        if text.starts_with("ENV_")
            || (text.starts_with("PROTO_BUILDER_") && text != "PROTO_BUILDER_HOME")
        {
            command.env_remove(name);
        }
    }
    command
}

#[test]
fn doctor_plan_and_build_agree_on_enabled_output_roots() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    for args in [
        vec!["--calc-path", "SERVER", "--copy-to-calc=true"],
        vec![
            "--calc-path",
            "server/protocol/nested",
            "--copy-to-calc=true",
        ],
        vec!["--server-path", "PROTO"],
    ] {
        for action in ["doctor", "plan", "build"] {
            let output = command(root)
                .arg(action)
                .args(&args)
                .args(["--json", "--non-interactive"])
                .output()
                .unwrap();
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(!output.status.success(), "{action}: {value}");
            assert_eq!(value["schema_version"], 1);
            assert_eq!(
                value["failure"]["code"], "E_OUTPUT_SCOPE",
                "{action}: {value}"
            );
            assert_eq!(value["failure"]["kind"], "invalid_paths");
        }
    }
    assert!(!root.join("server/protocol").exists());
    for action in ["doctor", "plan", "build"] {
        let output = command(root)
            .args([
                action,
                "--calc-path",
                "server",
                "--copy-to-calc=false",
                "--json",
                "--non-interactive",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{action}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    assert!(root.join("server/protocol/cmd/example.pb.go").is_file());
    // Adjacent names are not nested paths (server vs server-copy).
    let output = command(root)
        .args([
            "plan",
            "--calc-path",
            "server-copy",
            "--copy-to-calc=true",
            "--json",
            "--non-interactive",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    fs::create_dir_all(root.join("server/configs/cmd")).unwrap();
    fs::copy(
        root.join("proto/cmd/example.proto"),
        root.join("server/configs/cmd/example.proto"),
    )
    .unwrap();
    for action in ["doctor", "plan", "build"] {
        let output = command(root)
            .args([
                action,
                "--proto-path",
                "server/configs",
                "--protocol-config=false",
                "--json",
                "--non-interactive",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "disabled config output: {action}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let output = command(root)
            .args([
                action,
                "--proto-path",
                "server/configs",
                "--protocol-config=true",
                "--json",
                "--non-interactive",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value["failure"]["code"], "E_OUTPUT_SCOPE",
            "{action}: {value}"
        );
    }
}

struct Web {
    child: Child,
    base: String,
    token: String,
    client: Client,
}
impl Web {
    fn start(root: &Path) -> Self {
        let mut child = command(root)
            .args(["web", "--no-open", "--non-interactive"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let url = reqwest::Url::parse(line.trim()).unwrap();
        Self {
            child,
            base: format!("http://{}:{}", url.host_str().unwrap(), url.port().unwrap()),
            token: url
                .fragment()
                .unwrap()
                .strip_prefix("token=")
                .unwrap()
                .to_owned(),
            client: Client::builder()
                .timeout(Duration::from_secs(20))
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
    fn post(&self, path: &str, payload: &Value) -> Response {
        self.client
            .post(format!("{}{path}", self.base))
            .header("X-Proto-Token", &self.token)
            .json(payload)
            .send()
            .unwrap()
    }
    fn job(&self, action: &str, form: &Value) -> (String, Value) {
        let response = self.post("/api/jobs", &json!({"action":action,"form":form}));
        assert!(response.status().is_success());
        let id = response.json::<Value>().unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            let value = self
                .get(&format!("/api/jobs/{id}"))
                .json::<Value>()
                .unwrap();
            if value["finished"] == true {
                return (id, value);
            }
            assert!(Instant::now() < deadline, "task timed out");
            thread::sleep(Duration::from_millis(25));
        }
    }
}
impl Drop for Web {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn result_capabilities_export_exact_backup_bytes_and_reject_tampering() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let web = Web::start(root);
    let state: Value = web.get("/api/state").json().unwrap();
    let form = json!({"root":state["root"],"settings":state["settings"],"force":false,"adopt_existing":false});
    let (plan, preview) = web.job("plan", &form);
    assert_eq!(preview["status"], "success", "{preview}");
    assert_eq!(preview["form"], form);
    assert!(preview["locations"]["server"].is_null());
    assert!(web
        .post("/api/open", &json!({"job_id":plan,"kind":"server"}))
        .status()
        .is_client_error());
    assert!(web
        .post("/api/open", &json!({"job_id":"missing","kind":"server"}))
        .status()
        .is_client_error());
    assert!(web
        .post(
            "/api/open",
            &json!({"job_id":plan,"kind":"server","path":"C:\\Windows"})
        )
        .status()
        .is_client_error());
    let (first, built) = web.job("build", &form);
    assert_eq!(built["status"], "success", "{built}");
    let initial: Value = web.get(&format!("/api/backups/{first}")).json().unwrap();
    assert!(initial["files"].as_array().unwrap().is_empty());
    let go = root.join("server/protocol/act7001/bonus.pb.go");
    let generated = fs::read(&go).unwrap();
    let edited = b"manual original bytes\r\n\0\xff";
    // Existing legacy outputs may use Unicode names even though the older
    // bundled protoc cannot compile such names under every Windows code page.
    let stale = root.join("server/protocol/act7001/协议.pb.go");
    fs::write(&stale, edited).unwrap();
    let manifest_path = Path::new(initial["directory"].as_str().unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let key = stale.to_string_lossy().replace('\\', "/").to_lowercase();
    manifest["files"][key] =
        json!({"target":stale,"sha256":hex::encode(Sha256::digest(edited)),"scope":"go:act7001"});
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let (second, result) = web.job("build", &form);
    assert_eq!(result["status"], "success", "{result}");
    assert_eq!(fs::read(&go).unwrap(), generated);
    assert!(!stale.exists());
    assert_eq!(
        Path::new(result["locations"]["server"].as_str().unwrap()),
        root.join("server/protocol")
    );
    assert!(result["locations"]["calc"].is_null());
    let backups: Value = web.get(&format!("/api/backups/{second}")).json().unwrap();
    let entries = backups["files"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{backups}");
    assert_eq!(entries[0]["name"], "协议.pb.go");
    let route = entries[0]["download_url"].as_str().unwrap();
    assert_eq!(
        web.client
            .get(format!("{}{route}", web.base))
            .send()
            .unwrap()
            .status(),
        403
    );
    let response = web.get(route);
    assert_eq!(response.status(), 200);
    assert!(response.headers()["Content-Disposition"]
        .to_str()
        .unwrap()
        .contains("filename*=UTF-8''%E5%8D%8F%E8%AE%AE.pb.go"));
    assert_eq!(response.bytes().unwrap().as_ref(), edited);
    let directory = Path::new(backups["directory"].as_str().unwrap());
    let index = directory.join("index.json");
    let journal: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
    let saved = journal["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|op| op["target"].as_str().is_some_and(|p| Path::new(p) == stale))
        .unwrap()["backup"]
        .as_str()
        .unwrap();
    // Editing the on-disk journal cannot redirect the server's already-bound capability.
    fs::write(
        &index,
        b"{\"operations\":[{\"backup\":\"C:\\\\Windows\\\\win.ini\"}]}",
    )
    .unwrap();
    assert_eq!(web.get(route).bytes().unwrap().as_ref(), edited);
    assert!(web
        .get(&format!("/api/backup/{second}/99999"))
        .status()
        .is_client_error());
    assert!(web
        .get(&format!("/api/backup/{second}/0/extra"))
        .status()
        .is_client_error());
    assert!(web.get("/api/backups/missing").status().is_client_error());
    fs::write(saved, b"changed backup").unwrap();
    let changed = web.get(route);
    assert!(changed.status().is_client_error());
    assert!(changed.text().unwrap().contains("E_BACKUP_CHANGED"));
}
