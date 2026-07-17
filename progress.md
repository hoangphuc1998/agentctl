# Session Progress Log

## Current State

**Last Updated:** 2026-07-17 14:45 +07
**Session ID:** nvm-codex-launch-environment
**Active Feature:** feat-057 - NVM Codex Launch Environment

## Status

### What is Done

- [x] Confirmed the repository startup workflow and clean baseline, reviewed product/design context, feature state, and recent commits.
- [x] Reproduced the screenshot failure: tmux launches commands with non-interactive zsh, which does not source the NVM setup and reports `command not found: codex`.
- [x] Confirmed the user's interactive login shell resolves `/home/phuctth/.nvm/versions/node/v22.18.0/bin/codex` and runs Codex 0.144.5.
- [x] Added RED coverage requiring the generated remote launch to cross the interactive login-shell boundary.
- [x] Added a reusable pure login-shell wrapper and applied it to Codex/Claude launches, restores, and the managed Codex app-server.
- [x] Extended managed app-server launch coverage to require the same environment boundary.
- [x] Smoke-tested the exact nested-shell pattern and confirmed it runs `codex --version` successfully.
- [x] Completed focused and full repository verification.

### What is In Progress

- [x] Implementation is complete.
- [x] Focused RED/GREEN coverage is complete.
- [x] Full verification and continuity artifacts are complete.

### What is Next

1. Rebuild/relaunch Agent Manager, then restore the failed run or create a new one. The existing failed pane remains a shell by design.

## Blockers / Risks

- [x] No code blockers.
- [ ] `npm install` continues to report the repository's existing 5 audit findings (3 moderate, 1 high, 1 critical); dependency remediation was outside this feature.
- [ ] Codex app-server is an official integration surface but is primarily documented for deep integrations and may evolve; protocol parsing remains isolated and covered by schema-shaped tests.

## Decisions Made

- Resolve runtime-managed tools through `"${SHELL:-/bin/sh}" -lic` instead of assuming the desktop or tmux server inherited the user's interactive `PATH`.
- Wrap the complete argument-safe command string once at the shell boundary so agent command builders remain pure and unchanged.
- Use the same boundary for the app-server; fixing only the TUI would leave official status unavailable after a clean restart.

## Files Modified This Session

- `core/src/commands.rs` for the login-shell wrapper and Codex launch regression.
- `core/src/app.rs` for login-shell app-server startup and service coverage.
- `feature_list.json` and `progress.md` for completion evidence and handoff state.

## Evidence of Completion

- [x] RED: `codex_launch_uses_login_shell_environment_for_nvm_installations` failed while the generated command invoked raw `codex` from tmux.
- [x] GREEN: agent launch and managed app-server commands now use the interactive login shell.
- [x] Nested-shell smoke check returned `codex-cli 0.144.5`.
- [x] All core tests and all 12 tmux-restore tests passed.
- [x] `cargo check -p agent-manager-desktop --features tauri-app` passed.
- [x] `cargo fmt --all -- --check` and `git diff --check` passed.
- [x] Final `./init.sh` passed with 12 Vitest files/83 tests, npm build, 41 core tests, and all desktop Rust tests.
