use crate::{
    build,
    cli::Options,
    config::{self, Loaded, Settings},
    report::Reporter,
    scan, ui, util,
};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeSet, VecDeque},
    fs,
    io::Read,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const MAX_BODY: u64 = 256 * 1024;
const MAX_JOBS: usize = 8;

struct Job {
    id: String,
    project_root: PathBuf,
    reporter: Reporter,
    finished: AtomicBool,
    report_error: Mutex<Option<String>>,
}
struct State {
    loaded: Mutex<Loaded>,
    flags: Mutex<(bool, bool)>,
    jobs: Mutex<VecDeque<Arc<Job>>>,
    base: String,
    token: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Form {
    root: PathBuf,
    settings: Settings,
    #[serde(default)]
    force: bool,
    #[serde(default)]
    adopt_existing: bool,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Plan,
    Doctor,
    Build,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Doctor => "doctor",
            Self::Build => "build",
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartJob {
    action: Action,
    form: Form,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenProject {
    root: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveConfig {
    form: Form,
    expected_hash: Option<String>,
}

pub fn run(
    loaded: Loaded,
    options: &Options,
    port: u16,
    auto_open: bool,
    report: &Reporter,
) -> Result<()> {
    if options.json || options.interactive || options.dry_run {
        bail!("[E_ARGUMENT] web cannot be combined with --json, --interactive or --dry-run; use the web Preview action for a read-only plan");
    }
    let server = Server::http((Ipv4Addr::LOCALHOST, port))
        .map_err(|error| anyhow::anyhow!("[E_WEB] cannot listen on loopback: {error}"))?;
    let address = server
        .server_addr()
        .to_ip()
        .context("[E_WEB] missing listening address")?;
    let base = format!("http://{address}");
    let mut random = [0u8; 32];
    getrandom::getrandom(&mut random)
        .map_err(|e| anyhow::anyhow!("[E_WEB] session randomness: {e}"))?;
    let token = hex::encode(random);
    let launch = format!("{base}/#token={token}");
    let state = Arc::new(State {
        loaded: Mutex::new(loaded),
        flags: Mutex::new((options.force, options.adopt_existing)),
        jobs: Mutex::new(VecDeque::new()),
        base,
        token,
    });
    report.info(
        "web",
        report.text(
            "本地网页工作台已启动；Ctrl+C 关闭",
            "Local web workspace started; Ctrl+C to stop",
        ),
    );
    // The capability URL is shown only in the terminal, never persisted to the normal report.
    println!("{launch}");
    if auto_open {
        if let Err(error) = open_browser(&launch) {
            report.emit(
                "web",
                "warning",
                format!("请用浏览器打开上方完整地址 / Open the complete URL above: {error:#}"),
            );
        }
    }
    loop {
        if util::SHUTDOWN.load(Ordering::Acquire) {
            util::CANCELLED.store(true, Ordering::Release);
            if !state.busy() {
                break;
            }
        }
        if let Some(request) = server.recv_timeout(Duration::from_millis(150))? {
            handle(request, &state);
        }
    }
    report.result(json!({"address":state.base,"stopped":true}));
    Ok(())
}

impl State {
    fn busy(&self) -> bool {
        self.jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|j| !j.finished.load(Ordering::Acquire))
    }
    fn idle(&self) -> Result<()> {
        if self.busy() {
            bail!("[E_WEB_BUSY] 当前操作尚未结束 / An operation is still running");
        }
        if util::SHUTDOWN.load(Ordering::Acquire) {
            bail!("[E_WEB_BUSY] 服务正在关闭 / Server is stopping");
        }
        Ok(())
    }
    fn form(&self, form: &Form) -> Result<Loaded> {
        config::validate(&form.settings)?;
        let current = self
            .loaded
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let root = util::absolute(&std::env::current_dir()?, &form.root);
        if util::path_key(&root) != util::path_key(&current.root) {
            bail!("[E_WEB_CHANGED] 项目目录已改变，请先打开项目 / Open the changed project first");
        }
        if !root.is_dir() {
            bail!(
                "[E_CONFIG] project directory does not exist: {}",
                root.display()
            );
        }
        Ok(Loaded {
            settings: form.settings.clone(),
            ..current
        })
    }
    fn snapshot(&self) -> Result<Value> {
        let loaded = self.loaded.lock().unwrap_or_else(|e| e.into_inner());
        let last = self
            .jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .rev()
            .find(|j| util::path_key(&j.project_root) == util::path_key(&loaded.root))
            .map(|j| j.id.clone());
        let (force, adopt_existing) = *self.flags.lock().unwrap_or_else(|e| e.into_inner());
        Ok(
            json!({"version":env!("CARGO_PKG_VERSION"),"root":loaded.root,"settings":loaded.settings,
            "sources":loaded.sources,"project_file":loaded.project_file,
            "force":force,"adopt_existing":adopt_existing,"last_job":last}),
        )
    }
    fn start(self: &Arc<Self>, request: StartJob) -> Result<Value> {
        self.idle()?;
        let loaded = self.form(&request.form)?;
        util::write_json(
            &util::data_dir()?.join("recent-project.json"),
            &json!({"root":loaded.root}),
        )?;
        let options = Options {
            force: request.form.force,
            adopt_existing: request.form.adopt_existing,
            ..Default::default()
        };
        let job = Arc::new(Job {
            id: util::run_id(),
            project_root: loaded.root.clone(),
            reporter: Reporter::for_web(
                request.action.name(),
                &loaded.settings.lang,
                loaded.settings.log_retention,
            ),
            finished: AtomicBool::new(false),
            report_error: Mutex::new(None),
        });
        let command = ui::equivalent(&loaded, request.action.name(), &options);
        job.reporter.info("command", &command);
        job.reporter.context(json!({"project_root":loaded.root,"proto_root":loaded.settings.proto(&loaded.root),"configuration_sources":loaded.sources}));
        util::CANCELLED.store(false, Ordering::Release);
        *self.loaded.lock().unwrap_or_else(|e| e.into_inner()) = loaded.clone();
        *self.flags.lock().unwrap_or_else(|e| e.into_inner()) =
            (options.force, options.adopt_existing);
        {
            let mut jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
            while jobs.len() >= MAX_JOBS {
                jobs.pop_front();
            }
            jobs.push_back(job.clone());
        }
        let id = job.id.clone();
        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match request.action {
                Action::Plan => build::plan(&loaded, &job.reporter),
                Action::Doctor => build::doctor(&loaded, &job.reporter),
                Action::Build => {
                    crate::start_update_check(&loaded.settings, &job.reporter);
                    build::run(&loaded, &options, &job.reporter)
                }
            })).unwrap_or_else(|_| Err(anyhow::anyhow!("[E_INTERNAL] 构建任务意外退出，请查看终端 / The operation panicked; see the terminal")));
            if let Err(error) = job.reporter.finish(&result, None) {
                *job.report_error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("[E_REPORT] {error:#}"));
            }
            job.finished.store(true, Ordering::Release);
        });
        Ok(json!({"id":id,"command":command}))
    }
    fn job(&self, id: &str) -> Result<Arc<Job>> {
        self.jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|j| j.id == id)
            .cloned()
            .context("[E_WEB_NOT_FOUND] job not found; reports remain in the local log directory")
    }
}

