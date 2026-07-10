# Session Progress Log

## Current State

**Last Updated:** 2026-07-10 13:04 +07
**Session ID:** bulk-ignored-file-worktree-snapshot
**Active Feature:** feat-051 - Bulk Ignored File Worktree Snapshot

## Status

### What is Done

- [x] Confirmed the repo root, read AGENTS.md and README.md, reviewed feature_list.json, and checked recent commits.
- [x] Ran baseline `./init.sh`; Rust tests passed and npm checks were skipped because `node_modules` was absent.
- [x] Added Git commands for ignored-only previews and complete untracked snapshots.
- [x] Added safe regular-file count/size previews and confirmation thresholds at 100 MiB or 10,000 files.
- [x] Added `copy_ignored_files` to core and desktop create-run requests while preserving omitted-payload compatibility as disabled.
- [x] Moved preview and create-run filesystem work onto Tauri blocking tasks.
- [x] Added a default-enabled New Run toggle, debounced preview, stale-result protection, large-copy confirmation, and preview error handling.
- [x] Documented one-time worktree snapshots and their secret/size implications.

### What is In Progress

- [x] Implementation is complete.
- [x] Focused RED/GREEN coverage is complete.
- [x] Full Rust/frontend verification is complete.
- [x] Feature tracker and continuity artifacts are updated.

### What is Next

1. Optionally create a live run from a repository with an ignored `.env` and inspect the resulting worktree before agent launch.

## Blockers / Risks

- [x] No code blockers.
- [ ] Live desktop inspection was not performed; behavior is covered by core, Tauri model, modal, and App tests.
- [ ] Confirming the option can copy secrets and very large generated trees by design; the modal displays this warning and large snapshots require confirmation.
- [ ] `npm install` reports existing audit findings: 3 moderate, 1 high, and 1 critical.

## Decisions Made

- Keep the existing non-ignored snapshot behavior when the new toggle is disabled.
- Enable ignored-file copying by default for each New Run without persisting the choice.
- Preview only the additional Git-ignored candidates, but use `git ls-files --others -z` for the actual complete snapshot.
- Require confirmation when ignored candidates are at least 100 MiB or 10,000 regular files; confirmed copies have no hard cap.
- Preserve the existing safe-path, no-overwrite, symlink-skip, rollback, and forced worktree cleanup behavior.
- Keep the registry schema unchanged because the selection affects creation only.

## Files Modified This Session

- Core Git/file policy and create-run orchestration for ignored previews and complete snapshots.
- Tauri create/preview commands, wire models, and command registration.
- New Run modal, API/types, styling, and React/App regression coverage.
- README, prior snapshot design note, `feature_list.json`, and `progress.md`.

## Evidence of Completion

- [x] RED: `cargo test -p agentctl-core untracked_files` failed because preview types/functions and all/ignored Git commands did not exist.
- [x] RED: `cargo test -p agentctl-core ignored_untracked_files` failed because preview orchestration and `copy_ignored_files` did not exist.
- [x] RED: `npm test -- src/components/CreateRunModal.test.tsx` failed four new snapshot UX tests before implementation.
- [x] `npm install` completed after esbuild required execution outside the sandbox; it reported 5 existing vulnerabilities.
- [x] `cargo test -p agentctl-core` passed with 31 unit tests plus registry and diff integration tests.
- [x] `cargo test -p agent-manager-desktop --test desktop_models` passed with 5 tests.
- [x] `cargo check -p agent-manager-desktop --features tauri-app` passed.
- [x] `cargo test -p agent-manager-desktop --features tauri-app` passed all tests except the sandbox-restricted loopback bind; the failing `mobile_bridge_runtime` test passed outside the sandbox.
- [x] `npm test` passed with 11 files and 74 tests.
- [x] `npm run build`, `cargo fmt --check`, and `git diff --check` passed.
- [x] Final `./init.sh` after artifact updates passed with 74 Vitest tests, npm build, and cargo test.
