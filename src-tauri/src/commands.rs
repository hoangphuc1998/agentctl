use std::{path::PathBuf, str::FromStr};

use agentctl_core::{
    agent::AgentKind,
    app::{
        ensure_codex_app_server, preview_ignored_untracked_files, App, AppConfig, CommandRunner,
        NewRunRequest, SystemCommandRunner,
    },
    branches::{BranchLister, GitBranchLister},
    completion::{base_ref_candidates, repo_path_candidates},
    diff::load_run_diff,
    domain::DetectionSource,
    registry::SqliteRegistry,
    tmux::{PaneSnapshot, Tmux},
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

use crate::{
    codex_status_client::load_codex_thread_statuses,
    error::{DesktopError, DesktopResult},
    mobile_bridge::{BridgeBind, MobileBridgeStatus, PairingCode, PairingTime},
    models::{
        host_tool_statuses, ActionResult, CreateRunPayload, DashboardState,
        IgnoredFilesPreviewView, MergeActionResult, RunDiffView, Suggestion, TerminalStarted,
    },
    services::{
        agent_attention_event_for_transition, agent_system_notification_for_event,
        build_dashboard_state, is_stale_run, mark_selected_run_seen, observe_run,
        observed_state_after_refresh, suggestions_from_candidates,
    },
    state::DesktopState,
    terminal_plan::{terminal_link_command, TerminalLinkTarget},
    tmux_restore::{
        agent_manager_executable_from_environment,
        enable_tmux_restore as enable_tmux_restore_setup, save_tmux_restore_snapshot_best_effort,
        tmux_restore_status as current_tmux_restore_status, TmuxRestorePaths, TmuxRestoreStatus,
    },
};

const TMUX_SESSION: &str = "agentctl";

#[tauri::command]
pub async fn dashboard_state(
    app_handle: AppHandle,
    state: State<'_, DesktopState>,
    selected_run_id: Option<String>,
) -> DesktopResult<DashboardState> {
    if selected_run_id.is_some() {
        state.set_selected_run_id(selected_run_id)?;
    }
    let registry_path = state.registry_path().clone();
    let selected = state.selected_run_id()?;
    let dashboard =
        blocking_task(move || load_dashboard(&registry_path, &app_handle, selected)).await?;
    state.set_selected_run_id(dashboard.selected_run_id.clone())?;
    Ok(dashboard)
}

#[tauri::command]
pub async fn create_run(
    state: State<'_, DesktopState>,
    payload: CreateRunPayload,
) -> DesktopResult<ActionResult> {
    let registry_path = state.registry_path().clone();
    let agent = AgentKind::from_str(&payload.agent).map_err(DesktopError::Message)?;
    let request = NewRunRequest {
        repo_path: PathBuf::from(payload.repo_path),
        base_ref: payload.base_ref,
        tag: payload.tag,
        run_name: payload.run_name,
        agent,
        copy_ignored_files: payload.copy_ignored_files,
    };
    let run = blocking_task(move || {
        let registry = SqliteRegistry::open(&registry_path)?;
        let mut app = App::new(registry, SystemCommandRunner, AppConfig::from_environment());
        app.create_run(request).map_err(DesktopError::from)
    })
    .await?;
    save_tmux_restore_snapshot_best_effort();
    state.set_selected_run_id(Some(run.id.to_string()))?;
    Ok(ActionResult {
        message: format!("Created `{}`.", run.run_name),
        run: Some(run.into()),
    })
}

#[tauri::command]
pub async fn ignored_files_preview(repo_path: String) -> DesktopResult<IgnoredFilesPreviewView> {
    blocking_task(move || {
        let repo_path = PathBuf::from(repo_path);
        let mut runner = SystemCommandRunner;
        preview_ignored_untracked_files(&mut runner, &repo_path)
            .map(IgnoredFilesPreviewView::from)
            .map_err(DesktopError::from)
    })
    .await
}

#[tauri::command]
pub fn restore_run(state: State<'_, DesktopState>, run_id: String) -> DesktopResult<ActionResult> {
    let id = parse_uuid(&run_id)?;
    let mut app = app(&state)?;
    let run = app.restore_run(id)?;
    if let Some(run) = run {
        save_tmux_restore_snapshot_best_effort();
        state.set_selected_run_id(Some(run.id.to_string()))?;
        Ok(ActionResult {
            message: format!("Resumed `{}`.", run.run_name),
            run: Some(run.into()),
        })
    } else {
        Ok(ActionResult {
            message: format!("Run not found: {run_id}"),
            run: None,
        })
    }
}

#[tauri::command]
pub fn stop_run(state: State<'_, DesktopState>, run_id: String) -> DesktopResult<ActionResult> {
    let id = parse_uuid(&run_id)?;
    let mut app = app(&state)?;
    app.stop_run(id)?;
    save_tmux_restore_snapshot_best_effort();
    Ok(ActionResult {
        message: "Stopped run. Worktree and branch preserved.".to_string(),
        run: None,
    })
}

#[tauri::command]
pub fn end_run(state: State<'_, DesktopState>, run_id: String) -> DesktopResult<ActionResult> {
    let id = parse_uuid(&run_id)?;
    let mut app = app(&state)?;
    app.end_run(id)?;
    save_tmux_restore_snapshot_best_effort();
    Ok(ActionResult {
        message: "Ended run. Worktree and branch removed.".to_string(),
        run: None,
    })
}

#[tauri::command]
pub fn merge_run(
    state: State<'_, DesktopState>,
    run_id: String,
) -> DesktopResult<Option<MergeActionResult>> {
    let id = parse_uuid(&run_id)?;
    let mut app = app(&state)?;
    let result = app.merge_run(id)?;
    Ok(result.map(|result| MergeActionResult {
        message: format!(
            "Merged `{}` into `{}`.",
            result.run.run_name, result.target_branch
        ),
        target_branch: result.target_branch,
        run: result.run.into(),
    }))
}

#[tauri::command]
pub fn open_in_vscode(
    state: State<'_, DesktopState>,
    run_id: String,
) -> DesktopResult<ActionResult> {
    let id = parse_uuid(&run_id)?;
    let mut app = app(&state)?;
    let run = app.open_run_in_vscode(id)?;
    Ok(ActionResult {
        message: "Opened run worktree in VS Code.".to_string(),
        run: run.map(Into::into),
    })
}

#[tauri::command]
pub fn run_diff(
    state: State<'_, DesktopState>,
    run_id: String,
) -> DesktopResult<Option<RunDiffView>> {
    let id = parse_uuid(&run_id)?;
    let registry = registry(&state)?;
    let Some(run) = registry.get_run(id)? else {
        return Ok(None);
    };
    Ok(Some(load_run_diff(&run)?.into()))
}

#[tauri::command]
pub fn cleanup_stale_runs(state: State<'_, DesktopState>) -> DesktopResult<ActionResult> {
    let registry = registry(&state)?;
    let runs = registry.list_active_runs()?;
    let ids = runs
        .iter()
        .filter(|run| is_stale_run(run))
        .map(|run| run.id)
        .collect::<Vec<_>>();
    let mut app = App::new(registry, SystemCommandRunner, AppConfig::from_environment());
    for id in &ids {
        app.stop_run(*id)?;
    }
    save_tmux_restore_snapshot_best_effort();
    Ok(ActionResult {
        message: format!("Stopped {} stale runs.", ids.len()),
        run: None,
    })
}

#[tauri::command]
pub fn tmux_restore_status() -> TmuxRestoreStatus {
    current_tmux_restore_status(&TmuxRestorePaths::from_environment())
}

#[tauri::command]
pub fn enable_tmux_restore() -> DesktopResult<TmuxRestoreStatus> {
    let paths = TmuxRestorePaths::from_environment();
    let executable = agent_manager_executable_from_environment()?;
    enable_tmux_restore_setup(&paths, &executable)?;
    Ok(current_tmux_restore_status(&paths))
}

#[tauri::command]
pub fn repo_suggestions(
    state: State<'_, DesktopState>,
    input: String,
) -> DesktopResult<Vec<Suggestion>> {
    let registry = registry(&state)?;
    let candidates = repo_path_candidates(&input, &registry.recent_repo_paths()?);
    Ok(suggestions_from_candidates(candidates))
}

#[tauri::command]
pub fn base_ref_suggestions(repo_path: String, input: String) -> DesktopResult<Vec<Suggestion>> {
    let mut lister = GitBranchLister;
    let branches = lister.list_branches(&PathBuf::from(repo_path))?;
    Ok(suggestions_from_candidates(base_ref_candidates(
        &input, &branches,
    )))
}

#[tauri::command]
pub fn mobile_bridge_status(state: State<'_, DesktopState>) -> DesktopResult<MobileBridgeStatus> {
    let pairing = state.mobile_pairing()?;
    Ok(state.mobile_bridge()?.status(&pairing))
}

#[tauri::command]
pub fn issue_mobile_pairing_code(state: State<'_, DesktopState>) -> DesktopResult<PairingCode> {
    Ok(state.mobile_pairing()?.issue_code(PairingTime::now()))
}

#[tauri::command]
pub fn revoke_mobile_device(
    state: State<'_, DesktopState>,
    device_id: String,
) -> DesktopResult<MobileBridgeStatus> {
    {
        let mut pairing = state.mobile_pairing()?;
        pairing.revoke_device(&device_id);
    }
    let pairing = state.mobile_pairing()?;
    Ok(state.mobile_bridge()?.status(&pairing))
}

#[tauri::command]
pub fn start_mobile_bridge(state: State<'_, DesktopState>) -> DesktopResult<MobileBridgeStatus> {
    let bind = BridgeBind::default();
    let bridge_state = state.bridge_server_state();
    {
        let mut runtime = state.mobile_bridge()?;
        runtime
            .start(bridge_state, bind)
            .map_err(DesktopError::Message)?;
    }
    let pairing = state.mobile_pairing()?;
    Ok(state.mobile_bridge()?.status(&pairing))
}

#[tauri::command]
pub fn stop_mobile_bridge(state: State<'_, DesktopState>) -> DesktopResult<MobileBridgeStatus> {
    state.mobile_bridge()?.stop();
    let pairing = state.mobile_pairing()?;
    Ok(state.mobile_bridge()?.status(&pairing))
}

#[tauri::command]
pub fn start_terminal(
    app_handle: AppHandle,
    state: State<'_, DesktopState>,
    run_id: String,
    cols: u16,
    rows: u16,
) -> DesktopResult<TerminalStarted> {
    let id = parse_uuid(&run_id)?;
    let registry = registry(&state)?;
    let Some(run) = registry.get_run(id)? else {
        return Err(DesktopError::Message(format!("run not found: {run_id}")));
    };
    let terminal_id = state
        .terminals()?
        .start(app_handle, &run, TMUX_SESSION, cols, rows)?;
    Ok(TerminalStarted {
        terminal_id,
        run_id,
    })
}

#[tauri::command]
pub fn terminal_input(
    state: State<'_, DesktopState>,
    terminal_id: String,
    data: String,
) -> DesktopResult<()> {
    state.terminals()?.input(&terminal_id, &data)
}

#[tauri::command]
pub async fn open_terminal_link(
    state: State<'_, DesktopState>,
    run_id: String,
    target: TerminalLinkTarget,
) -> DesktopResult<()> {
    let id = parse_uuid(&run_id)?;
    let registry_path = state.registry_path().clone();
    blocking_task(move || {
        let registry = SqliteRegistry::open(&registry_path)?;
        let Some(run) = registry.get_run(id)? else {
            return Err(DesktopError::Message(format!("run not found: {run_id}")));
        };
        let command = terminal_link_command(&run, &target).map_err(DesktopError::Message)?;
        let mut runner = SystemCommandRunner;
        let mut args = Vec::with_capacity(command.args.len() + 1);
        args.push(command.program);
        args.extend(command.args);
        runner.run(&args)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub fn resize_terminal(
    state: State<'_, DesktopState>,
    terminal_id: String,
    cols: u16,
    rows: u16,
) -> DesktopResult<()> {
    state.terminals()?.resize(&terminal_id, cols, rows)
}

#[tauri::command]
pub fn close_terminal(state: State<'_, DesktopState>, terminal_id: String) -> DesktopResult<()> {
    state.terminals()?.close(&terminal_id)
}

fn registry(state: &DesktopState) -> DesktopResult<SqliteRegistry> {
    SqliteRegistry::open(state.registry_path()).map_err(DesktopError::from)
}

fn app(state: &DesktopState) -> DesktopResult<App<SystemCommandRunner>> {
    Ok(App::new(
        registry(state)?,
        SystemCommandRunner,
        AppConfig::from_environment(),
    ))
}

async fn blocking_task<T, F>(task: F) -> DesktopResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> DesktopResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|err| DesktopError::Message(format!("blocking task failed: {err}")))?
}

fn parse_uuid(value: &str) -> DesktopResult<Uuid> {
    Uuid::parse_str(value).map_err(|err| DesktopError::Message(err.to_string()))
}

fn load_dashboard(
    registry_path: &std::path::Path,
    app_handle: &AppHandle,
    selected_run_id: Option<String>,
) -> DesktopResult<DashboardState> {
    let registry = SqliteRegistry::open(registry_path)?;
    if let Ok(cwd) = std::env::current_dir() {
        let mut runner = SystemCommandRunner;
        if let Err(err) = ensure_codex_app_server(&mut runner, &cwd) {
            eprintln!("failed to ensure Codex app-server before status refresh: {err}");
        }
    }
    let codex_threads = load_codex_thread_statuses().unwrap_or_else(|err| {
        eprintln!("failed to load official Codex status; using tmux fallback: {err}");
        Vec::new()
    });
    let mut runs = refresh_active_runs(&registry, app_handle, &codex_threads)?;
    if let Some(run) = mark_selected_run_seen(
        &mut runs,
        selected_run_id.as_deref(),
        chrono::Utc::now().timestamp(),
    ) {
        registry.upsert_run(&run)?;
    }
    Ok(build_dashboard_state(
        runs,
        registry.active_repo_path()?,
        host_tool_statuses(),
        selected_run_id,
    ))
}

fn refresh_active_runs(
    registry: &SqliteRegistry,
    app_handle: &AppHandle,
    codex_threads: &[agentctl_core::codex_status::CodexThreadSnapshot],
) -> DesktopResult<Vec<agentctl_core::domain::RunRecord>> {
    let tmux = Tmux::new(TMUX_SESSION);
    let mut runs = registry.list_active_runs()?;
    for run in &mut runs {
        let snapshot = match run.tmux_window.as_deref() {
            Some(window) => match tmux.snapshot_window(window) {
                Ok(snapshot) => Some(snapshot),
                Err(err) => {
                    eprintln!(
                        "failed to refresh tmux state for run {} ({}): {err}",
                        run.id, window
                    );
                    None
                }
            },
            None => None,
        };
        let previous_state = run.observed_state;
        let unavailable_snapshot = unavailable_pane_snapshot();
        let observation = observe_run(
            run,
            snapshot.as_ref().unwrap_or(&unavailable_snapshot),
            codex_threads,
        );
        if snapshot.is_none() && observation.source != DetectionSource::Provider {
            continue;
        }
        let state = observed_state_after_refresh(observation.state, run.notification_seen_at);
        run.observed_state = state;
        run.detection_source = observation.source;
        run.agent_session_id = observation.agent_session_id;
        run.updated_at = chrono::Utc::now().timestamp();
        registry.upsert_run(run)?;
        if let Some(event) = agent_attention_event_for_transition(previous_state, run) {
            notify_agent_attention(app_handle, &event);
        }
    }
    Ok(runs)
}

fn unavailable_pane_snapshot() -> PaneSnapshot {
    PaneSnapshot {
        pane_active: false,
        current_command: String::new(),
        visible_text: String::new(),
    }
}

fn notify_agent_attention(app_handle: &AppHandle, event: &crate::models::AgentAttentionEvent) {
    let (title, body) = agent_system_notification_for_event(event);
    if let Err(err) = app_handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        eprintln!("failed to show agent attention notification: {err}");
    }

    if let Err(err) = app_handle.emit("agent:attention", event) {
        eprintln!("failed to emit agent attention event: {err}");
    }
}
