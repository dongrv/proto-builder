mod build;
mod cli;
mod config;
mod install;
mod process;
mod protocol;
mod report;
mod scan;
mod setup;
mod svn;
mod toolchain;
mod transaction;
mod ui;
mod update;
mod util;
mod web;

use anyhow::{bail, Context, Result};
use clap::Parser;
use cli::{Cli, Cmd, ConfigCmd, UpdateCmd};
use report::Reporter;
use std::{
    fs,
    io::{self, IsTerminal},
    sync::atomic::Ordering,
};

fn main() {
    let cli = Cli::parse();
    let _ = ctrlc::set_handler(|| {
        util::SHUTDOWN.store(true, Ordering::Relaxed);
        util::CANCELLED.store(true, Ordering::Relaxed);
    });
    let name = cli.command.as_ref().map(Cmd::name).unwrap_or("interactive");
    let language = cli.options.lang.as_deref().unwrap_or("zh-CN");
    let reporter = Reporter::new(name, cli.options.json, cli.options.verbose, language);
    let outcome = execute(&cli, &reporter);
    let report_path = reporter.finish(&outcome, cli.options.report_json.as_deref());
    if let Err(error) = &outcome {
        if !cli.options.json {
            eprintln!(
                "\nerror: {error:#}\n\n{}",
                reporter.text(
                    "已中止；请根据文件、行号和错误码修复后重试。",
                    "Aborted; correct the reported file/line and retry."
                )
            );
        }
    }
    if !cli.options.json {
        match &report_path {
            Ok(path) => eprintln!("    report  {}", path.display()),
            Err(error) => eprintln!("error: cannot save report: {error:#}"),
        }
    }
    if outcome.is_err() || report_path.is_err() {
        std::process::exit(if outcome.is_err() {
            report::exit_code(&outcome)
        } else {
            1
        });
    }
}
fn execute(cli: &Cli, report: &Reporter) -> Result<()> {
    match &cli.command {
        Some(Cmd::Licenses) => {
            let notices = [
                ("protobuf", include_str!("../assets/licenses/protobuf.txt")),
                (
                    "protobuf-go",
                    include_str!("../assets/licenses/protobuf-go.txt"),
                ),
                ("grpc-go", include_str!("../assets/licenses/grpc-go.txt")),
                (
                    "protoc-gen-doc",
                    include_str!("../assets/licenses/protoc-gen-doc.txt"),
                ),
                (
                    "Rust dependencies",
                    include_str!("../assets/licenses/rust-dependencies.txt"),
                ),
            ];
            if !report.json {
                for (name, text) in &notices {
                    println!("\n{name}\n{text}");
                }
            }
            report.result(serde_json::json!({"notices": notices}));
            return Ok(());
        }
        Some(Cmd::Version) => {
            report.result(serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"architecture":std::env::consts::ARCH,"toolchain":toolchain::manifest()?,"signed_updates_configured":update::trust_configured(),"update_public_key":update::public_key()}));
            report.info(
                "version",
                format!(
                    "proto-builder {} | {} {}",
                    env!("CARGO_PKG_VERSION"),
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ),
            );
            for tool in toolchain::manifest()?.tools {
                report.info(
                    "toolchain",
                    format!("{} {} | {}", tool.name, tool.version, tool.sha256),
                );
            }
            return Ok(());
        }
        Some(Cmd::Compat { baseline, actual }) => {
            if !baseline.is_dir() || !actual.is_dir() {
                bail!("[E_BASELINE] --baseline and --actual must both be existing directories");
            }
            let result = protocol::compare(baseline, actual)?;
            let passed = result["passed"].as_bool() == Some(true);
            report.result(result.clone());
            if !passed {
                for difference in result["differences"]
                    .as_array()
                    .context("difference array")?
                    .iter()
                    .take(20)
                {
                    report.info("difference", difference.to_string());
                }
                bail!("[E_COMPAT] byte comparison failed; baseline and actual outputs retained / 字节对比失败，双方产物已保留");
            }
            report.info(
                "compat",
                format!(
                    "{} {}",
                    result["equal_files"],
                    report.text("个文件逐字节一致", "files match byte-for-byte")
                ),
            );
            return Ok(());
        }
        Some(Cmd::Install { system, dir }) => {
            return install::install(*system, dir.as_deref(), report)
        }
        Some(Cmd::Uninstall { system, purge }) => {
            return install::uninstall(*system, *purge, report)
        }
        Some(Cmd::SelfUpdate(UpdateCmd::Rollback)) => return update::rollback(report),
        _ => {}
    }
    let mut loaded = config::load(&cli.options)?;
    let interactive = cli.options.interactive
        || (cli.command.is_none() && !cli.options.non_interactive && !cli.options.json);
    if (interactive || matches!(cli.command, Some(Cmd::Web { .. })))
        && cli.options.repo_root.is_none()
        && cli.options.config.is_none()
        && !loaded.settings.proto(&loaded.root).is_dir()
    {
        if let Ok(bytes) = fs::read(util::data_dir()?.join("recent-project.json")) {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(root) = value["root"]
                    .as_str()
                    .filter(|p| std::path::Path::new(p).is_dir())
                {
                    let mut options = cli.options.clone();
                    options.repo_root = Some(root.into());
                    loaded = config::load(&options)?;
                }
            }
        }
    }
    report.configure(&loaded.settings.lang, loaded.settings.log_retention);
    report.context(
        serde_json::json!({"project_root":loaded.root,"configuration_sources":loaded.sources}),
    );
    if let Some(Cmd::Web { port, no_open }) = &cli.command {
        return web::run(loaded, &cli.options, *port, !*no_open, report);
    }
    if interactive {
        if cli.options.non_interactive || cli.options.json {
            bail!("[E_ARGUMENT] --interactive cannot be combined with --non-interactive or --json");
        }
        let mut previous_error = None;
        let mut build_options = cli.options.clone();
        loop {
            let action = ui::wizard(&mut loaded, &mut build_options, previous_error.as_deref())?;
            report.configure(&loaded.settings.lang, loaded.settings.log_retention);
            if matches!(action, ui::Action::Build | ui::Action::Plan) {
                report.info(
                    "command",
                    ui::equivalent(
                        &loaded,
                        if matches!(action, ui::Action::Plan) {
                            "plan"
                        } else {
                            "build"
                        },
                        &build_options,
                    ),
                );
                util::write_json(
                    &util::data_dir()?.join("recent-project.json"),
                    &serde_json::json!({"root":loaded.root}),
                )?;
            }
            match action {
                ui::Action::Cancel => {
                    report.info(
                        "cancel",
                        report.text(
                            "已取消，未开始新的构建",
                            "Cancelled before starting another build",
                        ),
                    );
                    return Ok(());
                }
                ui::Action::Plan => {
                    let result = build::plan(&loaded, report);
                    if let Err(error) = result {
                        previous_error = Some(format!("{error:#}"));
                    } else {
                        previous_error = None;
                        eprintln!(
                            "{}",
                            report.text("按 Enter 返回向导", "Press Enter to return to the wizard")
                        );
                        let mut line = String::new();
                        io::stdin().read_line(&mut line)?;
                    }
                }
                ui::Action::Build => {
                    start_update_check(&loaded.settings, report);
                    match build::run(&loaded, &build_options, report) {
                        Ok(()) => return Ok(()),
                        Err(error) => {
                            report.emit("error", "error", format!("{error:#}"));
                            if util::CANCELLED.load(Ordering::Relaxed) {
                                return Err(error);
                            }
                            previous_error = Some(format!("{error:#}"));
                        }
                    }
                }
            }
        }
    }
    if cli.command.is_none() && !cli.options.non_interactive && !io::stdin().is_terminal() {
        bail!("[E_ARGUMENT] use an explicit command: proto-builder build --non-interactive");
    }
    match cli.command.as_ref().unwrap_or(&Cmd::Build) {
        Cmd::Build if cli.options.dry_run => build::plan(&loaded, report),
        Cmd::Build => {
            start_update_check(&loaded.settings, report);
            build::run(&loaded, &cli.options, report)
        }
        Cmd::Plan => build::plan(&loaded, report),
        Cmd::Doctor => build::doctor(&loaded, report),
        Cmd::Config(command) => config_command(&loaded, command, report),
        Cmd::SelfUpdate(UpdateCmd::Check) => {
            let release = update::check(&loaded.settings, report)?;
            report.result(serde_json::json!({"release":release,"signed_updates_configured":update::trust_configured()}));
            Ok(())
        }
        Cmd::SelfUpdate(UpdateCmd::Apply { channel }) => {
            let mut settings = loaded.settings.clone();
            if let Some(channel) = channel {
                settings.update.channel = channel.clone();
            }
            update::apply(&settings, report)
        }
        _ => unreachable!("handled independent commands"),
    }
}
fn start_update_check(settings: &config::Settings, report: &Reporter) {
    if settings.update.enabled {
        let settings = settings.clone();
        let report = report.clone();
        std::thread::spawn(move || update::background_check(&settings, &report));
    }
}
fn config_command(loaded: &config::Loaded, command: &ConfigCmd, report: &Reporter) -> Result<()> {
    match command {
        ConfigCmd::Show => {
            report.result(serde_json::json!({"settings":loaded.settings,"sources":loaded.sources,"project_root":loaded.root}));
            if !report.json {
                eprintln!("{}", toml::to_string_pretty(&loaded.settings)?);
            }
            Ok(())
        }
        ConfigCmd::Check => {
            config::validate(&loaded.settings)?;
            report.info("config", report.text("配置校验通过", "Configuration valid"));
            Ok(())
        }
        ConfigCmd::Init { output } => {
            let path = output
                .as_ref()
                .map(|p| util::absolute(&loaded.root, p))
                .unwrap_or_else(|| loaded.root.join("proto-builder.toml"));
            if path.exists() {
                bail!(
                    "[E_CONFIG] {} already exists; init will not overwrite it",
                    path.display()
                );
            }
            config::save(&path, &config::Settings::default())?;
            report.info("config", format!("{}", path.display()));
            Ok(())
        }
        ConfigCmd::Migrate { check } => {
            let file = loaded
                .project_file
                .as_ref()
                .context("[E_CONFIG] no project configuration found")?;
            let text = fs::read_to_string(file)?;
            let mut value: toml::Value = toml::from_str(&text)?;
            let old = value
                .get("config_version")
                .and_then(toml::Value::as_integer)
                .unwrap_or(0);
            if old == 1 {
                report.info(
                    "config",
                    report.text("配置无需迁移", "No migration required"),
                );
                return Ok(());
            }
            report.result(serde_json::json!({"file":file,"old_version":old,"new_version":1,"changes":["config_version = 1"]}));
            report.info(
                "config",
                format!("{}: config_version {old} → 1", file.display()),
            );
            if !*check {
                if !io::stdin().is_terminal() || report.json {
                    bail!("[E_CONFIG] config migrate writes require interactive confirmation; use --check for read-only validation");
                }
                eprintln!(
                    "{}",
                    report.text(
                        "确认写回？输入 yes（将保留 .bak）",
                        "Write migration? Type yes (creates .bak)"
                    )
                );
                let mut answer = String::new();
                io::stdin().read_line(&mut answer)?;
                if answer.trim() != "yes" {
                    bail!("[E_CANCELLED] configuration migration cancelled");
                }
                value["config_version"] = toml::Value::Integer(1);
                fs::copy(file, file.with_extension("toml.bak"))?;
                util::atomic_write(file, toml::to_string_pretty(&value)?.as_bytes())?;
            }
            Ok(())
        }
    }
}
