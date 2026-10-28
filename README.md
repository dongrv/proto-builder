# Proto Builder

面向当前 SVN 项目的 protobuf → Go 开发工具，支持 **Windows x86_64**。一个 EXE 即可使用，内置固定版本的 protoc 和插件，无需 BAT、Python、Go 或 Rust 运行环境。

- **网页工作台**：鼠标选择目录、活动 / 机器 ID、排除项，预览计划、编译、查看日志与保存配置。
- **终端与命令行**：键盘向导不捕获鼠标；命令行适合快速执行和重复操作。
- **可靠生成**：失败不部署，保护已有文件，提供备份、恢复及明确的文件 / 行号诊断。

## 安装

从 [v1.1.0 Release](https://github.com/dongrv/proto-builder/releases/tag/v1.1.0) 下载 Windows EXE，在 PowerShell 执行：

```powershell
.\proto-builder-1.1.0-windows-x86_64.exe install
```

默认安装到 `%LOCALAPPDATA%\ProtoBuilder\bin` 并添加用户 PATH。**新开终端**后即可使用，也可直接运行下载的 EXE。

## 开始使用

在项目根目录执行，推荐从网页工作台开始：

```powershell
proto-builder web       # 自动打开本地网页，用鼠标设置和执行
proto-builder           # 键盘交互向导
proto-builder doctor    # 检查环境
proto-builder plan      # 只读预览
proto-builder build     # 开始编译
```

从其他目录启动时，使用 `--repo-root D:\work\project` 指定项目。网页资源内置 EXE，只监听本机；关闭标签页不会停止任务，请点击「关闭工作台」或在启动终端按 Ctrl+C。

```powershell
proto-builder build --activity-id 7034,7035
proto-builder build --machine-id 6034
proto-builder build --activity-id 7034 --exclude experimental --exclude 'draft/**'
proto-builder build --no-svn-update
```

**构建前默认更新项目范围内的 SVN，需要已安装 `svn.exe`。** 可通过配置或 `--no-svn-update` 关闭；`plan` 始终只读，不更新 SVN、不生成文件。新工具运行不调用旧脚本。共享 HTML / Markdown 始终按完整非 RPC 协议集生成，不受 ID 筛选影响，显式排除项仍生效。

## 配置与更新

```powershell
proto-builder config init       # 创建项目配置
proto-builder config check      # 校验配置
proto-builder self-update check # 检查官方更新
proto-builder self-update apply # 下载、验签并更新
```

优先识别团队的 `ENV_CENTER_PROJECT`、`ENV_CAL_CENTER_PROJECT`、`ENV_PROTOBUF`、`ENV_DEVELOPER_NAME`。缺失时提示核对，首次编译成功后记住路径和名字，后续直接复用。配置示例见 [proto-builder.toml.example](proto-builder.toml.example)。网页也可预览配置差异并保存，自动更新仅提示，显式执行 `apply` 才替换程序。更多参数使用 `proto-builder --help`。

## 详细文档

- [使用与配置详解](docs/USAGE.md)：网页操作、筛选规则、配置优先级、生成物保护、更新与卸载。
- [SVN 日常操作](docs/SVN-WORKFLOW.md) · [旧工具弃用说明](docs/LEGACY-DEPRECATED.md)
- [构建、签名与发布](docs/RELEASING.md) · [验收记录](docs/VALIDATION.md) · [第三方许可](docs/THIRD-PARTY-NOTICES.md)