fn handle(mut request: Request, state: &Arc<State>) {
    let host = header(&request, "Host");
    let expected_host = state.base.strip_prefix("http://").unwrap();
    let origin = header(&request, "Origin");
    if host != Some(expected_host) || origin.is_some_and(|value| value != state.base) {
        respond(
            request,
            403,
            "application/json",
            br#"{"error":"Local origin required"}"#.to_vec(),
            None,
        );
        return;
    }
    let path = request.url().split('?').next().unwrap_or("").to_owned();
    if request.method() == &Method::Get {
        let asset = match path.as_str() {
            "/" => Some((
                "text/html; charset=utf-8",
                include_str!("../web/index.html"),
            )),
            "/app.js" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../web/app.js"),
            )),
            "/styles.css" => Some(("text/css; charset=utf-8", include_str!("../web/styles.css"))),
            "/favicon.svg" => Some(("image/svg+xml", include_str!("../web/favicon.svg"))),
            _ => None,
        };
        if let Some((mime, contents)) = asset {
            respond(request, 200, mime, contents.as_bytes().to_vec(), None);
            return;
        }
    }
    if !constant_equal(
        header(&request, "X-Proto-Token").unwrap_or(""),
        &state.token,
    ) {
        respond(
            request,
            403,
            "application/json",
            br#"{"error":"Session link expired or missing; reopen the complete terminal URL"}"#
                .to_vec(),
            None,
        );
        return;
    }
    let result = route(&mut request, state, &path);
    match result {
        Ok(value) => {
            let download = path
                .strip_prefix("/api/report/")
                .map(|id| format!("proto-builder-{id}.json"));
            respond(
                request,
                200,
                "application/json; charset=utf-8",
                serde_json::to_vec(&value).unwrap_or_default(),
                download,
            );
        }
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("E_WEB_BUSY") || message.contains("E_WEB_CHANGED") {
                409
            } else if message.contains("E_WEB_NOT_FOUND") {
                404
            } else if message.contains("E_WEB_BODY") {
                413
            } else {
                400
            };
            respond(
                request,
                status,
                "application/json; charset=utf-8",
                serde_json::to_vec(&json!({"error":message})).unwrap_or_default(),
                None,
            );
        }
    }
}
fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}
fn constant_equal(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |sum, (a, b)| sum | (a ^ b))
        == 0
}
fn body<T: serde::de::DeserializeOwned>(request: &mut Request) -> Result<T> {
    if header(request, "Content-Type")
        .is_none_or(|value| value.split(';').next().unwrap_or("").trim() != "application/json")
    {
        bail!("[E_WEB] Content-Type must be application/json");
    }
    if request
        .body_length()
        .is_some_and(|length| length as u64 > MAX_BODY)
    {
        bail!("[E_WEB_BODY] request is too large");
    }
    let mut bytes = Vec::new();
    request
        .as_reader()
        .take(MAX_BODY + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BODY {
        bail!("[E_WEB_BODY] request is too large");
    }
    serde_json::from_slice(&bytes).context("[E_WEB] invalid request JSON")
}
fn route(request: &mut Request, state: &Arc<State>, path: &str) -> Result<Value> {
    match (request.method().clone(), path) {
        (Method::Get, "/api/state") => state.snapshot(),
        (Method::Post, "/api/project") => {
            state.idle()?;
            let input: OpenProject = body(request)?;
            let root = util::absolute(&std::env::current_dir()?, &input.root);
            if !root.is_dir() {
                bail!(
                    "[E_CONFIG] project directory does not exist: {}",
                    root.display()
                );
            }
            let loaded = config::load(&Options {
                repo_root: Some(root),
                ..Default::default()
            })?;
            util::write_json(
                &util::data_dir()?.join("recent-project.json"),
                &json!({"root":loaded.root}),
            )?;
            *state.loaded.lock().unwrap_or_else(|e| e.into_inner()) = loaded;
            *state.flags.lock().unwrap_or_else(|e| e.into_inner()) = (false, false);
            state.snapshot()
        }
        (Method::Post, "/api/catalog") => {
            let form: Form = body(request)?;
            let mut loaded = state.form(&form)?;
            loaded.settings.assign_id = 0;
            loaded.settings.activity_ids.clear();
            loaded.settings.machine_ids.clear();
            loaded.settings.excludes.clear();
            let plan = scan::make(&loaded.root, &loaded.settings)?;
            let activities = plan
                .selected
                .iter()
                .filter_map(|i| i.activity_id.clone())
                .collect::<BTreeSet<_>>();
            let machines = plan
                .selected
                .iter()
                .filter_map(|i| i.machine_id.clone())
                .collect::<BTreeSet<_>>();
            Ok(
                json!({"activities":activities,"machines":machines,"directories":plan.groups.keys().filter(|p|p.as_str()!=".").collect::<Vec<_>>(),"proto_files":plan.scanned}),
            )
        }
        (Method::Get, "/api/directories") => {
            let url = reqwest::Url::parse(&format!("{}{}", state.base, request.url()))?;
            let loaded = state.loaded.lock().unwrap_or_else(|e| e.into_inner());
            let path = url
                .query_pairs()
                .find(|(key, _)| key == "path")
                .map(|(_, v)| PathBuf::from(v.as_ref()))
                .unwrap_or_else(|| loaded.root.clone());
            directories(&util::absolute(&loaded.root, &path))
        }
        (Method::Post, "/api/jobs") => state.start(body(request)?),
        (Method::Get, path) if path.starts_with("/api/jobs/") => {
            let job = state.job(&path[10..])?;
            let url = reqwest::Url::parse(&format!("{}{}", state.base, request.url()))?;
            let after = url
                .query_pairs()
                .find(|(key, _)| key == "after")
                .and_then(|(_, v)| v.parse().ok())
                .unwrap_or(0);
            let mut value = job.reporter.snapshot(after)?;
            value["finished"] = job.finished.load(Ordering::Acquire).into();
            value["report_error"] =
                json!(*job.report_error.lock().unwrap_or_else(|e| e.into_inner()));
            Ok(value)
        }
        (Method::Post, path) if path.starts_with("/api/cancel/") => {
            let job = state.job(&path[12..])?;
            if !job.finished.load(Ordering::Acquire) {
                util::CANCELLED.store(true, Ordering::Release);
            }
            Ok(json!({"cancel_requested":true}))
        }
        (Method::Get, path) if path.starts_with("/api/report/") => {
            state.job(&path[12..])?.reporter.snapshot(0)
        }
        (Method::Post, "/api/config/preview") => {
            state.idle()?;
            let form: Form = body(request)?;
            let loaded = state.form(&form)?;
            let (path, text) = configuration(&loaded)?;
            let previous = if path.is_file() {
                fs::read_to_string(&path)?
            } else {
                String::new()
            };
            let diff = similar::TextDiff::from_lines(&previous, &text)
                .unified_diff()
                .header("current", "proposed")
                .to_string();
            Ok(
                json!({"path":path,"text":text,"diff":diff,"expected_hash":if path.is_file(){Some(util::file_hash(&path)?)}else{None}}),
            )
        }
        (Method::Post, "/api/config/save") => {
            state.idle()?;
            let input: SaveConfig = body(request)?;
            let mut loaded = state.form(&input.form)?;
            let (path, _) = configuration(&loaded)?;
            let current = if path.is_file() {
                Some(util::file_hash(&path)?)
            } else {
                None
            };
            if current != input.expected_hash {
                bail!("[E_WEB_CHANGED] 配置已被其他程序修改，请重新预览 / Configuration changed; review it again");
            }
            let mut saved = loaded.settings.clone();
            saved.project_root = if path.parent() == Some(loaded.root.as_path()) {
                PathBuf::from(".")
            } else {
                loaded.root.clone()
            };
            util::no_symlinks(&path)?;
            config::save(&path, &saved)?;
            if !loaded.sources.contains(&path) {
                loaded.sources.push(path.clone());
            }
            loaded.project_file = Some(path.clone());
            *state.loaded.lock().unwrap_or_else(|e| e.into_inner()) = loaded;
            Ok(json!({"saved":true,"path":path}))
        }
        (Method::Post, "/api/shutdown") => {
            util::SHUTDOWN.store(true, Ordering::Release);
            util::CANCELLED.store(true, Ordering::Release);
            Ok(json!({"stopping":true}))
        }
        _ => bail!("[E_WEB_NOT_FOUND] route not found"),
    }
}
fn configuration(loaded: &Loaded) -> Result<(PathBuf, String)> {
    let path = loaded
        .project_file
        .clone()
        .unwrap_or_else(|| loaded.root.join("proto-builder.toml"));
    let mut saved = loaded.settings.clone();
    saved.project_root = if path.parent() == Some(loaded.root.as_path()) {
        PathBuf::from(".")
    } else {
        loaded.root.clone()
    };
    Ok((path, toml::to_string_pretty(&saved)?))
}
fn directories(path: &Path) -> Result<Value> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("[E_PATH] cannot open {}", path.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|t| t.is_dir() && !t.is_symlink())
        })
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| json!({"name":entry.file_name().to_string_lossy(),"path":entry.path()}))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry["name"].as_str().unwrap_or("").to_lowercase());
    let total = entries.len();
    entries.truncate(1000);
    #[cfg(windows)]
    let roots = {
        let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
        (0..26)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| format!("{}:\\", (b'A' + i as u8) as char))
            .collect::<Vec<_>>()
    };
    #[cfg(not(windows))]
    let roots = vec!["/".to_owned()];
    Ok(json!({"path":path,"parent":path.parent(),"roots":roots,"entries":entries,"total":total}))
}
fn respond(request: Request, status: u16, mime: &str, bytes: Vec<u8>, download: Option<String>) {
    let mut response = Response::from_data(bytes).with_status_code(StatusCode(status));
    for (name,value) in [
        ("Content-Type",mime), ("Cache-Control","no-store"), ("X-Content-Type-Options","nosniff"),
        ("Referrer-Policy","no-referrer"),
        ("Content-Security-Policy","default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; object-src 'none'; base-uri 'none'; form-action 'none'"),
    ] {
        if let Ok(header) = Header::from_bytes(name,value) { response.add_header(header); }
    }
    if let Some(name) = download {
        if let Ok(header) = Header::from_bytes(
            "Content-Disposition",
            format!("attachment; filename=\"{name}\""),
        ) {
            response.add_header(header);
        }
    }
    let _ = request.respond(response);
}
#[cfg(windows)]
fn open_browser(url: &str) -> Result<()> {
    let url = url.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let operation = "open".encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result as isize <= 32 {
        bail!("[E_WEB] Windows could not open the default browser");
    }
    Ok(())
}
#[cfg(not(windows))]
fn open_browser(url: &str) -> Result<()> {
    std::process::Command::new(if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    })
    .arg(url)
    .spawn()?;
    Ok(())
}
