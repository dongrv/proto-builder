# 旧协议构建工具已弃用

新工具入口统一为 `proto-builder`。项目根目录 `BuildProto2.bat`、`bin/build_proto.py`、`bin/export_cmd.py` 等原文件保持原样，暂不删除；仅供内部基准采样及迁移初期对比，不再属于日常构建流程。新工具没有 BAT、Shell 或 Python 包装入口。

| 原用途 | 新命令 |
| --- | --- |
| 全量构建 | `proto-builder build` |
| 原 assignID 筛选 | `proto-builder build --assign-id 7034` |
| 按活动或机器选择 | `proto-builder build --activity-id 7034` / `--machine-id 6034` |
| 查看计划 | `proto-builder plan` |
| 检查环境 | `proto-builder doctor` |
| 设置路径、筛选和开关 | `proto-builder` 交互向导或 TOML |
| 内部结果对比 | `proto-builder compat --baseline DIR --actual DIR` |

原始脚本不会被新工具调用。内部基准不进入发行包，也不增加日常用户维护负担。
