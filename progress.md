# Session Progress Log

## Current State

**Last Updated:** 2026-06-23 18:12 +07
**Session ID:** seen-completed-runs-and-need-input-notifications
**Active Feature:** feat-017 - Seen Completed Runs and Need Input Notifications

## Status

### What's Done

- [x] Completed the startup workflow in this worktree: confirmed the path, read project instructions/docs/feature tracker, reviewed recent commits, and ran `./init.sh`.
- [x] Confirmed this checkout is clean on branch `notification` before edits.
- [x] Reproduced the completed-run badge issue with a red App test: selecting a completed run only changed local selection and did not call `dashboard_state("run-2")` to clear the `Review` badge.
- [x] Reproduced the backend seen-state gap with red desktop service tests: selected completed runs were not marked `completed-seen`, and a seen completion could become `completed-unchecked` again on tmux refresh.
- [x] Reproduced the needs-input notification gap with a red core tmux test: visible `Need input` text was classified as `running`, so no `needs-user` transition or system notification could fire.
- [x] Implemented immediate backend refresh on run selection, selected completed-run seen persistence, completed-seen preservation across refresh, and literal need-input phrase detection.

### What's In Progress

- [x] Seen completed runs and need-input notification detection are implemented and verified.

### What's Next

1. Next session can run `./init.sh` immediately from this worktree.
2. Rebuild/reinstall the desktop package if you need this fix in an installed app rather than the dev checkout.

## Blockers / Risks

- [x] No known implementation blockers remain.
- [ ] The full Tauri shell was not manually launched against a live tmux status transition; regression coverage exercises selection refresh, backend seen-state behavior, Tauri app feature compilation, and the tmux classifier path that gates system notifications.

## Decisions Made

- **Viewing semantics:** Selecting a completed-unchecked run is treated as viewing it and marks it `completed-seen`.
- **Refresh semantics:** A completed run with `notification_seen_at` stays `completed-seen` when tmux still shows completed output.
- **Needs-input detection:** Literal `Need input`, `needs input`, `requires input`, and `waiting for input` terminal text classify as `needs-user`, which allows the existing backend system notification path to fire.
- **Notification authority:** Rust/Tauri remains the system notification owner; React still does not instantiate browser notifications.

## Files Modified This Session

- `src/App.tsx` and `src/App.test.tsx` - Refresh dashboard state through the backend when selecting a run so completed-row badges can clear immediately.
- `src-tauri/src/services.rs`, `src-tauri/src/commands.rs`, and `src-tauri/tests/desktop_state.rs` - Mark selected completed runs seen, preserve seen completions during refresh, and cover the service behavior.
- `core/src/tmux.rs` - Recognize literal need-input status text as `needs-user`.
- `feature_list.json` and `progress.md` - Record feature status and verification evidence.

## Evidence of Completion

- [x] RED: `npm test -- src/App.test.tsx` exited 1 because selecting `api-cleanup` did not call `dashboardState("run-2")`; the last call remained `dashboardState(null)`.
- [x] RED: `cargo test -p agent-manager-desktop completed_run` exited 101 with unresolved imports for the missing seen-state helpers.
- [x] RED: `cargo test -p agentctl-core visible_need_input_text_reports_needs_user` exited 101 because `Need input` classified as `Running` instead of `NeedsUser`.
- [x] GREEN: `npm test -- src/App.test.tsx` exited 0 with 9 tests passing.
- [x] GREEN: `cargo test -p agent-manager-desktop completed_run` exited 0 with 2 targeted tests passing.
- [x] GREEN: `cargo test -p agentctl-core visible_need_input_text_reports_needs_user` exited 0 with 1 targeted test passing.
- [x] `cargo fmt --check` exited 0.
- [x] `npm test` exited 0 with 8 files and 29 tests passing.
- [x] `npm run build` exited 0.
- [x] `cargo check -p agent-manager-desktop --features tauri-app` exited 0.
- [x] `./init.sh` exited 0, running npm test, npm run build, and cargo test. npm test reported 8 files and 29 tests passing; Rust reported 9 `desktop_state` tests and 13 `agentctl-core` unit tests passing.
