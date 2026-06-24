# Copy Untracked Files Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Copy non-ignored untracked repository files into new run worktrees and delete non-ignored untracked run files during end-run cleanup.

**Architecture:** Add one Git command builder for `git ls-files --others --exclude-standard -z`. Put parsing, safe relative path handling, copying, and deletion in `core/src/untracked_files.rs`, then call that module from the existing create-run and close/delete-run orchestration in `core/src/app.rs`.

**Tech Stack:** Rust core crate, existing `CommandRunner` test double, filesystem tests with `tempfile`, cargo test, `./init.sh`.

---

### Task 1: Git Command and File Helper Tests

**Files:**
- Modify: `core/src/commands.rs`
- Create: `core/src/untracked_files.rs`
- Modify: `core/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests covering the new command builder and helper behavior:

```rust
#[test]
fn untracked_files_command_excludes_ignored_files_and_uses_null_output() {
    let command = GitCommandBuilder::new().nonignored_untracked_files("/repo");

    assert_eq!(
        command,
        vec![
            "git",
            "-C",
            "/repo",
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z"
        ]
    );
}
```

```rust
#[test]
fn copy_untracked_files_preserves_relative_paths() {
    let source_root = tempfile::tempdir().expect("source root");
    let worktree_root = tempfile::tempdir().expect("worktree root");
    let source_file = source_root.path().join("notes").join("scratch.txt");
    std::fs::create_dir_all(source_file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&source_file, "draft").expect("write source");

    copy_untracked_files(source_root.path(), worktree_root.path(), "notes/scratch.txt\0")
        .expect("copied files");

    assert_eq!(
        std::fs::read_to_string(worktree_root.path().join("notes").join("scratch.txt"))
            .expect("read copied file"),
        "draft"
    );
}
```

```rust
#[test]
fn delete_untracked_files_removes_empty_parent_directories() {
    let worktree_root = tempfile::tempdir().expect("worktree root");
    let copied_file = worktree_root.path().join("notes").join("scratch.txt");
    std::fs::create_dir_all(copied_file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&copied_file, "draft").expect("write copied");

    delete_untracked_files(worktree_root.path(), "notes/scratch.txt\0").expect("deleted files");

    assert!(!copied_file.exists());
    assert!(!worktree_root.path().join("notes").exists());
}
```

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test -p agentctl-core untracked_files`

Expected: FAIL because `core/src/untracked_files.rs` and `GitCommandBuilder::nonignored_untracked_files` do not exist yet.

- [x] **Step 3: Implement minimal helper code**

Create `core/src/untracked_files.rs` with public `copy_untracked_files` and `delete_untracked_files` functions that parse NUL-separated relative paths, reject unsafe paths, copy regular files without overwriting existing targets, and delete files plus empty copied parent directories.

Add `pub mod untracked_files;` in `core/src/lib.rs`.

Add `GitCommandBuilder::nonignored_untracked_files(&self, repo_path: &str) -> Vec<String>` in `core/src/commands.rs`.

- [x] **Step 4: Run tests to verify they pass**

Run: `cargo test -p agentctl-core untracked_files`

Expected: PASS.

### Task 2: Create-Run Copy Integration

**Files:**
- Modify: `core/src/app.rs`

- [x] **Step 1: Write failing create-run regression test**

Add a test showing that `create_run_with_registry` asks Git for non-ignored untracked files, copies the returned file into the new worktree, and does so before launching tmux.

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p agentctl-core create_run_copies_nonignored_untracked_files_before_launching_agent`

Expected: FAIL because create-run does not list or copy untracked files yet.

- [x] **Step 3: Implement create-run copy call**

In `create_run_with_registry`, after `git worktree add` succeeds, run `git.nonignored_untracked_files` against the source repository and call `copy_untracked_files`. If either step fails, call the existing rollback helper and return the error.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p agentctl-core create_run_copies_nonignored_untracked_files_before_launching_agent`

Expected: PASS.

### Task 3: End-Run Cleanup Integration

**Files:**
- Modify: `core/src/app.rs`

- [x] **Step 1: Write failing end-run cleanup regression test**

Add a test showing that `close_and_delete_run_with_registry` lists non-ignored untracked files from the run worktree and deletes them before running `git worktree remove --force` and branch deletion.

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p agentctl-core close_and_delete_run_deletes_nonignored_untracked_files_before_removing_worktree`

Expected: FAIL because end-run cleanup currently removes the worktree without explicitly deleting non-ignored untracked files first.

- [x] **Step 3: Implement cleanup call**

In `close_and_delete_run_with_registry`, before `git.remove_worktree`, run `git.nonignored_untracked_files` against `run.worktree_path` and call `delete_untracked_files`.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p agentctl-core close_and_delete_run_deletes_nonignored_untracked_files_before_removing_worktree`

Expected: PASS.

### Task 4: Verification and Artifacts

**Files:**
- Modify: `feature_list.json`
- Modify: `progress.md`

- [x] **Step 1: Run focused Rust verification**

Run:

```bash
cargo fmt --check
cargo test -p agentctl-core
```

Expected: PASS.

- [x] **Step 2: Run standard verification**

Run: `./init.sh`

Expected: PASS. If npm checks are skipped because `node_modules` is absent, record that exactly.

- [x] **Step 3: Update artifacts**

Add `feat-027` to `feature_list.json` and update `progress.md` with behavior, tests, verification evidence, and any remaining risks.

- [x] **Step 4: Commit implementation**

Run:

```bash
git add core/src/commands.rs core/src/lib.rs core/src/untracked_files.rs core/src/app.rs feature_list.json progress.md docs/superpowers/specs/2026-06-24-copy-untracked-files-design.md docs/superpowers/plans/2026-06-24-copy-untracked-files.md
git commit -m "feat: copy untracked files into run worktrees"
```
