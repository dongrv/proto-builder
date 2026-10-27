# 当前项目的 SVN 使用方式

本工具服务于当前 SVN 项目，不需要 GitLab、Git 分支检测或 CI 接入。

1. 切换工作副本或 SVN 分支后，指定正确的项目根目录、业务服和计算服目录。可为各分支分别保存 TOML。
2. `proto-builder doctor` 检查环境，`proto-builder plan` 预览筛选；这两个命令不执行 SVN 更新。
3. `proto-builder build` 默认先对当前项目根目录执行一次 `svn update --non-interactive --accept postpone`。`[svn] update_paths = ["proto"]` 可以限定更新范围，路径必须在项目根目录内。空列表更新项目根目录，不会自动向上更新整个 SVN 工作副本。
4. 更新前已有冲突、更新失败或更新后产生文本、属性、树冲突，立即中止生成。普通本地修改遵循 SVN 正常合并行为。若 SVN 更新改变了构建配置，本次中止并提示重新执行，避免使用过时配置。
5. 如需离线构建或已经更新，可使用 `--no-svn-update`；也可用 `[svn] update = false` 保存偏好。`svn.exe` 不可用且更新开启时会明确失败，不会静默跳过。
6. 编译完成后自行检查 `svn status` 和产物差异，再按项目约定提交。工具不提交、不切分支、不 revert、不自动解决冲突，也不登记 SVN add/delete。

更新报告记录项目 URL、工作副本路径、revision，以及实际更新路径更新前后的快照。SVN 工作副本可能存在混合 revision，因此项目根目录的 revision 不代表所有子目录具有同一版本；应结合具体路径的记录理解。

首次使用新工具无需删除旧产物。完全一致的文件自动登记；不同的文件默认报冲突，使用 `--adopt-existing` 明确接管并备份。之后对已登记生成物的手动修改默认受保护，需要明确 `--force` 才覆盖。
