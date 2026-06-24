# Session Progress Log

## Current State

**Last Updated:** 2026-06-24 16:30 +07
**Session ID:** copy-untracked-files
**Active Feature:** feat-027 - Copy Untracked Files Into Run Worktrees

## Status

### What is Done

- [x] Added Git command support for `git ls-files --others --exclude-standard -z`.
- [x] Added safe helper logic for copying and deleting non-ignored untracked file paths.
- [x] Create-run now copies non-ignored untracked files from the source repo into the new worktree before launching the agent.
- [x] End-run cleanup now deletes non-ignored untracked files from the run worktree before removing the worktree and branch.
- [x] Git-ignored files are excluded by using Git's standard exclude rules.

### What is In Progress

- [x] Feature implementation is complete.
- [x] Core verification is complete.
- [x] Standard `./init.sh` verification is complete.

### What is Next

1. Commit the completed feature.

## Blockers / Risks

- [x] No unresolved blockers.
- [ ] `./init.sh` skipped npm checks because `node_modules` is not installed in this workspace; cargo verification ran and passed.

## Decisions Made

- Only non-ignored untracked files are copied; ignored files such as `node_modules`, build outputs, caches, and ignored `.env` files are excluded.
- Git remains the file-selection source of truth through `git ls-files --others --exclude-standard -z`.
- Source symlinks are skipped instead of followed; only regular files are copied.
- Copying refuses to overwrite an existing destination path in the new worktree.
- End-run cleanup deletes non-ignored untracked files in the run worktree before `git worktree remove --force` and branch deletion.

## Files Modified This Session

- docs/superpowers/specs/2026-06-24-copy-untracked-files-design.md - Records the approved design.
- docs/superpowers/plans/2026-06-24-copy-untracked-files.md - Records the implementation checklist.
- core/src/commands.rs - Adds the non-ignored untracked file listing command.
- core/src/lib.rs - Exposes the new untracked file helper module.
- core/src/untracked_files.rs - Adds safe copy/delete helpers and tests.
- core/src/app.rs - Wires copy-on-create and cleanup-on-end into run lifecycle behavior.
- feature_list.json - Adds completed feat-027 with current verification evidence.
- progress.md - Records this session status and verification evidence.

## Evidence of Completion

- [x] RED: `cargo test -p agentctl-core untracked_files` failed because `copy_untracked_files`, `delete_untracked_files`, and `GitCommandBuilder::nonignored_untracked_files` did not exist.
- [x] GREEN: `cargo test -p agentctl-core untracked_files` passed with 3 tests.
- [x] RED: `cargo test -p agentctl-core create_run_copies_nonignored_untracked_files_before_launching_agent` failed because the copied file was missing.
- [x] GREEN: `cargo test -p agentctl-core create_run_copies_nonignored_untracked_files_before_launching_agent` passed.
- [x] RED: `cargo test -p agentctl-core close_and_delete_run_deletes_nonignored_untracked_files_before_removing_worktree` failed because the copied file was still present.
- [x] GREEN: `cargo test -p agentctl-core close_and_delete_run_deletes_nonignored_untracked_files_before_removing_worktree` passed.
- [x] RED: `cargo test -p agentctl-core copy_untracked_files_skips_symlinks` failed because source symlinks were followed.
- [x] GREEN: `cargo test -p agentctl-core copy_untracked_files_skips_symlinks` passed.
- [x] Formatting: `cargo fmt --check` exited 0.
- [x] Core tests: `cargo test -p agentctl-core` passed with 22 tests.
- [x] Standard verification: `./init.sh` exited 0 with cargo tests passing; npm checks were skipped because `node_modules` is not installed.
