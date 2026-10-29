use crate::{
    build,
    cli::Options,
    config::{self, Loaded, Settings},
    report::Reporter,
    scan, ui, util,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
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
const MAX_BACKUP_BYTES: u64 = 128 * 1024 * 1024;

struct Job {
    id: String,
    project_root: PathBuf,
    reporter: Reporter,
    finished: AtomicBool,
    report_error: Mutex<Option<String>>,
    loaded: Loaded,
    form: Form,
    artifacts: Mutex<Result<JobArtifacts, String>>,
}
#[derive(Default, Serialize)]
struct Locations {
    project: PathBuf,
    server: Option<PathBuf>,
    calc: Option<PathBuf>,
    docs: Option<PathBuf>,
    backup: Option<PathBuf>,
}
#[derive(Default)]
struct JobArtifacts {
    locations: Locations,
    backups: Vec<BackupFile>,
}
struct BackupFile {
    target: PathBuf,
    path: PathBuf,
    name: String,
    sha256: String,
    size: u64,
}
struct State {
    loaded: Mutex<Loaded>,
    flags: Mutex<(bool, bool)>,
    jobs: Mutex<VecDeque<Arc<Job>>>,
    base: String,
    token: String,
}
#[derive(Clone, Serialize, Deserialize)]
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenArtifact {
    job_id: String,
    kind: ArtifactKind,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ArtifactKind {
    Docs,
    Server,
    Calc,
    Backup,
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
            bail!("[E_WEB_CHANGED] 协议项目根目录已改变，请先打开项目 / Open the changed protocol project first");
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
            "setup":crate::setup::status(&loaded)?,
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
            loaded: loaded.clone(),
            form: request.form.clone(),
            artifacts: Mutex::new(Ok(JobArtifacts::default())),
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
                    build::run(&loaded, &build::BuildPolicy { force: options.force, adopt_existing: options.adopt_existing }, &job.reporter)
                }
            })).unwrap_or_else(|_| Err(anyhow::anyhow!("[E_INTERNAL] 构建任务意外退出，请查看终端 / The operation panicked; see the terminal")));
            if let Err(error) = job.reporter.finish(&result, None) {
                *job.report_error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("[E_REPORT] {error:#}"));
            }
            *job.artifacts.lock().unwrap_or_else(|e| e.into_inner()) =
                capture_artifacts(&job).map_err(|e| format!("{e:#}"));
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
    if request.method() == &Method::Get && path.starts_with("/api/backup/") {
        match download_backup(state, &path) {
            Ok((name, bytes)) => {
                respond(request, 200, "application/octet-stream", bytes, Some(name))
            }
            Err(error) => respond(
                request,
                400,
                "application/json; charset=utf-8",
                serde_json::to_vec(&json!({"error":format!("{error:#}")})).unwrap_or_default(),
                None,
            ),
        }
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
        (Method::Post, "/api/open") => {
            let input: OpenArtifact = body(request)?;
            let job = state.job(&input.job_id)?;
            require_finished(&job)?;
            let artifacts = job.artifacts.lock().unwrap_or_else(|e| e.into_inner());
            let artifacts = artifacts.as_ref().map_err(|e| anyhow::anyhow!(e.clone()))?;
            let document = matches!(input.kind, ArtifactKind::Docs);
            let path = match input.kind {
                ArtifactKind::Docs => &artifacts.locations.docs,
                ArtifactKind::Server => &artifacts.locations.server,
                ArtifactKind::Calc => &artifacts.locations.calc,
                ArtifactKind::Backup => &artifacts.locations.backup,
            }
            .as_ref()
            .context("[E_WEB_NOT_FOUND] 此任务没有可打开的目标 / No artifact for this task")?;
            build::reject_reparse_points(path)?;
            if !(if document {
                path.is_file()
            } else {
                path.is_dir()
            }) {
                bail!(
                    "[E_PATH] 目标已不存在 / Artifact no longer exists: {}",
                    path.display()
                );
            }
            open_browser(&path.display().to_string())?;
            Ok(json!({"opened":true,"path":path}))
        }
        (Method::Get, path) if path.starts_with("/api/backups/") => {
            let job = state.job(&path[13..])?;
            require_finished(&job)?;
            let artifacts = job.artifacts.lock().unwrap_or_else(|e| e.into_inner());
            let artifacts = artifacts.as_ref().map_err(|e| anyhow::anyhow!(e.clone()))?;
            Ok(
                json!({"directory":artifacts.locations.backup,"files":artifacts.backups.iter().enumerate().map(|(index,file)|json!({"index":index,"target":file.target,"name":file.name,"size":file.size,"download_url":format!("/api/backup/{}/{index}",job.id)})).collect::<Vec<_>>()}),
            )
        }
        (Method::Get, path) if path.starts_with("/api/jobs/") => {
            let job = state.job(&path[10..])?;
            let url = reqwest::Url::parse(&format!("{}{}", state.base, request.url()))?;
            let after = url
                .query_pairs()
                .find(|(key, _)| key == "after")
                .and_then(|(_, v)| v.parse().ok())
                .unwrap_or(0);
            let finished = job.finished.load(Ordering::Acquire);
            let mut value = job.reporter.snapshot(after)?;
            value["finished"] = finished.into();
            value["report_error"] =
                json!(*job.report_error.lock().unwrap_or_else(|e| e.into_inner()));
            value["form"] = serde_json::to_value(&job.form)?;
            if finished {
                let artifacts = job.artifacts.lock().unwrap_or_else(|e| e.into_inner());
                match artifacts.as_ref() {
                    Ok(artifacts) => {
                        value["locations"] = serde_json::to_value(&artifacts.locations)?
                    }
                    Err(error) => value["artifacts_error"] = json!(error),
                }
            }
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
fn require_finished(job: &Job) -> Result<()> {
    if !job.finished.load(Ordering::Acquire) {
        bail!("[E_WEB_BUSY] 请等待任务结束 / Wait for the operation to finish");
    }
    Ok(())
}
fn checked_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    build::reject_reparse_points(path)?;
    let file = fs::File::open(path)
        .with_context(|| format!("[E_BACKUP] 无法读取文件 / Cannot read {}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > limit {
        bail!("[E_BACKUP] 文件类型或大小不受支持 / Unsupported file type or size");
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("[E_BACKUP] 文件过大 / File too large");
    }
    Ok(bytes)
}
// Bind file capabilities to a completed task. Never use client-provided paths or reread
// an editable journal to decide which file a later download may access.
fn capture_artifacts(job: &Job) -> Result<JobArtifacts> {
    let report = job.reporter.snapshot(0)?;
    let loaded = &job.loaded;
    let mut result = JobArtifacts::default();
    result.locations.project = loaded.root.clone();
    if report["command"] != "build" {
        return Ok(result);
    }
    let changes = &report["result"]["changes"];
    let targets = ["added", "modified", "removed", "unchanged"]
        .iter()
        .flat_map(|key| changes[key].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let server = loaded.settings.server(&loaded.root).join("protocol");
    let calc = loaded.settings.calc(&loaded.root).join("protocol");
    let docs = loaded.settings.proto(&loaded.root).join("protocol.html");
    for (slot, path, enabled) in [
        (&mut result.locations.server, &server, true),
        (
            &mut result.locations.calc,
            &calc,
            loaded.settings.copy_to_calc,
        ),
    ] {
        if enabled && path.is_dir() && targets.iter().any(|p| build::within(p, path)) {
            build::reject_reparse_points(path)?;
            *slot = Some(path.clone());
        }
    }
    if loaded.settings.docs
        && docs.is_file()
        && targets
            .iter()
            .any(|p| util::path_key(p) == util::path_key(&docs))
    {
        build::reject_reparse_points(&docs)?;
        result.locations.docs = Some(docs);
    }
    let Some(directory) = changes["backup"].as_str().map(PathBuf::from) else {
        return Ok(result);
    };
    let state = util::project_state(
        &loaded.root,
        &loaded.settings.server(&loaded.root),
        &loaded.settings.calc(&loaded.root),
    )?;
    let expected_parent = state.join("backups");
    if !directory.is_absolute()
        || directory
            .parent()
            .is_none_or(|p| util::path_key(p) != util::path_key(&expected_parent))
    {
        bail!("[E_BACKUP] 备份目录不属于此任务 / Backup directory is outside this task");
    }
    build::reject_reparse_points(&directory)?;
    let journal: BackupIndex = serde_json::from_slice(&checked_file(
        &directory.join("index.json"),
        4 * 1024 * 1024,
    )?)?;
    if util::path_key(&journal.backup_dir) != util::path_key(&directory) {
        bail!("[E_BACKUP] 备份索引目录不匹配 / Backup index directory mismatch");
    }
    let allowed = ["modified", "removed"]
        .iter()
        .flat_map(|key| changes[key].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .map(|p| util::path_key(Path::new(p)))
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for (index, op) in journal.operations.into_iter().enumerate() {
        let Some(backup) = op.backup else {
            continue;
        };
        // The transaction also backs up its private manifest; never export it.
        if util::path_key(&op.target) == util::path_key(&state.join("manifest.json")) {
            continue;
        }
        if !allowed.contains(&util::path_key(&op.target))
            || !seen.insert(util::path_key(&op.target))
            || util::path_key(&backup) != util::path_key(&directory.join(format!("{index:06}.bak")))
        {
            bail!("[E_BACKUP] 备份索引超出任务范围 / Backup index exceeds this task's outputs");
        }
        let bytes = checked_file(&backup, MAX_BACKUP_BYTES)?;
        let name = op
            .target
            .file_name()
            .context("[E_BACKUP] Missing original file name")?
            .to_string_lossy()
            .into_owned();
        result.backups.push(BackupFile {
            target: op.target,
            path: backup,
            name,
            sha256: util::hash(&bytes),
            size: bytes.len() as u64,
        });
    }
    result.locations.backup = Some(directory);
    Ok(result)
}
#[derive(Deserialize)]
struct BackupIndex {
    backup_dir: PathBuf,
    operations: Vec<BackupOperation>,
}
#[derive(Deserialize)]
struct BackupOperation {
    target: PathBuf,
    backup: Option<PathBuf>,
}
fn download_backup(state: &State, route: &str) -> Result<(String, Vec<u8>)> {
    let segments = route
        .strip_prefix("/api/backup/")
        .context("[E_WEB_NOT_FOUND] Invalid download")?
        .split('/')
        .collect::<Vec<_>>();
    if segments.len() != 2 {
        bail!("[E_WEB_NOT_FOUND] Invalid backup download");
    }
    let job = state.job(segments[0])?;
    require_finished(&job)?;
    let index: usize = segments[1]
        .parse()
        .context("[E_WEB_NOT_FOUND] Invalid backup index")?;
    let artifacts = job.artifacts.lock().unwrap_or_else(|e| e.into_inner());
    let artifacts = artifacts.as_ref().map_err(|e| anyhow::anyhow!(e.clone()))?;
    let file = artifacts
        .backups
        .get(index)
        .context("[E_WEB_NOT_FOUND] Backup not found")?;
    let bytes = checked_file(&file.path, MAX_BACKUP_BYTES)?;
    if bytes.len() as u64 != file.size || util::hash(&bytes) != file.sha256 {
        bail!(
            "[E_BACKUP_CHANGED] 备份已被修改，拒绝导出 / Backup changed since this task completed"
        );
    }
    Ok((file.name.clone(), bytes))
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
        let encoded = name
            .as_bytes()
            .iter()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() || b"-._~".contains(byte) {
                    (*byte as char).to_string()
                } else {
                    format!("%{byte:02X}")
                }
            })
            .collect::<String>();
        let fallback = if name.is_ascii() && !name.contains(['"', '\\', '\r', '\n']) {
            name.as_str()
        } else {
            "backup"
        };
        if let Ok(header) = Header::from_bytes(
            "Content-Disposition",
            format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}"),
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
