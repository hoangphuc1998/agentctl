use agent_manager_desktop::tmux_restore::{
    refresh_tmux_restore_hook, restore_tmux_session_if_missing, rewrite_resurrect_state,
    stable_agent_manager_executable, tmux_restore_needed, tmux_restore_status,
    upsert_agent_manager_tmux_config, TmuxRestorePaths,
};
use agentctl_core::{
    agent::AgentKind,
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

fn run(
    name: &str,
    agent: AgentKind,
    lifecycle: Lifecycle,
    tmux_window: &str,
    agent_session_id: Option<Uuid>,
) -> RunRecord {
    RunRecord {
        id: Uuid::new_v4(),
        repo_path: PathBuf::from("/repos/agent-manager"),
        repo_name: "agent-manager".to_string(),
        tag: "default".to_string(),
        run_name: name.to_string(),
        agent,
        lifecycle,
        observed_state: ObservedState::Running,
        detection_source: DetectionSource::Tmux,
        branch: name.to_string(),
        base_ref: "main".to_string(),
        base_commit: None,
        worktree_path: PathBuf::from(format!("/repos/agent-manager-worktrees/{name}")),
        tmux_session: Some("agentctl".to_string()),
        tmux_window: Some(tmux_window.to_string()),
        tmux_pane: None,
        agent_session_id,
        notification_seen_at: None,
        created_at: 1,
        updated_at: 2,
    }
}

#[test]
fn rewrite_resurrect_state_replaces_managed_codex_pane_with_resume_command() {
    let input = "\
pane\tagentctl\t1\t1\t:* \t0\t:\t:/repos/agent-manager-worktrees/fix-restart\t1\tnode\t:node /home/me/.nvm/bin/codex
window\tagentctl\t1\t:agentctl__default__fix-restart__17164bd9\t1\t:* \tabcd\t:
";
    let runs = vec![run(
        "fix-restart",
        AgentKind::Codex,
        Lifecycle::Active,
        "agentctl__default__fix-restart__17164bd9",
        None,
    )];

    let output = rewrite_resurrect_state(input, &runs, "agentctl");

    assert!(output.contains("curl -fsS http://127.0.0.1:17655/readyz"));
    assert!(output.contains("codex --remote ws://127.0.0.1:17655 resume --last"));
    assert!(!output.contains(":node /home/me/.nvm/bin/codex"));
}

#[test]
fn rewrite_resurrect_state_uses_claude_session_id_when_available() {
    let session_id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
    let input = "\
pane\tagentctl\t2\t1\t:* \t0\t:\t:/repos/agent-manager-worktrees/review\t1\tnode\t:node /home/me/bin/claude
window\tagentctl\t2\t:agentctl__default__review__aaaaaaaa\t1\t:* \tabcd\t:
";
    let runs = vec![run(
        "review",
        AgentKind::Claude,
        Lifecycle::Active,
        "agentctl__default__review__aaaaaaaa",
        Some(session_id),
    )];

    let output = rewrite_resurrect_state(input, &runs, "agentctl");

    assert!(output.contains("\tnode\t:claude --resume 11111111-2222-3333-4444-555555555555\n"));
}

#[test]
fn rewrite_resurrect_state_leaves_unmanaged_and_inactive_panes_untouched() {
    let input = "\
pane\tother\t1\t1\t:* \t0\t:\t:/tmp\t1\tssh\t:ssh host
pane\tagentctl\t2\t1\t:* \t0\t:\t:/repos/agent-manager-worktrees/ended\t1\tnode\t:node /home/me/bin/codex
window\tother\t1\t:ops\t1\t:* \tabcd\t:
window\tagentctl\t2\t:agentctl__default__ended__bbbbbbbb\t1\t:* \tabcd\t:
";
    let runs = vec![run(
        "ended",
        AgentKind::Codex,
        Lifecycle::Ended,
        "agentctl__default__ended__bbbbbbbb",
        None,
    )];

    let output = rewrite_resurrect_state(input, &runs, "agentctl");

    assert!(output.contains("\tssh\t:ssh host\n"));
    assert!(output.contains("\tnode\t:node /home/me/bin/codex\n"));
    assert!(!output.contains(":codex resume --last"));
}

#[test]
fn tmux_config_upsert_adds_guarded_plugin_restore_block() {
    let config =
        upsert_agent_manager_tmux_config("set -g mouse on\n", "/opt/Agent Manager/agent-manager");

    assert!(config.contains("set -g mouse on\n"));
    assert!(config.contains("# >>> Agent Manager tmux restore >>>"));
    assert!(config.contains("set -g @plugin 'tmux-plugins/tmux-resurrect'"));
    assert!(config.contains("set -g @plugin 'tmux-plugins/tmux-continuum'"));
    assert!(config.contains("set -g @continuum-restore 'on'"));
    assert!(config.contains("set -g @continuum-boot 'on'"));
    assert!(config.contains("set -g @resurrect-processes 'codex claude'"));
    assert!(config.contains(
        "set -g @resurrect-hook-pre-restore-pane-processes \"'/opt/Agent Manager/agent-manager' __tmux-resurrect-rewrite\""
    ));
    assert!(config.contains("run '~/.tmux/plugins/tpm/tpm'"));
    assert!(config.contains("# <<< Agent Manager tmux restore <<<"));
}

#[test]
fn tmux_config_upsert_replaces_existing_agent_manager_block() {
    let existing = "\
set -g mouse on
# >>> Agent Manager tmux restore >>>
set -g @continuum-restore 'off'
# <<< Agent Manager tmux restore <<<
set -g status-left '#S'
";

    let config = upsert_agent_manager_tmux_config(existing, "/usr/bin/agent-manager");

    assert_eq!(
        config
            .matches("# >>> Agent Manager tmux restore >>>")
            .count(),
        1
    );
    assert_eq!(
        config
            .matches("# <<< Agent Manager tmux restore <<<")
            .count(),
        1
    );
    assert!(!config.contains("set -g @continuum-restore 'off'"));
    assert!(config.contains("set -g @continuum-restore 'on'"));
    assert!(config.contains("set -g mouse on\n"));
    assert!(config.contains("set -g status-left '#S'\n"));
}

#[test]
fn stable_agent_manager_executable_prefers_appimage_path_over_mount_binary() {
    let current_exe = Path::new("/tmp/.mount_Agent EbnBbO/usr/bin/agent-manager");

    let executable = stable_agent_manager_executable(
        Some(OsStr::new(
            "/home/me/Documents/target/release/bundle/appimage/Agent Manager_0.1.0_amd64.AppImage",
        )),
        current_exe,
    );

    assert_eq!(
        executable,
        PathBuf::from(
            "/home/me/Documents/target/release/bundle/appimage/Agent Manager_0.1.0_amd64.AppImage"
        )
    );
}

#[test]
fn stable_agent_manager_executable_falls_back_to_current_exe_without_appimage() {
    let current_exe = Path::new("/usr/bin/agent-manager");

    let executable = stable_agent_manager_executable(None, current_exe);

    assert_eq!(executable, PathBuf::from("/usr/bin/agent-manager"));
}

#[test]
fn refresh_tmux_restore_hook_rewrites_existing_stale_agent_manager_block() {
    let temp = tempfile::tempdir().unwrap();
    let paths = TmuxRestorePaths::for_home(temp.path());
    fs::write(
        paths.config_path(),
        upsert_agent_manager_tmux_config("set -g mouse on\n", "/old/worktree/agent-manager"),
    )
    .unwrap();

    let changed = refresh_tmux_restore_hook(&paths, Path::new("/usr/bin/agent-manager")).unwrap();
    let config = fs::read_to_string(paths.config_path()).unwrap();

    assert!(changed);
    assert!(config.contains("set -g mouse on\n"));
    assert!(!config.contains("/old/worktree/agent-manager"));
    assert!(config.contains("/usr/bin/agent-manager __tmux-resurrect-rewrite"));
}

#[test]
fn refresh_tmux_restore_hook_leaves_unmanaged_tmux_config_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let paths = TmuxRestorePaths::for_home(temp.path());
    fs::write(paths.config_path(), "set -g mouse on\n").unwrap();

    let changed = refresh_tmux_restore_hook(&paths, Path::new("/usr/bin/agent-manager")).unwrap();
    let config = fs::read_to_string(paths.config_path()).unwrap();

    assert!(!changed);
    assert_eq!(config, "set -g mouse on\n");
}

