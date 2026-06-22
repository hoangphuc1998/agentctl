use agent_manager_desktop::{
    models::RunView,
    services::{build_dashboard_state, suggestions_from_candidates},
};
use agentctl_core::{
    agent::AgentKind,
    completion::CompletionCandidate,
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
};
use std::path::PathBuf;
use uuid::Uuid;

fn run(name: &str, state: ObservedState, source: DetectionSource) -> RunRecord {
    RunRecord {
        id: Uuid::new_v4(),
        repo_path: PathBuf::from("/repos/agent-manager"),
        repo_name: "agent-manager".to_string(),
        tag: "feature".to_string(),
        run_name: name.to_string(),
        agent: AgentKind::Codex,
        lifecycle: Lifecycle::Active,
        observed_state: state,
        detection_source: source,
        branch: name.to_string(),
        base_ref: "main".to_string(),
        worktree_path: PathBuf::from(format!("/repos/agent-manager-worktrees/{name}")),
        tmux_session: Some("agentctl".to_string()),
        tmux_window: Some(format!("agent-manager__feature__{name}")),
        tmux_pane: None,
        agent_session_id: None,
        notification_seen_at: None,
        created_at: 1,
        updated_at: 2,
    }
}

#[test]
fn dashboard_state_counts_active_runs_and_stale_unknown_runs() {
    let active = vec![
        run("login-flow", ObservedState::Running, DetectionSource::Tmux),
        run(
            "missing-window",
            ObservedState::Unknown,
            DetectionSource::Unknown,
        ),
    ];
    let state = build_dashboard_state(
        active,
        Some(PathBuf::from("/repos/agent-manager")),
        vec![],
        None,
    );

    assert_eq!(state.active_count, 2);
    assert_eq!(state.stale_count, 1);
    assert_eq!(
        state.active_repo_path,
        Some("/repos/agent-manager".to_string())
    );
    assert_eq!(
        state.selected_run_id,
        state.repos[0].runs.first().map(|run| run.id.clone())
    );
}

#[test]
fn dashboard_state_can_reuse_existing_selected_run() {
    let active = vec![
        run("first", ObservedState::Running, DetectionSource::Tmux),
        run("second", ObservedState::Running, DetectionSource::Tmux),
    ];
    let selected = RunView::from(active[1].clone()).id;

    let state = build_dashboard_state(active, None, vec![], Some(selected.clone()));

    assert_eq!(state.selected_run_id, Some(selected));
}

#[test]
fn suggestions_preserve_value_and_detail_for_frontend_autocomplete() {
    let candidates = vec![CompletionCandidate {
        value: "/repos/agent-manager/".to_string(),
        detail: "recent repo".to_string(),
    }];

    let suggestions = suggestions_from_candidates(candidates);

    assert_eq!(suggestions[0].value, "/repos/agent-manager/");
    assert_eq!(suggestions[0].detail, "recent repo");
}
