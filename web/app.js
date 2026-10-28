"use strict";

(() => {
  const $ = (id) => document.getElementById(id);
  const translations = {
    跳到工作台: "Skip to workspace",
    工作台: "Workspace",
    使用说明: "Documentation",
    正在连接: "Connecting",
    暖白模式: "Light mode",
    深色模式: "Dark mode",
    "协议构建，从容有序。": "Protocol builds. Clearly done.",
    "选好协议，预览计划，一次完成编译与同步。":
      "Select protocols, preview the plan, then build and sync.",
    当前项目: "Current project",
    正在读取项目: "Reading project",
    项目与路径: "Project & paths",
    保存配置: "Save configuration",
    项目目录: "Project directory",
    打开项目: "Open project",
    协议目录: "Protocol directory",
    业务服目录: "Server directory",
    计算服目录: "Calculation server directory",
    "相对路径以项目目录为起点。设置默认仅对本次生效。":
      "Relative paths start at the project directory. Changes apply to this session until saved.",
    选择协议: "Select protocols",
    重置筛选: "Reset filters",
    "活动 ID": "Activity IDs",
    "机器 ID": "Machine IDs",
    全部: "All",
    "空选表示全部；公共协议始终保留。活动与机器同时选择时取交集。":
      "No selection means all. Common protocols are retained; activity and machine filters intersect.",
    排除目录: "Excluded directories",
    构建选项: "Build options",
    "编译前更新 SVN": "Update SVN before build",
    同步计算服: "Sync calculation server",
    生成协议文档: "Generate documentation",
    "HTML / Markdown 为完整共享文档，不受 ID 筛选影响；排除目录仍生效。":
      "HTML / Markdown always cover the complete project, independent of ID filters; exclusions still apply.",
    生成协议号配置: "Generate protocol IDs",
    更多选项: "More options",
    "并行数 · 0 为自动": "Parallel jobs · 0 = automatic",
    "旧 ID 规则 · 0 为全部": "Legacy ID · 0 = all",
    "单条命令超时 / 秒": "Command timeout / seconds",
    界面与报告语言: "Interface & report language",
    "接管已有生成物，覆盖前备份":
      "Adopt existing outputs, with a backup before replacement",
    "覆盖已手动修改的生成物，覆盖前备份":
      "Replace manually edited outputs, with a backup",
    预览计划: "Preview plan",
    开始编译: "Build protocols",
    检查环境: "Check environment",
    构建视图: "Build view",
    准备就绪: "Ready",
    计划与结果: "Plan & results",
    运行日志: "Run log",
    停止操作: "Stop operation",
    本次操作已中止: "Operation stopped",
    操作已取消: "Operation cancelled",
    "先看清计划，再放心构建。": "A clear plan. A confident build.",
    "从左侧选择协议，点击「预览计划」，查看即将编译的目录和文件。":
      "Select protocols on the left and preview the directories and files to be compiled.",
    "预览不会更新 SVN 或写入生成文件":
      "Preview does not update SVN or write generated files",
    显示未变文件: "Show unchanged files",
    显示更多: "Show more",
    本次操作的实时记录: "Live events for this operation",
    详细日志: "Detailed log",
    "操作开始后，日志会显示在这里。":
      "Events appear here when an operation starts.",
    等价命令: "Equivalent command",
    复制命令: "Copy command",
    "下载完整报告 ↗": "Download full report ↗",
    "本地执行，结果可查。": "Local execution. Traceable results.",
    关闭工作台: "Close workspace",
    选择目录: "Choose a directory",
    前往: "Go",
    取消: "Cancel",
    选择此目录: "Choose this directory",
    保存项目配置: "Save project configuration",
    "确认以下变更；已有配置会保留一份备份。":
      "Review the changes. The existing configuration will be backed up.",
    确认保存: "Save changes",
    "关闭工作台？": "Close the workspace?",
    "正在执行的操作会停止。可随时从终端重新启动网页工作台。":
      "Any active operation will stop. You can restart the web workspace from the terminal.",
    继续使用: "Keep working",
    本地已连接: "Connected locally",
    服务已关闭: "Server stopped",
    正在执行: "Running",
    已完成: "Completed",
    已取消: "Cancelled",
    失败: "Failed",
    正在停止: "Stopping",
    未找到匹配项: "No matching items",
    "暂无可选项；可输入 ID 后按 Enter 添加":
      "No options yet; type an ID and press Enter",
    "请输入数字 ID，可用逗号分隔": "Enter numeric IDs, separated by commas",
    目录尚未读取: "Directory not loaded",
    "正在读取目录…": "Reading directory…",
    此目录下没有文件夹: "No subdirectories here",
    "正在读取协议…": "Reading protocols…",
    "无法读取协议，请检查协议目录后重试。":
      "Cannot read protocols. Check the protocol directory and try again.",
    配置已保存: "Configuration saved",
    配置未发生变化: "Configuration is unchanged",
    命令已复制: "Command copied",
    路径已复制: "Path copied",
    "复制失败，请手动选择文本复制":
      "Copy failed. Select and copy the text manually.",
    没有匹配的结果: "No matching results",
    所有生成物均未变化: "All generated files are unchanged",
    扫描文件: "Scanned files",
    待编译文件: "Selected files",
    编译目录: "Directories",
    筛选跳过: "Filtered out",
    新增: "Added",
    修改: "Modified",
    删除: "Removed",
    未变: "Unchanged",
    检查通过: "Checks passed",
    新增文件: "Added",
    修改文件: "Modified",
    删除文件: "Removed",
    未变文件: "Unchanged",
    配置: "Configuration",
    工具链: "Toolchain",
    输入协议: "Protocol inputs",
    输出权限: "Output permissions",
    已通过: "Passed",
    读取配置: "Configuration",
    准备工具链: "Preparing toolchain",
    扫描协议: "Scanning protocols",
    编译协议: "Compiling protocols",
    校验生成物: "Validating outputs",
    同步文件: "Syncing files",
    保存报告: "Saving report",
    预览协议计划: "Previewing plan",
    检查开发环境: "Checking environment",
    "正在生成…": "Generating…",
    "搜索或输入 ID": "Search or enter IDs",
    "搜索目录，或输入 glob 后按 Enter":
      "Search directories, or enter a glob and press Enter",
    搜索目录或文件: "Search directories or files",
    筛选当前目录中的文件夹: "Filter subdirectories",
    "服务连接失败，请检查终端，并用终端中的完整地址重新打开网页。":
      "Connection failed. Check the terminal and reopen its complete URL.",
    选择项目目录: "Choose project directory",
    选择协议目录: "Choose protocol directory",
    选择业务服目录: "Choose server directory",
    选择计算服目录: "Choose calculation server directory",
    "搜索活动 ID": "Search activity IDs",
    "搜索机器 ID": "Search machine IDs",
    搜索或输入排除规则: "Search or enter exclusions",
    添加排除规则: "Add exclusion",
    主导航: "Main navigation",
    构建信息: "Build information",
    构建进度: "Build progress",
    搜索结果: "Search results",
    关闭: "Close",
    上一级: "Parent directory",
    磁盘: "Drive",
    目录路径: "Directory path",
    筛选文件夹: "Filter folders",
    切换配色: "Change theme",
    "缺少会话凭据，请用终端中的完整地址打开网页。":
      "Session token missing. Open the complete URL shown in the terminal.",
  };
  const labels = [...document.querySelectorAll("[data-i18n]")].map((el) => [
    el,
    el.textContent,
  ]);
  const placeholders = [...document.querySelectorAll("[placeholder]")].map(
    (el) => [el, el.placeholder],
  );
  const attributes = [
    ...document.querySelectorAll("[aria-label], [title]"),
  ].flatMap((el) =>
    ["aria-label", "title"]
      .filter((key) => el.hasAttribute(key))
      .map((key) => [el, key, el.getAttribute(key)]),
  );
  let language = "zh-CN";
  const t = (text) =>
    language === "en-US" ? translations[text] || text : text;
  const localized = (zh, en) => (language === "en-US" ? en : zh);
  const number = (value) => Number(value || 0).toLocaleString(language);
  const state = {
    loaded: null,
    catalog: { activities: [], machines: [], directories: [] },
    activity: new Set(),
    machine: new Set(),
    exclude: new Set(),
    busy: false,
    stopped: false,
    cancelling: false,
    job: null,
    events: [],
    eventCount: 0,
    report: null,
    shown: 60,
    folder: null,
    folderTarget: null,
    save: null,
    catalogVersion: 0,
  };
  let token = new URLSearchParams(location.hash.slice(1)).get("token");
  try {
    if (token) sessionStorage.setItem("proto-session", token);
    else token = sessionStorage.getItem("proto-session");
    document.documentElement.dataset.theme =
      localStorage.getItem("proto-theme") || "dark";
  } catch {
    /* Storage is optional; the current session still works. */
  }
  history.replaceState(null, "", location.pathname + location.search);

  function node(tag, text, className) {
    const el = document.createElement(tag);
    if (text !== undefined) el.textContent = text;
    if (className) el.className = className;
    return el;
  }
  function notice(message, error = false) {
    $("notice").textContent = message;
    $("notice").hidden = !message;
    $("notice").classList.toggle("error", error);
  }
  async function api(path, payload, download = false) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 20000);
    try {
      const response = await fetch(path, {
        method: payload === undefined ? "GET" : "POST",
        headers: {
          "X-Proto-Token": token || "",
          ...(payload === undefined
            ? {}
            : { "Content-Type": "application/json" }),
        },
        body: payload === undefined ? undefined : JSON.stringify(payload),
        signal: controller.signal,
        cache: "no-store",
        credentials: "omit",
      });
      if (download && response.ok) return response.blob();
      const data = await response.json();
      if (!response.ok)
        throw new Error(data.error || `HTTP ${response.status}`);
      return data;
    } catch (error) {
      if (error instanceof TypeError || error.name === "AbortError")
        throw new Error(
          t("服务连接失败，请检查终端，并用终端中的完整地址重新打开网页。"),
        );
      throw error;
    } finally {
      clearTimeout(timer);
    }
  }
  const attempt = (operation) => async (event) => {
    event?.preventDefault();
    try {
      await operation(event);
    } catch (error) {
      notice(error.message, true);
    }
  };
  function themeLabel() {
    $("theme-label").textContent = t(
      document.documentElement.dataset.theme === "dark"
        ? "暖白模式"
        : "深色模式",
    );
  }
  function translate() {
    document.documentElement.lang = language;
    for (const [el, text] of labels) el.textContent = t(text);
    for (const [el, text] of placeholders) el.placeholder = t(text);
    for (const [el, key, text] of attributes) el.setAttribute(key, t(text));
    document.title = localized(
      "Proto Builder · 协议构建工作台",
      "Proto Builder · Protocol workspace",
    );
    $("connection").textContent = t(
      state.stopped ? "服务已关闭" : state.loaded ? "本地已连接" : "正在连接",
    );
    themeLabel();
    if (state.catalog.proto_files !== undefined) catalogSummary();
    renderPickers();
    if (state.report) renderReport(state.report);
    if (state.events.length) renderLogs();
    if (state.folder) renderFolders();
  }
  function fill(data) {
    state.loaded = data;
    const s = data.settings;
    $("version").textContent = `v${data.version}`;
    $("root").value = data.root;
    for (const name of ["proto", "server", "calc"])
      $(`${name}-path`).value = s[`${name}_path`];
    for (const [id, value] of Object.entries({
      "svn-update": s.svn.update,
      "copy-to-calc": s.copy_to_calc,
      docs: s.docs,
      "protocol-config": s.protocol_config,
      force: data.force,
      "adopt-existing": data.adopt_existing,
    }))
      $(id).checked = Boolean(value);
    for (const [id, value] of Object.entries({
      jobs: s.jobs,
      "assign-id": s.assign_id,
      timeout: s.timeout_seconds,
      lang: s.lang,
    }))
      $(id).value = value;
    state.activity = new Set(s.activity_ids);
    state.machine = new Set(s.machine_ids);
    state.exclude = new Set(s.excludes);
    language = s.lang;
    $("project-name").textContent =
      data.root.split(/[\\/]/).filter(Boolean).pop() || data.root;
    $("project-name").title = data.root;
    setBusy(false);
    translate();
  }
  function form() {
    if (!$("settings-form").reportValidity())
      throw new Error(
        localized("请检查构建参数。", "Check the build parameters."),
      );
    const s = structuredClone(state.loaded.settings);
    for (const name of ["proto", "server", "calc"])
      s[`${name}_path`] = $(`${name}-path`).value.trim();
    for (const [id, key] of [
      ["copy-to-calc", "copy_to_calc"],
      ["docs", "docs"],
      ["protocol-config", "protocol_config"],
    ])
      s[key] = $(id).checked;
    s.svn.update = $("svn-update").checked;
    s.activity_ids = [...state.activity];
    s.machine_ids = [...state.machine];
    s.excludes = [...state.exclude];
    for (const [id, key] of [
      ["jobs", "jobs"],
      ["assign-id", "assign_id"],
      ["timeout", "timeout_seconds"],
    ]) {
      const value = Number($(id).value);
      if (!Number.isSafeInteger(value) || value < 0)
        throw new Error(
          localized(
            "请输入有效的整数参数。",
            "Enter valid integer parameters.",
          ),
        );
      s[key] = value;
    }
    s.lang = $("lang").value;
    return {
      root: $("root").value.trim(),
      settings: s,
      force: $("force").checked,
      adopt_existing: $("adopt-existing").checked,
    };
  }
  function setBusy(busy) {
    state.busy = busy;
    $("settings-fields").disabled = busy || state.stopped || !state.loaded;
    for (const id of ["plan", "build", "doctor"])
      $(id).disabled = busy || state.stopped || !state.loaded;
    $("run-progress").hidden = !busy;
    $("cancel").disabled = !busy || state.cancelling;
    $("cancel").textContent = t(state.cancelling ? "正在停止" : "停止操作");
  }
  async function loadCatalog() {
    const version = ++state.catalogVersion;
    $("catalog-summary").textContent = t("正在读取协议…");
    try {
      const data = await api("/api/catalog", form());
      if (version !== state.catalogVersion) return;
      state.catalog = data;
      catalogSummary();
      renderPickers();
    } catch (error) {
      if (version !== state.catalogVersion) return;
      state.catalog = { activities: [], machines: [], directories: [] };
      $("catalog-summary").textContent =
        t("无法读取协议，请检查协议目录后重试。");
      renderPickers();
      notice(error.message, true);
    }
  }
  function catalogSummary() {
    const data = state.catalog;
    $("catalog-summary").textContent = localized(
      `${number(data.proto_files)} 个协议 · ${number(data.directories.length)} 个目录`,
      `${number(data.proto_files)} protocols · ${number(data.directories.length)} directories`,
    );
  }
  function renderPicker(kind, choices) {
    const selection = state[kind],
      query = $(`${kind}-search`).value.trim().toLowerCase();
    const values = [...new Set([...choices, ...selection])]
      .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }))
      .filter((v) => v.toLowerCase().includes(query));
    const list = $(`${kind}-options`);
    list.replaceChildren();
    for (const value of values) {
      const label = node("label", undefined, "choice"),
        input = node("input");
      input.type = "checkbox";
      input.checked = selection.has(value);
      input.value = value;
      input.addEventListener("change", () => {
        input.checked ? selection.add(value) : selection.delete(value);
        updateCounts();
      });
      label.append(input, node("span", value));
      list.append(label);
    }
    if (!values.length)
      list.append(
        node(
          "p",
          t(
            choices.length
              ? "未找到匹配项"
              : "暂无可选项；可输入 ID 后按 Enter 添加",
          ),
          "choice-empty",
        ),
      );
  }
  function updateCounts() {
    for (const kind of ["activity", "machine"])
      $(`${kind}-count`).textContent = state[kind].size
        ? localized(`已选 ${state[kind].size}`, `${state[kind].size} selected`)
        : t("全部");
    $("exclude-count").textContent = number(state.exclude.size);
  }
  function renderPickers() {
    renderPicker("activity", state.catalog.activities);
    renderPicker("machine", state.catalog.machines);
    const list = $("exclude-options"),
      query = $("exclude-search").value.toLowerCase().trim();
    list.replaceChildren();
    for (const value of [
      ...new Set([...state.catalog.directories, ...state.exclude]),
    ]
      .sort()
      .filter((v) => v.toLowerCase().includes(query))) {
      const label = node("label", undefined, "choice"),
        input = node("input");
      input.type = "checkbox";
      input.checked = state.exclude.has(value);
      input.addEventListener("change", () => {
        input.checked ? state.exclude.add(value) : state.exclude.delete(value);
        updateCounts();
      });
      label.append(input, node("span", value));
      list.append(label);
    }
    if (!list.childElementCount)
      list.append(node("p", t("未找到匹配项"), "choice-empty"));
    updateCounts();
  }
  function addIDs(kind) {
    const input = $(`${kind}-search`),
      ids = input.value.trim().split(/[,，\s]+/);
    if (ids.some((id) => !/^\d+$/.test(id))) {
      notice(t("请输入数字 ID，可用逗号分隔"), true);
      return;
    }
    ids.forEach((id) => state[kind].add(id));
    input.value = "";
    notice("");
    renderPickers();
  }
  function addExclude() {
    const value = $("exclude-search").value.trim();
    if (value) {
      state.exclude.add(value);
      $("exclude-search").value = "";
      renderPickers();
    }
  }
  async function openProject() {
    const data = await api("/api/project", { root: $("root").value.trim() });
    state.report = null;
    state.catalog = { activities: [], machines: [], directories: [] };
    state.job = null;
    state.events = [];
    state.eventCount = 0;
    $("result-content").hidden = true;
    $("empty-state").hidden = false;
    $("error-box").hidden = true;
    $("completion-box").hidden = true;
    document.title = "Proto Builder · 协议构建工作台";
    $("download-report").hidden = true;
    $("command-text").textContent = "";
    $("elapsed").textContent = "";
    $("logs").replaceChildren();
    $("log-count").textContent = "0";
    fill(data);
    notice("");
    await loadCatalog();
  }
  async function browse(id) {
    state.folderTarget = id;
    $("folder-dialog").showModal();
    try {
      await loadFolders($(id).value || state.loaded.root);
    } catch {
      await loadFolders(state.loaded.root);
    }
  }
  async function loadFolders(path) {
    $("folder-list").replaceChildren(
      node("p", t("正在读取目录…"), "folder-empty"),
    );
    $("folder-choose").disabled = true;
    try {
      state.folder = await api(
        `/api/directories?path=${encodeURIComponent(path)}`,
      );
      $("folder-path").value = state.folder.path;
      $("folder-filter").value = "";
      $("folder-drive").replaceChildren(
        ...state.folder.roots.map((root) => {
          const option = node("option", root);
          option.value = root;
          option.selected = state.folder.path
            .toLowerCase()
            .startsWith(root.toLowerCase());
          return option;
        }),
      );
      $("folder-up").disabled = !state.folder.parent;
      $("folder-choose").disabled = false;
      renderFolders();
    } catch (error) {
      $("folder-list").replaceChildren(
        node("p", error.message, "folder-empty"),
      );
      throw error;
    }
  }
  function renderFolders() {
    const query = $("folder-filter").value.trim().toLowerCase(),
      list = $("folder-list");
    const entries = state.folder.entries.filter((entry) =>
      entry.name.toLowerCase().includes(query),
    );
    list.replaceChildren(
      ...entries.map((entry) => {
        const button = node("button", entry.name, "folder-item");
        button.type = "button";
        button.addEventListener(
          "click",
          attempt(() => loadFolders(entry.path)),
        );
        return button;
      }),
    );
    if (!entries.length)
      list.append(node("p", t("此目录下没有文件夹"), "folder-empty"));
    $("folder-count").textContent = localized(
      `${number(entries.length)} 个目录${state.folder.total > 1000 ? "（仅显示前 1,000 项，可输入完整路径）" : ""}`,
      `${number(entries.length)} directories${state.folder.total > 1000 ? " (first 1,000; enter a full path to navigate)" : ""}`,
    );
  }
  function tab(name) {
    for (const id of ["result", "log"]) {
      $(`${id}-tab`).setAttribute("aria-selected", String(id === name));
      $(`${id}-view`).hidden = id !== name;
    }
  }
  const phases = {
    startup: "读取配置",
    command: "读取配置",
    config: "读取配置",
    toolchain: "准备工具链",
    scan: "扫描协议",
    generate: "编译协议",
    compile: "编译协议",
    protocol: "校验生成物",
    commit: "同步文件",
    sync: "同步文件",
    report: "保存报告",
    plan: "预览协议计划",
    doctor: "检查开发环境",
    svn: "SVN",
    update: "Update",
  };
  async function start(action) {
    notice("");
    const input = form();
    const job = await api("/api/jobs", { action, form: input });
    state.job = job.id;
    state.events = [];
    state.eventCount = 0;
    state.report = null;
    state.shown = 60;
    state.cancelling = false;
    $("command-text").textContent = job.command;
    $("error-box").hidden = true;
    $("completion-box").hidden = true;
    document.title = "Proto Builder · 协议构建工作台";
    $("empty-state").hidden = false;
    $("result-content").hidden = true;
    $("download-report").hidden = true;
    $("progress").removeAttribute("value");
    $("logs").replaceChildren();
    $("log-count").textContent = "0";
    setBusy(true);
    tab("log");
    $("run-state").dataset.state = "running";
    $("status-text").textContent = t("正在执行");
    await poll(job.id);
  }
  async function poll(id) {
    if (state.stopped || state.job !== id) return;
    try {
      const data = await api(
        `/api/jobs/${encodeURIComponent(id)}?after=${state.eventCount}`,
      );
      if (state.stopped || state.job !== id) return;
      appendLogs(data.events || []);
      state.eventCount = data.event_count;
      state.report = data;
      $("connection").textContent = t("本地已连接");
      $("log-count").textContent = number(state.eventCount);
      $("elapsed").textContent = `${Number(data.duration_seconds).toFixed(1)}s`;
      $("phase-text").textContent = t(phases[data.phase] || data.phase);
      for (const event of data.events || []) {
        if (event.phase === "command")
          $("command-text").textContent = event.message;
        const progress = event.message.match(/\[(\d+)\/(\d+)\]/);
        if (progress) {
          $("progress").max = Number(progress[2]);
          $("progress").value = Number(progress[1]);
        }
      }
      if (data.finished) {
        setBusy(false);
        $("download-report").hidden = false;
        renderReport(data);
        if (data.status === "success") {
          tab("result");
          $("result-view").scrollTop = 0;
          $("completion-box").scrollIntoView({ block: "nearest" });
        }
        if (data.status === "success" && data.command === "build")
          loadCatalog();
        if (data.report_error) notice(data.report_error, true);
      } else {
        setBusy(true);
        setTimeout(() => poll(id), 450);
      }
    } catch (error) {
      notice(error.message, true);
      $("connection").textContent = localized(
        "连接中断，正在重试",
        "Disconnected; retrying",
      );
      // Keep controls locked until the outcome is known; a failed HTTP request does not stop a build.
      if (!state.stopped && state.job === id) setTimeout(() => poll(id), 2000);
    }
  }
  function logNode(event) {
    const row = node("div", undefined, `log-entry ${event.level}`);
    row.append(
      node("time", `${event.elapsed_seconds.toFixed(2)}s`),
      node("span", event.phase, "log-phase"),
      node("span", event.message, "log-message"),
    );
    return row;
  }
  function appendLogs(events) {
    const logs = $("logs"),
      follow = logs.scrollHeight - logs.scrollTop - logs.clientHeight < 70;
    state.events.push(...events);
    for (const event of events)
      if (event.level !== "debug" || $("debug-logs").checked)
        logs.append(logNode(event));
    if (follow) logs.scrollTop = logs.scrollHeight;
  }
  function renderLogs() {
    $("logs").replaceChildren(
      ...state.events
        .filter((event) => event.level !== "debug" || $("debug-logs").checked)
        .map(logNode),
    );
  }
  function metric(value, label) {
    const el = node("div", undefined, "metric");
    el.append(node("strong", number(value)), node("span", t(label)));
    return el;
  }
  function renderReport(report) {
    const cancelled = report.exit_code === 130;
    $("run-state").dataset.state = report.finished
      ? cancelled
        ? "cancelled"
        : report.status
      : "running";
    $("status-text").textContent = t(
      report.finished
        ? cancelled
          ? "已取消"
          : report.status === "success"
            ? "已完成"
            : "失败"
        : "正在执行",
    );
    $("error-box").hidden = !report.error;
    $("error-title").textContent = t(
      cancelled ? "操作已取消" : "本次操作已中止",
    );
    $("error-message").textContent = report.error || "";
    $("diagnostics").replaceChildren(
      ...(report.diagnostics || []).map((diagnostic) => {
        const el = node("div", undefined, "diagnostic"),
          path = `${diagnostic.file}:${diagnostic.line}${diagnostic.column ? `:${diagnostic.column}` : ""}`;
        const button = node("button", path);
        button.type = "button";
        button.title = localized("点击复制文件位置", "Copy file location");
        button.addEventListener(
          "click",
          attempt(() => copy(path, "路径已复制")),
        );
        el.append(button, node("p", diagnostic.message));
        return el;
      }),
    );
    const result = report.result;
    const success = report.finished && report.status === "success";
    $("completion-box").hidden = !success;
    if (success) {
      const title =
        report.command === "build"
          ? localized("编译完成", "Build complete")
          : report.command === "plan"
            ? localized(
                "预览完成，未写入文件",
                "Preview complete — no files written",
              )
            : localized("环境检查完成", "Environment check complete");
      const elapsed = Number(report.duration_seconds || 0).toFixed(1);
      const c = result?.changes;
      $("completion-title").textContent = title;
      $("completion-summary").textContent = c
        ? localized(
            `耗时 ${elapsed} 秒 · 新增 ${number(c.added.length)} · 修改 ${number(c.modified.length)} · 删除 ${number(c.removed.length)} · 未变 ${number(c.unchanged.length)}。生成物已安全写回。`,
            `${elapsed}s · ${number(c.added.length)} added · ${number(c.modified.length)} modified · ${number(c.removed.length)} removed · ${number(c.unchanged.length)} unchanged. Outputs safely deployed.`,
          )
        : localized(
            `耗时 ${elapsed} 秒，详情见下方结果。`,
            `${elapsed}s. See the results below.`,
          );
      document.title = `✓ ${title} · Proto Builder`;
    }
    $("empty-state").hidden = Boolean(result);
    $("result-content").hidden = !result;
    $("unchanged-label").hidden = !result?.changes;
    if (!result) return;
    const metrics = $("metrics");
    if (report.command === "plan") {
      metrics.replaceChildren(
        metric(result.scanned, "扫描文件"),
        metric(result.selected.length, "待编译文件"),
        metric(Object.keys(result.groups).length, "编译目录"),
        metric(result.filtered.length + result.excluded.length, "筛选跳过"),
      );
      $("result-note").textContent = localized(
        `预览未写入任何生成物。${result.excluded.length} 个文件被排除，${result.filtered.length} 个文件被 ID 筛选跳过，${result.import_only.length} 个文件仅作为导入依赖。`,
        `Preview wrote no generated files. ${result.excluded.length} excluded, ${result.filtered.length} skipped by ID filters, ${result.import_only.length} used only as imports.`,
      );
      if (result.documentation_inputs?.length)
        $("result-note").textContent += localized(
          ` 共享 HTML / Markdown 将由完整的 ${number(result.documentation_inputs.length)} 个非 RPC 协议生成，不受 ID 筛选影响。`,
          ` Shared HTML / Markdown will cover all ${number(result.documentation_inputs.length)} non-RPC protocols, independent of ID filters.`,
        );
    } else if (result.changes) {
      const c = result.changes;
      metrics.replaceChildren(
        ...[
          ["added", "新增"],
          ["modified", "修改"],
          ["removed", "删除"],
          ["unchanged", "未变"],
        ].map(([key, label]) => metric(c[key].length, label)),
      );
      $("result-note").textContent = localized(
        `${number(result.proto_files)} 个协议 · ${number(result.groups)} 个目录 · ${number(result.commands)} 条编译命令${c.adopted?.length ? ` · 接管 ${c.adopted.length} 个已有文件` : ""}${c.backup ? `。备份：${c.backup}` : ""}`,
        `${number(result.proto_files)} protocols · ${number(result.groups)} directories · ${number(result.commands)} compiler commands${c.adopted?.length ? ` · ${c.adopted.length} existing files adopted` : ""}${c.backup ? `. Backup: ${c.backup}` : ""}`,
      );
      if (result.documentation_proto_files)
        $("result-note").textContent += localized(
          `。共享文档：完整 ${number(result.documentation_proto_files)} 个非 RPC 协议。`,
          `. Shared documents: all ${number(result.documentation_proto_files)} non-RPC protocols.`,
        );
    } else if (result.checks) {
      metrics.replaceChildren(
        metric(
          result.checks.filter((check) => check.passed).length,
          "检查通过",
        ),
      );
      $("result-note").textContent = localized(
        "检查配置、工具链、输入协议和目录权限。",
        "Configuration, toolchain, input protocols and directory permissions checked.",
      );
    }
    renderResults();
  }
  function renderResults() {
    const report = state.report;
    if (!report?.result) return;
    const result = report.result,
      query = $("result-search").value.toLowerCase(),
      list = $("result-list");
    const matches = (value) => value.toLowerCase().includes(query);
    let rows = [];
    if (report.command === "plan") {
      rows = Object.entries(result.groups)
        .map(([group, indices]) => ({
          group,
          files: indices.map((index) => result.selected[index].relative),
        }))
        .filter((row) => matches(row.group) || row.files.some(matches))
        .map((row) => {
          const el = node("details", undefined, "group-row"),
            summary = node("summary"),
            files = node("ul");
          summary.append(
            node("span", row.group),
            node("span", number(row.files.length)),
          );
          for (const file of row.files) files.append(node("li", file));
          el.append(summary, files);
          return el;
        });
    } else if (result.changes) {
      const labels = {
        added: "新增文件",
        modified: "修改文件",
        removed: "删除文件",
        unchanged: "未变文件",
      };
      for (const key of [
        "added",
        "modified",
        "removed",
        ...($("show-unchanged").checked ? ["unchanged"] : []),
      ]) {
        for (const path of result.changes[key])
          if (matches(path)) {
            const el = node("div", undefined, `file-row ${key}`);
            el.append(
              node("span", t(labels[key]), "file-kind"),
              node("span", path, "file-path"),
            );
            rows.push(el);
          }
      }
    } else if (result.checks) {
      const names = {
        configuration: "配置",
        toolchain: "工具链",
        inputs: "输入协议",
        "output-permissions": "输出权限",
        svn: "SVN",
      };
      rows = result.checks
        .filter((check) => matches(JSON.stringify(check)))
        .map((check) => {
          const el = node("div", undefined, "check-result");
          el.append(
            node(
              "strong",
              `${t(names[check.check] || check.check)} · ${t("已通过")}`,
            ),
            node("pre", JSON.stringify(check, null, 2)),
          );
          return el;
        });
    }
    list.replaceChildren(...rows.slice(0, state.shown));
    if (!rows.length)
      list.append(
        node(
          "p",
          t(
            result.changes &&
              !query &&
              !result.changes.added.length &&
              !result.changes.modified.length &&
              !result.changes.removed.length
              ? "所有生成物均未变化"
              : "没有匹配的结果",
          ),
          "choice-empty",
        ),
      );
    $("more-results").hidden = rows.length <= state.shown;
    $("more-results").textContent = localized(
      `显示更多（${number(Math.min(rows.length, state.shown))} / ${number(rows.length)}）`,
      `Show more (${number(Math.min(rows.length, state.shown))} / ${number(rows.length)})`,
    );
  }
  async function copy(text, success) {
    try {
      await navigator.clipboard.writeText(text);
      notice(t(success));
    } catch {
      notice(t("复制失败，请手动选择文本复制"), true);
    }
  }
  async function previewConfig() {
    const input = form(),
      preview = await api("/api/config/preview", input);
    state.save = { form: input, expected_hash: preview.expected_hash };
    $("save-path").textContent = preview.path;
    $("config-diff").textContent = preview.diff || t("配置未发生变化");
    $("save-dialog").showModal();
  }
  async function saveConfig() {
    $("confirm-save").disabled = true;
    try {
      const saved = await api("/api/config/save", state.save);
      $("save-dialog").close();
      notice(`${t("配置已保存")} · ${saved.path}`);
    } finally {
      $("confirm-save").disabled = false;
    }
  }
  async function downloadReport() {
    const blob = state.stopped
      ? new Blob(
          [JSON.stringify({ ...state.report, events: state.events }, null, 2)],
          { type: "application/json" },
        )
      : await api(
          `/api/report/${encodeURIComponent(state.job)}`,
          undefined,
          true,
        );
    const url = URL.createObjectURL(blob),
      link = node("a");
    link.href = url;
    link.download = `proto-builder-${state.job}.json`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  async function shutdown() {
    const running = state.busy;
    await api("/api/shutdown", {});
    state.stopped = true;
    setBusy(false);
    $("shutdown-dialog").close();
    $("connection").textContent = t("服务已关闭");
    $("shutdown").disabled = true;
    notice(
      running
        ? localized(
            "停止请求已发送；当前操作停止后服务将退出。",
            "Stop requested. The server will exit after the current operation stops.",
          )
        : localized(
            "工作台已关闭。可运行 proto-builder web 再次打开。",
            "Workspace stopped. Run proto-builder web to open it again.",
          ),
    );
  }

  $("settings-form").addEventListener("submit", (event) =>
    event.preventDefault(),
  );
  for (const id of ["plan", "build", "doctor"])
    $(id).addEventListener(
      "click",
      attempt(() => start(id)),
    );
  $("open-project").addEventListener("click", attempt(openProject));
  $("root").addEventListener("keydown", (event) => {
    if (event.key === "Enter") attempt(openProject)(event);
  });
  $("proto-path").addEventListener("change", () => {
    if (!state.busy) loadCatalog();
  });
  for (const kind of ["activity", "machine"]) {
    $(`${kind}-search`).addEventListener("input", () =>
      renderPicker(
        kind,
        state.catalog[kind === "activity" ? "activities" : "machines"],
      ),
    );
    $(`${kind}-search`).addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        addIDs(kind);
      }
    });
  }
  $("exclude-search").addEventListener("input", renderPickers);
  $("exclude-search").addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      addExclude();
    }
  });
  $("add-exclude").addEventListener("click", addExclude);
  $("clear-filters").addEventListener("click", () => {
    for (const key of ["activity", "machine", "exclude"]) {
      state[key].clear();
      $(`${key}-search`).value = "";
    }
    $("assign-id").value = 0;
    renderPickers();
  });
  $("lang").addEventListener("change", () => {
    language = $("lang").value;
    translate();
  });
  $("theme").addEventListener("click", () => {
    const theme =
      document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem("proto-theme", theme);
    } catch {
      /* Optional preference. */
    }
    themeLabel();
  });
  for (const button of document.querySelectorAll("[data-browse]"))
    button.addEventListener(
      "click",
      attempt(() => browse(button.dataset.browse)),
    );
  for (const button of document.querySelectorAll("[data-close]"))
    button.addEventListener("click", () => $(button.dataset.close).close());
  $("folder-go").addEventListener(
    "click",
    attempt(() => loadFolders($("folder-path").value)),
  );
  $("folder-path").addEventListener("keydown", (event) => {
    if (event.key === "Enter")
      attempt(() => loadFolders($("folder-path").value))(event);
  });
  $("folder-up").addEventListener(
    "click",
    attempt(() => loadFolders(state.folder.parent)),
  );
  $("folder-drive").addEventListener(
    "change",
    attempt(() => loadFolders($("folder-drive").value)),
  );
  $("folder-filter").addEventListener("input", renderFolders);
  $("folder-choose").addEventListener(
    "click",
    attempt(async () => {
      $(state.folderTarget).value = state.folder.path;
      $("folder-dialog").close();
      if (state.folderTarget === "root") await openProject();
      else if (state.folderTarget === "proto-path") await loadCatalog();
    }),
  );
  $("result-tab").addEventListener("click", () => tab("result"));
  $("log-tab").addEventListener("click", () => tab("log"));
  for (const id of ["result-search", "show-unchanged"])
    $(id).addEventListener(id === "result-search" ? "input" : "change", () => {
      state.shown = 60;
      renderResults();
    });
  $("more-results").addEventListener("click", () => {
    state.shown += 60;
    renderResults();
  });
  $("debug-logs").addEventListener("change", renderLogs);
  $("cancel").addEventListener(
    "click",
    attempt(async () => {
      await api(`/api/cancel/${encodeURIComponent(state.job)}`, {});
      state.cancelling = true;
      setBusy(state.busy);
    }),
  );
  $("save-config").addEventListener("click", attempt(previewConfig));
  $("confirm-save").addEventListener("click", attempt(saveConfig));
  $("copy-command").addEventListener(
    "click",
    attempt(() => copy($("command-text").textContent, "命令已复制")),
  );
  $("download-report").addEventListener("click", attempt(downloadReport));
  $("shutdown").addEventListener("click", () =>
    $("shutdown-dialog").showModal(),
  );
  $("confirm-shutdown").addEventListener("click", attempt(shutdown));
  async function init() {
    themeLabel();
    if (!token) {
      notice(t("缺少会话凭据，请用终端中的完整地址打开网页。"), true);
      return;
    }
    try {
      const data = await api("/api/state");
      fill(data);
      await loadCatalog();
      if (data.last_job) {
        state.job = data.last_job;
        state.events = [];
        state.eventCount = 0;
        setBusy(true);
        await poll(data.last_job);
      }
    } catch (error) {
      notice(error.message, true);
    }
  }
  init();
})();
