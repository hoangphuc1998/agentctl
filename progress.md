# Session Progress Log

## Current State

**Last Updated:** 2026-07-02 13:22 +07
**Session ID:** committed-only-diff-review
**Active Feature:** feat-050 - Committed-Only Diff Review

## Status

### What is Done

- [x] Confirmed the repo root, read AGENTS.md and README.md, reviewed feature_list.json, and checked recent commits.
- [x] Ran baseline `./init.sh`; it exited 0 with npm test, npm run build, and cargo test passing.
- [x] Added RED core coverage proving run diffs must ignore uncommitted tracked edits and untracked binary files.
- [x] Changed the run diff engine to compare `base_commit` to `HEAD` only.
- [x] Removed the run-diff untracked-file merge path so the Diff tab only receives committed branch changes.
- [x] Updated `feature_list.json` with completed feat-050 evidence.

### What is In Progress

- [x] Implementation is complete.
- [x] Focused verification is complete.
- [x] Broader Rust/frontend checks are complete.
- [x] Final `./init.sh` after artifact updates exited 0.

### What is Next

1. Optionally inspect the Diff tab on a run that has both committed changes and uncommitted local edits to confirm only committed changes are visible.

## Blockers / Risks

- [x] No code blockers.
- [ ] Live manual desktop inspection was not performed in this session; behavior is covered by core and React tests.

## Decisions Made

- Treat "committed diff" as the tree diff from the stored run `base_commit` to the run worktree `HEAD`.
- Exclude uncommitted tracked changes, staged-but-uncommitted changes, ignored files, and untracked files from the diff review.
- Keep the Tauri and TypeScript API shape unchanged; the existing diff payload now contains committed files only.

## Files Modified This Session

- `core/src/diff.rs` - Uses `git diff <base_commit> HEAD` for numstat, name-status, and patch output.
- `core/tests/run_diff.rs` - Covers committed-only behavior and untracked-file exclusion.
- `feature_list.json`, `progress.md` - Record feature status and evidence.

## Evidence of Completion

- [x] Baseline `./init.sh` exited 0 with npm test, npm run build, and cargo test passing.
- [x] RED: `cargo test -p agentctl-core --test run_diff` failed because the previous implementation included untracked files and uncommitted worktree edits.
- [x] GREEN: `cargo test -p agentctl-core --test run_diff` exited 0 with 2 tests passing.
- [x] `cargo fmt --check` exited 0 after formatting.
- [x] `cargo test -p agentctl-core` exited 0.
- [x] `cargo test -p agent-manager-desktop run_diff_view_preserves_file_status_counts_and_patch_text` exited 0.
- [x] `npm test -- src/App.test.tsx` exited 0 with 25 tests passing.
- [x] `git diff --check` exited 0.
- [x] Final `./init.sh` exited 0 with npm test, npm run build, and cargo test passing.
