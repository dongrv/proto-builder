"use strict";

(() => {
  const $ = (id) => document.getElementById(id);
  const translations = {
    加载项目: "Load project",
    核对项目路径: "Review project paths",
    已确认项目路径: "Confirmed project paths",
    "添加输入的 ID": "Add entered IDs",
    清空搜索: "Clear search",
    "已选 ID": "Selected IDs",
    已选排除目录: "Selected exclusions",
    技术详情: "Technical details",
    当前设置尚未保存: "Current settings are not saved",
    "加载其他项目会放弃本页尚未保存的设置。可返回保存，或放弃后继续加载。":
      "Loading another project discards unsaved settings on this page. Return to save them, or discard and continue.",
    保留当前设置: "Keep current settings",
    放弃并加载: "Discard and load",
    本次生成物备份: "Backups from this build",
    "按原文件名导出备份；下载不会覆盖项目文件。":
      "Export backups with their original filenames. Downloads do not replace project files.",
    "环境变量信息需要核对；下方已有路径仍可使用。请确认实际目录，编译成功后会记住路径和名字。":
      "Review the environment information. Existing paths below can still be used. Confirm the directories; paths and name are remembered after a successful build.",
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
    协议项目根目录: "Protocol project root",
    打开项目: "Open project",
    "Proto 源文件目录": "Proto source directory",
    业务服目录: "Server directory",
    计算服目录: "Calculation server directory",
    开发者名字: "Developer name",
    请核对项目路径与环境变量: "Check project paths and environment variables",
    "团队配置缺失、无效或路径存在冲突，请核对下方信息。路径和名字会在编译成功后自动记住。":
      "Team configuration is missing, invalid or has conflicting paths. Check the details below. Paths and name are remembered after a successful build.",
    "协议仓库的根目录，对应 ENV_PROTOBUF，通常包含 proto 文件夹。业务服与计算服的 Go 工程路径分别填写在下方。":
      "The protocol repository root, corresponding to ENV_PROTOBUF, usually contains a proto folder. Enter the Go server project paths separately below.",
    "Proto 源文件目录通常为根目录下的 proto。相对路径以协议项目根目录为起点；路径和名字在编译成功后记住。":
      "Proto sources usually live in the root's proto folder. Relative paths start at the protocol project root; paths and name are remembered after a successful build.",
    "以下是启动时从当前进程读取的环境变量；上方手动输入优先。修改 Windows 环境变量后，请重新启动终端或启动本工具的应用。":
      "These environment variables were read from the current process at startup. Manual inputs above take priority. After changing Windows environment variables, restart the terminal or application that launches this tool.",
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
    "pb.go 默认以本次生成结果覆盖，覆盖前自动备份；以下选项用于其他生成物及过期文件。":
      "pb.go files use the new generated result by default, with a backup before replacement. These options apply to other outputs and stale files.",
    接管已有生成物的操作说明: "How to adopt existing outputs",
    覆盖手动修改生成物的操作说明: "How to replace manually edited outputs",
    "用于首次迁移：勾选后，已有但尚未登记的 HTML、Markdown、JSON 等生成物与本次结果不同时，先备份原文件，再覆盖并登记；未勾选则遇到此类差异时中止。":
      "For first-time migration: when checked, existing untracked HTML, Markdown, JSON and other outputs that differ are backed up, replaced and registered. Otherwise, differences stop the build.",
    "pb.go 无需勾选，默认备份后覆盖。操作步骤：核对项目路径和编译范围，按需勾选，再点击「开始编译」。完成后从结果中的备份目录及 index.json 查找原文件。":
      "pb.go files are backed up and replaced by default. Check the project paths and build scope, select this option if needed, then click Build protocols. Find original files using the backup directory and index.json shown in the results.",
    "用于已经登记后又被手动修改的其他生成物：勾选后，先备份，再以本次生成结果覆盖；本次范围内已过时且被修改的产物也会备份后清理。未勾选则中止并列出冲突文件。":
      "For other registered outputs edited by hand: when checked, back up and replace them with the generated result. Modified stale outputs in this scope are also backed up and removed. Otherwise, stop and list conflicting files.",
    "仍有对应协议的 pb.go 默认备份后覆盖，无需勾选。此选项不会跳过 SVN 冲突、扩大编译范围或删除未登记文件。需要保留手写逻辑时，请放到独立的 Go 文件中。":
      "pb.go files with corresponding protocols are backed up and replaced by default. This option does not bypass SVN conflicts, expand the build scope or remove untracked files. Keep handwritten logic in separate Go files.",
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
    选择协议项目根目录: "Choose protocol project root",
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
    language === "en-US"
      ? translations[text.replace(/\s+/g, " ").trim()] || text
      : text;
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
    savedFingerprint: null,
    runForm: null,
    starting: false,
    pendingProject: null,
    summaryFingerprint: null,
    disposed: false,
  };
  const lifetime = new AbortController();
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
    $("action-notice").textContent = message;
    $("action-notice").hidden = !message;
    $("action-notice").classList.toggle("error", error);
  }
  async function api(path, payload, download = false) {
    const controller = new AbortController();
    const stop = () => controller.abort();
    lifetime.signal.addEventListener("abort", stop, { once: true });
    if (lifetime.signal.aborted) controller.abort();
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
      if (
        error instanceof TypeError ||
        error instanceof SyntaxError ||
        error.name === "AbortError"
      ) {
        const connectionError = new Error(
          t("服务连接失败，请检查终端，并用终端中的完整地址重新打开网页。"),
        );
        connectionError.uncertainResponse = true;
        throw connectionError;
      }
      throw error;
    } finally {
      clearTimeout(timer);
      lifetime.signal.removeEventListener("abort", stop);
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
    renderSetup();
    renderCurrent();
  }
  function renderSetup() {
    const setup = state.loaded?.setup;
    $("setup-notice").hidden = !setup?.needs_confirmation;
    $("project-paths-summary").textContent = t(
      setup?.needs_confirmation ? "核对项目路径" : "已确认项目路径",
    );
    $("setup-issues").replaceChildren(
      ...(setup?.issues || []).map((issue) => node("li", issue)),
    );
    const environment = Object.entries(setup?.environment || {});
    $("environment-summary").textContent = localized(
      `环境变量识别 · 已读取 ${environment.length} 项`,
      `Environment variables · ${environment.length} read`,
    );
    $("environment-values").replaceChildren(
      ...environment.flatMap(([name, value]) => [
        node("dt", name),
        node("dd", value),
      ]),
    );
  }
  function fill(data) {
    state.loaded = data;
    const s = data.settings;
    $("version").textContent = `v${data.version}`;
    $("root").value = data.root;
    $("developer-name").value = s.developer_name || "";
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
    state.savedFingerprint = fingerprint(form(false));
    $("project-paths").open = Boolean(data.setup?.needs_confirmation);
    for (const kind of ["activity", "machine", "exclude"])
      $(`${kind}-search`).value = "";
    setBusy(false);
    translate();
  }
  function form(validate = true) {
    if (validate && !$("settings-form").reportValidity())
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
      if (validate && (!Number.isSafeInteger(value) || value < 0))
        throw new Error(
          localized(
            "请输入有效的整数参数。",
            "Enter valid integer parameters.",
          ),
        );
      s[key] = value;
    }
    s.lang = $("lang").value;
    s.developer_name = $("developer-name").value.trim();
    return {
      root: $("root").value.trim(),
      settings: s,
      force: $("force").checked,
      adopt_existing: $("adopt-existing").checked,
    };
  }
  function samePath(a, b) {
    const normalize = (value) =>
      String(value || "")
        .replaceAll("\\", "/")
        .replace(/\/+$/, "")
        .toLowerCase();
    return normalize(a) === normalize(b);
  }
  function fingerprint(value) {
    const sort = (item, key = "") => {
      if (Array.isArray(item))
        return [...item]
          .map((v) => sort(v))
          .sort((a, b) => String(a).localeCompare(String(b)));
      if (item && typeof item === "object")
        return Object.fromEntries(
          Object.keys(item)
            .sort()
            .map((k) => [k, sort(item[k], k)]),
        );
      if (typeof item === "string" && (key === "root" || key.endsWith("_path")))
        return item
          .trim()
          .replaceAll("\\", "/")
          .replace(/\/+$/, "")
          .toLowerCase();
      return item;
    };
    return JSON.stringify(sort(value));
  }
  function pendingInputs() {
    return ["activity", "machine", "exclude"].filter((kind) => {
      const value = $(`${kind}-search`).value.trim();
      if (!value) return false;
      const values = kind === "exclude" ? [value] : value.split(/[,，\s]+/);
      return values.some((id) => !state[kind].has(id));
    });
  }
  function renderCurrent() {
    if (!state.loaded) return;
    const current = form(false),
      pending = pendingInputs();
    for (const kind of ["activity", "machine", "exclude"]) {
      const tags = $(`${kind}-selected`);
      tags.replaceChildren(
        ...[...state[kind]]
          .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }))
          .map((value) => {
            const button = node("button", `${value} ×`, "selection-tag");
            button.type = "button";
            button.disabled = state.busy || state.stopped;
            button.setAttribute(
              "aria-label",
              localized(`移除 ${value}`, `Remove ${value}`),
            );
            button.addEventListener("click", () => {
              state[kind].delete(value);
              renderPickers();
            });
            return button;
          }),
      );
      if (kind !== "exclude")
        $(`add-${kind}`).disabled = !$(`${kind}-search`).value.trim();
      $(`clear-${kind}-search`).disabled = !$(`${kind}-search`).value.trim();
      const message = $(`${kind}-pending`);
      message.hidden = !pending.includes(kind);
      message.textContent = localized(
        kind === "exclude"
          ? "搜索内容尚未加入排除范围；请添加、勾选或清空。"
          : "输入尚未加入选择；请添加、勾选或清空。",
        "Search text is not selected. Add, select, or clear it.",
      );
    }
    const ids = (set) => [...set].join(", ");
    const scope =
      state.activity.size || state.machine.size
        ? [
            state.activity.size
              ? localized(
                  `活动 ${ids(state.activity)}`,
                  `Activities ${ids(state.activity)}`,
                )
              : "",
            state.machine.size
              ? localized(
                  `机器 ${ids(state.machine)}`,
                  `Machines ${ids(state.machine)}`,
                )
              : "",
          ]
            .filter(Boolean)
            .join(localized(" ∩ ", " ∩ "))
        : localized("全部协议", "All protocols");
    const summary = $("execution-summary");
    const scopeFingerprint = `${language}:${fingerprint(current)}`;
    const notes = [];
    if (Number(current.settings.assign_id))
      notes.push(
        localized(
          `旧 ID ${current.settings.assign_id}`,
          `Legacy ID ${current.settings.assign_id}`,
        ),
      );
    if (state.exclude.size)
      notes.push(
        localized(
          `排除 ${ids(state.exclude)}`,
          `Exclude ${ids(state.exclude)}`,
        ),
      );
    notes.push(localized("公共协议保留", "Common protocols retained"));
    notes.push(
      current.settings.docs
        ? localized(
            "文档覆盖完整非 RPC 协议（遵循排除项）",
            "Documents cover all non-RPC protocols, with exclusions",
          )
        : localized("不生成文档", "No documentation"),
    );

    const destinations = `${localized("业务服", "Server")}: ${current.settings.server_path}${current.settings.copy_to_calc ? ` · ${localized("计算服", "Calculation server")}: ${current.settings.calc_path}` : ""}`;
    if (state.summaryFingerprint !== scopeFingerprint) {
      summary.replaceChildren(
        node(
          "strong",
          localized(`本次范围：${scope}`, `Current scope: ${scope}`),
        ),
        node("span", notes.join(" · ")),
        node("span", destinations, "summary-path"),
        node(
          "span",
          localized(
            "Go 文件有差异时先备份，再以本次结果覆盖。",
            "Differing Go files are backed up and replaced by this build.",
          ),
        ),
      );
      state.summaryFingerprint = scopeFingerprint;
    }
    const dirty =
      state.runForm && fingerprint(current) !== fingerprint(state.runForm);
    const snapshot = $("snapshot-notice");
    snapshot.hidden = !dirty;
    snapshot.textContent = localized(
      "参数已变更；下方结果和等价命令属于上次操作。可重新预览当前范围。",
      "Parameters changed. Results and the equivalent command below belong to the previous operation. Preview the current scope again.",
    );
    $("completion-box").classList.toggle("stale", Boolean(dirty));
    const rootChanged = !samePath(current.root, state.loaded.root);
    $("root").setAttribute("aria-invalid", String(rootChanged));
    $("root-changed-note").hidden = !rootChanged;
    $("root-changed-note").textContent = localized(
      "目录已修改，请先点击「加载项目」。",
      "The directory changed. Click Load project first.",
    );
    if (rootChanged) $("project-paths").open = true;
    $("path-overview").replaceChildren(
      ...[
        [localized("协议项目", "Protocol project"), state.loaded.root],
        [localized("业务服", "Server"), current.settings.server_path],
        ...(current.settings.copy_to_calc
          ? [
              [
                localized("计算服", "Calculation server"),
                current.settings.calc_path,
              ],
            ]
          : []),
      ].map(([label, path]) => {
        const line = node("span", `${label}: ${path}`);
        line.title = path;
        return line;
      }),
    );
  }
  function focusField(id) {
    const field = $(id);
    if (!field) return;
    for (
      let parent = field.parentElement;
      parent;
      parent = parent.parentElement
    )
      if (parent.tagName === "DETAILS") parent.open = true;
    field.focus();
    field.scrollIntoView({ block: "nearest" });
  }
  function actionButton(label, action) {
    const button = node("button", label, "button secondary small");
    button.type = "button";
    button.addEventListener("click", attempt(action));
    return button;
  }
  function renderFailure(report) {
    const failure = report.failure || {};
    const kinds = {
      invalid_paths: [
        "输出目录配置有冲突。",
        "Output paths conflict.",
        "请修改业务服或计算服目录；不需要同步时可关闭计算服同步。",
        "Correct the server paths, or disable calculation sync if it is not needed.",
        "server-path",
      ],
      no_matching_protocols: [
        "当前筛选没有匹配的协议。",
        "No protocols match the current filters.",
        "核对活动、机器 ID 的交集及排除项，然后重新预览。",
        "Review the intersection of activity/machine IDs and exclusions, then preview again.",
        "activity-search",
      ],
      invalid_protocols: [
        "协议文件需要修正。",
        "Protocol sources need correction.",
        "按下方文件及行号修正源文件，再重新编译。",
        "Correct the source files at the locations below, then build again.",
      ],
      svn_conflict: [
        "SVN 工作副本存在冲突。",
        "The SVN working copy has conflicts.",
        "先在 SVN 中处理冲突，再重新执行。",
        "Resolve the SVN conflicts before running again.",
      ],
      svn_failed: [
        "SVN 操作未完成。",
        "The SVN operation did not complete.",
        "检查 SVN 连接、权限和工作副本状态，再重试。",
        "Check SVN connectivity, permissions and working-copy status before retrying.",
      ],
      unowned_output: [
        "其他已有生成物尚未接管。",
        "Other existing outputs have not been adopted.",
        "核对冲突文件；确认允许替换时，选择接管已有生成物后重试。",
        "Review conflicting files. Select adoption only if replacing those files is intended.",
        "adopt-existing",
      ],
      modified_output: [
        "其他生成物包含手动修改。",
        "Other outputs contain manual edits.",
        "先核对需要保留的修改；确认后再选择覆盖手动修改的生成物。",
        "Review edits that must be retained before explicitly choosing to replace modified outputs.",
        "force",
      ],
      modified_stale_output: [
        "过期产物包含手动修改。",
        "Stale outputs contain manual edits.",
        "先核对文件；允许清理时再选择覆盖手动修改的生成物。",
        "Review the files before explicitly allowing removal of modified stale outputs.",
        "force",
      ],
      output_conflict: [
        "生成文件存在冲突。",
        "Generated outputs have conflicts.",
        "逐项核对冲突文件，不要直接强制重试。",
        "Review each conflicting file before choosing a replacement policy.",
      ],
      process_timeout: [
        "编译任务超过设定时间。",
        "The compiler exceeded its time limit.",
        "检查日志和协议规模，必要时调整超时后重试。",
        "Review logs and protocol size; adjust the timeout if needed.",
        "timeout",
      ],
      target_busy: [
        "目标正在被其他任务使用。",
        "Another task is using the target.",
        "等待现有任务结束后再重试。",
        "Wait for the current task to finish before retrying.",
      ],
      inputs_changed: [
        "执行期间输入或配置发生变化。",
        "Inputs or configuration changed during execution.",
        "重新核对配置和协议，预览后再执行。",
        "Review the changed configuration and protocols, then preview and run again.",
      ],
      report_storage: [
        "报告保存失败。",
        "The report could not be saved.",
        "先核对下方写回状态，再检查报告目录权限与剩余空间。",
        "Check the output state below before inspecting report-directory permissions and free space.",
      ],
      output_transaction: [
        "写回或恢复过程遇到问题。",
        "Output commit or recovery encountered a problem.",
        "保留日志与备份，核对文件状态后再处理。",
        "Keep the logs and backups, and inspect file state before proceeding.",
      ],
      cancelled: [
        "操作已取消。",
        "Operation cancelled.",
        "核对写回状态，按需修改参数后重新执行。",
        "Check output state and run again when appropriate.",
      ],
      invalid_configuration: [
        "配置需要核对。",
        "Configuration needs review.",
        "检查配置文件和下方详情，修正后再执行。",
        "Review the configuration and details, correct them, then run again.",
        "root",
      ],
    };
    const data = kinds[failure.kind] || [
      "本次操作未完成。",
      "The operation did not complete.",
      "查看文件位置和技术详情，修正原因后再执行。",
      "Review file locations and technical details, correct the cause, then run again.",
    ];
    $("error-summary").textContent = localized(data[0], data[1]);
    $("error-next-step").textContent = localized(
      `下一步：${data[2]}`,
      `Next: ${data[3]}`,
    );
    const states = {
      not_written: ["生成物未写回。", "Generated outputs were not written."],
      committed: [
        "生成物已写回；请勿因报告错误而重复覆盖。",
        "Generated outputs were committed. A report error does not require repeating the build.",
      ],
      rolled_back: [
        "本次写回已回滚。",
        "This output transaction was rolled back.",
      ],
      recovered: [
        "已恢复上次未完成的事务；本次写回状态请结合结果核对。",
        "A previous incomplete transaction was recovered. Check this operation's results before proceeding.",
      ],
      recovery_required: [
        "文件状态需检查；请保留备份并处理恢复问题。",
        "File state needs inspection. Preserve backups and resolve recovery issues.",
      ],
    };
    const output = states[report.output_state] || [
      "写回状态未确认，请核对日志与文件。",
      "Output state is not confirmed. Check logs and files.",
    ];
    $("error-output-state").textContent = localized(...output);
    const actions = $("error-actions");
    actions.replaceChildren();
    if (data[4])
      actions.append(
        actionButton(localized("修改相关设置", "Review related settings"), () =>
          focusField(data[4]),
        ),
      );
    if (failure.files?.length)
      actions.append(
        actionButton(localized("复制相关文件清单", "Copy affected files"), () =>
          copy(
            failure.files.join("\n"),
            localized("文件清单已复制", "File list copied"),
          ),
        ),
      );
    if (failure.suggested_action && !kinds[failure.kind])
      $("error-next-step").textContent += ` (${failure.suggested_action})`;
  }
  function renderResultActions(report) {
    const box = $("result-actions");
    box.replaceChildren();
    const locations = report.locations || {};
    if (!report.finished) {
      box.hidden = true;
      return;
    }
    for (const [kind, zh, en] of [
      ["docs", "查看协议文档", "View protocol docs"],
      ["server", "打开业务服产物", "Open server outputs"],
      ["calc", "打开计算服产物", "Open calculation outputs"],
    ]) {
      if (locations[kind])
        box.append(
          actionButton(localized(zh, en), async () => {
            await api("/api/open", { job_id: state.job, kind });
          }),
        );
    }
    if (locations.backup || report.result?.changes?.backup) {
      box.append(
        actionButton(localized("浏览本次备份", "Browse backups"), showBackups),
      );
      box.append(
        actionButton(localized("复制备份路径", "Copy backup path"), () =>
          copy(
            locations.backup || report.result.changes.backup,
            localized("备份路径已复制", "Backup path copied"),
          ),
        ),
      );
    }
    if (report.artifacts_error)
      box.append(
        node(
          "p",
          localized(
            `结果已保留，但备份入口暂不可用：${report.artifacts_error}`,
            `Results are retained, but backup access is unavailable: ${report.artifacts_error}`,
          ),
          "pending-note",
        ),
      );
    box.hidden = !box.childElementCount;
  }
  async function showBackups() {
    const job = state.job;
    $("backup-dialog").showModal();
    $("backup-path").textContent = "";
    $("backup-files").replaceChildren(
      node("p", localized("正在读取备份…", "Loading backups…")),
    );
    try {
      const result = await api(`/api/backups/${encodeURIComponent(job)}`);
      $("backup-path").textContent = result.directory || "";
      $("backup-files").replaceChildren(
        ...result.files.map((file) => {
          const row = node("div", undefined, "backup-row");
          const name = node("div");
          name.append(
            node("strong", file.name),
            node("p", file.target, "mono"),
          );
          const button = actionButton(
            localized("导出原文件", "Export original"),
            async () => {
              try {
                saveBlob(
                  await api(file.download_url, undefined, true),
                  file.name,
                );
              } catch (error) {
                row.append(node("p", error.message, "pending-note"));
              }
            },
          );
          row.append(name, button);
          return row;
        }),
      );
      if (!result.files.length)
        $("backup-files").append(
          node(
            "p",
            localized(
              "本次没有需要备份的原文件。",
              "No original files needed backup in this build.",
            ),
          ),
        );
    } catch (error) {
      $("backup-files").replaceChildren(
        node("p", error.message, "pending-note"),
      );
    }
  }
  function saveBlob(blob, filename) {
    const url = URL.createObjectURL(blob),
      link = node("a");
    link.href = url;
    link.download = filename;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  function setBusy(busy) {
    state.busy = busy;
    $("settings-fields").disabled = busy || state.stopped || !state.loaded;
    for (const id of ["plan", "build", "doctor"])
      $(id).disabled = busy || state.stopped || !state.loaded;
    $("run-progress").hidden = !busy;
    $("cancel").disabled = !busy || state.starting || state.cancelling;
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
        renderCurrent();
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
    renderCurrent();
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
        renderCurrent();
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
  async function openProject(confirmed = false) {
    const target = $("root").value.trim();
    if (samePath(target, state.loaded.root)) {
      await loadCatalog();
      notice(
        localized(
          "已刷新协议列表，当前选择与结果已保留。",
          "Protocol list refreshed. Current selections and results were retained.",
        ),
      );
      return;
    }
    const current = form(false);
    current.root = state.loaded.root;
    if (
      !confirmed &&
      (fingerprint(current) !== state.savedFingerprint ||
        pendingInputs().length)
    ) {
      state.pendingProject = target;
      $("project-dialog").showModal();
      return;
    }
    const data = await api("/api/project", { root: $("root").value.trim() });
    state.report = null;
    state.runForm = null;
    state.catalog = { activities: [], machines: [], directories: [] };
    state.job = null;
    state.events = [];
    state.eventCount = 0;
    $("result-content").hidden = true;
    $("empty-state").hidden = false;
    $("error-box").hidden = true;
    $("completion-box").hidden = true;
    if ($("report-storage-warning")) $("report-storage-warning").hidden = true;
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
  async function recoverStart(previousJob, input, action) {
    while (!state.stopped && !state.disposed) {
      $("phase-text").textContent = localized(
        "正在确认启动状态…",
        "Confirming task status…",
      );
      $("connection").textContent = localized(
        "正在确认任务，请勿重复提交",
        "Confirming task; do not resubmit",
      );
      try {
        const current = await api("/api/state");
        if (state.stopped || state.disposed) return null;
        if (
          !current.last_job ||
          current.last_job === previousJob ||
          !samePath(current.root, input.root)
        )
          return null;
        const candidate = await api(
          `/api/jobs/${encodeURIComponent(current.last_job)}`,
        );
        if (state.stopped || state.disposed) return null;
        if (
          candidate.command !== action ||
          !candidate.form ||
          fingerprint(candidate.form) !== fingerprint(input)
        )
          return null;
        notice(
          localized(
            "启动响应中断，已找到匹配任务并恢复结果。",
            "The start response was interrupted. A matching task was found and its status recovered.",
          ),
        );
        return { id: current.last_job, command: "" };
      } catch (error) {
        if (state.stopped || state.disposed) return null;
        notice(
          localized(
            "暂时无法确认任务是否已启动。正在重连，操作保持锁定；不会重复提交。",
            "Task startup cannot yet be confirmed. Reconnecting with controls locked; the request will not be resubmitted.",
          ),
          true,
        );
        await new Promise((resolve) => {
          const finish = () => {
            clearTimeout(timer);
            lifetime.signal.removeEventListener("abort", finish);
            resolve();
          };
          const timer = setTimeout(finish, 2000);
          lifetime.signal.addEventListener("abort", finish, { once: true });
          if (lifetime.signal.aborted) finish();
        });
      }
    }
    return null;
  }
  async function start(action) {
    notice("");
    if (state.busy || state.starting) return;
    const pending = pendingInputs();
    if (pending.length) {
      renderCurrent();
      const input = $(`${pending[0]}-search`);
      input.focus();
      notice(
        localized(
          "搜索内容尚未加入范围，请先添加、勾选或清空搜索后再执行。",
          "Search text is not part of the selection. Add, select, or clear it before running.",
        ),
        true,
      );
      return;
    }
    const input = form();
    let previousJob = state.job;
    state.starting = true;
    setBusy(true);
    $("phase-text").textContent = localized("正在启动…", "Starting…");
    $("run-state").dataset.state = "running";
    $("status-text").textContent = localized("正在启动", "Starting");
    let job;
    let submitted = false;
    try {
      // Read a fresh checkpoint so an older completed task cannot be mistaken
      // for this request when its response is lost.
      const checkpoint = await api("/api/state");
      if (state.stopped || state.disposed) return;
      previousJob = checkpoint.last_job;
      submitted = true;
      job = await api("/api/jobs", { action, form: input });
    } catch (error) {
      if (submitted && error.uncertainResponse)
        job = await recoverStart(previousJob, input, action);
      if (state.stopped || state.disposed) return;
      if (!job) {
        setBusy(false);
        $("connection").textContent = error.uncertainResponse
          ? localized("连接不可用", "Connection unavailable")
          : t("本地已连接");
        $("run-state").dataset.state = "failed";
        $("status-text").textContent = localized(
          "启动未确认",
          "Start not confirmed",
        );
        throw error;
      }
    } finally {
      state.starting = false;
    }
    state.runForm = structuredClone(input);
    state.job = job.id;
    state.events = [];
    state.eventCount = 0;
    state.report = null;
    state.shown = 60;
    state.cancelling = false;
    $("command-text").textContent = job.command;
    $("error-box").hidden = true;
    $("completion-box").hidden = true;
    if ($("report-storage-warning")) $("report-storage-warning").hidden = true;
    document.title = "Proto Builder · 协议构建工作台";
    $("empty-state").hidden = false;
    $("result-content").hidden = true;
    $("download-report").hidden = true;
    $("result-actions").hidden = true;
    renderCurrent();
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
    if (state.stopped || state.disposed || state.job !== id) return;
    try {
      const data = await api(
        `/api/jobs/${encodeURIComponent(id)}?after=${state.eventCount}`,
      );
      if (state.stopped || state.disposed || state.job !== id) return;
      appendLogs(data.events || []);
      state.eventCount = data.event_count;
      state.report = data;
      if (data.form) state.runForm = data.form;
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
        } else if (data.error) {
          $("error-box").focus({ preventScroll: true });
          $("error-box").scrollIntoView({ block: "nearest" });
        }
        if (data.status === "success" && data.command === "build") {
          const current = await api("/api/state");
          state.loaded.setup = current.setup;
          renderSetup();
          $("project-paths").open = Boolean(current.setup?.needs_confirmation);
          loadCatalog();
        }
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
      if (!state.stopped && !state.disposed && state.job === id)
        setTimeout(() => poll(id), 2000);
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
  function renderReportStorage(report) {
    let warning = $("report-storage-warning");
    if (!warning && !report.report_error) return;
    if (!warning) {
      warning = node("div", undefined, "notice");
      warning.id = "report-storage-warning";
      warning.setAttribute("role", "alert");
      $("completion-box").before(warning);
    }
    warning.hidden = !report.report_error;
    if (!report.report_error) return;
    const states = {
      committed: [
        "生成物已写回。报告保存失败不会撤销编译结果，无需因此重新编译。",
        "Generated outputs were committed. Report storage failure does not undo the build; rebuilding is not required for this warning.",
      ],
      not_written: [
        "生成物未写回。请同时查看本次操作结果或原始错误。",
        "Generated outputs were not written. Also review the operation result or original error.",
      ],
      rolled_back: [
        "本次写回已回滚。请保留当前结果并查看原始错误。",
        "This output transaction was rolled back. Keep the current result and review the original error.",
      ],
      recovered: [
        "已恢复上次未完成的事务；请结合本次结果核对文件状态。",
        "A previous incomplete transaction was recovered. Check file state against this operation's result.",
      ],
      recovery_required: [
        "文件状态尚需恢复检查。请保留备份，勿直接重复覆盖。",
        "File state requires recovery checks. Preserve backups and do not blindly repeat replacement.",
      ],
    };
    const output = states[
      report.output_state ?? report.result?.output_state
    ] || [
      "写回状态未确认，请核对当前结果和文件后再决定是否重试。",
      "Output state is not confirmed. Review the current result and files before deciding whether to retry.",
    ];
    const details = node("details");
    details.append(
      node("summary", t("技术详情")),
      node("p", report.report_error, "mono"),
    );
    warning.replaceChildren(
      node("strong", localized("报告保存失败", "Report storage failed")),
      node("p", localized(...output)),
      node(
        "p",
        localized(
          "可先下载当前报告，再检查日志目录的权限与可用空间。",
          "Download the current report, then check log-directory permissions and free space.",
        ),
      ),
      actionButton(
        localized("下载当前报告", "Download current report"),
        downloadReport,
      ),
      details,
    );
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
    renderFailure(report);
    renderResultActions(report);
    renderReportStorage(report);
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
    renderCurrent();
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
      const sections = new Map();
      const normalized = (value) =>
        String(value || "")
          .replaceAll("\\", "/")
          .replace(/\/+$/, "");
      for (const key of [
        "added",
        "modified",
        "removed",
        ...($("show-unchanged").checked ? ["unchanged"] : []),
      ]) {
        for (const path of result.changes[key]) {
          if (!matches(path)) continue;
          const locations = report.locations || {},
            full = normalized(path);
          let category = localized("其他生成物", "Other outputs"),
            display = path;
          for (const [location, zh, en] of [
            ["server", "业务服", "Server"],
            ["calc", "计算服", "Calculation server"],
            ["project", "协议项目", "Protocol project"],
          ]) {
            const base = normalized(locations[location]);
            if (
              base &&
              full.toLowerCase().startsWith(base.toLowerCase() + "/")
            ) {
              category = localized(zh, en);
              display = full.slice(base.length + 1);
              break;
            }
          }
          if (/\.(html|md)$/i.test(path))
            category = localized("共享文档", "Shared documentation");
          const el = node("div", undefined, `file-row ${key}`),
            name = node("span", display, "file-path");
          name.title = path;
          el.append(
            node("span", t(labels[key]), "file-kind"),
            name,
            actionButton(localized("复制路径", "Copy path"), () =>
              copy(path, "路径已复制"),
            ),
          );
          if (!sections.has(category)) sections.set(category, []);
          sections.get(category).push(el);
        }
      }
      for (const [category, items] of sections) {
        rows.push(
          node("h3", `${category} · ${items.length}`, "result-group-title"),
          ...items,
        );
      }
    } else if (result.checks) {
      const names = {
        configuration: "配置",
        toolchain: "工具链",
        inputs: "输入协议",
        "output-permissions": "输出权限",
        svn: "SVN",
        "output-roots": localized("输出目录范围", "Output scope"),
        "output-layout": localized("输出目录布局", "Output directory layout"),
      };
      rows = result.checks
        .filter((check) => matches(JSON.stringify(check)))
        .map((check) => {
          const el = node("div", undefined, "check-result");
          el.append(
            node(
              "strong",
              `${t(names[check.check] || check.check)} · ${t(check.passed ? "已通过" : "失败")}`,
            ),
            (() => {
              const details = node("details"),
                summary = node("summary", t("技术详情"));
              details.append(
                summary,
                node("pre", JSON.stringify(check, null, 2)),
              );
              return details;
            })(),
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
      state.savedFingerprint = fingerprint(state.save.form);
      notice(`${t("配置已保存")} · ${saved.path}`);
      renderCurrent();
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
    lifetime.abort();
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
  window.addEventListener("pagehide", () => {
    state.disposed = true;
    lifetime.abort();
  });
  window.addEventListener("pageshow", (event) => {
    if (event.persisted && state.disposed) location.reload();
  });
  for (const id of ["plan", "build", "doctor"])
    $(id).addEventListener(
      "click",
      attempt(() => start(id)),
    );
  $("open-project").addEventListener(
    "click",
    attempt(() => openProject()),
  );
  $("root").addEventListener("keydown", (event) => {
    if (event.key === "Enter") attempt(() => openProject())(event);
  });
  $("proto-path").addEventListener("change", () => {
    if (!state.busy) loadCatalog();
  });
  for (const kind of ["activity", "machine"]) {
    $(`${kind}-search`).addEventListener("input", () => {
      renderPicker(
        kind,
        state.catalog[kind === "activity" ? "activities" : "machines"],
      );
      renderCurrent();
    });
    $(`add-${kind}`).addEventListener("click", () => addIDs(kind));
    $(`clear-${kind}-search`).addEventListener("click", () => {
      $(`${kind}-search`).value = "";
      notice("");
      renderPickers();
    });
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
  $("clear-exclude-search").addEventListener("click", () => {
    $("exclude-search").value = "";
    notice("");
    renderPickers();
  });
  for (const event of ["input", "change"])
    $("settings-form").addEventListener(event, renderCurrent);
  $("keep-project").addEventListener("click", () => {
    $("root").value = state.loaded.root;
    state.pendingProject = null;
    $("project-dialog").close();
    renderCurrent();
  });
  $("discard-project").addEventListener(
    "click",
    attempt(async () => {
      $("root").value = state.pendingProject;
      $("project-dialog").close();
      await openProject(true);
      state.pendingProject = null;
    }),
  );
  $("project-dialog").addEventListener("cancel", () => {
    $("root").value = state.loaded.root;
    state.pendingProject = null;
    renderCurrent();
  });
  $("clear-filters").addEventListener("click", () => {
    notice("");
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
      renderCurrent();
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
