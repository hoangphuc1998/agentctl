# Session Progress Log

## Current State

**Last Updated:** 2026-07-27 18:06 +07
**Session ID:** standalone-codex-sessions
**Active Feature:** feat-064 - Standalone Codex Sessions

## Status

### What is Done

- [x] Diagnosed the live shared Codex app-server at more than 4 GiB private RSS.
- [x] Reproduced the remote launch, readiness, and restore coupling with focused RED tests.
- [x] Changed new and restored Codex runs to independent standalone CLI processes.
- [x] Removed app-server startup, health probing, WebSocket polling, and protocol status code.
- [x] Preserved exact Codex resume IDs through a read-only local state database adapter.
- [x] Restored per-pane tmux status observation and kept same-folder identity assignment.
- [x] Updated README migration and architecture guidance.
- [x] Completed focused, feature-build, and full repository verification.

### What is In Progress

- [x] Implementation, tests, verification, and continuity artifacts are complete.

### What is Next

1. Rebuild and relaunch Agent Manager to make newly created and restored runs standalone.
2. Stop any Codex runs created by the older remote architecture.
3. After no legacy remote runs remain, remove the unused service with
   `tmux kill-session -t agentctl-codex`.

## Blockers / Risks

- [x] No code blockers.
- [ ] Existing live remote Codex panes continue using the legacy app-server until stopped; the
  migration deliberately does not disconnect active work.
- [ ] Standalone status uses terminal/tmux observation rather than provider ThreadStatus.
- [ ] `npm install` reports 6 existing audit findings (3 moderate, 2 high, 1 critical);
  dependency remediation remains outside this feature.

## Decisions Made

- Each managed Codex run owns its own CLI process and releases its own resources when stopped.
- Agent Manager no longer starts, probes, or connects to `codex app-server`.
- Codex's local `state_5.sqlite` is opened read-only and used only for CLI thread identity.
- Only active root CLI threads are eligible for assignment; archived, remote-surface, and
  subagent threads are excluded.
- Persisted session IDs remain authoritative, and unbound same-folder sessions retain the
  creation-time guard introduced by feat-063.

## Files Modified This Session

- `core/src/commands.rs` and `core/src/app.rs` for standalone launch/restore lifecycle.
- `core/src/codex_thread.rs` for the minimal thread identity domain type.
- `src-tauri/src/codex_state.rs` for read-only local Codex identity loading.
- `src-tauri/src/commands.rs` and `src-tauri/src/services.rs` for SQLite identity assignment and
  tmux-based status refresh.
- Removed the app-server protocol client/status modules and their obsolete tests/dependency.
- Updated tmux restore, desktop state, Codex state, create-run, README, feature, and progress
  coverage.

## Evidence of Completion

- [x] RED command tests returned remote `--remote ws://127.0.0.1:17655` arguments before the fix.
- [x] RED create-run and tmux-resurrect tests retained /readyz and remote startup before the fix.
- [x] Focused standalone command, create-run, tmux restore, Codex state, and desktop state tests
  passed after implementation.
- [x] `cargo check -p agent-manager-desktop --features tauri-app` passed.
- [x] `cargo fmt --all -- --check` and `git diff --check` passed.
- [x] Final `./init.sh` passed with 12 Vitest files/88 tests, npm production build, 41 core tests,
  45 desktop tests, and all doc tests.
