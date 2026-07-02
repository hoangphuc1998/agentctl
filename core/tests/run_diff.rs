use std::{
    path::{Path, PathBuf},
    process::Command,
};

use agentctl_core::{
    agent::AgentKind,
    diff::load_run_diff,
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn run_diff_includes_only_committed_changes_against_base() {
    let repo = git_repo();
    let base_commit = git_output(repo.path(), ["rev-parse", "HEAD"]);
    git(repo.path(), ["checkout", "-b", "feature/diff-review"]);
    std::fs::write(
        repo.path().join("src").join("lib.rs"),
        "pub fn value() -> u8 {\n    2\n}\n",
    )
    .expect("modify tracked file");
    std::fs::write(repo.path().join("notes.md"), "new note\nsecond line\n")
        .expect("write untracked file");
    std::fs::write(repo.path().join("ignored.log"), "ignore me\n").expect("write ignored file");
    git(repo.path(), ["add", "src/lib.rs"]);
    git(repo.path(), ["commit", "-m", "change tracked file"]);
    std::fs::write(
        repo.path().join("src").join("lib.rs"),
        "pub fn value() -> u8 {\n    3\n}\n",
    )
    .expect("modify worktree file");

    let diff = load_run_diff(&run_record(repo.path(), &base_commit)).expect("run diff");

    assert_eq!(diff.base_commit.as_deref(), Some(base_commit.as_str()));
    assert_eq!(diff.file_count, 1);
    assert_eq!(diff.additions, 1);
    assert_eq!(diff.deletions, 1);
    assert!(diff.warning.is_none());

    let tracked = diff
        .files
        .iter()
        .find(|file| file.path == "src/lib.rs")
        .expect("tracked file");
    assert_eq!(tracked.status, "modified");
    assert_eq!(tracked.additions, 1);
    assert_eq!(tracked.deletions, 1);
    assert_eq!(tracked.binary, false);
    assert!(tracked.patch.as_deref().unwrap_or("").contains("-    1"));
    assert!(tracked.patch.as_deref().unwrap_or("").contains("+    2"));
    assert!(!tracked.patch.as_deref().unwrap_or("").contains("+    3"));
    assert!(!diff.files.iter().any(|file| file.path == "notes.md"));
    assert!(!diff.files.iter().any(|file| file.path == "ignored.log"));
}

#[test]
fn run_diff_ignores_binary_untracked_files() {
    let repo = git_repo();
    let base_commit = git_output(repo.path(), ["rev-parse", "HEAD"]);
    std::fs::write(repo.path().join("asset.bin"), [0, 159, 146, 150]).expect("write binary");

    let diff = load_run_diff(&run_record(repo.path(), &base_commit)).expect("run diff");

    assert_eq!(diff.file_count, 0);
    assert!(diff.files.is_empty());
}

fn git_repo() -> TempDir {
    let repo = tempfile::tempdir().expect("repo");
    git(repo.path(), ["init"]);
    git(repo.path(), ["config", "user.email", "agent@example.test"]);
    git(repo.path(), ["config", "user.name", "Agent Manager"]);
    std::fs::create_dir_all(repo.path().join("src")).expect("mkdir src");
    std::fs::write(
        repo.path().join("src").join("lib.rs"),
        "pub fn value() -> u8 {\n    1\n}\n",
    )
    .expect("write source");
    std::fs::write(repo.path().join(".gitignore"), "ignored.log\n").expect("write gitignore");
    git(repo.path(), ["add", "."]);
    git(repo.path(), ["commit", "-m", "base"]);
    repo
}

fn run_record(worktree_path: &Path, base_commit: &str) -> RunRecord {
    RunRecord {
        id: Uuid::new_v4(),
        repo_path: PathBuf::from(worktree_path),
        repo_name: "repo".to_string(),
        tag: "feature".to_string(),
        run_name: "diff-review".to_string(),
        agent: AgentKind::Codex,
        lifecycle: Lifecycle::Active,
        observed_state: ObservedState::Running,
        detection_source: DetectionSource::Tmux,
        branch: "feature/diff-review".to_string(),
        base_ref: "main".to_string(),
        base_commit: Some(base_commit.to_string()),
        worktree_path: PathBuf::from(worktree_path),
        tmux_session: Some("agentctl".to_string()),
        tmux_window: Some("repo__feature__diff-review".to_string()),
        tmux_pane: None,
        agent_session_id: None,
        notification_seen_at: None,
        created_at: 1,
        updated_at: 2,
    }
}

fn git<const N: usize>(repo_path: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output<const N: usize>(repo_path: &Path, args: [&str; N]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