#[test]
fn tmux_restore_status_reports_missing_plugin_setup() {
    let temp = tempfile::tempdir().unwrap();
    let paths = TmuxRestorePaths::for_home(temp.path());

    let status = tmux_restore_status(&paths);

    assert!(!status.configured);
    assert!(!status.tpm_installed);
    assert!(!status.resurrect_installed);
    assert!(!status.continuum_installed);
    assert!(!status.saved_state_exists);
}

#[test]
fn tmux_restore_status_reports_configured_plugin_setup() {
    let temp = tempfile::tempdir().unwrap();
    let paths = TmuxRestorePaths::for_home(temp.path());
    fs::create_dir_all(paths.tpm_dir()).unwrap();
    fs::create_dir_all(paths.resurrect_dir()).unwrap();
    fs::create_dir_all(paths.continuum_dir()).unwrap();
    fs::create_dir_all(paths.resurrect_save_dir()).unwrap();
    fs::write(
        paths.config_path(),
        upsert_agent_manager_tmux_config("", "/usr/bin/agent-manager"),
    )
    .unwrap();
    fs::write(paths.last_resurrect_file(), "pane\tagentctl\n").unwrap();

    let status = tmux_restore_status(&paths);

    assert!(status.configured);
    assert!(status.tpm_installed);
    assert!(status.resurrect_installed);
    assert!(status.continuum_installed);
    assert!(status.saved_state_exists);
    assert_eq!(status.config_path, paths.config_path().to_string_lossy());
}

#[test]
fn tmux_restore_requires_a_saved_snapshot_and_missing_session() {
    let temp = tempfile::tempdir().unwrap();
    let paths = TmuxRestorePaths::for_home(temp.path());
    fs::create_dir_all(paths.tpm_dir()).unwrap();
    fs::create_dir_all(paths.resurrect_dir()).unwrap();
    fs::create_dir_all(paths.continuum_dir()).unwrap();
    fs::write(
        paths.config_path(),
        upsert_agent_manager_tmux_config("", "/usr/bin/agent-manager"),
    )
    .unwrap();

    let without_snapshot = tmux_restore_status(&paths);
    assert!(without_snapshot.configured);
    assert!(!without_snapshot.saved_state_exists);
    assert!(!tmux_restore_needed(&without_snapshot, false));
    restore_tmux_session_if_missing(&paths, "test-agentctl-no-snapshot").unwrap();

    fs::create_dir_all(paths.resurrect_save_dir()).unwrap();
    fs::write(paths.last_resurrect_file(), "pane\tagentctl\n").unwrap();
    let with_snapshot = tmux_restore_status(&paths);
    assert!(tmux_restore_needed(&with_snapshot, false));
    assert!(!tmux_restore_needed(&with_snapshot, true));
}
