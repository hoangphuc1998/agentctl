# Session Progress Log

## Current State

**Last Updated:** 2026-07-25 23:13 +07
**Session ID:** direct-folder-agent-sessions
**Active Feature:** feat-061 - Direct Folder Agent Sessions

## Status

### What is Done

- [x] Passed the baseline startup workflow before implementation.
- [x] Added a separate folder-session registry table while preserving the CLI-compatible worktree `runs` table.
- [x] Added direct Codex and Claude launch, restore, observation, stop, end, and editor-open behavior for existing folders.
- [x] Kept Git diff, merge, worktree deletion, branch deletion, and file deletion out of folder-session flows.
- [x] Added a Worktree/Folder selector to the existing New Run modal with recent-folder suggestions and contextual defaults.
- [x] Supported multiple named sessions in one folder and assigned Codex provider threads one-to-one.
- [x] Kept direct folder sessions out of the Mobile Bridge dashboard, resume route, and terminal stream.
- [x] Documented the desktop folder workflow and completed focused and full verification.

### What is In Progress

- [x] Implementation, tests, documentation, verification, and continuity artifacts are complete.

### What is Next

1. Rebuild/relaunch Agent Manager to use direct folder sessions in the installed desktop application.
2. Create multiple named sessions against one folder to validate the desired real-world Codex/Claude workflow.

## Blockers / Risks

- [x] No code blockers.
- [ ] `npm install` reports 6 existing audit findings (3 moderate, 2 high, 1 critical); dependency remediation was outside this feature.
- [ ] The currently installed desktop package was not rebuilt during this source change.

## Decisions Made

- Store direct folders separately so older CLI/mobile code continues to see only Git worktree runs.
- Reuse the existing run model at desktop boundaries with an explicit workspace kind and workspace path.
- Allow several sessions in the same folder; preserve exact Codex thread IDs first, then assign unclaimed matching threads newest-first.
- Treat folders as desktop-only and reject direct folder IDs even if a mobile caller guesses one.
- End means kill and forget the folder session record from active views, never delete its folder or files.

## Files Modified This Session

- `core/src/{domain,registry,app}.rs` for workspace typing, folder persistence, orchestration, and safe lifecycle behavior.
- `src-tauri/src/{commands,models,services,tmux_restore,mobile_bridge_server}.rs` for desktop exposure, grouping, status matching, restart restore, and mobile exclusion.
- `src/{App,api,types}.ts*` and `src/components/*` for the direct-folder creation and management UI.
- Rust and React fixtures/tests for folder persistence, lifecycle safety, shared-folder Codex matching, UI creation, and hidden Git actions.
- `README.md`, `feature_list.json`, and `progress.md` for usage and completion evidence.

## Evidence of Completion

- [x] Folder lifecycle coverage proves launch/editor/end issue no Git commands, merge is rejected, and an existing file remains unchanged.
- [x] Migration coverage proves folder sessions persist outside the legacy `runs` table and remain available to the desktop session query.
- [x] Desktop-state coverage proves two Codex sessions in one folder claim distinct provider threads.
- [x] React coverage proves folder-mode payloads and folder-safe controls without Diff or Merge.
- [x] `npm test` passed with 12 files and 87 tests; `npm run build` passed.
- [x] `cargo test --workspace`, `cargo check -p agent-manager-desktop --features tauri-app`, `cargo fmt --all -- --check`, and `git diff --check` passed.
- [x] Final `./init.sh` passed with 12 Vitest files/87 tests, npm build, 40 core tests, and all desktop Rust tests.
