# Slash Branch Worktrees Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve `/` hierarchy from run names in created branch names and default worktree folders.

**Architecture:** Keep the naming policy in `core/src/worktree.rs` as pure Rust functions. `core/src/app.rs` should call those functions once during create-run and keep tmux window names flattened through the existing `window_name` path.

**Tech Stack:** Rust core crate, existing `SqliteRegistry` in-memory tests, existing command runner test double, cargo test, `./init.sh`.

---

### Task 1: Path-Aware Naming Tests

**Files:**
- Modify: `core/src/worktree.rs`

- [x] **Step 1: Write the failing tests**

Add tests at the bottom of `core/src/worktree.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{default_branch_name, default_sibling_worktree_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn branch_name_preserves_slash_hierarchy() {
        assert_eq!(default_branch_name("feature/login"), "feature/login");
    }

    #[test]
    fn sibling_worktree_path_preserves_slash_hierarchy() {
        let path =
            default_sibling_worktree_path(Path::new("/repos/agent-manager"), "feature/login");

        assert_eq!(
            path,
            PathBuf::from("/repos/agent-manager-worktrees/feature/login")
        );
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test -p agentctl-core worktree::tests::`

Expected: FAIL because branch and worktree helpers currently flatten or drop `/`.

### Task 2: Create-Run Regression Test

**Files:**
- Modify: `core/src/app.rs`

- [x] **Step 1: Write the failing test**

Add this test inside the existing `#[cfg(test)] mod tests` in `core/src/app.rs`:

```rust
#[test]
fn create_run_preserves_slash_hierarchy_in_branch_and_worktree_path() {
    let registry = SqliteRegistry::in_memory().expect("registry");
    let mut runner = RecordingRunner {
        created_window_visible_after_list_calls: Some(1),
        ..RecordingRunner::default()
    };
    let repo_root = tempfile::tempdir().expect("repo root");
    let repo_path = repo_root.path().join("repo");

    let run = create_run_with_registry(
        &registry,
        &mut runner,
        &AppConfig::for_session("agentctl-test"),
        NewRunRequest {
            repo_path: repo_path.clone(),
            base_ref: "HEAD".to_string(),
            tag: "default".to_string(),
            run_name: "feature/login".to_string(),
            agent: AgentKind::Codex,
        },
    )
    .expect("created run");

    let expected_worktree_path = repo_root
        .path()
        .join("repo-worktrees")
        .join("feature")
        .join("login");
    assert_eq!(run.branch, "feature/login");
    assert_eq!(run.worktree_path, expected_worktree_path);

    let add_worktree = runner
        .commands
        .iter()
        .find(|command| command_contains(command, "worktree") && command_contains(command, "add"))
        .expect("git worktree add command");
    assert!(add_worktree.contains(&"feature/login".to_string()));
    assert!(add_worktree.contains(&path_str(&run.worktree_path).to_string()));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p agentctl-core create_run_preserves_slash_hierarchy_in_branch_and_worktree_path`

Expected: FAIL because current create-run stores `featurelogin` or `feature-login` style flattened values instead of `feature/login`.

### Task 3: Minimal Naming Implementation

**Files:**
- Modify: `core/src/worktree.rs`
- Modify: `core/src/app.rs`

- [x] **Step 1: Implement slash-aware slug construction**

In `core/src/worktree.rs`, add a path-aware slug helper that splits on `/`, sanitizes each segment with existing `sanitize_slug`, removes fallback-only empty segments, and joins valid segments with `/`. Update `default_branch_name` to use that helper.

- [x] **Step 2: Update worktree path construction**

In `core/src/worktree.rs`, make `default_sibling_worktree_path` join each segment of the slash-aware slug as a filesystem path component under `<repo-name>-worktrees`.

- [x] **Step 3: Update create-run call site**

In `core/src/app.rs`, compute the branch from `request.run_name`, pass that branch into `default_sibling_worktree_path`, and leave the flattened `run_slug` for tmux window names.

- [x] **Step 4: Run targeted tests**

Run:

```bash
cargo test -p agentctl-core worktree::tests::
cargo test -p agentctl-core create_run_preserves_slash_hierarchy_in_branch_and_worktree_path
```

Expected: PASS.

### Task 4: Verification and Session Artifacts

**Files:**
- Modify: `feature_list.json`
- Modify: `progress.md`

- [x] **Step 1: Run formatting and focused core tests**

Run:

```bash
cargo fmt --check
cargo test -p agentctl-core
```

Expected: PASS.

- [x] **Step 2: Run standard verification**

Run: `./init.sh`

Expected: PASS. If npm dependencies are unavailable, record exactly what `./init.sh` reports.

- [x] **Step 3: Update artifacts**

Add `feat-026` to `feature_list.json` and update `progress.md` with the feature status, tests, and known risks.

- [x] **Step 4: Commit implementation**

Run:

```bash
git add core/src/worktree.rs core/src/app.rs feature_list.json progress.md docs/superpowers/plans/2026-06-24-slash-branch-worktrees.md
git commit -m "feat: preserve slash branch worktree paths"
```
