# Codex 与其他 Agent 调用

预发布版 `1.3.0-dev.1` 使用 **Agent API v2**。机器调用统一为 JSON 请求或 MCP，旧 `--agent` 参数及 v1 Agent 请求已删除，不做兼容转换。普通 CLI、键盘向导和网页仍可使用；普通命令的 `--json` 输出完整内部报告，与 Agent v2 响应是两种不同格式。

## 开始调用

```powershell
proto-builder agent capabilities
proto-builder agent run --request D:\Temp\proto-plan.json
proto-builder mcp
```

能力发现不要求项目，也不创建报告。`agent run` 从 UTF-8 文件或 `--request -` 的标准输入读取一个严格校验的 JSON 请求，不打开界面、不等待交互，也不会因请求错误回退到构建。`agent --help` 也返回 v2 JSON，帮助文本位于 `result.help`；根命令的 `--help` / `--version` 仍是普通文本。完整示例见 [JSON 请求接口](AGENT-REQUESTS.md)；客户端连接见 [MCP 接入](MCP.md)。

可调用操作为 `capabilities`、`config_show`、`config_check`、`catalog`、`doctor`、`plan`、`build`、`report_read`。安装、自更新、Skill 导出仍是独立的人类 CLI 命令，不是 Agent API 操作。Agent 操作不发起后台更新提示。

先用 `config_show` 确认实际协议项目、业务服和计算服目录，再用 `catalog` 查询 ID、`plan` 核对范围，按用户已有授权执行 `build`。构建必须在 `params.options.repo_root` 显式提供绝对协议项目根目录。根目录通常包含 `proto/`，与两个 Go 工程根目录不同。

`plan` 只扫描范围，不更新 SVN、不编译、不写入生成物，也不锁定后续构建输入。构建仍按有效配置更新 SVN；配置、工具链、生成规则、备份和事务保护与普通 CLI 共用。Agent 不会静默关闭 SVN、开启强制覆盖或扩大编译范围。

## 配置来源

`config_show` 始终返回 `result.settings`、`result.project_root` 和 `result.explanation`，不需要额外 explain 参数：

- `explanation.fields` 按字段索引，每项含有效 `value`、最终 `source` 和覆盖 `history`，相同值的显式覆盖也会记录。
- 路径字段包含 `resolved_path` 与 `directory_exists`。项目配置中的 `project_root` 可能为 `.`，实际目标应看解析后的绝对路径。
- 请求字段来源使用 `kind: "request"`，名称如 `params.options.repo_root`、`params.options.docs`。普通 CLI 则保留 `kind: "cli"` 及真实参数名。
- `team_environment` 说明本工具支持的环境变量识别和首次使用问题，不导出整个进程环境。兼容的业务服变量按真实的 `ENV_PROJECT` 名字记录。

配置依次合并默认值、用户/项目配置、本机成功记录、团队环境变量、`PROTO_BUILDER_*` 环境变量、显式请求选项。显式配置和路径解析例外见 [使用说明](USAGE.md)。JSON 中省略字段表示继承，数组 `[]` 清空已有筛选，`false` 明确关闭；不要传 `null`。

## 响应协议 v2

JSON 请求入口的 stdout 是一个 JSON 对象，以换行结束；stderr 是进度日志，两者不能混合解析。MCP 使用相同业务响应，放在工具结果的 `structuredContent` 与 JSON 文本 `content` 内。

| 字段 | 含义 |
| --- | --- |
| `api`、`api_version` | 固定为 `proto-builder.agent`、`2`，用于识别接口 |
| `request_id`、`operation` | 请求关联 ID 与操作；无法识别请求时可为空 |
| `tool.name`、`tool.version` | 工具身份与软件版本 |
| `run.id`、`run.duration_seconds` | 本次运行标识与耗时 |
| `status`、`exit_code` | success / blocked / failed / cancelled；进程入口的退出码与响应一致 |
| `context` | 完整上下文，包括有效根目录、配置来源、selection、policy、可用的 SVN 摘要等 |
| `result` | 操作结果；计划和构建使用计数摘要，目录与报告使用分页结果 |
| `output.state` | not_written / writing / committed / rolled_back / recovering / recovered / recovery_required |
| `output.changed` | 生成物是否变更：true / false / null（无法确认） |
| `output.recovered_previous_transaction` | 是否处理过先前中断的事务 |
| `error` | 成功时为空，失败时为结构化错误 |
| `diagnostics.items`、`total`、`truncated` | 最多 20 条源文件诊断、总数及是否截断 |
| `warnings.count` | 完整报告中的警告数 |
| `report.path`、`report.error` | 完整报告路径；报告落盘失败时路径为空并说明原因 |

