use agent_manager_desktop::models::{repo_tree_from_runs, RunView};
use agentctl_core::{
    agent::AgentKind,
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
