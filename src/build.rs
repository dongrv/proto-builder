use crate::{
    cli::Options,
    config::{Loaded, Settings},
    process, protocol,
    report::Reporter,
    scan, svn, toolchain,
    transaction::{self, Output},
    util,
};
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub fn plan(loaded: &Loaded, report: &Reporter) -> Result<()> {
    crate::setup::notify(loaded, report)?;
    let p = scan::make(&loaded.root, &loaded.settings)?;
    report.info(
        "plan",
        format!(
            "{} {} | {} {} | {} {} | {} {}",
            report.text("扫描", "scanned"),
            p.scanned,
            report.text("选中", "selected"),
            p.selected.len(),
            report.text("排除", "excluded"),
            p.excluded.len(),
            report.text("筛选跳过", "filtered"),
            p.filtered.len()
        ),
    );
    for (group, indices) in &p.groups {
        report.info(
            "select",
            format!(
                "{group}: {}",
                indices
                    .iter()
                    .map(|i| p.selected[*i].relative.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    for skipped in p.excluded.iter().chain(&p.filtered) {
        report.detail("skip", format!("{}: {}", skipped.reason, skipped.file));
    }
    for dependency in &p.import_only {
        report.detail("import", format!("{dependency} (read only)"));
    }
    let server = loaded.settings.server(&loaded.root).join("protocol");
    let calc = loaded.settings.calc(&loaded.root).join("protocol");
    report.info(
        "output",
        format!("server={} | calc={}", server.display(), calc.display()),
    );
    let state = util::project_state(
        &loaded.root,
        &loaded.settings.server(&loaded.root),
        &loaded.settings.calc(&loaded.root),
    )?;
    let manifest = transaction::read_manifest(&state)?;
    for item in manifest.files.values() {
        if item.target.is_file() && util::file_hash(&item.target)? != item.sha256 {
            report.emit(
                "conflict",
                "warning",
                format!("{} (modified)", item.target.display()),
            );
        }
    }
    report.result(serde_json::to_value(&p)?);
    Ok(())
}
pub fn run(loaded: &Loaded, options: &Options, report: &Reporter) -> Result<()> {
    let root = &loaded.root;
    let s = &loaded.settings;
    crate::setup::notify(loaded, report)?;
    let proto = s.proto(root);
    let server = s.server(root);
    let calc = s.calc(root);
    if !proto.is_dir() {
        bail!(
            "[E_INPUT] protobuf directory does not exist: {}",
            proto.display()
        );
    }
    let roots = vec![
        proto.clone(),
        server.join("protocol"),
        server.join("configs"),
        calc.join("protocol"),
    ];
    validate_roots(&roots)?;
    let lock_paths = vec![
        root.clone(),
        server.join("protocol"),
        server.join("configs"),
        calc.join("protocol"),
    ];
    let _locks = transaction::Locks::acquire(&lock_paths)?;
    let state = util::project_state(root, &server, &calc)?;
    transaction::recover(&state, report)?;
    let config_hashes = loaded
        .sources
        .iter()
        .map(|p| Ok((p.clone(), util::file_hash(p)?)))
        .collect::<Result<Vec<_>>>()?;
    let svn = svn::update(root, s, report)?;
    for (path, before) in config_hashes {
        if util::file_hash(&path)? != before {
            bail!("[E_CONFIG_CHANGED] SVN changed {}; re-run to load the updated configuration / SVN 已更新配置，请重新执行",path.display());
        }
    }
    let p = scan::make(root, s)?;
    let tools = toolchain::resolve(root, s, report)?;
    report.context(json!({"project_root":root,"proto_root":proto,"developer_name":s.developer_name,"config_sources":loaded.sources,"toolchain":tools.provenance,"svn":svn,"jobs":s.workers(),"selected_proto_files":p.selected.len(),"excluded":p.excluded,"filtered":p.filtered}));
    report.info("source", proto.display().to_string());
    report.info(
        "generate",
        format!(
            "{} {} | {} {} | {} {}",
            report.text("任务组", "groups"),
            p.groups.len(),
            report.text("协议文件", "proto files"),
            p.selected.len(),
            report.text("并行度", "jobs"),
            s.workers()
        ),
    );
    let workspace = tempfile::Builder::new()
        .prefix("proto-builder-")
        .tempdir()?;
    let input = workspace.path().join("input");
    let input_proto = input.join("proto");
    fs::create_dir_all(&input_proto)?;
    let originals = scan::discover(&proto)?;
    let mut fingerprints = vec![];
    for source in originals {
        let relative = source.strip_prefix(&proto)?;
        let destination = input_proto.join(relative);
        let bytes = fs::read(&source)?;
        fingerprints.push((source, util::hash(&bytes)));
        util::atomic_write(&destination, &bytes)?;
    }
    let go_out = workspace.path().join("server");
    fs::create_dir(&go_out)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(s.workers())
        .build()?;
    let failed = AtomicBool::new(false);
    let commands = AtomicUsize::new(0);
    let complete = AtomicUsize::new(0);
    for layer in &p.layers {
        let results: Vec<Result<()>> = pool.install(|| {
            layer
                .par_iter()
                .map(|group| {
                    if failed.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    let result = (|| -> Result<()> {
                        let indices = &p.groups[group];
                        let inputs = indices
                            .iter()
                            .map(|i| p.selected[*i].relative.clone())
                            .collect::<Vec<_>>();
                        protoc(&tools, &input_proto, &go_out, &inputs, false, s, report)?;
                        commands.fetch_add(1, Ordering::Relaxed);
                        if indices.iter().any(|i| p.selected[*i].grpc) {
                            protoc(&tools, &input_proto, &go_out, &inputs, true, s, report)?;
                            commands.fetch_add(1, Ordering::Relaxed);
                        }
                        let done = complete.fetch_add(1, Ordering::Relaxed) + 1;
                        report.info(
                            "compile",
                            format!(
                                "[{done}/{}] {group} ({} files)",
                                p.groups.len(),
                                inputs.len()
                            ),
                        );
                        Ok(())
                    })();
                    if result.is_err() {
                        failed.store(true, Ordering::Relaxed);
                    }
                    result
                })
                .collect()
        });
        let failures = results
            .into_iter()
            .filter_map(Result::err)
            .map(|e| format!("{e:#}"))
            .collect::<Vec<_>>();
        if !failures.is_empty() {
            bail!(
                "[E_BUILD] {}\n{}",
                report.text(
                    "生成失败，未写回任何产物",
                    "Generation failed; no outputs deployed"
                ),
                failures.join("\n")
            );
        }
    }
    let mut outputs = vec![];
    let mut scopes = BTreeSet::new();
    for group in p.groups.keys() {
        scopes.insert(format!("go:{group}"));
        if s.copy_to_calc {
            scopes.insert(format!("calc:{group}"));
        }
    }
    for source in util::files(&go_out)? {
        let rel = source.strip_prefix(&go_out)?;
        let group = rel
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| ".".into());
        outputs.push(Output {
            target: server.join("protocol").join(rel),
            source: source.clone(),
            scope: format!("go:{group}"),
        });
    }
    if s.docs {
        let docs = workspace.path().join("docs");
        fs::create_dir(&docs)?;
        // Shared documents describe the project, independently of Go ID selection.
        let doc_inputs = &p.documentation_inputs;
        if !doc_inputs.is_empty() {
            report.info(
                "docs",
                format!(
                    "{}: {}",
                    report.text(
                        "生成完整共享文档（不受 ID 筛选影响；应用排除项）",
                        "Generating complete shared documents (independent of ID filters; exclusions apply)",
                    ),
                    doc_inputs.len()
                ),
            );
            for format in ["markdown,protocol.md", "html,protocol.html"] {
                let mut command = Command::new(tools.get("protoc")?);
                command
                    .current_dir(&input_proto)
                    .args(doc_inputs)
                    .arg("-I")
                    .arg(".")
                    .arg(format!(
                        "--plugin=protoc-gen-doc={}",
                        tools.get("protoc-gen-doc")?.display()
                    ))
                    .arg(format!("--doc_opt={format}"))
                    .arg(format!("--doc_out={}", docs.display()));
                process::run(&mut command, "docs", s.timeout_seconds, report)?;
                commands.fetch_add(1, Ordering::Relaxed);
            }
            for source in util::files(&docs)? {
                outputs.push(Output {
                    target: proto.join(source.file_name().context("document file name")?),
                    source,
                    scope: "docs".into(),
                });
            }
            scopes.insert("docs".into());
        }
    }
    if s.protocol_config {
        report.info(
            "config",
            report.text(
                "生成完整协议编号配置",
                "Generating complete protocol ID configuration",
            ),
        );
        let config_out = workspace.path().join("config");
        let input_settings = Settings {
            proto_path: PathBuf::from("proto"),
            ..s.clone()
        };
        let generated =
            protocol::generate(&input, &config_out, &input_settings).map_err(|error| {
                anyhow::anyhow!(format!("{error:#}").replace(
                    &input_proto.display().to_string(),
                    &proto.display().to_string()
                ))
            })?;
        for source in generated {
            let relative = source.strip_prefix(&config_out)?;
            outputs.push(Output {
                target: proto.join(relative),
                source: source.clone(),
                scope: "config".into(),
            });
        }
        for name in ["fatcat", "reels", "thunder"] {
            outputs.push(Output {
                source: config_out.join("ProtoConfig.json"),
                target: server
                    .join("configs")
                    .join(name)
                    .join("sys/ProtoConfig.json"),
                scope: "config".into(),
            });
        }
        scopes.insert("config".into());
    }
    if s.copy_to_calc {
        let exclude = scan::Excludes::new(&s.excludes)?;
        let mut candidates = BTreeMap::new();
        let owned = transaction::read_manifest(&state)?;
        let generated_targets = outputs
            .iter()
            .map(|o| util::path_key(&o.target))
            .collect::<BTreeSet<_>>();
        for source in util::files(&server.join("protocol"))? {
            let relative = source.strip_prefix(server.join("protocol"))?.to_path_buf();
            // A stale owned server file scheduled for removal must not be copied back into calc.
            if owned
                .files
                .get(&util::path_key(&source))
                .is_some_and(|item| {
                    (p.full_build || scopes.contains(&item.scope))
                        && !generated_targets.contains(&util::path_key(&source))
                })
            {
                continue;
            }
            if calc_eligible(&relative)
                && !exclude.matches(&relative.to_string_lossy().replace('\\', "/"))
            {
                candidates.insert(relative, source);
            }
        }
        for source in util::files(&go_out)? {
            let relative = source.strip_prefix(&go_out)?.to_path_buf();
            if calc_eligible(&relative) {
                candidates.insert(relative, source);
            }
        }
        for (relative, source) in candidates {
            let group = relative
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| ".".into());
            outputs.push(Output {
                source,
                target: calc.join("protocol").join(relative),
                scope: format!("calc:{group}"),
            });
        }
    }
    util::check_cancelled()?;
    let observed = scan::discover(&proto)?.into_iter().collect::<BTreeSet<_>>();
    let expected = fingerprints
        .iter()
        .map(|(path, _)| path.clone())
        .collect::<BTreeSet<_>>();
    if observed != expected {
        bail!("[E_INPUT_CHANGED] protobuf input list changed during generation; re-run / 协议文件清单在生成期间发生变化，请重试");
    }
    for (path, expected) in fingerprints {
        if !path.is_file() || util::file_hash(&path)? != expected {
            bail!("[E_INPUT_CHANGED] {} changed during generation; re-run / 输入在生成期间发生变化，请重试",path.display());
        }
    }
    let changes = transaction::commit(
        &state,
        &outputs,
        &transaction::Policy {
            force: options.force,
            adopt_existing: options.adopt_existing,
            scopes: &scopes,
            full_go: p.full_build,
            full_calc: p.full_build && s.copy_to_calc,
            roots: &roots,
        },
        report,
    )?;
    let hashes = outputs
        .iter()
        .map(|o| {
            Ok(json!({"target":o.target,"sha256":util::file_hash(&o.source)?,"scope":o.scope}))
        })
        .collect::<Result<Vec<_>>>()?;
    report.result(json!({"proto_files":p.selected.len(),"documentation_proto_files":p.documentation_inputs.len(),"groups":p.groups.len(),"commands":commands.load(Ordering::Relaxed),"changes":changes,"outputs":hashes,"legacy_baseline":"not-run-at-runtime"}));
    match crate::setup::remember(loaded) {
        Ok(()) => report.info("setup", report.text("已记住本机项目路径与开发者信息", "Local project paths and developer information remembered")),
        Err(error) => report.emit("setup", "warning", format!("生成物已写回，但本机设置保存失败，下次仍需确认 / Outputs deployed, but local setup could not be saved: {error:#}")),
    }
    Ok(())
}
fn protoc(
    tools: &toolchain::Tools,
    input: &Path,
    output: &Path,
    files: &[String],
    grpc: bool,
    s: &Settings,
    report: &Reporter,
) -> Result<()> {
    let name = if grpc { "go-grpc" } else { "go" };
    let mut command = Command::new(tools.get("protoc")?);
    command
        .current_dir(input)
        .args(files)
        .arg("-I")
        .arg(".")
        .arg(format!(
            "--{name}_out=paths=source_relative:{}",
            output.display()
        ))
        .arg(format!(
            "--plugin=protoc-gen-{name}={}",
            tools.get(&format!("protoc-gen-{name}"))?.display()
        ));
    process::run(
        &mut command,
        if grpc { "grpc" } else { "protoc" },
        s.timeout_seconds,
        report,
    )?;
    Ok(())
}
fn calc_eligible(path: &Path) -> bool {
    let parts = path
        .components()
        .map(|p| p.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if parts
        .last()
        .is_some_and(|n| n == "activity.pb.go" || n == "business.pb.go")
    {
        return false;
    }
    if path.extension().is_none_or(|x| x != "go") {
        return false;
    }
    parts[..parts.len().saturating_sub(1)]
        .iter()
        .all(|n| n.starts_with("rpc") || n.starts_with("cmd"))
}
fn validate_roots(roots: &[PathBuf]) -> Result<()> {
    for (i, root) in roots.iter().enumerate() {
        util::no_symlinks(root)?;
        for other in &roots[..i] {
            if root.starts_with(other) || other.starts_with(root) {
                bail!(
                    "[E_OUTPUT_SCOPE] overlapping output roots: {} and {}",
                    root.display(),
                    other.display()
                );
            }
        }
    }
    Ok(())
}
pub fn doctor(loaded: &Loaded, report: &Reporter) -> Result<()> {
    let s = &loaded.settings;
    let tools = toolchain::resolve(&loaded.root, s, report)?;
    let p = scan::make(&loaded.root, s)?;
    let mut checks = vec![];
    checks.push(json!({"check":"configuration","passed":true,"sources":loaded.sources}));
    checks.push(json!({"check":"toolchain","passed":true,"details":tools.provenance}));
    checks.push(json!({"check":"inputs","passed":true,"selected":p.selected.len()}));
    if s.svn.update {
        checks.push(
            json!({"check":"svn","passed":true,"details":svn::snapshot(&loaded.root,s,report)?}),
        );
        let conflicts = svn::conflicts(&loaded.root, s, report)?;
        if !conflicts.is_empty() {
            bail!("[E_SVN_CONFLICT] {}", conflicts.join("\n"));
        }
    }
    for path in [s.server(&loaded.root), s.calc(&loaded.root)] {
        util::no_symlinks(&path)?;
        let parent = path
            .ancestors()
            .find(|p| p.is_dir())
            .context("no existing output ancestor")?;
        let probe = tempfile::NamedTempFile::new_in(parent)
            .with_context(|| format!("[E_PERMISSIONS] {}", parent.display()))?;
        drop(probe);
        checks.push(json!({"check":"output-permissions","path":path,"passed":true}));
    }
    report.result(json!({"checks":checks}));
    report.info(
        "doctor",
        report.text(
            "配置、工具链、输入、权限检查通过",
            "Configuration, toolchain, inputs and permissions verified",
        ),
    );
    Ok(())
}
