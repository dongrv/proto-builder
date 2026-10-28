# Proto Builder v1.2.0

本次增加 Codex / Agent 调用能力，完善团队环境识别，并修复局部编译覆盖完整协议文档及 Windows 长路径问题。支持 Windows x86_64；仍以一个 Rust EXE 提供工具链、网页和终端能力，无 BAT、Python 或前端运行依赖。

## 主要变化

- **Agent 调用**：`--agent` 提供版本化 JSON 摘要和非交互执行，进度输出到 stderr。错误包含类别、建议动作、原文件及行列、冲突明细，以及真实的生成物写回状态；报告保存失败也会返回 JSON。
- **配置来源**：`config show --explain` 展示有效配置、绝对路径、来源和覆盖历史。优先识别团队环境变量，兼容业务服变量 `ENV_PROJECT`；首次编译成功后记住本机项目路径与开发者信息。
- **内置 Skill**：`skill --output <目录>` 导出 Codex 调用说明，拒绝覆盖已有文件；Skill 不依赖脚本或 MCP 服务。
- **共享文档完整性**：选择活动、机器或旧 ID 时，Go 按选择范围生成，HTML/Markdown 仍包含完整非 RPC 协议集。显式排除项继续生效。
- **网页反馈**：完成后显示明显的常驻成功提示、耗时与真实变更数量；协议项目根目录和环境变量识别说明更清楚。
- **默认更新生成 Go**：`.pb.go`（含 gRPC）与本次生成结果不同时，默认备份后覆盖，无需勾选接管或强制覆盖；网页两项选项尾部增加可展开的问号，说明其他生成物和过期文件的处理步骤。
- **诊断与兼容**：活动和机器筛选无交集时解释匹配数量与调整方式；支持较长的工具数据目录、工具链执行路径及 Skill 导出目录。

默认构建前更新 SVN；`plan` 始终只读。其他格式的已有文件保护、备份、事务恢复和固定工具链保持兼容。重复编译使用相同输出路径，不会自动生成 v2/v3 文件；仅清理本次范围内已登记的过期产物。

## 安装与更新

```powershell
.\proto-builder-1.2.0-windows-x86_64.exe install
# 已安装旧版本：
proto-builder self-update check
proto-builder self-update apply
```

## Agent 示例

```powershell
proto-builder config show --explain --agent
proto-builder plan --activity-id 7001,7002 --agent
proto-builder build --activity-id 7001,7002 --agent
proto-builder skill --output "$env:USERPROFILE\.agents\skills\proto-builder"
```

Agent 必须显式指定子命令。先核对配置中的项目及输出路径；Agent 使用同一套 `.pb.go` 默认覆盖规则，不自动关闭 SVN 或额外启用 `--force`、`--adopt-existing`。已有 Skill 需要先自行保留或选择新的导出目录。

## 验证与附件

46 项 Rust 测试和 19 个直接 EXE 场景通过，覆盖真实内置插件编译、重复构建、默认覆盖及备份、过期产物清理、其他格式保护、原文件诊断、报告失败、长路径、Skill 导出及本地 SVN 更新后编译。此前已完成真实项目副本的全量与局部文档字节对比，详细记录见 [验收记录](https://github.com/dongrv/proto-builder/blob/v1.2.0/docs/VALIDATION.md)。

日常使用只需 Windows EXE。官方发布使用原有 Ed25519 身份签名；源码包包含 `.agents/skills/proto-builder/SKILL.md` 和网页资源，文档包保留 `docs/` 结构。内部 baseline、私钥、运行日志及项目产物均不进入发布附件。
