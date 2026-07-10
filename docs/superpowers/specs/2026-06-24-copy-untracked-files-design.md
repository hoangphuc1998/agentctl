# Copy Untracked Files Design

> **2026-07-10 update:** feat-051 extends this behavior with a default-enabled
> New Run option that can include Git-ignored files, plus a count/size preview
> and confirmation at 100 MiB or 10,000 files. The original non-ignored-only
> behavior remains available when that option is disabled.

## Goal

When a user creates a run from a repository, Agent Manager should copy the repository's non-ignored untracked files into the new worktree before launching the agent. When the run is ended, those non-ignored untracked files in the run worktree should be deleted as part of the existing worktree and branch cleanup.

## Scope

This feature changes backend create-run and end-run behavior only. It does not change the New Run form, registry schema, merge behavior, stop-run behavior, or tmux naming.

Git-ignored files are intentionally excluded. Files such as `node_modules`, build outputs, caches, and ignored `.env` files should not be copied by this feature.

## File Selection

The source of truth for copied files is Git:

```bash
git -C <repo> ls-files --others --exclude-standard -z
```

The `--exclude-standard` flag excludes files matched by `.gitignore`, `.git/info/exclude`, and global Git excludes. The `-z` flag keeps filenames with spaces and newlines parseable.

Only regular files are copied. Directory entries are ignored because Git reports files. Paths must remain relative to the repository root; absolute paths and parent-directory components are rejected before touching the filesystem.

## Component Shape

`core/src/commands.rs` owns the new Git command builder for listing non-ignored untracked files.

`core/src/untracked_files.rs` owns parsing, safe relative path validation, file copy, and cleanup helpers. This keeps filesystem policy outside the create/end-run orchestration.

`core/src/app.rs` wires the helpers into:

- `create_run_with_registry`, after `git worktree add` succeeds and before the agent tmux window is launched.
- `close_and_delete_run_with_registry`, before `git worktree remove --force` and branch deletion.

## Error Handling

If copying fails during create-run, Agent Manager should roll back the created worktree and branch and return the copy error. This matches the existing rollback behavior for failed tmux launch setup.

If a copied destination path already exists in the new worktree, copying should fail rather than overwrite a tracked file or unexpected user file.

End-run cleanup remains best-effort for tmux kill, but deleting untracked files, removing the worktree, and deleting the branch remain part of the required cleanup path. A failure in these cleanup steps should still surface to the caller.

## Testing

Rust regression tests should cover:

- Creating a run copies a non-ignored untracked file into the worktree before launching the agent.
- Creating a run does not copy Git-ignored files because the file list comes from `git ls-files --others --exclude-standard`.
- Ending a run deletes non-ignored untracked files from the worktree before running `git worktree remove --force` and deleting the branch.

Full verification remains `./init.sh`.
