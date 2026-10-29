# Proto Builder v1.3.0-dev.1

本预发布版将 Agent 调用统一为 **Agent API v2 / MCP stdio**，同时改进网页选择、预览、结果反馈和键盘向导。支持 Windows x86_64，仍由一个 Rust EXE 提供固定工具链、网页和终端能力，无 BAT、Python 或前端运行依赖。

GitHub Release 标记为 prerelease，签名渠道为 beta；v1.2.0 保持 Latest stable。软件版本为 `1.3.0-dev.1`，Agent 协议版本为 `2`，两者独立。

## Agent 接口升级

**旧 `--agent` 参数及 v1 Agent 请求已删除，不做兼容转换。** 原调用方和内置 Skill 使用者需要迁移新接口；普通 CLI、键盘向导、网页和普通命令的 `--json` 继续可用。

```powershell
proto-builder agent capabilities
proto-builder agent run --request D:\Temp\proto-request.json
proto-builder mcp
```

JSON CLI 与 MCP 共用类型化请求、应用服务和错误处理，提供 capabilities、config_show、config_check、catalog、doctor、plan、build、report_read 共 8 种操作。

- JSON 请求使用 `api_version: 2`、operation、params，可附 request_id；MCP 工具 arguments 直接采用 params 对象。
- 配置、单次构建策略及日志开关分别放在 params.options、params.policy、params.verbose。省略字段继承配置，`[]` 清空筛选，false 明确覆盖；未知字段、null 和错误类型直接拒绝。
- 响应区分执行状态、文件写回状态和报告保存状态。报告保存失败仍可能已完成写回，调用方应检查 output.state 和 output.changed 后再决定是否重试。
- 目录查询与报告支持分页；内联错误和冲突有数量、长度限制，完整证据保留在本机报告。能力发现和报告读取不创建或轮转日志。
- MCP 支持工具发现、请求关联、单任务执行和协作取消；一个 EXE 即可接入，无额外服务或端口。

JSON 示例及迁移规则见 [请求接口](AGENT-REQUESTS.md)，客户端配置见 [MCP 接入](MCP.md)。构建必须显式提供绝对协议项目根目录。已有 Skill 可先保留旧文件，再使用 `skill --output <新目录>` 导出新版。

## 交互改进

- 已选 ID 常驻显示为标签；未加入选集的输入会阻止直接编译，减少误执行全量构建。
- 修改参数后明确标记旧预览过期；环境检查、预览和构建采用一致的输出目录校验。
- 同项目重新加载保留选择和结果；切换项目时提示处理未保存修改。
- 完成后展示真实增删改数量，并提供协议文档、产物目录和备份入口。错误清楚说明原因、文件是否改变及下一步处理。
- 键盘向导支持长路径光标编辑、水平滚动、统一 Ctrl+C 取消及可滚动错误页；CLI 帮助按用途分组，新增明确的 `--no-protocol-config` 参数。

完整交互说明及验收范围见 [交互体验优化](UX-IMPROVEMENTS.md)。

## 安装与更新

从 [本版 Release](https://github.com/dongrv/proto-builder/releases/tag/v1.3.0-dev.1) 下载 Windows EXE，执行：

```powershell
.\proto-builder-1.3.0-dev.1-windows-x86_64.exe install
```

安装后新开终端。已安装版本也可主动选择 beta 渠道更新到其最新版本：

```powershell
proto-builder self-update apply --channel beta
```

默认 stable 渠道不会选择本预发布版。构建仍按配置默认更新 SVN；`.pb.go` 继续默认备份后更新，其他输出保留保护。局部 Go 构建继续保留完整适用范围的共享文档。

## 验证与附件

84 项 Rust 测试、58 个独立 EXE 验收场景及 MCP stdio 冒烟通过，覆盖 8 种操作、Schema、请求拒绝、配置来源、隔离项目真实编译、重复字节一致、覆盖备份、错误行号、长路径、取消和报告保存失败。普通 CLI、网页和键盘状态机均包含回归；格式和 Clippy 严格检查通过。详细记录见 [验收记录](VALIDATION.md)。

MCP 已完成协议子进程测试，尚未完成真实 Codex 客户端联调；键盘向导尚未完成真实终端全流程人工验收。上述本地检查不代表线上升级与回退已验收，相关结果应以发布后的单独记录为准。

日常运行只需 EXE。发布使用既有 Ed25519 身份签名，提供清单、签名、公钥、SHA256SUMS、配置示例、源码及文档附件。源码包含内置 Skill、网页和工具链资源；私钥、内部 baseline、验收脚本、日志和业务工程产物不进入发布附件。
