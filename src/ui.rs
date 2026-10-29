use crate::{
    cli::Options,
    config::{self, Loaded, Settings},
    util,
};
use anyhow::{bail, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
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
                        SelectionKind::Id,
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
                        SelectionKind::Exclude,
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
            if format!("{error:#}").contains("[E_CANCELLED]") {
                return Err(error);
            }
            if !scroll_text("操作失败 / Operation failed", &format!("{error:#}"), false)? {
                bail!("[E_CANCELLED] 用户退出向导 / User exited wizard");
            }
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
    let mut settings = loaded.settings.clone();
    settings.activity_ids.clear();
    settings.machine_ids.clear();
    settings.assign_id = 0;
    settings.excludes.clear();
    let plan = crate::scan::make(&loaded.root, &settings)?;
    let mut values = plan
        .selected
        .iter()
        .filter_map(|item| {
            if activity {
                item.activity_id.clone()
            } else {
                item.machine_id.clone()
            }
        })
        .collect::<BTreeSet<_>>();
    values.extend(
        if activity {
            &loaded.settings.activity_ids
        } else {
            &loaded.settings.machine_ids
        }
        .iter()
        .cloned(),
    );
    Ok(values.into_iter().collect())
}

#[derive(Default)]
struct Editor {
    chars: Vec<char>,
    cursor: usize,
    start: usize,
}
impl Editor {
    fn new(text: &str) -> Self {
        let chars = text.chars().filter(|c| !c.is_control()).collect::<Vec<_>>();
        let cursor = chars.len();
        Self {
            chars,
            cursor,
            start: 0,
        }
    }
    fn text(&self) -> String {
        self.chars.iter().collect()
    }
    fn insert(&mut self, value: &str) {
        for c in value.chars().filter(|c| !c.is_control()) {
            self.chars.insert(self.cursor, c);
            self.cursor += 1;
        }
    }
    fn key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(key.code, KeyCode::Char('u' | 'U')) {
                self.chars.clear();
                self.cursor = 0;
                self.start = 0;
            }
            return;
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            return;
        }
        match key.code {
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.chars.len()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.chars.len(),
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.chars.remove(self.cursor);
            }
            KeyCode::Delete if self.cursor < self.chars.len() => {
                self.chars.remove(self.cursor);
            }
            KeyCode::Char(c) if !c.is_control() => self.insert(&c.to_string()),
            _ => {}
        }
    }
    fn viewport(&mut self, width: usize) -> (String, u16) {
        let width = width.max(1);
        self.start = self.start.min(self.cursor);
        let columns = |chars: &[char]| {
            chars
                .iter()
                .map(|c| unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0))
                .sum::<usize>()
        };
        let mut col = columns(&self.chars[self.start..self.cursor]);
        while self.start < self.cursor && col >= width {
            col = col.saturating_sub(
                unicode_width::UnicodeWidthChar::width(self.chars[self.start]).unwrap_or(0),
            );
            self.start += 1;
        }
        let mut used = 0;
        let visible = self.chars[self.start..]
            .iter()
            .take_while(|c| {
                used += unicode_width::UnicodeWidthChar::width(**c).unwrap_or(0);
                used <= width
            })
            .collect::<String>();
        (visible, col as u16)
    }
}
fn input(title: &str, initial: &str) -> Result<Option<String>> {
    let mut editor = Editor::new(initial);
    let result = (|| -> Result<Option<String>> {
        loop {
            let (width, _) = terminal::size()?;
            let (visible, col) = editor.viewport(width.saturating_sub(2) as usize);
            execute!(io::stdout(), Clear(ClearType::All))?;
            row(1, title, false)?;
            row(3, &visible, false)?;
            row(
                5,
                "Enter: confirm | Esc: back | Ctrl+C: exit | Ctrl+U: clear",
                false,
            )?;
            row(
                6,
                "← → / Home / End / Delete: edit | 长路径可左右滚动",
                false,
            )?;
            execute!(io::stdout(), cursor::MoveTo(col + 1, 3), cursor::Show)?;
            io::stdout().flush()?;
            match read_event()? {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => return Ok(Some(editor.text())),
                    _ => editor.key(key),
                },
                Event::Paste(value) => editor.insert(&value),
                _ => {}
            }
        }
    })();
    execute!(io::stdout(), cursor::Hide)?;
    result
}
#[derive(Clone, Copy)]
enum SelectionKind {
    Id,
    Exclude,
}
fn manual_values(text: &str, kind: SelectionKind) -> Result<Vec<String>> {
    let values = match kind {
        SelectionKind::Id => {
            let values = text
                .split(|c: char| c == ',' || c == '，' || c.is_whitespace())
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if values
                .iter()
                .any(|v| !v.chars().all(|c| c.is_ascii_digit()))
            {
                bail!(
                    "[E_FILTER] 请输入数字 ID，用逗号分隔 / Enter numeric IDs separated by commas"
                );
            }
            values
        }
        SelectionKind::Exclude => {
            let values = text
                .split([',', '，'])
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            crate::scan::Excludes::new(&values)?;
            values
        }
    };
    Ok(values)
}
fn multi(
    title: &str,
    options: &[String],
    initial: &[String],
    kind: SelectionKind,
) -> Result<Option<Vec<String>>> {
    let mut options = options
        .iter()
        .cloned()
        .chain(initial.iter().cloned())
        .collect::<BTreeSet<_>>();
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
            "↑↓: select | Space: toggle | Enter: confirm | Insert: add ID/glob | Esc: back",
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
                KeyCode::Insert => {
                    let title = match kind {
                        SelectionKind::Id => "添加 ID，逗号分隔 / Add numeric IDs, comma-separated",
                        SelectionKind::Exclude => {
                            "添加排除路径或 glob / Add exclusion paths or globs"
                        }
                    };
                    if let Some(text) = input(title, "")? {
                        match manual_values(&text, kind) {
                            Ok(values) => {
                                options.extend(values.iter().cloned());
                                marked.extend(values);
                                query.clear();
                                cursor = 0;
                                offset = 0;
                            }
                            Err(error) => {
                                if !scroll_text(
                                    "输入无效 / Invalid input",
                                    &format!("{error:#}"),
                                    false,
                                )? {
                                    bail!("[E_CANCELLED] 用户退出向导 / User exited wizard");
                                }
                            }
                        }
                    }
                    continue;
                }
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
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && !c.is_control() =>
                {
                    query.push(c);
                    cursor = 0;
                    offset = 0;
                    false
                }
                _ => false,
            },
            Event::Paste(text) => {
                query.extend(text.chars().filter(|c| !c.is_control()));
                cursor = 0;
                offset = 0;
                false
            }
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
fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(2);
    text.lines()
        .flat_map(|line| {
            let mut lines = vec![];
            let mut current = String::new();
            let mut used = 0;
            for c in line.chars() {
                let c = if c == '\t' { ' ' } else { c };
                if c.is_control() {
                    continue;
                }
                let size = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if used + size > width && !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    used = 0;
                }
                current.push(c);
                used += size;
            }
            lines.push(current);
            lines
        })
        .collect()
}
fn scroll_text(title: &str, text: &str, save: bool) -> Result<bool> {
    let mut offset = 0usize;
    loop {
        let (width, height) = terminal::size()?;
        let lines = wrap_lines(text, width.saturating_sub(2) as usize);
        let size = height.saturating_sub(4).max(1) as usize;
        let last = lines.len().saturating_sub(size);
        offset = offset.min(last);
        execute!(io::stdout(), Clear(ClearType::All))?;
        row(
            0,
            &format!("{title}  [{}/{}]", offset + 1, lines.len().max(1)),
            false,
        )?;
        for (i, line) in lines.iter().skip(offset).take(size).enumerate() {
            row(i as u16 + 1, line, false)?;
        }
        row(
            height.saturating_sub(2),
            if save {
                "Y: save with backup | N / Esc: back | ↑↓ PgUp/PgDn Home/End: scroll"
            } else {
                "Enter / Esc: back | Q: exit | ↑↓ PgUp/PgDn Home/End: scroll"
            },
            false,
        )?;
        io::stdout().flush()?;
        if let Event::Key(key) = read_event()? {
            match key.code {
                KeyCode::Char('y' | 'Y')
                    if save && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    return Ok(true)
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') if save => return Ok(false),
                KeyCode::Enter | KeyCode::Esc if !save => return Ok(true),
                KeyCode::Char('q' | 'Q') if !save => return Ok(false),
                KeyCode::Down => offset = (offset + 1).min(last),
                KeyCode::Up => offset = offset.saturating_sub(1),
                KeyCode::PageDown => offset = (offset + size).min(last),
                KeyCode::PageUp => offset = offset.saturating_sub(size),
                KeyCode::Home => offset = 0,
                KeyCode::End => offset = last,
                _ => {}
            }
        }
    }
}
fn review(text: &str) -> Result<bool> {
    scroll_text("配置预览 / Configuration preview", text, true)
}
pub fn result_view(
    settings: &Settings,
    value: &serde_json::Value,
    report_path: &str,
) -> Result<bool> {
    let _terminal = Terminal::enter()?;
    let text = result_text(settings, value, report_path)?;
    scroll_text(tr(settings, "执行结果", "Operation result"), &text, false)
}
fn result_text(
    settings: &Settings,
    value: &serde_json::Value,
    report_path: &str,
) -> Result<String> {
    let output = match value["output_state"].as_str() {
        Some("not_written") => tr(settings, "未写回本次生成物", "No generated outputs written"),
        Some("committed") => tr(settings, "生成物已写回", "Generated outputs committed"),
        Some("rolled_back") => tr(settings, "本次写回已回滚", "Current write rolled back"),
        Some("recovered") => tr(
            settings,
            "已恢复上次中断事务",
            "Previous interrupted transaction recovered",
        ),
        Some("recovery_required" | "writing" | "recovering") => tr(
            settings,
            "状态需核对，请查看报告与备份",
            "Inspect report and backups before proceeding",
        ),
        _ => tr(
            settings,
            "状态未知，请查看完整报告",
            "Unknown; inspect complete report",
        ),
    };
    let mut text = format!(
        "{}: {}\n{}: {}\n{}: {}\n\n",
        tr(settings, "状态", "Status"),
        if value["report_error"].is_string() {
            tr(
                settings,
                "报告保存失败，请核对下方写回状态",
                "Report could not be saved; check output state below",
            )
        } else if value["status"] == "success" {
            tr(settings, "完成", "Completed")
        } else {
            tr(settings, "已中止", "Stopped")
        },
        tr(settings, "生成物写回状态", "Output state"),
        output,
        tr(settings, "完整报告", "Complete report"),
        report_path
    );
    if let Some(error) = value["error"].as_str() {
        text.push_str(error);
        text.push_str("\n\n");
    }
    if let Some(action) = value["failure"]["suggested_action"].as_str() {
        let (zh, en) = match action {
            "review_filters" => (
                "核对活动、机器和排除条件，再重新预览。",
                "Review activity, machine and exclusion filters, then preview again.",
            ),
            "correct_paths" => (
                "修正输入与输出目录，避免重叠，再检查环境。",
                "Correct input/output directories and overlaps, then run doctor.",
            ),
            "correct_protocol_sources" => (
                "按下方原文件行号修复协议，再重新预览或编译。",
                "Correct protocol sources at the reported file/line, then retry.",
            ),
            "resolve_svn_conflicts" => (
                "先处理 SVN 冲突，再重新预览。",
                "Resolve SVN conflicts before previewing again.",
            ),
            "review_existing_outputs" => (
                "核对冲突文件和备份策略，再决定是否启用接管或覆盖。",
                "Review conflicting outputs and backups before enabling adoption or replacement.",
            ),
            "reload_configuration_and_plan" => (
                "输入已变化，重新加载配置并预览。",
                "Reload changed inputs/configuration and preview again.",
            ),
            "retry_after_current_build" => (
                "等待当前构建结束再重试。",
                "Wait for the current build to finish before retrying.",
            ),
            "correct_configuration" | "correct_arguments" => (
                "修正配置或参数，再重新预览。",
                "Correct configuration or arguments and preview again.",
            ),
            "check_report_storage_and_output_state" | "inspect_output_state_and_recovery" => (
                "先核对写回状态、备份和报告目录权限，避免重复执行。",
                "Inspect output state, backups and report-directory permissions before retrying.",
            ),
            "resume_if_requested" => (
                "已取消；可返回修改参数，或退出。",
                "Cancelled; return to edit settings or exit.",
            ),
            _ => (
                "查看完整报告和上述诊断，修复对应原因后重试。",
                "Inspect the full report and diagnostics; fix the cause before retrying.",
            ),
        };
        text.push_str(&format!(
            "{}: {}\n\n",
            tr(settings, "下一步", "Next step"),
            tr(settings, zh, en)
        ));
    }
    if let Some(items) = value["diagnostics"].as_array() {
        for item in items {
            let column = item["column"]
                .as_u64()
                .map(|v| format!(":{v}"))
                .unwrap_or_default();
            text.push_str(&format!(
                "{}:{}{column}\n{}\n\n",
                item["file"].as_str().unwrap_or(""),
                item["line"],
                item["message"].as_str().unwrap_or("")
            ));
        }
    }
    if let Some(error) = value["report_error"].as_str() {
        text.push_str(&format!("\n[E_REPORT] {error}\n"));
    }
    if let Some(inputs) = value["result"]["selected"].as_array() {
        text.push_str(&format!(
            "{}: {}\n",
            tr(settings, "待编译协议", "Selected protocols"),
            inputs.len()
        ));
        text.push_str(&format!(
            "{}: {}\n\n",
            tr(
                settings,
                "完整共享文档协议数",
                "Full shared documentation inputs"
            ),
            value["result"]["documentation_inputs"]
                .as_array()
                .map_or(0, Vec::len)
        ));
        for item in inputs {
            text.push_str(item["relative"].as_str().unwrap_or(""));
            text.push('\n');
        }
    } else if !value["result"].is_null() {
        text.push_str(&serde_json::to_string_pretty(&value["result"])?);
    }
    Ok(text)
}
fn is_interrupt(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c' | 'C')) && key.modifiers.contains(KeyModifiers::CONTROL)
}
fn accepted_event(event: Event) -> Result<Option<Event>> {
    match event {
        Event::Mouse(_) | Event::FocusGained | Event::FocusLost => Ok(None),
        Event::Key(key) if key.kind == KeyEventKind::Release => Ok(None),
        Event::Key(key) if is_interrupt(key) => {
            bail!("[E_CANCELLED] 用户取消操作 / Cancelled by user")
        }
        event => Ok(Some(event)),
    }
}
fn read_event() -> Result<Event> {
    loop {
        if let Some(event) = accepted_event(event::read()?)? {
            return Ok(event);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn editor_edits_middle_of_long_unicode_path_and_keeps_cursor_visible() {
        let initial = format!("D:/{}终点", "很长的路径/".repeat(30));
        let mut editor = Editor::new(&initial);
        let (visible, col) = editor.viewport(20);
        assert!(visible.ends_with("终点"));
        assert!(col < 20);
        editor.key(key(KeyCode::Home));
        editor.key(key(KeyCode::Right));
        editor.key(key(KeyCode::Delete));
        editor.insert(":replacement");
        assert!(editor.text().starts_with("D:replacement/"));
        editor.key(key(KeyCode::End));
        editor.key(key(KeyCode::Backspace));
        editor.key(key(KeyCode::Left));
        editor.insert("新");
        assert!(editor.text().ends_with("新终"));
        editor.key(key(KeyCode::Home));
        let (visible, col) = editor.viewport(20);
        assert!(visible.starts_with("D:replacement/"));
        assert_eq!(col, 0);
    }
    #[test]
    fn control_keys_do_not_insert_letters_and_interrupt_is_shared() {
        let mut editor = Editor::new("keep");
        let interrupt = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_interrupt(interrupt));
        let error = accepted_event(Event::Key(interrupt)).unwrap_err();
        assert_eq!(crate::report::exit_code(&Err(error)), 130);
        assert!(accepted_event(Event::FocusGained).unwrap().is_none());
        editor.key(interrupt);
        editor.key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(editor.text(), "keep");
        assert!(!is_interrupt(key(KeyCode::Char('c'))));
        editor.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(editor.text(), "");
        editor.insert("中文\r\n路径\t");
        assert_eq!(editor.text(), "中文路径");
    }
    #[test]
    fn diagnostic_wrapping_retains_every_unicode_character() {
        let path = format!(
            "D:/{}文件.proto:123:5: invalid field",
            "中文目录/".repeat(30)
        );
        let wrapped = wrap_lines(&path, 78);
        assert!(wrapped.len() > 1);
        assert_eq!(wrapped.concat(), path);
        for line in wrapped {
            assert!(unicode_width::UnicodeWidthStr::width(line.as_str()) <= 78);
        }
    }
    #[test]
    fn wizard_catalog_uses_package_comments_and_relative_paths() {
        let dir = tempfile::Builder::new()
            .prefix("act9999-parent-")
            .tempdir()
            .unwrap();
        let proto = dir.path().join("proto/misc");
        std::fs::create_dir_all(&proto).unwrap();
        std::fs::write(
            proto.join("one.proto"),
            "syntax=\"proto3\";\npackage act7001;\nmessage One {}\n",
        )
        .unwrap();
        std::fs::write(proto.join("two.proto"), "syntax=\"proto3\";\npackage ordinary;\n// activity_id=7002\n// machine_id=5001\nmessage Two {}\n").unwrap();
        let settings = Settings {
            activity_ids: vec!["8888".into()],
            excludes: vec!["misc".into()],
            assign_id: 999,
            ..Default::default()
        };
        let loaded = Loaded {
            settings,
            root: dir.path().into(),
            sources: vec![],
            project_file: None,
            origins: Default::default(),
        };
        assert_eq!(
            id_options(&loaded, true).unwrap(),
            vec!["7001", "7002", "8888"]
        );
        assert_eq!(id_options(&loaded, false).unwrap(), vec!["5001"]);
    }
    #[test]
    fn manual_ids_and_globs_validate_without_dropping_custom_choices() {
        assert_eq!(
            manual_values("7001，7002 7003", SelectionKind::Id).unwrap(),
            vec!["7001", "7002", "7003"]
        );
        assert!(manual_values("7001,act7002", SelectionKind::Id).is_err());
        assert_eq!(
            manual_values("draft/**,with space", SelectionKind::Exclude).unwrap(),
            vec!["draft/**", "with space"]
        );
        assert!(manual_values("[", SelectionKind::Exclude).is_err());
    }
    #[test]
    fn result_text_distinguishes_committed_output_from_report_failure() {
        let settings = Settings {
            lang: "en-US".into(),
            ..Default::default()
        };
        let value = serde_json::json!({"status":"success","output_state":"committed","report_error":"disk full","diagnostics":[{"file":"D:/long/file.proto","line":12,"column":null,"message":"bad field"}]});
        let text = result_text(&settings, &value, "report failed").unwrap();
        assert!(text.contains("Generated outputs committed"));
        assert!(text.contains("disk full"));
        assert!(text.contains("D:/long/file.proto:12\nbad field"));
        assert!(!text.contains(":null"));
    }
}
