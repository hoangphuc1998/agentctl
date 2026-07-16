use agent_manager_desktop::terminal_plan::{
    terminal_link_command, tmux_attach_command, TerminalLinkTarget,
};
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

#[test]
fn terminal_web_link_command_allows_only_http_protocols() {
    let command = terminal_link_command(
        &run(),
        &TerminalLinkTarget::Url {
            url: "https://example.com/docs?q=tmux".to_string(),
        },
    )
    .expect("web link command");

    assert_eq!(command.program, "xdg-open");
    assert_eq!(command.args, vec!["https://example.com/docs?q=tmux"]);
    assert!(terminal_link_command(
        &run(),
        &TerminalLinkTarget::Url {
            url: "javascript:alert(1)".to_string(),
        }
    )
    .is_err());
}

#[test]
fn terminal_file_link_command_opens_an_existing_worktree_file_at_location() {
    let worktree = tempfile::tempdir().expect("worktree");
    let source = worktree.path().join("src/main.rs");
    std::fs::create_dir_all(source.parent().expect("source parent")).expect("source directory");
    std::fs::write(&source, "fn main() {}\n").expect("source file");
    let mut run = run();
    run.worktree_path = worktree.path().to_path_buf();

    let command = terminal_link_command(
        &run,
        &TerminalLinkTarget::File {
            path: "src/main.rs".to_string(),
            line: Some(12),
            column: Some(4),
        },
    )
    .expect("file link command");

    assert_eq!(command.program, "code");
    assert_eq!(command.args[0], "--goto");
    assert_eq!(command.args[1], format!("{}:12:4", source.display()));
}

#[test]
fn terminal_file_link_command_rejects_missing_and_outside_files() {
    let worktree = tempfile::tempdir().expect("worktree");
    let outside = tempfile::NamedTempFile::new().expect("outside file");
    let mut run = run();
    run.worktree_path = worktree.path().to_path_buf();

    assert!(terminal_link_command(
        &run,
        &TerminalLinkTarget::File {
            path: "missing.rs".to_string(),
            line: None,
            column: None,
        }
    )
    .is_err());
    assert!(terminal_link_command(
        &run,
        &TerminalLinkTarget::File {
            path: outside.path().display().to_string(),
            line: None,
            column: None,
        }
    )
    .is_err());
}
