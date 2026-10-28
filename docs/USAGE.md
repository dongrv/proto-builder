# 使用与配置详解

当前 SVN 项目的 protobuf → Go 开发工具。日常只需一个 Windows x86_64 的 `proto-builder.exe`，以及可选的 TOML 配置。无 BAT、Python、Go、Rust 运行环境依赖。固定版本的 protoc 和 Go、gRPC、文档插件已压缩内置，首次使用自动解压到用户缓存，每次运行验证 SHA256。

**默认在构建开始时执行一次项目范围的 SVN 更新。** 该功能使用已安装的 `svn.exe`；可通过配置或 `--no-svn-update` 关闭。不会更新整个工作副本的上级目录，不会自动提交、切分支或解决冲突。

当前交付 Windows x86_64；其他平台保留外部工具链接口，尚未作为受支持的平台验收。

## 安装与开始使用

从 [GitHub Releases](https://github.com/dongrv/proto-builder/releases) 下载发布的 Windows x86_64 EXE。发布文件名包含版本号，下载后直接执行安装：

```powershell
.\proto-builder-1.1.0-windows-x86_64.exe install
```

默认安装到 `%LOCALAPPDATA%\ProtoBuilder\bin` 并注册用户 PATH；新开终端即可使用。也可直接运行下载的程序，或用 `install --dir D:\DevTools\ProtoBuilder` 自选安装目录。`install --system` 使用 Program Files 和系统 PATH，需要管理员终端。

在项目根目录执行：

```powershell
proto-builder doctor
proto-builder plan
proto-builder build
```

需要鼠标操作时运行 `proto-builder web`，自动在默认浏览器打开本地工作台。无参数执行 `proto-builder` 打开键盘向导，支持方向键、空格多选、搜索、粘贴和简单输入；终端不捕获鼠标，鼠标移动、点击、滚轮和焦点事件不触发界面重绘。

从其他目录启动时可以指定根目录：

```powershell
proto-builder --repo-root D:\work\go\src\shengyou\docs\branches\beta
```

网页和向导会记住最近使用的项目。完整命令行、键盘向导和网页共用相同的构建逻辑，操作前提供可复用的 PowerShell 命令。

## 网页工作台

```powershell
proto-builder web
proto-builder web --repo-root D:\work\go\src\shengyou\docs\branches\beta
proto-builder web --port 8779
proto-builder web --no-open
```

页面以 Claude Code 官网的炭黑、米白衬线标题和克制的橙色为视觉参考，支持暖白配色及中英文切换。HTML、CSS、JavaScript 全部内置 EXE，无需 Node.js、额外服务、联网资源或前端安装步骤。需要支持现代 JavaScript 的浏览器，推荐 Windows 上的 Edge 或 Chrome。

- 用目录选择器浏览磁盘和文件夹，或直接输入相对、绝对路径；更换项目目录后点击「打开项目」。
- 活动 ID、机器 ID 从真实协议自动读取，支持搜索、多选，输入数字后按 Enter 可添加；排除项支持目录和 glob。空选、交集、公共依赖等规则与 CLI 相同。
- 「预览计划」列出实际扫描、筛选和编译范围；「检查环境」验证开发环境；「开始编译」执行 SVN 更新、生成和安全写回。
- 构建时查看增量日志和阶段、停止当前操作，结束后按新增、修改、删除、未变查看文件，搜索结果或下载完整 JSON 报告。错误保留原文件、行、列，点击文件位置可复制。
- 设置默认只在当前会话生效。「保存配置」先展示差异，确认后写入项目 TOML 并备份；配置被其他程序修改时拒绝覆盖，需重新预览。

同一网页服务一次只执行一项任务，执行期间锁定参数；停止后可以直接调整重试。刷新页面可恢复当前或最近一次任务。关闭浏览器标签页不会停止构建；点击页面底部「关闭工作台」或在启动终端按 Ctrl+C 才会停止服务及当前任务。

服务只监听 `127.0.0.1`，默认随机空闲端口。启动时生成临时会话凭据，通过终端显示的完整地址进入；API 校验凭据、Host 和 Origin。固定端口不改变这个限制，不提供远程访问或任意静态文件下载。`web` 不与 `--json`、`--interactive`、`--dry-run` 混用；只读操作在页面点击「预览计划」，机器输出使用原 CLI。

## 常用命令

```powershell
# 明确筛选活动或机器；同类多选为 OR，不同类条件为 AND，公共协议保留
proto-builder plan --activity-id 7034,7035
proto-builder build --machine-id 6034
proto-builder build --activity-id 7034 --exclude experimental --exclude 'draft/**'

# 迁移时需要保留旧脚本的 ID 规则，可显式使用 assign-id
proto-builder build --assign-id 7034

# 仅预览，既不更新 SVN，也不生成或部署文件
proto-builder build --dry-run

# 独立控制功能
proto-builder build --no-svn-update --no-docs --no-copy
proto-builder build --no-config

# 首次接管已有且与新产物不同的生成文件：明确授权并保留备份
proto-builder build --adopt-existing

# 已登记的生成物被手动修改：默认报冲突；确认覆盖时先备份
proto-builder build --force

# 可复现的机器输出
proto-builder build --jobs 4 --json --non-interactive
proto-builder build --verbose --report-json D:\Temp\proto-build.json
proto-builder build --timeout-seconds 180
proto-builder version
proto-builder --help
```

`--assignID`、`--server_path`、`--calc_path`、`--proto_path` 仍是命令参数别名；旧脚本入口已弃用。没有 `--direct` 等绕过安全写回的模式。

活动 ID 从 `act<数字>`、`activity<数字>` 的路径或 package 识别；机器 ID 从 `slot<数字>`、`machine<数字>`、`rpc<数字>` 识别。也支持独立注释 `// activity_id=7034`、`// machine_id=6034`。显式元数据与目录识别冲突会给出文件和行号。筛选使用完整 ID 匹配。指定 ID 却没有对应协议时会失败，不会只生成公共协议并宣称成功。

排除项支持目录名、相对于 proto 的路径、glob，优先于 ID 筛选。被排除或未选中的文件仍可作为 import 依赖被读取，但不为其生成 Go 文件。缺失或循环 import 会中止执行。

## 配置

```powershell
proto-builder config init
proto-builder config show
proto-builder config check
proto-builder --config D:\Configs\project.toml plan
```

配置结构参见 [proto-builder.toml.example](../proto-builder.toml.example)。合并优先级从低到高为：内置默认值 → 用户配置 → 项目配置（或 `--config` 指定文件）→ 环境变量 → CLI。显式配置替代自动发现的项目配置。数组整体替换；CLI 传入空值可清空 ID 或排除项，例如 `--activity-id=`。

用户配置在 `%LOCALAPPDATA%\ProtoBuilder\config.toml`。项目配置从项目起点向上查找 `proto-builder.toml`。配置中的 `project_root` 相对于配置所在目录；`proto_path`、`server_path`、`calc_path` 相对于最终项目根目录。CLI 的 `--repo-root` 相对于执行目录。

常用环境变量包括 `PROTO_BUILDER_PROJECT_ROOT`、`PROTO_BUILDER_SERVER_PATH`、`PROTO_BUILDER_CALC_PATH`、`PROTO_BUILDER_JOBS`、`PROTO_BUILDER_LANG`、`PROTO_BUILDER_SVN_UPDATE`。诊断、测试或便携使用可用 `PROTO_BUILDER_HOME` 独立存储工具数据。未知配置项、无效值或未来的配置版本直接报错，避免拼写错误被忽略。

版本 0 的配置可读入并在内存中升级。`config migrate --check` 只预览；`config migrate` 确认后写回并保留 `.toml.bak`。不自动覆盖用户配置。

`--no-docs`、`--no-config`、`--no-copy`、`--no-svn-update` 关闭对应功能；`--docs=true`、`--protocol-config=true`、`--copy-to-calc=true`、`--svn-update=true` 可以显式覆盖配置中的关闭设置。

## 生成与结果可靠性

固定工具链来自当前项目原始四个 EXE，版本和哈希记录在 `assets/windows-x86_64/manifest.json`：protoc 3.15.7、protoc-gen-go 1.27.1、protoc-gen-go-grpc 1.2.0、protoc-gen-doc 1.5.0。默认不会静默回退到 PATH 或仓库 bin；显式启用外部工具链时记录路径、版本和 SHA256，产物不再保证与内置版本相同。

生成遵循当前项目规则：Go 使用原插件选项；RPC 按原规则生成 gRPC；**共享的 protocol.md / protocol.html 始终使用完整的非 RPC 协议集（应用显式排除项），不受活动 ID、机器 ID 或 assign-id 筛选影响。** 局部编译只缩小 Go 生成范围，不会删掉共享文档中的未选模块。文档内任意协议生成失败时，本次构建整体不写回；需要只生成局部 Go 文件时可关闭文档。协议号配置扫描整个协议集（应用排除规则），保持原脚本的编号、字段顺序、CRLF 和 JSON 序列化行为。**ID 筛选不缩减全局 ProtoConfig.json**，这是原有行为。生成的协议配置部署到业务服 `configs/{fatcat,reels,thunder}/sys`；计算服同步保留当前项目原有目录规则。

执行流程：取得相关输出目录锁 → SVN 更新和冲突检查 → 输入快照 → 有界并行生成 → 校验 → 统一写回。`jobs = 0` 使用可用 CPU 数减一、至少一个工作线程。相同项目或相同目标目录的并发构建会失败并提示等待；不同独立项目可并行。

所有生成结果先放临时目录；任意生成步骤失败均不部署。写回使用持久化操作日志、完整备份和单文件原子替换，写回错误自动恢复，意外中断后下一次构建先恢复。跨多个目录的文件替换不是操作系统级的全局原子事务；备份与恢复日志保障可恢复性。

工具仅清理已登记、属于本次构建范围且已过时的生成文件。选择性构建保留未选目录；`--no-copy` 保留计算服产物；无关手写文件保留。首次遇到完全相同的既有文件可直接登记；首次遇到内容不同的文件需要 `--adopt-existing`，之后手动修改已登记文件需要 `--force`。这两种情况都会保存被替换内容。

运行结束打印真实的新增、修改、未变、清理和接管计数。错误保留原始 protoc 诊断及文件、行、列，并明确中断。Ctrl+C 取消当前构建及工具子进程树。默认报告保存在用户数据目录的 `logs`；JSON 包含配置来源、工具链、SVN、阶段、耗时、筛选、输出哈希、变更和诊断。默认保留 20 次报告，同时限制日志总量。

常见退出码：`0` 成功；`2` 参数或配置错误；`3` ID 无匹配；`4` SVN 更新或冲突；`5` 工具链问题；`6` 签名或完整性失败；`7` 输出修改冲突；`8` 目标被占用；`9` 基准差异；`130` 取消。其他执行失败为 `1`。完整错误码仍在报告内。

## 安装更新与卸载

```powershell
proto-builder self-update check
proto-builder self-update apply
proto-builder self-update apply --channel beta
proto-builder self-update rollback
proto-builder uninstall
```

默认 stable 渠道，后台检查有缓存且不会等待网络完成后才构建；只提示更新，显式 `apply` 才替换程序。`[update] enabled = false` 关闭自动检查，不禁止手动检查。更新必须通过内置 Ed25519 公钥签名、版本、平台、大小和 SHA256 校验，并预先执行新程序的版本检查。保留更新前 EXE 供回退；下载或校验失败保留原程序。整个替换过程无 BAT 或 Python。

更新围绕 [dongrv/proto-builder](https://github.com/dongrv/proto-builder) 的 GitHub Releases。支持显式 HTTPS 镜像与代理，不使用隐式第三方代理；镜像也必须保留官方签名。API 受限时 stable 尝试官方 latest 签名清单；beta 需要 API 或已配置镜像。API token 可通过 `PROTO_BUILDER_GITHUB_TOKEN` 提供，仅发送给 GitHub API。

卸载只移除安装程序及由它添加的 PATH 项。默认保留用户配置和缓存。`uninstall --purge` 额外清理工具链缓存及偏好；仍保留项目、生成代码、所有权清单、备份、日志、内部基准和发布密钥。

## 验证与开发

原工具仅用于内部基准采样和迁移初期对比，产品代码从不调用旧脚本。内部样本保存在用户数据目录的 `baselines`，不纳入项目或发行包。初始版本已验证全量和 7034、6034 两个旧 ID 场景的字节基准。当前共享文档采用完整协议集：全量文档沿用固定插件及原参数；局部构建的文档范围按新的完整文档规则验收，Go 和协议 JSON 的生成规则保持一致。

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bin proto-builder
```

测试包含真实内置插件生成、并发一致性、用户修改保护、失败不部署、过时文件清理、事务失败回滚，以及本地 SVN 仓库的限定目录更新和冲突中止。网页集成测试覆盖会话保护、配置冲突、只读预览、真实构建、错误行号、取消及恢复；另在浏览器和 Windows 终端进行交互验收。

各版本的测试结果、字节基准和线上升级验收见 [验收记录](VALIDATION.md)。

其他说明：[SVN 日常操作](SVN-WORKFLOW.md)、[旧工具弃用说明](LEGACY-DEPRECATED.md)、[签名与发布](RELEASING.md)、[验收记录](VALIDATION.md)、[第三方许可](THIRD-PARTY-NOTICES.md)。
