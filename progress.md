# Session Progress Log

## Current State

**Last Updated:** 2026-07-17 15:15 +07
**Session ID:** remote-codex-worktree-context
**Active Feature:** feat-058 - Remote Codex Worktree Context

## Status

### What is Done

- [x] Confirmed the repository startup workflow and clean baseline, reviewed product/design context, feature state, and recent commits.
- [x] Inspected the live app-server and found cwd `/usr` plus inherited `PWD=/tmp/.mount_Agent.../usr`, whose AppImage mount no longer exists.
- [x] Verified the affected worktree has no broken project Codex configuration and Codex doctor found no path override.
- [x] Started an isolated fresh app-server and proved both this repository and the affected worktree can start remote TUI threads successfully.
- [x] Proved a remote TUI without `--cd` adopts the app-server cwd, while Codex's supported `--cd` keeps the intended worktree through bootstrap.
- [x] Added RED/GREEN coverage and passed the worktree explicitly for fresh and resumed remote Codex sessions.
- [x] Anchored app-server startup at `$HOME` and replaced only an existing service whose pane cwd differs from that stable anchor.
- [x] Completed focused and full repository verification.

### What is In Progress

- [x] Implementation, verification, and continuity artifacts are complete.

### What is Next

1. Rebuild/relaunch Agent Manager, then restore the failed run. Startup/dashboard refresh replaces the stale AppImage-backed server; restored/new sessions target their own worktrees.

## Blockers / Risks

- [x] No code blockers.
- [ ] `npm install` continues to report the repository's existing 5 audit findings (3 moderate, 1 high, 1 critical); dependency remediation was outside this feature.
- [ ] Codex app-server is an official integration surface but is primarily documented for deep integrations and may evolve; protocol parsing remains isolated and covered by schema-shaped tests.

## Decisions Made

- Treat the app-server as a shared process whose cwd must be stable and independent of any AppImage mount or deletable run worktree.
- Pass `--cd <worktree>` on every remote Codex launch/restore because the remote server, not the local TUI process, creates the thread.
- Compare the managed service pane cwd with `$HOME`; preserve a matching healthy server and replace a mismatched legacy server.

## Files Modified This Session

- `core/src/commands.rs` for explicit remote worktree arguments and tmux service inspection/replacement commands.
- `core/src/app.rs` for stable-home app-server lifecycle and stale AppImage-server coverage.
- `src-tauri/tests/tmux_restore.rs` for worktree-preserving restored commands.
- `feature_list.json` and `progress.md` for completion evidence and handoff state.

## Evidence of Completion

- [x] RED: `codex_remote_launch_targets_the_run_worktree` showed remote launch omitted `--cd`.
- [x] GREEN: fresh and resume commands include the exact worktree, including tmux-resurrect rewrites.
- [x] RED/GREEN: stale AppImage-anchored app-server sessions are replaced before use.
- [x] Live isolated-server smoke tests confirmed `--cd` preserves the affected worktree through remote bootstrap.
- [x] All core tests and all 12 tmux-restore tests passed.
- [x] `cargo check -p agent-manager-desktop --features tauri-app` passed.
- [x] `cargo fmt --all -- --check` and `git diff --check` passed.
- [x] Final `./init.sh` passed with 12 Vitest files/83 tests, npm build, 43 core tests, and all desktop Rust tests.
