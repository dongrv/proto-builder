---
name: proto-builder
description: Use the proto-builder Rust CLI to inspect protobuf build configuration, preview selected activity or machine protocols, generate Go code and shared documentation, and diagnose build failures. Use for this tool's protobuf workflows, not unrelated Go builds or source control operations.
---

Use the installed `proto-builder` executable. All execution is native; no BAT, Python, browser service or MCP server is required.

## Establish the target

For a new project or unclear configuration, run:

```powershell
proto-builder config show --explain --agent
```

Read `result.explanation.fields`: each field has its effective value, source and override history; project paths also have a resolved absolute path and directory-existence result. The protocol project root usually contains `proto/`; it is separate from the server and calculation server Go roots. `ENV_PROTOBUF` supplies the protocol project; `ENV_CENTER_PROJECT` supplies the server, with `ENV_PROJECT` used only if the former is unset or empty. `ENV_CAL_CENTER_PROJECT` supplies the calculation server and `ENV_DEVELOPER_NAME` supplies the name. Valid environment variables override remembered paths; explicit CLI arguments override environment variables.

Check the resolved project against the user's intended project. Prefer passing the verified root explicitly with `--repo-root` on subsequent commands so a recent project or different launch directory cannot redirect the work. If the target is ambiguous, obtain the missing target information before building.

Review `result.explanation.team_environment` for detected variables and setup issues. Identical server and calculation server roots need checking against the intended project layout; do not silently substitute a guessed calculation server directory.

## Preview and execute

Use `--agent` on every invocation: it implies non-interactive JSON output. Commands require explicit subcommands; no wizard or browser is started. Example after resolving the project root:

```powershell
proto-builder plan --agent --repo-root 'D:\work\go\src\shengyou\docs\branches\beta' --activity-id 7001,7002
proto-builder build --agent --repo-root 'D:\work\go\src\shengyou\docs\branches\beta' --activity-id 7001,7002
```

The example root must be replaced with the verified project, and IDs must reflect the user's request. Do not change configured output roots or project configuration just to make a failed command succeed.

- `plan` scans and describes scope; it does not run SVN update, generate staged outputs or promise an exact deployment diff. After source edits or an SVN update, its earlier counts may change.
- `doctor --agent` checks the environment when a build prerequisite needs diagnosis; it does not build.
- Empty activity or machine selection means all. Multiple IDs within a type form a union. Selecting both activity and machine types intersects their constraints; unrelated IDs can therefore match nothing. Shared protocols remain included.
- Go generation follows the selected scope. Shared HTML/Markdown and protocol ID JSON retain the full applicable protocol collection; explicit exclusions still apply. Local Go compilation must not silently narrow shared documentation.
- Builds update SVN by default when configured, preserve local modifications according to SVN behavior, and abort on conflicts. Do not add `--no-svn-update` in response to a failure unless the user's instructions authorize changing that behavior.
- Generated `.pb.go` files, including gRPC files, replace differing existing copies by default after backing them up, even when previously untracked or edited by hand. Other generated formats and modified stale files remain protected. Do not automatically retry their conflicts with `--force` or `--adopt-existing`; inspect the files and the user's authorized scope. Never repair `.pb.go` directly to hide a protobuf source error.
- Do not commit SVN changes or install/update the tool as part of a protocol build unless requested.

## Interpret results

Stdout is one JSON object, identified by `interface: "agent"` and `schema_version: 1`; stderr contains progress. Check both the process exit code and the JSON `exit_code`/`status`. Treat malformed or unsupported output as a tool-interface problem, not a successful build. Existing `--json` returns a different, complete report format.

Success returns build counts and change counts. `generated_files_changed` is `true` or `false` when known, or `null` when the tool cannot establish it. `output_state` distinguishes no write, writing, committed, rollback and recovery. A reporting failure can occur after outputs were committed: inspect these fields before deciding whether another build is needed.

Failures expose `error.code`, `kind`, `retryable`, `suggested_action`, paths and conflicts, plus original-file diagnostics. Suggestions explain a next step; they do not authorize destructive operations. `retryable` means retry after resolving the stated condition, not an instruction to loop.

Common decisions:

- `invalid_arguments`, `invalid_configuration`, `invalid_paths`: correct the identified argument or path; use configuration explanation when relevant.
- `no_matching_protocols`: inspect filters and the full plan evidence; do not silently switch to a full build.
- `invalid_protocols`: correct the original `.proto` at the reported file/line, then rebuild the intended scope.
- `svn_conflict`, `svn_failed`: inspect the SVN failure; avoid automatic revert, cleanup, conflict acceptance or disabling update.
- `unowned_output`, `modified_output`, `modified_stale_output`, `output_conflict`: inspect existing files and authorization before choosing adoption or replacement.
- `target_busy`: wait for the other build to finish before retrying; do not delete locks or kill another developer's process.
- `inputs_changed`: reload configuration and preview again. Make at most one automatic retry after confirming the intended target; report repeated concurrent changes.
- `output_transaction`, `report_storage`: inspect `output_state` and evidence first; preserve backups and recovery journals.

Inline diagnostic and conflict lists contain at most 20 items. The complete evidence, file lists, warnings and events are in `report_path`; read only needed sections when the count exceeds the inline list or a failure needs investigation. A null report path with `report_error` means storage failed. Do not claim unchanged outputs if `generated_files_changed` is null.

Report the verified project, selected scope, SVN outcome when available, success/failure, change counts, and actionable original-file errors. Link the complete report or backup when it helps review. A successful local build does not imply a release was published.
