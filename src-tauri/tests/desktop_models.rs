use agent_manager_desktop::models::{repo_tree_from_runs, RunDiffFileView, RunDiffView, RunView};
use agentctl_core::{
    agent::AgentKind,
    diff::{RunDiff, RunDiffFile},
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
};
use std::path::PathBuf;
use uuid::Uuid;

fn run(repo_name: &str, run_name: &str, tag: &str, state: ObservedState) -> RunRecord {
    RunRecord {
        id: Uuid::new_v4(),
        repo_path: PathBuf::from(format!("/repos/{repo_name}")),
        repo_name: repo_name.to_string(),
        tag: tag.to_string(),
        run_name: run_name.to_string(),
        agent: AgentKind::Codex,
        lifecycle: Lifecycle::Active,
        observed_state: state,
        detection_source: DetectionSource::Tmux,
        branch: run_name.to_string(),
        base_ref: "main".to_string(),
        base_commit: None,
        worktree_path: PathBuf::from(format!("/repos/{repo_name}-worktrees/{run_name}")),
        tmux_session: Some("agentctl".to_string()),
        tmux_window: Some(format!("{repo_name}__{run_name}")),
        tmux_pane: None,
        agent_session_id: None,
        notification_seen_at: None,
        created_at: 1,
        updated_at: 2,
    }
}

#[test]
fn repo_tree_groups_repositories_with_child_runs_only() {
    let runs = vec![
        run(
            "agent-manager",
            "login-flow",
            "feature",
            ObservedState::Running,
        ),
        run(
            "agent-manager",
            "api-cleanup",
            "review",
            ObservedState::NeedsUser,
        ),
        run(
            "agentctl",
            "desktop",
            "default",
            ObservedState::CompletedUnchecked,
        ),
    ];

    let tree = repo_tree_from_runs(&runs);

    assert_eq!(tree.len(), 2);
    assert_eq!(tree[0].repo_name, "agent-manager");
    assert_eq!(tree[0].runs.len(), 2);
    assert_eq!(tree[0].runs[0].run_name, "api-cleanup");
    assert_eq!(tree[0].runs[0].tag, "review");
    assert_eq!(tree[1].repo_name, "agentctl");
    assert_eq!(tree[1].runs[0].observed_state, "completed-unchecked");
}

#[test]
fn run_view_preserves_cli_registry_metadata_for_frontend_actions() {
    let source = run(
        "agent-manager",
        "login-flow",
        "feature",
        ObservedState::Running,
    );

    let view = RunView::from(source.clone());

    assert_eq!(view.id, source.id.to_string());
    assert_eq!(view.repo_path, "/repos/agent-manager");
    assert_eq!(view.agent, "codex");
    assert_eq!(view.lifecycle, "active");
    assert_eq!(view.detection_source, "tmux");
    assert_eq!(view.base_ref, "main");
    assert_eq!(
        view.worktree_path,
        "/repos/agent-manager-worktrees/login-flow"
    );
}

#[test]
fn run_diff_view_preserves_file_status_counts_and_patch_text() {
    let diff = RunDiff {
        run_id: "run-1".to_string(),
        base_ref: "main".to_string(),
        base_commit: Some("abc123".to_string()),
        worktree_path: "/repos/agent-manager-worktrees/diff-review".to_string(),
        files: vec![RunDiffFile {
            path: "src/App.tsx".to_string(),
            old_path: None,
            status: "modified".to_string(),
            additions: 2,
            deletions: 1,
            binary: false,
            patch: Some("@@ -1 +1 @@\n-old\n+new\n".to_string()),
            message: None,
        }],
        file_count: 1,
        additions: 2,
        deletions: 1,
        generated_at: 42,
        warning: Some("fallback base".to_string()),
    };

    let view = RunDiffView::from(diff);

    assert_eq!(view.run_id, "run-1");
    assert_eq!(view.base_ref, "main");
    assert_eq!(view.base_commit.as_deref(), Some("abc123"));
    assert_eq!(view.file_count, 1);
    assert_eq!(view.additions, 2);
    assert_eq!(view.deletions, 1);
    assert_eq!(view.generated_at, 42);
    assert_eq!(view.warning.as_deref(), Some("fallback base"));
    assert_eq!(
        view.files,
        vec![RunDiffFileView {
            path: "src/App.tsx".to_string(),
            old_path: None,
            status: "modified".to_string(),
            additions: 2,
            deletions: 1,
            binary: false,
            patch: Some("@@ -1 +1 @@\n-old\n+new\n".to_string()),
            message: None,
        }]
    );
}
