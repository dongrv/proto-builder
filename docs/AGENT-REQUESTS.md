# Agent JSON 请求 v2

预发布版 `1.3.0-dev.1` 的机器接口独立于人类命令行参数。`agent capabilities` 返回实际支持的操作、请求和响应 Schema、限制与副作用；以运行中的 EXE 为准。

```powershell
proto-builder agent capabilities
proto-builder agent run --request D:\Temp\proto-plan.json
```

请求文件是 UTF-8 JSON，支持 UTF-8 BOM，最大 1 MiB。也可向 `proto-builder agent run --request -` 管道写入 UTF-8 JSON 并关闭 stdin；不要使用交互终端输入或混入其他文本。文件方式可避免 PowerShell 管道编码差异。

## 请求结构

```json
{
  "api_version": 2,
  "request_id": "preview-7001-7002",
  "operation": "plan",
  "params": {
    "options": {
      "repo_root": "D:/verified/protocol-project",
      "activity_ids": ["7001", "7002"],
      "machine_ids": []
    }
  }
}
```

替换示例路径为已核对的实际协议项目根目录。`api_version`、`operation`、`params` 必填；`params` 必须是对象，能力查询也要传 `{}`。可选 `request_id` 最多 128 个 Unicode 字符，仅用于关联，不是幂等键。

旧 `schema_version: 1` 请求、顶层 options、缺失或非对象 params、未知字段、类型错误和 null 均明确拒绝，不降级执行。旧 `--agent` 已移除。`agent run` 与 `agent capabilities` 不接受额外全局设置，包括 `--json`、`--non-interactive`、`--repo-root`、`--force`；所有业务入参放在 params 中。

## 操作与参数

| operation | params |
| --- | --- |
| capabilities | `{}`；无需项目，不创建日志 |
| config_show | 可选 options、verbose；返回有效配置、路径及来源解释 |
| config_check | 可选 options、verbose；校验配置 |
| doctor | 可选 options、verbose；检查环境、工具链和输出布局 |
| plan | 可选 options、verbose；只读预览编译范围 |
| build | options 必须含绝对 repo_root；可选 policy、verbose |
| catalog | 可选 options、verbose、query、offset、limit；分页查询协议 |
| report_read | 必填 path；可选 section、offset、limit；无需加载项目 |

`verbose` 是操作级日志设置，位于 params，默认 false；不写入项目配置。API 不包含安装、更新、Skill 导出或任意命令执行。

### options

| 字段 | 类型及作用 |
| --- | --- |
| config | 配置文件路径 |
| repo_root | 协议项目根目录；build 必须显式绝对路径 |
| proto_path、server_path、calc_path | Proto 源目录、业务服、计算服目录 |
| developer_name | 开发者名字 |
| activity_ids、machine_ids、excludes | 字符串数组，整体替换继承值 |
| assign_id | 原项目兼容 ID 规则的非负整数 |
| docs、protocol_config、copy_to_calc、svn_update | 显式布尔开关 |
| jobs | 0–256；0 使用自动并行度 |
| timeout_seconds | 单条底层命令时限，正整数 |
| lang | zh-CN 或 en-US |

省略字段继承配置；`[]` 明确清空筛选，`false` 明确关闭。`null` 不表示继承，数字 ID 必须写成字符串，例如 `"7001"`。每个筛选数组最多 1000 项，条目不能为空字符串；直接使用 `[]` 清空。相对输出路径按最终项目根目录解析，config 相对于进程工作目录解析；Agent 应尽量传核对后的绝对路径。

多个同类 ID 取并集，活动和机器条件同时选择时取交集，公共协议保留。排除项优先于 ID 筛选。选定 ID 无匹配仍返回 `E_NO_MATCH`，不会退回全量。局部 Go 构建不缩减共享文档及协议配置的完整适用集合。

### build 的独立策略

```json
{
  "api_version": 2,
  "request_id": "build-7001",
  "operation": "build",
  "params": {
    "options": {
      "repo_root": "D:/verified/protocol-project",
      "activity_ids": ["7001"]
    },
    "policy": {"adopt_existing": false, "force": false},
    "verbose": false
  }
}
```

policy 两个字段默认 false，仅在本次构建生效。`.pb.go` 内容不同时默认备份后覆盖，不需要开启它们。`adopt_existing` 用于接管其他已有且不同的生成物，`force` 用于替换已修改的受管理生成物或清理已修改过期产物；失败后不要自动打开。传在 options 或非 build 操作中会被拒绝。

构建默认沿用 SVN 更新策略；只有用户授权改变时才设置 `options.svn_update`。请求不会自动提交源码、切换分支或修改持久项目配置。`plan` 是范围预览，不是锁定的执行计划或后续写回授权。

## 协议目录查询

```json
{
  "api_version": 2,
  "operation": "catalog",
  "params": {
    "options": {"repo_root": "D:/verified/protocol-project"},
    "query": "7001",
    "offset": 0,
    "limit": 20
  }
}
```

query 按文件、目录及 ID 元数据匹配，最多 256 个 Unicode 字符，只影响展示，不改变真实选择范围。响应包括 `result.items`、`total`、`next_offset`，以及 selected_total、documentation_proto_files、分类计数和 ID/目录 facets。facets 最多返回 500 项，并提供 total/truncated。

目录查询遵循 options 与继承的筛选。如果有意查询全部，明确把 activity_ids、machine_ids、excludes 清空。文本搜索无结果返回空页；有效 ID 筛选无匹配仍报错。

## 报告分页

计划或构建返回完整报告的 `report.path`。以此真实路径发起查询，不要猜测日志目录或运行编号：

```json
{
  "api_version": 2,
  "request_id": "events-page-1",
  "operation": "report_read",
  "params": {
    "path": "C:/verified/path/from-response.json",
    "section": "events",
    "offset": 0,
    "limit": 20
  }
}
```

path 指向本工具完整内部报告（`schema_version: 1`），最大 32 MiB。Agent 摘要不是报告文件。section 可为 summary（默认）、events、diagnostics、selected、excluded、filtered、changes、outputs、import_only、documentation_inputs。

目录和报告的分页均为 offset ≥ 0、limit 1–500，默认 100。`next_offset: null` 表示结束，超界 offset 返回空页。报告页的 `result.source` 保留原运行 ID、状态、写回状态和 SHA256；跨页哈希变化时重新读取，不能拼接不同快照。历史构建失败不影响读取操作自身成功。

报告查询及能力发现不创建或轮转日志；错误查询也不会淘汰源报告。普通构建的报告保留策略仍适用，长期证据需另行保存。

默认 summary 页的错误文字最多 1200 个字符，截断时追加省略号并设置 `error_truncated`；failure 中的 files、conflicts 各最多 20 项，附带 `files_total` / `conflicts_total` 和对应的 `*_truncated`。完整证据仍保存在源报告。events、diagnostics 等详细页按条数分页，保留每条原始内容，limit 不代表字符或字节上限。

## 结果处理

stdout 是 [Agent v2 响应](AGENT-INTERFACE.md#响应协议-v2)，stderr 是进度。检查 status、exit_code 和 output；尤其不要把失败、取消或断线等同于未写回。`output.changed` 可为 null，报告失败时仍可能 `output.state: "committed"`。

同一 request_id 再次构建仍会再次执行，包括 SVN 更新。先查看结果及报告，再决定是否重试。MCP 使用同一 params 模型，但不要在工具 arguments 中传 api_version、operation 或 request_id，详见 [MCP](MCP.md)。
