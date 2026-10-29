# 构建、签名与 GitHub 发布

日常用户只安装 `proto-builder.exe`。`src/bin/release.rs` 是维护者使用的 Rust 打包程序，不是产品的运行依赖，不需要发布它，也没有发布脚本或 CI 依赖。

`web/` 是网页源文件，构建时通过 `include_str!` 内置产品 EXE；用户无需另行下载网页目录或安装前端依赖。

`.agents/skills/proto-builder/SKILL.md` 是内置 Agent 调用说明，编译 EXE 时需要保留在源码包内；用户可用 `skill --output` 从同一 EXE 导出，无额外运行依赖。Agent 接口协议与软件版本分别管理，规则见 [Agent 接口](AGENT-INTERFACE.md)。

## 发布身份

官方更新公钥固定在 `assets/update-public-key.hex`，产品编译时内置。已按维护者授权在本机生成 Ed25519 发布密钥，私钥位于仓库外：

```text
%LOCALAPPDATA%\ProtoBuilder\release-keys\official\signing-key.dpapi
```

私钥使用 Windows 当前用户 DPAPI 加密，不能提交到 SVN/Git 或上传到 Releases。解密依赖当前 Windows 账户的 DPAPI 环境，单独复制加密文件到其他账户或机器不等于可用备份。发布维护者应保留可恢复的账户和密钥备份；当前公钥不可随意重新生成，否则已安装程序无法信任新签名。

只有初始化新的独立发布身份时才使用：

```powershell
cargo run --locked --bin release -- keygen --directory D:\Private\new-release-identity
```

命令拒绝覆盖已有密钥目录。现有官方身份已经生成，无需再次执行。产品信任锚轮换需要单独设计和迁移，不应替换密钥后继续使用原更新链路。

## 构建与打包

从仓库根目录执行，先更新 Cargo.toml 的产品版本、检查 Cargo.lock 和依赖许可，再完成校验：

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bin proto-builder

cargo run --locked --bin release -- pack --binary target/release/proto-builder.exe --key "$env:LOCALAPPDATA\ProtoBuilder\release-keys\official\signing-key.dpapi" --output dist/v1.2.0 --version 1.2.0 --channel stable
```

打包器检查产品版本、Windows x86_64 PE、产品内置公钥是否与签名私钥一致，拒绝覆盖已存在的签名产物。输出：

- `proto-builder-1.2.0-windows-x86_64.exe`
- `release.json`：仓库、版本、渠道、平台、文件名、大小、SHA256
- `release.json.sig`：对 release.json 原始字节的 Ed25519 签名（十六进制）
- `SHA256SUMS`
- `public-key.hex`：便于人工核对；更新校验只信任程序内置公钥

另外提供 README、配置示例、包含 docs/ 的文档压缩包和带提交标识及 SHA256 的源码快照；完整说明和第三方许可位于 docs/，许可文本也已内置 EXE。打包过程不调用旧协议脚本，内部基准不进入包。

### 1.3.0-dev.1 预发布打包

检出本次发布源码，确认 Cargo.toml / Cargo.lock 的产品版本为 `1.3.0-dev.1`，完成上述检查及优化构建。维护者为每次打包选择新的空目录，例如：

```powershell
$releaseOutput = Join-Path $PWD ('dist/publish-v1.3.0-dev.1-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
cargo run --locked --bin release -- pack --binary target/release/proto-builder.exe --key "$env:LOCALAPPDATA\ProtoBuilder\release-keys\official\signing-key.dpapi" --output $releaseOutput --version 1.3.0-dev.1 --channel beta
```

这会生成 `proto-builder-1.3.0-dev.1-windows-x86_64.exe` 及对应签名资源，清单渠道为 `beta`。不要直接复用已有本地开发包目录，也不要覆盖已签名或已发布的附件。补充文档和源码附件后，重新生成覆盖全部附件的 SHA256SUMS；签名清单及其签名字节保持原样。

创建 `v1.3.0-dev.1` 标签和 GitHub Release 时标记 **prerelease**，保留 v1.2.0 为 Latest stable。版本说明使用 [RELEASE-1.3.0-dev.1.md](RELEASE-1.3.0-dev.1.md)，明确旧 `--agent` 接口已删除。默认 stable 更新不会选择本预发布版；用户主动安装对应 EXE，或执行 `self-update apply --channel beta` 选择最新预发布更新。

发布后核对远程标签对应的提交、下载附件的大小和哈希，并从匿名下载的原始清单验证签名。线上升级和回退的实测结果应单独记录；本地构建、签名或 MCP 子进程测试不能代替这些结果。

## 正式发布

本地打包不自动向 GitHub 写入。确认候选版本后，在 `dongrv/proto-builder` 创建与版本一致的 `v1.2.0` Release，将上述原始文件作为附件发布，并填写变更说明。**不要重新格式化 release.json、改变换行或替换同版本附件**，否则签名或哈希会失效。修复必须发布新版本。beta 使用 `1.3.0-beta.1` 等 SemVer，并将 GitHub Release 标记为 prerelease，清单渠道使用 beta。

在线更新验收使用独立用户数据目录及已安装的旧版本：执行 `self-update check`、`self-update apply`，确认新版本可启动，再执行 `self-update rollback` 确认旧版本恢复。首次 1.0.0 已使用隔离的本地测试旧版本 `0.0.0-acceptance` 完成官方线上下载、验签、替换和逐字节回退。后续每个正式版本仍应使用上一正式版本回归这一步。

官方 stable 更新发现优先 GitHub API，API 不可用时读取 `/releases/latest/download/release.json` 和签名。可选镜像目录结构：

```text
<mirror>/latest/release.json
<mirror>/latest/release.json.sig
<mirror>/beta/release.json
<mirror>/beta/release.json.sig
<mirror>/v1.0.0/release.json
<mirror>/v1.0.0/release.json.sig
<mirror>/v1.0.0/proto-builder-1.0.0-windows-x86_64.exe
```

镜像必须使用 HTTPS。渠道清单与对应版本目录的文件应来自相同已签名发布；客户端不会允许镜像跳过签名。当前网络的 GitHub API 可能触发限流，不能把请求失败解释成“仓库没有版本”。

## 发布前边界

内部字节基准覆盖当前全量及典型 ID 样本；输入或插件发生变化时重新采样和对比。已固定插件版本不能静默升级，否则可能改变 Go 或文档字节。源程序不需要根目录 bin 或旧脚本，复制本目录即可独立构建。
