use agent_manager_desktop::terminal_plan::tmux_attach_command;
use agentctl_core::{
    agent::AgentKind,
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
};
use std::path::PathBuf;
use uuid::Uuid;

fn run() -> RunRecord {
    RunRecord {
        id: Uuid::new_v4(),
        repo_path: PathBuf::from("/repos/agent-manager"),
        repo_name: "agent-manager".to_string(),
        tag: "feature".to_string(),
        run_name: "login-flow".to_string(),
        agent: AgentKind::Codex,
        lifecycle: Lifecycle::Active,
        observed_state: ObservedState::Running,
        detection_source: DetectionSource::Tmux,
        branch: "login-flow".to_string(),
        base_ref: "main".to_string(),
        base_commit: None,
        worktree_path: PathBuf::from("/repos/agent-manager-worktrees/login-flow"),
        tmux_session: Some("agentctl".to_string()),
        tmux_window: Some("agent-manager__feature__login-flow".to_string()),
        tmux_pane: None,
        agent_session_id: None,
        notification_seen_at: None,
        created_at: 1,
        updated_at: 2,
    }
}

#[test]
fn tmux_attach_command_targets_selected_run_window() {
    let command = tmux_attach_command(&run(), "fallback").expect("command");

    assert_eq!(command.program, "env");
    assert_eq!(
        command.args,
        vec![
            "TERM=xterm-256color".to_string(),
            "COLORTERM=truecolor".to_string(),
            "tmux".to_string(),
            "attach-session".to_string(),
            "-t".to_string(),
            "agentctl:agent-manager__feature__login-flow".to_string(),
        ]
    );
}

#[test]
fn tmux_attach_command_requires_a_window_name() {
    let mut run = run();
    run.tmux_window = None;

    let err = tmux_attach_command(&run, "fallback").expect_err("missing window should fail");

    assert!(err.contains("does not have a tmux window"));
}
