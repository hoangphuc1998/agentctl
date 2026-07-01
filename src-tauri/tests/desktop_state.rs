use agent_manager_desktop::{
    models::{AgentAttentionEvent, RunView},
    services::{
        agent_attention_event_for_transition, agent_system_notification_for_event,
        build_dashboard_state, is_restorable_run, is_stale_run, mark_selected_run_seen,
        observed_state_after_refresh, suggestions_from_candidates,
    },
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
        base_commit: None,
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
fn dashboard_state_counts_active_runs_and_restorable_unknown_runs() {
    let active = vec![
        run("login-flow", ObservedState::Running, DetectionSource::Tmux),
        run(
            "missing-window-after-reboot",
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
    assert_eq!(state.stale_count, 0);
    assert_eq!(state.restorable_count, 1);
    assert!(state.repos[0]
        .runs
        .iter()
        .any(|run| run.run_name == "missing-window-after-reboot" && run.restorable));
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
fn dashboard_state_counts_attention_runs() {
    let active = vec![
        run("login-flow", ObservedState::Running, DetectionSource::Tmux),
        run(
            "copy-review",
            ObservedState::NeedsUser,
            DetectionSource::Heuristic,
        ),
        run(
            "api-cleanup",
            ObservedState::CompletedUnchecked,
            DetectionSource::Heuristic,
        ),
        run(
            "done-already",
            ObservedState::CompletedSeen,
            DetectionSource::Heuristic,
        ),
    ];

    let state = build_dashboard_state(active, None, vec![], None);

    assert_eq!(state.attention_count, 2);
}

#[test]
fn dashboard_attention_events_only_fire_on_new_attention_states() {
    let needs_user = run(
        "copy-review",
        ObservedState::NeedsUser,
        DetectionSource::Heuristic,
    );
    let completed = run(
        "api-cleanup",
        ObservedState::CompletedUnchecked,
        DetectionSource::Heuristic,
    );
    let running = run("login-flow", ObservedState::Running, DetectionSource::Tmux);

    assert_eq!(
        agent_attention_event_for_transition(ObservedState::Running, &needs_user),
        Some(AgentAttentionEvent {
            run_id: needs_user.id.to_string(),
            run_name: "copy-review".to_string(),
            repo_name: "agent-manager".to_string(),
            agent: "codex".to_string(),
            observed_state: "needs-user".to_string(),
            title: "Agent needs input".to_string(),
            body: "copy-review in agent-manager is waiting for you.".to_string(),
        })
    );
    assert_eq!(
        agent_attention_event_for_transition(ObservedState::Running, &completed),
        Some(AgentAttentionEvent {
            run_id: completed.id.to_string(),
            run_name: "api-cleanup".to_string(),
            repo_name: "agent-manager".to_string(),
            agent: "codex".to_string(),
            observed_state: "completed-unchecked".to_string(),
            title: "Agent completed".to_string(),
            body: "api-cleanup in agent-manager is ready for review.".to_string(),
        })
    );
    assert_eq!(
        agent_attention_event_for_transition(ObservedState::NeedsUser, &needs_user),
        None
    );
    assert_eq!(
        agent_attention_event_for_transition(ObservedState::CompletedUnchecked, &completed),
        None
    );
    assert_eq!(
        agent_attention_event_for_transition(ObservedState::Unknown, &running),
        None
    );
}

#[test]
fn agent_system_notification_uses_attention_event_text() {
    let completed = run(
        "api-cleanup",
        ObservedState::CompletedUnchecked,
        DetectionSource::Heuristic,
    );
    let event = agent_attention_event_for_transition(ObservedState::Running, &completed).unwrap();

    let (title, body) = agent_system_notification_for_event(&event);

    assert_eq!(title, "Agent completed");
    assert_eq!(body, "api-cleanup in agent-manager is ready for review.");
}

#[test]
fn selecting_completed_run_marks_it_seen_before_building_dashboard() {
    let completed = run(
        "api-cleanup",
        ObservedState::CompletedUnchecked,
        DetectionSource::Heuristic,
    );
    let selected = completed.id.to_string();
    let mut active = vec![
        run("login-flow", ObservedState::Running, DetectionSource::Tmux),
        completed,
    ];

    let marked = mark_selected_run_seen(&mut active, Some(&selected), 42);

    assert!(marked.is_some());
    assert_eq!(active[1].observed_state, ObservedState::CompletedSeen);
    assert_eq!(active[1].notification_seen_at, Some(42));
    let state = build_dashboard_state(active, None, vec![], Some(selected.clone()));
    assert_eq!(state.selected_run_id, Some(selected));
    assert_eq!(state.attention_count, 0);
    assert_eq!(state.repos[0].runs[0].observed_state, "completed-seen");
}

#[test]
fn completed_run_with_seen_timestamp_stays_seen_after_refresh_detection() {
    assert_eq!(
        observed_state_after_refresh(ObservedState::CompletedUnchecked, Some(42)),
        ObservedState::CompletedSeen
    );
    assert_eq!(
        observed_state_after_refresh(ObservedState::CompletedUnchecked, None),
        ObservedState::CompletedUnchecked
    );
    assert_eq!(
        observed_state_after_refresh(ObservedState::NeedsUser, Some(42)),
        ObservedState::NeedsUser
    );
}

#[test]
fn restorable_runs_are_not_stale_cleanup_candidates() {
    let restorable = run(
        "missing-window-after-reboot",
        ObservedState::Unknown,
        DetectionSource::Unknown,
    );

    assert!(is_restorable_run(&restorable));
    assert!(!is_stale_run(&restorable));
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
