use crate::{
    cli::Options,
    config::{self, Loaded, Settings},
    util,
};
use anyhow::{bail, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    collections::BTreeSet,
    io::{self, IsTerminal, Write},
    path::PathBuf,
};

struct Terminal;
impl Terminal {
    fn enter() -> Result<Self> {
        terminal::enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen, cursor::Hide) {
            let _ = terminal::disable_raw_mode();
            return Err(error.into());
        }
        let _ = execute!(
            io::stdout(),
            event::DisableMouseCapture,
            event::EnableBracketedPaste
        );
        Ok(Self)
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            SetAttribute(Attribute::Reset),
            event::DisableMouseCapture,
            event::DisableBracketedPaste,
            cursor::Show,
            LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}
pub enum Action {
    Build,
    Plan,
    Cancel,
}
fn tr<'a>(s: &Settings, zh: &'a str, en: &'a str) -> &'a str {
    if s.lang == "en-US" {
        en
    } else {
        zh
    }
}
fn row(y: u16, text: &str, active: bool) -> Result<()> {
    let mut out = io::stdout();
    let (width, height) = terminal::size()?;
    if y >= height {
        return Ok(());
    }
    let mut columns = 0;
    let text = text
        .chars()
        .filter(|c| !c.is_control())
        .take_while(|c| {
            columns += unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0);
            columns <= width.saturating_sub(2) as usize
        })
        .collect::<String>();
    queue!(
        out,
        cursor::MoveTo(1, y),
        SetAttribute(if active {
            Attribute::Reverse
        } else {
            Attribute::Reset
        }),
        Print(text),
        SetAttribute(Attribute::Reset)
    )?;
    Ok(())
}
pub fn wizard(
    loaded: &mut Loaded,
    options: &mut Options,
    last_error: Option<&str>,
) -> Result<Action> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("[E_TERMINAL] interactive mode requires a terminal; use build --non-interactive");
    }
    let _terminal = Terminal::enter()?;
    let mut selected = 0;
    let mut offset = 0;
    let setup = crate::setup::status(loaded)?;
    let mut notice = last_error.map(str::to_owned).unwrap_or_else(|| {
        if setup.needs_confirmation {
            tr(
                &loaded.settings,
                "请核对项目/协议/业务服/计算服路径；成功编译后自动记住。",
                "Check project/proto/server/calc paths; remembered after a successful build.",
            )
            .into()
        } else {
            String::new()
        }
    });
    loop {
        execute!(io::stdout(), Clear(ClearType::All))?;
        let s = &loaded.settings;
        row(0, "proto-builder  |  protobuf → Go", false)?;
        row(
            1,
            tr(
                s,
                "↑↓ 选择 / Enter 编辑或执行 / Esc 退出 / 网页操作：proto-builder web",
                "Arrows to select / Enter to edit or run / Esc to exit / Browser: proto-builder web",
            ),
            false,
        )?;
        let values = vec![
            format!(
                "{}: {}",
                tr(s, "协议项目根目录", "Protocol project root"),
                loaded.root.display()
            ),
            format!(
                "{}: {}",
                tr(s, "活动 ID", "Activity IDs"),
                list(&s.activity_ids)
            ),
            format!(
                "{}: {}",
                tr(s, "机器 ID", "Machine IDs"),
                list(&s.machine_ids)
            ),
            format!("{}: {}", tr(s, "排除目录", "Exclude"), list(&s.excludes)),
            toggle(tr(s, "生成文档", "Documentation"), s.docs),
            toggle(
                tr(s, "生成协议配置", "Protocol configuration"),
                s.protocol_config,
            ),
            toggle(
                tr(s, "同步计算服", "Copy to calculation server"),
                s.copy_to_calc,
            ),
            toggle(tr(s, "先更新 SVN", "Update SVN first"), s.svn.update),
            format!("{}: {}", tr(s, "语言", "Language"), s.lang),
            format!(
                "{}: {}",
                tr(s, "协议目录", "Proto directory"),
                s.proto_path.display()
            ),
            format!(
                "{}: {}",
                tr(s, "业务服目录", "Server directory"),
                s.server_path.display()
            ),
            format!(
                "{}: {}",
                tr(s, "计算服目录", "Calculation server directory"),
                s.calc_path.display()
            ),
            format!(
                "{}: {}",
                tr(s, "并行数（0 为自动）", "Workers (0 = auto)"),
                s.jobs
            ),
            format!(
                "{}: {}",
                tr(s, "旧 ID 规则（0 为全部）", "Legacy ID (0 = all)"),
                s.assign_id
            ),
            toggle(
                tr(
                    s,
                    "接管已有生成物（先备份）",
                    "Adopt existing outputs (back up first)",
                ),
                options.adopt_existing,
            ),
            toggle(
                tr(
                    s,
                    "覆盖已修改生成物（先备份）",
                    "Replace modified owned outputs (back up first)",
                ),
                options.force,
            ),
            tr(
                s,
                "保存为项目配置（显示内容后确认）",
                "Save project configuration (review before saving)",
            )
            .into(),
            tr(
                s,
                "保存用户偏好（语言、更新检查）",
                "Save user preferences (language, update checks)",
            )
            .into(),
            tr(s, "预览编译计划", "Preview plan").into(),
            tr(s, "开始编译", "Build").into(),
            format!(
                "{}: {}",
                tr(s, "开发者名字", "Developer name"),
                s.developer_name
            ),
        ];
        let (_, height) = terminal::size()?;
        let available = height.saturating_sub(9).max(1) as usize;
        if selected < offset {
            offset = selected;
        }
        if selected >= offset + available {
            offset = selected + 1 - available;
        }
        for (index, value) in values.iter().enumerate().skip(offset).take(available) {
            row(
                (index - offset) as u16 + 3,
                &format!(" {}  {value}", if selected == index { "›" } else { " " }),
                selected == index,
            )?;
        }
        let (_, height) = terminal::size()?;
        let footer = height.saturating_sub(4);
        row(
            footer,
            tr(
                s,
                "路径和名字在编译成功后记住；其他选项可另存配置。",
                "Paths and name are remembered after a successful build; save other options as configuration.",
            ),
            false,
        )?;
        row(footer + 1, &equivalent(loaded, "build", options), false)?;
        if !notice.is_empty() {
            row(footer + 2, &notice.replace(['\r', '\n'], " "), false)?;
        }
        io::stdout().flush()?;
        let activate = match read_event()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Esc => return Ok(Action::Cancel),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(Action::Cancel)
                }
                KeyCode::Up => {
                    selected = selected.saturating_sub(1);
                    false
                }
                KeyCode::Down => {
                    selected = (selected + 1).min(values.len() - 1);
                    false
                }
                KeyCode::Enter => true,
                _ => false,
            },
            _ => false,
        };
        if !activate {
            continue;
        }
        notice.clear();
        if selected == 18 {
            return Ok(Action::Plan);
        }
        if selected == 19 {
            config::validate(&loaded.settings)?;
            return Ok(Action::Build);
        }
        let operation = (|| -> Result<()> {
            match selected {
                0 => {
                    if let Some(value) = input(
                        tr(
                            &loaded.settings,
                            "协议项目根目录（通常包含 proto 文件夹）",
                            "Protocol project root (usually contains proto folder)",
                        ),
                        &loaded.root.display().to_string(),
                    )? {
                        let root = util::absolute(&std::env::current_dir()?, &PathBuf::from(value));
                        let candidate = config::load(&crate::cli::Options {
                            repo_root: Some(root),
                            ..Default::default()
                        })?;
                        if candidate.settings.proto(&candidate.root).is_dir() {
                            *loaded = candidate;
                        } else {
                            notice = tr(
                                &loaded.settings,
                                "目录内没有配置的 proto 文件夹",
                                "Configured proto directory not found",
                            )
                            .into();
                        }
                    }
                }
                1 | 2 => {
                    let activity = selected == 1;
                    let options = id_options(loaded, activity)?;
                    let old = if activity {
                        &loaded.settings.activity_ids
                    } else {
                        &loaded.settings.machine_ids
                    };
                    if let Some(values) = multi(
                        if activity {
                            tr(
                                &loaded.settings,
                                "选择活动，输入搜索；空选表示全部",
                                "Activities: type to search; empty selects all",
                            )
                        } else {
                            tr(
                                &loaded.settings,
                                "选择机器，输入搜索；空选表示全部",
                                "Machines: type to search; empty selects all",
                            )
                        },
                        &options,
                        old,
                    )? {
                        if activity {
                            loaded.settings.activity_ids = values
                        } else {
                            loaded.settings.machine_ids = values
                        }
                    }
                }
                3 => {
                    let proto = loaded.settings.proto(&loaded.root);
                    let mut choices = BTreeSet::new();
                    for path in crate::scan::discover(&proto)? {
                        if let Some(parent) = path
                            .strip_prefix(&proto)?
                            .parent()
                            .filter(|p| !p.as_os_str().is_empty())
                        {
                            choices.insert(parent.to_string_lossy().replace('\\', "/"));
                        }
                    }
                    for value in &loaded.settings.excludes {
                        choices.insert(value.clone());
                    }
                    if let Some(values) = multi(
                        tr(
                            &loaded.settings,
                            "选择排除目录；输入搜索",
                            "Exclude directories: type to search",
                        ),
                        &choices.into_iter().collect::<Vec<_>>(),
                        &loaded.settings.excludes,
                    )? {
                        loaded.settings.excludes = values;
                    }
                }
                4 => loaded.settings.docs = !loaded.settings.docs,
                5 => loaded.settings.protocol_config = !loaded.settings.protocol_config,
                6 => loaded.settings.copy_to_calc = !loaded.settings.copy_to_calc,
                7 => loaded.settings.svn.update = !loaded.settings.svn.update,
                8 => {
                    loaded.settings.lang = if loaded.settings.lang == "zh-CN" {
                        "en-US"
                    } else {
                        "zh-CN"
                    }
                    .into()
                }
                9..=11 => {
                    let (label, path) = match selected {
                        9 => (
                            tr(&loaded.settings, "协议目录", "Proto directory"),
                            &loaded.settings.proto_path,
                        ),
                        10 => (
                            tr(&loaded.settings, "业务服目录", "Server directory"),
                            &loaded.settings.server_path,
                        ),
                        _ => (
                            tr(
                                &loaded.settings,
                                "计算服目录",
                                "Calculation server directory",
                            ),
                            &loaded.settings.calc_path,
                        ),
                    };
                    if let Some(value) = input(label, &path.display().to_string())? {
                        if value.trim().is_empty() {
                            bail!("[E_CONFIG] path cannot be empty");
                        }
                        let path = PathBuf::from(value.trim());
                        match selected {
                            9 => loaded.settings.proto_path = path,
                            10 => loaded.settings.server_path = path,
                            _ => loaded.settings.calc_path = path,
                        }
                    }
                }
                12 | 13 => {
                    let mut candidate = loaded.settings.clone();
                    let (label, initial) = if selected == 12 {
                        (
                            tr(&candidate, "并行数（0 为自动）", "Workers (0 = auto)"),
                            candidate.jobs.to_string(),
                        )
                    } else {
                        (
                            tr(&candidate, "旧 ID 规则（0 为全部）", "Legacy ID (0 = all)"),
                            candidate.assign_id.to_string(),
                        )
                    };
                    if let Some(value) = input(label, &initial)? {
                        if selected == 12 {
                            candidate.jobs = value.trim().parse()?;
                        } else {
                            candidate.assign_id = value.trim().parse()?;
                        }
                        config::validate(&candidate)?;
                        loaded.settings = candidate;
                    }
                }
                14 => options.adopt_existing = !options.adopt_existing,
                15 => options.force = !options.force,
                16 => {
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
                    let text = toml::to_string_pretty(&saved)?;
                    let old = std::fs::read_to_string(&path).unwrap_or_default();
                    let diff = similar::TextDiff::from_lines(&old, &text)
                        .unified_diff()
                        .header("current", "proposed")
                        .to_string();
                    if review(&format!("{}\n{}", path.display(), diff))? {
                        config::save(&path, &saved)?;
                        loaded.project_file = Some(path);
                        notice = tr(
                            &loaded.settings,
                            "已保存，原配置已备份",
                            "Saved; original configuration backed up",
                        )
                        .into();
                    }
                }
                17 => {
                    let path = util::user_config()?;
                    let mut value: toml::Value = if path.is_file() {
                        toml::from_str(&std::fs::read_to_string(&path)?)?
                    } else {
                        toml::Value::Table(Default::default())
                    };
                    value["lang"] = toml::Value::String(loaded.settings.lang.clone());
                    value["update"] = toml::Value::try_from(&loaded.settings.update)?;
                    let text = toml::to_string_pretty(&value)?;
                    if review(&format!("{}\n{}", path.display(), text))? {
                        if path.is_file() {
                            std::fs::copy(&path, path.with_extension("toml.bak"))?;
                        }
                        util::atomic_write(&path, text.as_bytes())?;
                        notice = "Saved".into();
                    }
                }
                20 => {
                    if let Some(value) = input(
                        tr(&loaded.settings, "开发者名字", "Developer name"),
                        &loaded.settings.developer_name,
                    )? {
                        loaded.settings.developer_name = value.trim().to_owned();
                        config::validate(&loaded.settings)?;
                    }
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = operation {
            notice = format!("{error:#}");
        }
    }
}
fn list(values: &[String]) -> String {
    if values.is_empty() {
        "all / 全部".into()
    } else {
        values.join(", ")
    }
}
fn toggle(name: &str, on: bool) -> String {
    format!("[{}] {name}", if on { "x" } else { " " })
}
fn id_options(loaded: &Loaded, activity: bool) -> Result<Vec<String>> {
    let re = regex::Regex::new(if activity {
        r"(?:act|activity)(\d+)"
    } else {
        r"(?:slot|machine|rpc)(\d+)"
    })?;
    let mut values = BTreeSet::new();
    for file in crate::scan::discover(&loaded.settings.proto(&loaded.root))? {
        for cap in re.captures_iter(&file.to_string_lossy()) {
            values.insert(cap[1].to_owned());
        }
    }
    for value in if activity {
        &loaded.settings.activity_ids
    } else {
        &loaded.settings.machine_ids
    } {
        values.insert(value.clone());
    }
    Ok(values.into_iter().collect())
}
fn input(title: &str, initial: &str) -> Result<Option<String>> {
    let mut text = initial.to_owned();
    loop {
        execute!(io::stdout(), Clear(ClearType::All))?;
        row(1, title, false)?;
        row(3, &text, false)?;
        row(5, "Enter: confirm | Esc: cancel | Ctrl+U: clear", false)?;
        io::stdout().flush()?;
        match read_event()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => return Ok(Some(text)),
                KeyCode::Backspace => {
                    text.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => text.clear(),
                KeyCode::Char(c) => text.push(c),
                _ => {}
            },
            Event::Paste(value) => text.push_str(&value),
            _ => {}
        }
    }
}
fn multi(title: &str, options: &[String], initial: &[String]) -> Result<Option<Vec<String>>> {
    let mut marked = initial.iter().cloned().collect::<BTreeSet<_>>();
    let mut query = String::new();
    let mut cursor = 0usize;
    let mut offset = 0usize;
    loop {
        let (_, height) = terminal::size()?;
        let available = height.saturating_sub(8).max(1) as usize;
        let filtered = options
            .iter()
            .filter(|v| v.to_lowercase().contains(&query.to_lowercase()))
            .collect::<Vec<_>>();
        cursor = cursor.min(filtered.len().saturating_sub(1));
        if cursor < offset {
            offset = cursor;
        }
        if cursor >= offset + available {
            offset = cursor + 1 - available;
        }
        execute!(io::stdout(), Clear(ClearType::All))?;
        row(0, title, false)?;
        row(1, &format!("Search / 搜索: {query}"), false)?;
        row(
            2,
            "↑↓: select | Space: toggle | Enter: confirm | Esc: cancel",
            false,
        )?;
        for (index, value) in filtered.iter().enumerate().skip(offset).take(available) {
            row(
                4 + (index - offset) as u16,
                &format!(
                    "[{}] {}",
                    if marked.contains(*value) { "x" } else { " " },
                    value
                ),
                index == cursor,
            )?;
        }
        row(
            height.saturating_sub(2),
            &format!(
                "{} selected | Ctrl+U: clear search | Ctrl+A: toggle visible",
                marked.len()
            ),
            false,
        )?;
        io::stdout().flush()?;
        let toggle = match read_event()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => return Ok(Some(marked.into_iter().collect())),
                KeyCode::Up => {
                    cursor = cursor.saturating_sub(1);
                    false
                }
                KeyCode::Down => {
                    cursor = (cursor + 1).min(filtered.len().saturating_sub(1));
                    false
                }
                KeyCode::Char(' ') => true,
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    query.clear();
                    cursor = 0;
                    false
                }
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let all = filtered.iter().all(|v| marked.contains(*v));
                    for value in &filtered {
                        if all {
                            marked.remove(*value);
                        } else {
                            marked.insert((*value).clone());
                        }
                    }
                    false
                }
                KeyCode::Backspace => {
                    query.pop();
                    cursor = 0;
                    offset = 0;
                    false
                }
                KeyCode::Char(c) => {
                    query.push(c);
                    cursor = 0;
                    offset = 0;
                    false
                }
                _ => false,
            },
            _ => false,
        };
        if toggle {
            if let Some(value) = filtered.get(cursor) {
                if !marked.remove(*value) {
                    marked.insert((*value).clone());
                }
            }
        }
    }
}
fn review(text: &str) -> Result<bool> {
    let mut offset = 0usize;
    let lines = text.lines().collect::<Vec<_>>();
    loop {
        let (_, h) = terminal::size()?;
        let size = h.saturating_sub(4) as usize;
        execute!(io::stdout(), Clear(ClearType::All))?;
        row(0, "配置预览 / Configuration preview", false)?;
        for (i, line) in lines.iter().skip(offset).take(size).enumerate() {
            row(i as u16 + 1, line, false)?;
        }
        row(
            h.saturating_sub(2),
            "Y: save (backup existing) | N / Esc: cancel | ↑↓: scroll",
            false,
        )?;
        io::stdout().flush()?;
        match read_event()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Char('y' | 'Y') => return Ok(true),
                KeyCode::Esc | KeyCode::Char('n' | 'N') => return Ok(false),
                KeyCode::Down => offset = (offset + 1).min(lines.len().saturating_sub(size)),
                KeyCode::Up => offset = offset.saturating_sub(1),
                _ => {}
            },
            _ => {}
        }
    }
}
fn read_event() -> Result<Event> {
    loop {
        match event::read()? {
            Event::Mouse(_) | Event::FocusGained | Event::FocusLost => continue,
            Event::Key(key) if key.kind == KeyEventKind::Release => continue,
            event => return Ok(event),
        }
    }
}
pub fn equivalent(loaded: &Loaded, command: &str, options: &Options) -> String {
    let s = &loaded.settings;
    let quote = |value: &str| format!("'{}'", value.replace('\'', "''"));
    let mut args = vec![
        format!("proto-builder {command} --non-interactive"),
        format!("--repo-root {}", quote(&loaded.root.display().to_string())),
    ];
    if let Some(config) = &loaded.project_file {
        args.push(format!("--config {}", quote(&config.display().to_string())));
    }
    for (flag, value) in [
        ("--proto-path", &s.proto_path),
        ("--server-path", &s.server_path),
        ("--calc-path", &s.calc_path),
    ] {
        args.push(format!("{flag} {}", quote(&value.display().to_string())));
    }
    for (flag, values) in [
        ("--activity-id", &s.activity_ids),
        ("--machine-id", &s.machine_ids),
        ("--exclude", &s.excludes),
    ] {
        args.push(format!("{flag}={}", quote(&values.join(","))));
    }
    args.push(format!("--assign-id {} --jobs {} --timeout-seconds {} --lang {} --docs={} --protocol-config={} --copy-to-calc={} --svn-update={}",s.assign_id,s.jobs,s.timeout_seconds,s.lang,s.docs,s.protocol_config,s.copy_to_calc,s.svn.update));
    if options.adopt_existing {
        args.push("--adopt-existing".into());
    }
    args.push(format!("--developer-name={}", quote(&s.developer_name)));
    if options.force {
        args.push("--force".into());
    }
    args.join(" ")
}
