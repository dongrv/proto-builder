# MCP：连接 Codex 与其他 Agent

`1.3.0-dev.1` 预发布版内置 MCP stdio 服务，由同一个 EXE 提供，无需 Node、Python、BAT、额外服务或端口。普通 CLI 和网页保留；机器接口统一使用 Agent API v2，旧 `--agent` 已删除。不支持 MCP 的 Agent 可以使用 [JSON 请求](AGENT-REQUESTS.md)。请从 [v1.3.0-dev.1 Release](https://github.com/dongrv/proto-builder/releases/tag/v1.3.0-dev.1) 下载对应 EXE；稳定版 v1.2.0 不提供本页的新接口。

## Codex 配置

将下面片段合并到你的 Codex `config.toml`，把 `command` 换成实际 EXE 的绝对路径。不要覆盖已有配置。该示例没有修改客户端的审批策略。

```toml
[mcp_servers.proto_builder]
command = 'D:\tools\proto-builder\proto-builder.exe'
args = ['mcp']
startup_timeout_sec = 10
tool_timeout_sec = 600
```

也可在可用的 Codex CLI 中注册：

```powershell
codex mcp add proto_builder -- 'D:\tools\proto-builder\proto-builder.exe' mcp
```

EXE 路径和 `mcp` 参数应分开配置，不能把整条命令塞进 `command`。不要添加 `--repo-root` 等全局项目参数，项目与筛选放入每次调用的 `options`；构建覆盖策略放在单独的 `policy`，详细日志开关放在 `verbose`。新增连接后按客户端要求重新加载；在支持 `/mcp` 的 Codex CLI 中可以查看连接情况。配置字段和 CLI 注册方式依据 [Codex 官方 MCP 文档](https://developers.openai.com/codex/mcp/)。

`tool_timeout_sec` 是客户端等待整次工具调用的时限；请求中的 `options.timeout_seconds` 是每条底层编译命令的时限，两者作用不同。大项目可按实测调整客户端时限。客户端超时不等于文件未写回。

其他支持 stdio MCP 的客户端，同样设置 executable/command 为本 EXE、arguments/args 为 `["mcp"]` 即可；客户端配置文件格式由各客户端决定。

## 可调用工具

| 工具 | 用途 |
| --- | --- |
| `proto_builder_capabilities` | 查询操作、参数 schema、输出版本、限制与默认策略 |
| `proto_builder_config_show` | 有效路径、配置值、逐字段来源与覆盖历史 |
| `proto_builder_config_check` | 校验配置 |
| `proto_builder_catalog` | 根据实际扫描元数据搜索和分页查看协议文件、ID、目录、依赖 |
| `proto_builder_doctor` | 检查工具链、输出布局和权限 |
| `proto_builder_plan` | 预览编译与共享文档范围 |
| `proto_builder_build` | 按原有 SVN、备份与事务规则生成文件 |
| `proto_builder_report_read` | 分页读取报告、诊断、选中清单、输出和变更 |

示例：调用 `proto_builder_plan`，参数如下：

```json
{
  "options": {
    "repo_root": "D:/work/go/src/shengyou/docs/branches/beta",
    "activity_ids": ["7001", "7002"],
    "machine_ids": []
  }
}
```

MCP 工具 arguments 直接对应 JSON 请求中的 `params` 对象，按工具的 `inputSchema` 校验。不要包一层 params，也不能传 `api_version`、`operation` 或 `request_id`；这些外层字段会被拒绝。关联 ID 由 JSON-RPC 请求 ID 提供。`proto_builder_capabilities` 使用空对象 `{}`。构建必须明确给出绝对协议项目根目录，避免工作目录推断造成误操作。省略选项继承配置，`[]` 清空筛选，`false` 明确覆盖；null 和未知字段被拒绝。构建参数可另带 `policy: {"force": false, "adopt_existing": false}`，默认均为 false。`.pb.go` 按既定规则默认备份后更新，其他输出冲突不会自动升级覆盖策略。

建议先查配置和实际范围，在已有用户授权内构建。已有授权无需重复确认；用户没有授权的源码管理、安装更新等操作不属于构建调用。服务不提供任意 shell、SVN 提交、GitLab 操作或客户端配置写入工具。

## 状态与取消

- 一个连接同时执行一个工具。第二个执行请求会明确返回 busy，不会悄悄排队编译；协议 `ping` 和取消通知仍可处理。
- 当前协议版本返回 `structuredContent`，同时保留 JSON 文本 `content`。业务失败使用 `isError: true`，内部为 `api: "proto-builder.agent"`、`api_version: 2` 响应；必须检查 `output.state` 和 `output.changed`。
- `notifications/cancelled` 按 JSON-RPC `requestId` 取消当前调用。取消是协作式的，已经进入提交的事务可能完成写回；不能把发出取消当作未修改文件的证明。
- EOF 或 Ctrl+C 会请求取消并等待当前工作线程完成收尾。不要强杀进程后立即覆盖或删除事务日志；异常退出仍由现有事务恢复机制处理。
- JSON-RPC 请求 ID 在同一连接内不可重复；字符串 ID 最多 128 个 Unicode 字符，每连接最多 65,536 个请求，达到后重连。响应中的 `request_id` 关联 JSON-RPC 调用，不是幂等键；不能在 arguments 里另行设置。
- stdout 仅传输 MCP JSON-RPC 帧；进度输出到 stderr。输入单帧最多 1 MiB，报告读取最多 32 MiB，单页最多 500 项。调用方应持续读取输出。
- 只读标记描述协议项目源文件和生成物：查询或预览仍可能保存本机运行报告；doctor 可准备工具链缓存和临时探测权限。能力查询与报告查询不会写日志。

支持协议版本 `2025-11-25`、`2025-06-18`、`2024-11-05`。旧版 `2024-11-05` 通过文本 `content` 返回 JSON，不声明结构化输出 schema。协议实现依据 [MCP stdio](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)、[工具](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)和[生命周期](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)。

当前提供工具调用、发现和取消；不提供 MCP Resources、Prompts、后台持久任务或实时进度通知。详细事件在任务完成后按报告分页读取，执行期间通过 stderr 查看进度。

## 兼容边界与验证

Agent v2 是本预发布版的破坏性接口调整；客户端应重新读取 tools/list，按新的 inputSchema 和 outputSchema 调用，不缓存旧参数定义。普通 CLI 的 `--json` 和持久报告继续使用内部完整报告 v1，MCP 工具输出使用 Agent v2，两个版本号独立。

84 项 Rust 测试及 58 个独立 EXE 验收场景通过，覆盖 JSON 请求/普通 CLI 构建字节一致、空数组和 false 覆盖、严格参数拒绝、写回保护、报告分页，以及 MCP 三版本协商、工具发现、关联 ID、忙状态、取消、EOF 收尾和报告保存失败后的真实写回状态。MCP stdio 冒烟验证了初始化、8 个工具发现、调用和共用 Schema。检查结果见 [验收记录](VALIDATION.md)。

客户端示例只提供配置方法，不会自动改动 Codex 全局设置。目前完成的是协议子进程测试，尚未完成真实 Codex 客户端联调。