计划的 `result.counts` 包含 scanned、selected、documentation_proto_files、groups、excluded、filtered、import_only，另有 `result.full_build`。构建的 `result.counts` 包含 proto_files、documentation_proto_files、groups、commands；`result.changes` 包含 added、modified、removed、unchanged、adopted 计数，`result.backup` 指向备份。接管是修改的子集，不能重复相加。

`context.policy.protobuf_overwrite: "generated_with_backup"` 表示 `.pb.go`（含 gRPC）不同内容默认先备份后覆盖。HTML、Markdown、JSON 和被修改的过期产物仍有保护。`force` / `adopt_existing` 放在构建请求的 `params.policy`，不属于持久配置，也不能用于其他操作。

详细文件、哈希、事件和完整诊断保存在 `report.path` 对应的内部报告，按 [report_read](AGENT-REQUESTS.md#报告分页) 读取所需部分。内部报告继续使用 `schema_version: 1`；不要将它、旧 Agent 摘要或网页响应当作 v2 响应。

### 错误与写回状态

错误提供稳定的 `code`、`kind`、`retryable`、`suggested_action`，以及 `files` / `file_count`、`conflicts` / `conflict_count`。原 protoc 诊断保留原文件、行和列。并行构建的外层错误不会隐藏底层取消、超时或协议错误。不要按本地化 `message` 文本判断错误类别。

```json
{
  "code": "E_OUTPUT_CONFLICT",
  "kind": "unowned_output",
  "retryable": false,
  "suggested_action": "review_existing_outputs",
  "message": "已有文档尚未接管，写回已中止。",
  "files": ["D:/verified/protocol-project/proto/protocol.html"],
  "file_count": 1,
  "conflicts": [{"file": "D:/verified/protocol-project/proto/protocol.html", "kind": "unowned_output"}],
  "conflict_count": 1
}
```

内联文件、冲突、诊断各最多 20 项；单条错误和诊断消息最多 1200 个 Unicode 字符，截断时追加省略号。完整证据保留在内部报告。`retryable` 表示条件解除后可重试，不是要求立即循环；处理建议也不代表用户已经授权覆盖或修改配置。

**失败、取消或客户端超时不等于生成物未变更。** `committed` 表示生成物事务已完成；报告保存失败也保留此状态。若构建成功后报告落盘失败，返回 `E_REPORT` 和退出码 1，同时保留结果、实际写回状态和 `report.error`。若构建本身失败，优先保留原错误。

`rolled_back` 表示本次失败写回已恢复，`recovery_required` 表示恢复未完成，应保留备份和事务日志。处理过先前中断事务时，`output.changed` 保守返回 null；不能解释为未修改。该字段只描述生成物，不代表没有 SVN 更新、报告、缓存或本机档案写入。

## 导出 Codex Skill

```powershell
proto-builder skill --output "$env:USERPROFILE\.agents\skills\proto-builder"
```

该命令原样导出内置单文件 Skill，不依赖协议项目，不覆盖已有文件，也不更改客户端配置。源码见 [.agents/skills/proto-builder/SKILL.md](../.agents/skills/proto-builder/SKILL.md)。Skill 覆盖项目识别、能力发现、预览、构建、错误处理与汇报；已有用户授权可沿用，无需每次构建重复确认。

本工具不提供任意 shell、SVN 提交、GitLab 分支操作或远程控制。旧 `--agent` 始终被拒绝：普通 CLI 入口返回解析错误，已进入 agent 入口的错误使用 v2 JSON。调用方应直接迁移 v2，不能自动尝试旧入口。

## 实现边界

`api_contract.rs` 定义类型化请求、操作注册表和公开参数 Schema；`agent_api.rs` 管理校验、运行及报告生命周期；`service.rs` 直接调用配置、扫描和构建能力。JSON CLI 与 MCP 只负责传输，不再将请求转换为人类 CLI 命令。

原生 `ConfigOverrides` 区分省略与显式覆盖，`BuildPolicy` 单独表达本次构建策略。`failure.rs` 提供各入口共用的错误分类；`agent.rs` 将完整运行记录转换为 v2 响应。普通 CLI、网页和 Agent 仍使用同一套生成及事务实现，避免不同入口产生不同文件。
