# Codex 与其他 Agent 调用

本页描述 v1.2.0 新增的 Agent 接口。v1.1.0 及更早 EXE 不支持 `--agent`，请先更新工具。现有 `--json` 完整报告格式与普通 CLI 继续兼容。

## 开始调用

```powershell
proto-builder config show --explain --agent
proto-builder doctor --agent --repo-root D:\work\go\src\shengyou\docs\branches\beta
proto-builder plan --agent --repo-root D:\work\go\src\shengyou\docs\branches\beta --activity-id 7001,7002
proto-builder build --agent --repo-root D:\work\go\src\shengyou\docs\branches\beta --activity-id 7001,7002
```

`--agent` 隐含 `--non-interactive` 和机器输出，必须有明确子命令。禁止与键盘向导、网页工作台混用；未知子命令、错误参数以及不兼容选项均返回 JSON，不打开界面或等待输入。`--help --agent` 和 `--version --agent` 也返回 JSON。省略子命令会报错，避免 Agent 意外启动全量构建。

先确认最终协议项目根目录、业务服和计算服目录，再把核对后的根目录显式传给后续操作。根目录通常包含 `proto/`；业务服、计算服则是 Go 工程根目录。CLI、网页及 Agent 使用同一写回规则：`.pb.go` 默认备份后以本次生成结果覆盖，其他格式及已修改过期产物仍受保护。Agent 模式不会关闭 SVN 更新或自行启用额外的强制覆盖、接管选项。Agent 模式不发起后台版本提示，显式自更新命令仍可使用。

`plan` 仅扫描范围，不更新 SVN、不生成临时或正式产物；其输出不是已完成的编译差异，也不是后续写回的授权或固定执行计划。构建中的 SVN 更新、输入快照和事务保护继续生效。

## 配置解释

`config show --explain` 可与普通输出、`--json` 或 `--agent` 搭配。在原有配置结果上增加 `explanation`：

- `fields` 按字段名索引，例如 `server_path`、`developer_name`、`svn.update`。
- 每项含 `value`、最终 `source` 和按优先级排列的 `history`，同值的显式覆盖也有记录。
- 每个来源包含 `kind`、`name` 和该层生效的 `value`。`name` 为实际配置文件、环境变量或 CLI 参数名；业务服兼容变量按 `ENV_PROJECT` 原名记录。
- 四个项目路径字段另外返回 `resolved_path` 和 `directory_exists`，相对路径依照正常配置规则解析。
- `project_root` 的配置值可能是 `.`；应使用 `resolved_path` 或结果顶层 `project_root` 获取实际根目录。配置所在目录参与根目录解析时，来源标记为 `project_config_directory`。
- `team_environment` 包含环境变量识别、需核对的问题及本机成功记录是否匹配。环境变量和来源仅覆盖本工具支持的字段，不导出整个进程环境。

来源类型包括 `default`、`user_config`、`project_config`、`explicit_config`、`project_config_directory`、`local_setup`、`environment`、`working_directory`、`cli`。历史是实际应用层的记录，不通过比较最终值猜测来源。显式配置、本机档案及环境覆盖规则详见 [使用说明](USAGE.md)。

## 输出协议 v1

stdout 为单个 JSON 对象，以换行结束，不夹带彩色控制码、进度条或其他文本。stderr 输出普通进度；默认省略逐目录选择清单并限制编译进度打印频率，`--verbose` 可查看详细过程。完整事件始终保存在本机报告内。

独立的 `schema_version: 1` 与 `interface: "agent"` 标识 Agent 协议。正常完整报告也有自己的版本字段，不能仅凭 `schema_version` 判断是哪种格式。消费者应允许新增字段，对未知主版本明确报错。

| 字段 | 含义 |
| --- | --- |
| `tool`、`version`、`run_id`、`command` | 工具身份、软件版本、本次运行标识与主命令 |
| `status` | `success`、`blocked`、`failed` 或 `cancelled` |
| `exit_code` | 与实际进程退出码一致；成功为 0 |
| `duration_seconds` | 实际耗时 |
| `project_root`、`proto_root`、`server_root`、`calc_root` | 有效绝对路径；配置尚未加载或无项目命令时可能为空 |
| `selection` | 本次活动、机器、旧 ID 及排除条件；无项目命令时可能为空 |
| `policy` | 有效 SVN、文档、配置、同步、强制覆盖、接管和工具链策略 |
| `svn` | 构建取得结果后的更新摘要，包括前后版本号和更新路径数；失败或未执行时可能为空，不表示已更新 |
| `output_state` | 生成物写回状态，见下文 |
| `generated_files_changed` | `true` 有变更、`false` 无变更、`null` 无法确认 |
| `recovered_previous_transaction` | 本次是否处理过先前中断的写回事务 |
| `result` | 各命令的精简结果；配置查询保留设置及所请求的解释信息 |
| `error` | 成功为空，失败为结构化错误 |
| `diagnostics`、`diagnostic_count` | 前 20 条源文件诊断及总数 |
| `warning_count` | 完整报告的警告数 |
| `report_path`、`report_error` | 完整报告路径；报告保存失败时路径为空并给出原因 |

构建 `result` 包含协议数、文档协议数、分组数、命令数、变更计数及备份目录。`changes` 中的 `added`、`modified`、`removed`、`unchanged`、`adopted` 是数量，接管是修改的子集，不能重复相加当作文件总数。预览返回扫描、选中、排除、筛选跳过、依赖、分组和完整文档的数量。完整文件清单和哈希通过 `report_path` 读取。

`policy.protobuf_overwrite: "generated_with_backup"` 表示 `.pb.go` 默认覆盖规则，与 `force`、`adopt_existing` 是否为 true 无关。其他格式或已修改过期文件的冲突仍需单独处理，不应自动追加这两个选项。

### 错误

```json
{
  "code": "E_OUTPUT_CONFLICT",
  "kind": "unowned_output",
  "retryable": false,
  "suggested_action": "review_existing_outputs",
  "message": "本次输出存在尚未接管的文件，写回已中止。",
  "files": ["D:/server/protocol/act7001/act7001.pb.go"],
  "file_count": 1,
  "conflicts": [{"file": "D:/server/protocol/act7001/act7001.pb.go", "kind": "unowned_output"}],
  "conflict_count": 1
}
```

示例的 `message` 仅作说明；消费者不能依赖其语言和具体文字。错误类别和处理建议使用固定英文标识，文件冲突来自事务检查的结构化数据；原 protoc 诊断仍保留原文件、行和列。并行生成的 `E_BUILD` 包装不会遮蔽底层错误码；取消优先返回 `E_CANCELLED`。混合输出冲突使用 `output_conflict`，各文件仍保留自己的类别。

`files` 和 `conflicts` 各最多内联 20 项，并提供总数；错误及单条诊断消息最多 1200 个 Unicode 字符，截断时追加省略号。完整报告中的 `error`、`failure`、`diagnostics` 和 `events` 保留完整证据。没有可靠文件位置的错误可以返回空文件列表。处理建议不代表用户已授权覆盖、接管或修改配置。

`retryable: true` 只用于目标被占用、输入或配置变化等可在条件解除后重试的情况；不意味着立即重复执行。协议、配置、SVN 冲突、输出冲突、超时等应先检查原因。退出码仍沿用 [使用说明](USAGE.md) 的约定。

### 写回与报告失败

`output_state` 可能为：

- `not_written`：尚未写回生成物。
- `writing`：开始写回，未取得明确完成或恢复结果。
- `committed`：已提交生成物事务，即使后续清理或报告失败也保留此状态。
- `rolled_back`：本次失败的事务已恢复原生成物。
- `recovering`、`recovered`：正在恢复或已恢复先前中断的事务。
- `recovery_required`：回滚未完成，需保留备份和事务日志并排查。

处理过先前中断事务时，工具不假定启动时的生成物状态，`generated_files_changed` 保守返回 `null`。不能将未知状态解释为未修改。

报告保存失败仍保证输出 Agent JSON：若构建本身成功则返回 `E_REPORT` 和退出码 1，并保留实际写回状态与结果；若构建本身失败则保留原错误，同时通过 `report_error` 说明报告问题。不要因 `status: failed` 就断言生成物未改变或直接重复执行。

Agent 接口只记录生成物写回；`generated_files_changed: false` 不代表没有更新 SVN、写入本机报告、准备工具链缓存或执行显式安装命令。

## 导出 Codex Skill

```powershell
proto-builder skill --output "$env:USERPROFILE\.agents\skills\proto-builder"
```

这个命令把内置 `SKILL.md` 原样导出到指定目录，不依赖当前协议项目或脚本运行环境，不覆盖已有 `SKILL.md`，不会改动 Codex 全局配置。若需要先审查，导出到任意新的本机目录，再放入团队选定的 Skill 目录。

源码中的 Skill 位于 [.agents/skills/proto-builder/SKILL.md](../.agents/skills/proto-builder/SKILL.md)。它定义项目识别、范围预览、构建、失败处理和结果汇报规则；允许沿用已有用户授权，不要求每次构建重复确认。能力边界仍是当前项目的协议工具，不包含 SVN 提交或 GitLab 工作流。
