use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

use uuid::Uuid;

use crate::{
    agent::{AgentKind, LaunchPlan},
    commands::{
        login_shell_command, shell_command_with_failure_diagnostics, shell_join,
        AgentCommandBuilder, EditorCommandBuilder, GitCommandBuilder, TerminalColorEnvironment,
        TmuxCommandBuilder, CODEX_APP_SERVER_READY_URL, CODEX_APP_SERVER_TMUX_SESSION,
        CODEX_APP_SERVER_URL,
    },
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord, WorkspaceKind},
    registry::{RegistryResult, SqliteRegistry},
    tmux::window_list_contains,
    untracked_files::{
        copy_untracked_files, delete_untracked_files, preview_untracked_files,
        UntrackedFilesPreview,
    },
    worktree::{default_branch_name, default_sibling_worktree_path, sanitize_slug},
};

const TMUX_WINDOW_VERIFY_ATTEMPTS: usize = 5;
const TMUX_WINDOW_VERIFY_DELAY_MS: u64 = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub tmux_session: String,
    pub terminal_color_environment: Option<TerminalColorEnvironment>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tmux_session: "agentctl".to_string(),
            terminal_color_environment: None,
        }
    }
}

impl AppConfig {
    pub fn from_environment() -> Self {
        Self {
            tmux_session: "agentctl".to_string(),
            terminal_color_environment: Some(TerminalColorEnvironment::capture()),
        }
    }

    pub fn for_session_from_environment(session: impl Into<String>) -> Self {
        Self {
            tmux_session: session.into(),
            terminal_color_environment: Some(TerminalColorEnvironment::capture()),
        }
    }

    pub fn for_session(session: impl Into<String>) -> Self {
        Self {
            tmux_session: session.into(),
            terminal_color_environment: None,
        }
    }

    pub fn with_terminal_color_environment(
        session: impl Into<String>,
        terminal_color_environment: TerminalColorEnvironment,
    ) -> Self {
        Self {
            tmux_session: session.into(),
            terminal_color_environment: Some(terminal_color_environment),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewRunRequest {
    pub repo_path: PathBuf,
    pub base_ref: String,
    pub tag: String,
    pub run_name: String,
    pub agent: AgentKind,
    pub copy_ignored_files: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewFolderSessionRequest {
    pub folder_path: PathBuf,
    pub tag: String,
    pub run_name: String,
    pub agent: AgentKind,
}

pub trait CommandRunner {
    fn run(&mut self, command: &[String]) -> io::Result<()>;
    fn succeeds(&mut self, command: &[String]) -> io::Result<bool>;
    fn output(&mut self, command: &[String]) -> io::Result<String>;
}

#[derive(Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&mut self, command: &[String]) -> io::Result<()> {
        let output = command_output(command)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(command_error(command, &output))
        }
    }

    fn succeeds(&mut self, command: &[String]) -> io::Result<bool> {
        Ok(command_output(command)?.status.success())
    }

    fn output(&mut self, command: &[String]) -> io::Result<String> {
        let output = command_output(command)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(command_error(command, &output))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeRunResult {
    pub run: RunRecord,
    pub target_branch: String,
}

pub struct App<R> {
    registry: SqliteRegistry,
    runner: R,
    config: AppConfig,
}

impl<R> App<R>
where
    R: CommandRunner,
{
    pub fn new(registry: SqliteRegistry, runner: R, config: AppConfig) -> Self {
        Self {
            registry,
            runner,
            config,
        }
    }

    pub fn registry(&self) -> &SqliteRegistry {
        &self.registry
    }

    pub fn create_run(&mut self, request: NewRunRequest) -> RegistryResult<RunRecord> {
        create_run_with_registry(&self.registry, &mut self.runner, &self.config, request)
    }

    pub fn create_folder_session(
        &mut self,
        request: NewFolderSessionRequest,
    ) -> RegistryResult<RunRecord> {
        create_folder_session_with_registry(&self.registry, &mut self.runner, &self.config, request)
    }

    pub fn end_run(&mut self, id: Uuid) -> RegistryResult<()> {
        end_run_with_registry(&self.registry, &mut self.runner, &self.config, id)
    }

    pub fn stop_run(&mut self, id: Uuid) -> RegistryResult<()> {
        stop_run_with_registry(&self.registry, &mut self.runner, &self.config, id)
    }

    pub fn open_run_in_vscode(&mut self, id: Uuid) -> RegistryResult<Option<RunRecord>> {
        open_run_in_vscode_with_registry(&self.registry, &mut self.runner, id)
    }

    pub fn merge_run(&mut self, id: Uuid) -> RegistryResult<Option<MergeRunResult>> {
        merge_run_with_registry(&self.registry, &mut self.runner, id)
    }

    pub fn close_and_delete_run(&mut self, id: Uuid) -> RegistryResult<()> {
        close_and_delete_run_with_registry(&self.registry, &mut self.runner, &self.config, id)
    }

    pub fn restore_run(&mut self, id: Uuid) -> RegistryResult<Option<RunRecord>> {
        let Some(mut run) = self.registry.get_run(id)? else {
            return Ok(None);
        };

        let tmux = TmuxCommandBuilder::new(
            run.tmux_session
                .as_deref()
                .unwrap_or(&self.config.tmux_session),
        );
        ensure_tmux_session(
            &mut self.runner,
            &tmux,
            self.config.terminal_color_environment.as_ref(),
        )?;
        if run.agent == AgentKind::Codex {
            ensure_codex_app_server(&mut self.runner, &run.worktree_path)?;
        }

        let command = AgentCommandBuilder::new().restore(LaunchPlan {
            agent: run.agent,
            worktree_path: run.worktree_path.clone(),
            session_id: run.agent_session_id,
        });
        let window = run.tmux_window.clone().unwrap_or_else(|| {
            window_name(
                &run.repo_name,
                &run.tag,
                &sanitize_slug(&run.run_name),
                run.id,
            )
        });
        let new_window = tmux.new_window(
            &window,
            path_str(&run.worktree_path),
            &shell_command_with_failure_diagnostics(&command),
        );
        self.runner.run(&new_window)?;
        ensure_tmux_window(&mut self.runner, &tmux, &window)?;

        let now = now_ts();
        run.lifecycle = Lifecycle::Active;
        run.observed_state = ObservedState::Running;
        run.detection_source = DetectionSource::Tmux;
        run.tmux_window = Some(window);
        run.updated_at = now;
        self.registry.upsert_run(&run)?;
        Ok(Some(run))
    }
}

pub fn end_run_with_registry<R>(
    registry: &SqliteRegistry,
    runner: &mut R,
    config: &AppConfig,
    id: Uuid,
) -> RegistryResult<()>
where
    R: CommandRunner,
{
    close_and_delete_run_with_registry(registry, runner, config, id)
}

pub fn stop_run_with_registry<R>(
    registry: &SqliteRegistry,
    runner: &mut R,
    config: &AppConfig,
    id: Uuid,
) -> RegistryResult<()>
where
    R: CommandRunner,
{
    let now = now_ts();
    if let Some(run) = registry.get_run(id)? {
        if let Some(window) = run.tmux_window.as_deref() {
            let command = TmuxCommandBuilder::new(
                run.tmux_session.as_deref().unwrap_or(&config.tmux_session),
            )
            .kill_window(window);
            let _ = runner.run(&command);
        }
    }
    registry.set_lifecycle(id, Lifecycle::Stopped, now)
}

pub fn open_run_in_vscode_with_registry<R>(
    registry: &SqliteRegistry,
    runner: &mut R,
    id: Uuid,
) -> RegistryResult<Option<RunRecord>>
where
    R: CommandRunner,
{
    let Some(run) = registry.get_run(id)? else {
        return Ok(None);
    };
    let command = EditorCommandBuilder::new().open(path_str(&run.worktree_path));
    runner.run(&command)?;
    Ok(Some(run))
}

pub fn merge_run_with_registry<R>(
    registry: &SqliteRegistry,
    runner: &mut R,
    id: Uuid,
) -> RegistryResult<Option<MergeRunResult>>
where
    R: CommandRunner,
{
    let Some(run) = registry.get_run(id)? else {
        return Ok(None);
    };
    if run.is_folder() {
        return Err("folder sessions do not support Git merge".into());
    }
    let git = GitCommandBuilder::new();
    ensure_clean(runner, &git, &run.repo_path, "repository")?;
    ensure_clean(runner, &git, &run.worktree_path, "worktree")?;
    let target_branch = default_merge_target(runner, &git, &run.repo_path);
    runner.run(&git.checkout_branch(path_str(&run.repo_path), &target_branch))?;
    if let Err(err) = runner.run(&git.merge_branch(path_str(&run.repo_path), &run.branch)) {
        let _ = runner.run(&git.merge_abort(path_str(&run.repo_path)));
        return Err(err.into());
    }
    Ok(Some(MergeRunResult { run, target_branch }))
}

pub fn close_and_delete_run_with_registry<R>(
    registry: &SqliteRegistry,
    runner: &mut R,
    config: &AppConfig,
    id: Uuid,
) -> RegistryResult<()>
where
    R: CommandRunner,
{
    let now = now_ts();
    if let Some(run) = registry.get_run(id)? {
        if let Some(window) = run.tmux_window.as_deref() {
            let command = TmuxCommandBuilder::new(
                run.tmux_session.as_deref().unwrap_or(&config.tmux_session),
            )
            .kill_window(window);
            let _ = runner.run(&command);
        }
        if run.is_worktree() {
            let git = GitCommandBuilder::new();
            let untracked_files =
                runner.output(&git.nonignored_untracked_files(path_str(&run.worktree_path)))?;
            delete_untracked_files(&run.worktree_path, &untracked_files)?;
            runner.run(
                &git.remove_worktree(path_str(&run.repo_path), path_str(&run.worktree_path)),
            )?;
            runner.run(&git.delete_branch(path_str(&run.repo_path), &run.branch))?;
        }
    }
    registry.set_lifecycle(id, Lifecycle::Ended, now)
}

pub fn create_run_with_registry<R>(
    registry: &SqliteRegistry,
    runner: &mut R,
    config: &AppConfig,
    request: NewRunRequest,
) -> RegistryResult<RunRecord>
where
    R: CommandRunner,
{
    let now = now_ts();
    let id = Uuid::new_v4();
    let repo_name = repo_name(&request.repo_path);
    let tag = sanitize_slug(&request.tag);
    let run_slug = sanitize_slug(&request.run_name);
    let branch = default_branch_name(&request.run_name);
    let worktree_path = default_sibling_worktree_path(&request.repo_path, &branch);
    let git = GitCommandBuilder::new();
    let base_commit = runner
        .output(&git.rev_parse_commit(path_str(&request.repo_path), &request.base_ref))?
        .trim()
        .to_string();
    let tmux_window = window_name(&repo_name, &tag, &run_slug, id);
    let agent_session_id = match request.agent {
        AgentKind::Codex => None,
        AgentKind::Claude => Some(Uuid::new_v4()),
    };

    let tmux = TmuxCommandBuilder::new(&config.tmux_session);
    ensure_tmux_session(runner, &tmux, config.terminal_color_environment.as_ref())?;

    let add_worktree = git.add_worktree(
        path_str(&request.repo_path),
        path_str(&worktree_path),
        &branch,
        &request.base_ref,
    );
    runner.run(&add_worktree)?;
    let list_untracked_files = if request.copy_ignored_files {
        git.all_untracked_files(path_str(&request.repo_path))
    } else {
        git.nonignored_untracked_files(path_str(&request.repo_path))
    };
    let untracked_files = match runner.output(&list_untracked_files) {
        Ok(untracked_files) => untracked_files,
        Err(err) => {
            rollback_created_resources(
                runner,
                &tmux,
                &git,
                &request.repo_path,
                &worktree_path,
                &branch,
                &tmux_window,
                false,
            );
            return Err(err.into());
        }
    };
    if let Err(err) = copy_untracked_files(&request.repo_path, &worktree_path, &untracked_files) {
        rollback_created_resources(
            runner,
            &tmux,
            &git,
            &request.repo_path,
            &worktree_path,
            &branch,
            &tmux_window,
            false,
        );
        return Err(err.into());
    }

    if request.agent == AgentKind::Codex {
        if let Err(err) = ensure_codex_app_server(runner, &worktree_path) {
            rollback_created_resources(
                runner,
                &tmux,
                &git,
                &request.repo_path,
                &worktree_path,
                &branch,
                &tmux_window,
                false,
            );
            return Err(err.into());
        }
    }

    let agent_command = AgentCommandBuilder::new().launch(LaunchPlan {
        agent: request.agent,
        worktree_path: worktree_path.clone(),
        session_id: agent_session_id,
    });
    let new_window = tmux.new_window(
        &tmux_window,
        path_str(&worktree_path),
        &shell_command_with_failure_diagnostics(&agent_command),
    );
    if let Err(err) = runner.run(&new_window) {
        rollback_created_resources(
            runner,
            &tmux,
            &git,
            &request.repo_path,
            &worktree_path,
            &branch,
            &tmux_window,
            false,
        );
        return Err(err.into());
    }
    if let Err(err) = ensure_tmux_window(runner, &tmux, &tmux_window) {
        rollback_created_resources(
            runner,
            &tmux,
            &git,
            &request.repo_path,
            &worktree_path,
            &branch,
            &tmux_window,
            true,
        );
        return Err(err.into());
    }

    let run = RunRecord {
        id,
        workspace_kind: WorkspaceKind::Worktree,
        repo_path: request.repo_path,
        repo_name,
        tag,
        run_name: request.run_name,
        agent: request.agent,
        lifecycle: Lifecycle::Active,
        observed_state: ObservedState::Running,
        detection_source: DetectionSource::Tmux,
        branch,
        base_ref: request.base_ref,
        base_commit: Some(base_commit),
        worktree_path,
        tmux_session: Some(config.tmux_session.clone()),
        tmux_window: Some(tmux_window),
        tmux_pane: None,
        agent_session_id,
        notification_seen_at: None,
        created_at: now,
        updated_at: now,
    };
    if let Err(err) = registry.upsert_run(&run) {
        rollback_created_resources(
            runner,
            &tmux,
            &git,
            &run.repo_path,
            &run.worktree_path,
            &run.branch,
            run.tmux_window.as_deref().unwrap_or_default(),
            true,
        );
        return Err(err);
    }
    registry.set_active_repo_path(&run.repo_path)?;
    Ok(run)
}

pub fn create_folder_session_with_registry<R>(
    registry: &SqliteRegistry,
    runner: &mut R,
    config: &AppConfig,
    request: NewFolderSessionRequest,
) -> RegistryResult<RunRecord>
where
    R: CommandRunner,
{
    let folder_path = fs::canonicalize(&request.folder_path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "failed to resolve folder {}: {err}",
                request.folder_path.display()
            ),
        )
    })?;
    if !folder_path.is_dir() {
        return Err(format!("folder path is not a directory: {}", folder_path.display()).into());
    }

    let now = now_ts();
    let id = Uuid::new_v4();
    let folder_name = workspace_name(&folder_path);
    let tag = sanitize_slug(&request.tag);
    let run_slug = sanitize_slug(&request.run_name);
    let tmux_window = window_name(&folder_name, &tag, &run_slug, id);
    let agent_session_id = match request.agent {
        AgentKind::Codex => None,
        AgentKind::Claude => Some(Uuid::new_v4()),
    };
    let tmux = TmuxCommandBuilder::new(&config.tmux_session);
    ensure_tmux_session(runner, &tmux, config.terminal_color_environment.as_ref())?;
    if request.agent == AgentKind::Codex {
        ensure_codex_app_server(runner, &folder_path)?;
    }

    let agent_command = AgentCommandBuilder::new().launch(LaunchPlan {
        agent: request.agent,
        worktree_path: folder_path.clone(),
        session_id: agent_session_id,
    });
    let new_window = tmux.new_window(
        &tmux_window,
        path_str(&folder_path),
        &shell_command_with_failure_diagnostics(&agent_command),
    );
    runner.run(&new_window)?;
    if let Err(err) = ensure_tmux_window(runner, &tmux, &tmux_window) {
        let _ = runner.run(&tmux.kill_window(&tmux_window));
        return Err(err.into());
    }

    let session = RunRecord {
        id,
        workspace_kind: WorkspaceKind::Folder,
        repo_path: folder_path.clone(),
        repo_name: folder_name,
        tag,
        run_name: request.run_name,
        agent: request.agent,
        lifecycle: Lifecycle::Active,
        observed_state: ObservedState::Running,
        detection_source: DetectionSource::Tmux,
        branch: String::new(),
        base_ref: String::new(),
        base_commit: None,
        worktree_path: folder_path.clone(),
        tmux_session: Some(config.tmux_session.clone()),
        tmux_window: Some(tmux_window.clone()),
        tmux_pane: None,
        agent_session_id,
        notification_seen_at: None,
        created_at: now,
        updated_at: now,
    };
    if let Err(err) = registry.upsert_run(&session) {
        let _ = runner.run(&tmux.kill_window(&tmux_window));
        return Err(err);
    }
    registry.set_active_folder_path(&folder_path)?;
    Ok(session)
}

pub fn preview_ignored_untracked_files<R>(
    runner: &mut R,
    repo_path: &Path,
) -> RegistryResult<UntrackedFilesPreview>
where
    R: CommandRunner,
{
    let git = GitCommandBuilder::new();
    let untracked_files = runner.output(&git.ignored_untracked_files(path_str(repo_path)))?;
    Ok(preview_untracked_files(repo_path, &untracked_files)?)
}

fn ensure_tmux_window<R>(runner: &mut R, tmux: &TmuxCommandBuilder, window: &str) -> io::Result<()>
where
    R: CommandRunner,
{
    for attempt in 0..TMUX_WINDOW_VERIFY_ATTEMPTS {
        let windows = runner.output(&tmux.list_windows())?;
        if window_list_contains(&windows, window) {
            return Ok(());
        }
        if attempt + 1 < TMUX_WINDOW_VERIFY_ATTEMPTS {
            std::thread::sleep(Duration::from_millis(TMUX_WINDOW_VERIFY_DELAY_MS));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("tmux window was not created or exited immediately: {window}"),
    ))
}

fn ensure_tmux_session<R>(
    runner: &mut R,
    tmux: &TmuxCommandBuilder,
    terminal_color_environment: Option<&TerminalColorEnvironment>,
) -> io::Result<()>
where
    R: CommandRunner,
{
    if !runner.succeeds(&tmux.has_session())? {
        runner.run(&tmux.new_detached_session())?;
    }
    configure_tmux_terminal_setup(runner, tmux, terminal_color_environment)
}

pub fn ensure_codex_app_server<R>(runner: &mut R, cwd: &Path) -> io::Result<()>
where
    R: CommandRunner,
{
    let tmux = TmuxCommandBuilder::new(CODEX_APP_SERVER_TMUX_SESSION);
    let server_cwd = codex_app_server_working_directory(cwd);
    if runner.succeeds(&tmux.has_session())? {
        let current_path = runner.output(&tmux.pane_current_path("app-server"))?;
        if Path::new(current_path.trim()) == server_cwd
            && runner.succeeds(&codex_app_server_ready_command())?
        {
            return Ok(());
        }
        runner.run(&tmux.kill_session())?;
    }

    let command = login_shell_command(&[
        "codex".to_string(),
        "app-server".to_string(),
        "--listen".to_string(),
        CODEX_APP_SERVER_URL.to_string(),
    ]);
    runner.run(&tmux.new_service_session("app-server", path_str(&server_cwd), &command))
}

fn codex_app_server_ready_command() -> Vec<String> {
    vec![
        "curl".to_string(),
        "--fail".to_string(),
        "--silent".to_string(),
        "--show-error".to_string(),
        "--max-time".to_string(),
        "1".to_string(),
        CODEX_APP_SERVER_READY_URL.to_string(),
    ]
}

fn codex_app_server_working_directory(fallback: &Path) -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| fallback.to_path_buf())
}

fn configure_tmux_terminal_setup<R>(
    runner: &mut R,
    tmux: &TmuxCommandBuilder,
    terminal_color_environment: Option<&TerminalColorEnvironment>,
) -> io::Result<()>
where
    R: CommandRunner,
{
    for command in tmux.terminal_setup_commands() {
        runner.run(&command)?;
    }
    if let Some(environment) = terminal_color_environment {
        for command in tmux.terminal_color_environment_commands(environment) {
            runner.run(&command)?;
        }
        if environment.uses_truecolor() {
            let current = runner.output(&tmux.show_global_option("terminal-overrides"))?;
            if let Some(updated) = TmuxCommandBuilder::terminal_overrides_with_rgb(&current) {
                runner.run(&tmux.set_server_option("terminal-overrides", &updated))?;
            }
        }
    }
    Ok(())
}

fn rollback_created_resources<R>(
    runner: &mut R,
    tmux: &TmuxCommandBuilder,
    git: &GitCommandBuilder,
    repo_path: &Path,
    worktree_path: &Path,
    branch: &str,
    tmux_window: &str,
    window_created: bool,
) where
    R: CommandRunner,
{
    if window_created {
        let _ = runner.run(&tmux.kill_window(tmux_window));
    }
    let _ = runner.run(&git.remove_worktree(path_str(repo_path), path_str(worktree_path)));
    let _ = runner.run(&git.delete_branch(path_str(repo_path), branch));
}

fn ensure_clean<R>(
    runner: &mut R,
    git: &GitCommandBuilder,
    path: &Path,
    label: &str,
) -> RegistryResult<()>
where
    R: CommandRunner,
{
    let status = runner.output(&git.status_porcelain(path_str(path)))?;
    if status.trim().is_empty() {
        Ok(())
    } else {
        Err(format!("{label} has uncommitted changes: {}", path.display()).into())
    }
}

fn default_merge_target<R>(runner: &mut R, git: &GitCommandBuilder, repo_path: &Path) -> String
where
    R: CommandRunner,
{
    if let Some(branch) = runner
        .output(&git.origin_head_ref(path_str(repo_path)))
        .ok()
        .and_then(|value| parse_origin_head(&value))
    {
        return branch;
    }

    runner
        .output(&git.local_branches(path_str(repo_path)))
        .ok()
        .and_then(|value| parse_local_default_branch(&value))
        .unwrap_or_else(|| "master".to_string())
}

fn parse_origin_head(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    value
        .rsplit_once('/')
        .map(|(_, branch)| branch)
        .or(Some(value))
        .filter(|branch| !branch.is_empty())
        .map(str::to_string)
}

fn parse_local_default_branch(value: &str) -> Option<String> {
    let branches = value
        .lines()
        .map(str::trim)
        .filter(|branch| !branch.is_empty())
        .collect::<Vec<_>>();
    if branches.contains(&"main") {
        return Some("main".to_string());
    }
    if branches.contains(&"master") {
        return Some("master".to_string());
    }
    branches.first().map(|branch| (*branch).to_string())
}

fn repo_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string()
}

fn workspace_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_str().unwrap_or("folder"))
        .to_string()
}

fn window_name(repo_name: &str, tag: &str, run_slug: &str, id: Uuid) -> String {
    let id = id.simple().to_string();
    format!(
        "{}__{}__{}__{}",
        sanitize_slug(repo_name),
        sanitize_slug(tag),
        sanitize_slug(run_slug),
        &id[..8],
    )
}

fn path_str(path: &Path) -> &str {
    path.to_str()
        .expect("agentctl currently supports UTF-8 filesystem paths")
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn command_output(command: &[String]) -> io::Result<Output> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty command"))?;
    Command::new(program).args(args).output()
}

fn command_error(command: &[String], output: &Output) -> io::Error {
    let mut message = format!(
        "command failed with status {}: {}",
        output.status,
        shell_join(command)
    );
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        message.push_str("\nstderr: ");
        message.push_str(&stderr);
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        message.push_str("\nstdout: ");
        message.push_str(&stdout);
    }
    io::Error::other(message)
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::{
        agent::AgentKind,
        commands::{CODEX_APP_SERVER_READY_URL, CODEX_APP_SERVER_TMUX_SESSION},
        registry::SqliteRegistry,
    };

    use super::{
        close_and_delete_run_with_registry, create_folder_session_with_registry,
        create_run_with_registry, ensure_codex_app_server, merge_run_with_registry,
        open_run_in_vscode_with_registry, path_str, preview_ignored_untracked_files, AppConfig,
        CommandRunner, NewFolderSessionRequest, NewRunRequest,
    };

    #[derive(Default)]
    struct RecordingRunner {
        commands: Vec<Vec<String>>,
        created_window_visible_after_list_calls: Option<usize>,
        created_window: Option<String>,
        list_windows_calls: usize,
        rev_parse_output: String,
        untracked_files_output: String,
        codex_server_exists: bool,
        codex_server_ready: bool,
        codex_server_path: Option<String>,
    }

    impl CommandRunner for RecordingRunner {
        fn run(&mut self, command: &[String]) -> io::Result<()> {
            if command_contains(command, "new-window") {
                self.created_window = window_name_arg(command);
            }
            self.commands.push(command.to_vec());
            Ok(())
        }

        fn succeeds(&mut self, command: &[String]) -> io::Result<bool> {
            self.commands.push(command.to_vec());
            if command_contains(command, "has-session")
                && command_contains(command, CODEX_APP_SERVER_TMUX_SESSION)
            {
                return Ok(self.codex_server_exists);
            }
            Ok(self.codex_server_ready
                && command_contains(command, "curl")
                && command_contains(command, CODEX_APP_SERVER_READY_URL))
        }

        fn output(&mut self, command: &[String]) -> io::Result<String> {
            self.commands.push(command.to_vec());
            if command_contains(command, "ls-files") {
                return Ok(self.untracked_files_output.clone());
            }
            if command_contains(command, "rev-parse") {
                return Ok(if self.rev_parse_output.is_empty() {
                    "abc123\n".to_string()
                } else {
                    self.rev_parse_output.clone()
                });
            }
            if command_contains(command, "list-windows") {
                self.list_windows_calls += 1;
                if let (Some(window), Some(visible_after)) = (
                    &self.created_window,
                    self.created_window_visible_after_list_calls,
                ) {
                    if self.list_windows_calls >= visible_after {
                        return Ok(format!("dashboard\n{window}\n"));
                    }
                }
            }
            if command_contains(command, "display-message")
                && command
                    .iter()
                    .any(|part| part.contains(CODEX_APP_SERVER_TMUX_SESSION))
            {
                return Ok(self.codex_server_path.clone().unwrap_or_default());
            }
            Ok("dashboard\n".to_string())
        }
    }

    #[test]
    fn codex_app_server_replaces_session_from_stale_appimage_directory() {
        let mut runner = RecordingRunner {
            codex_server_exists: true,
            codex_server_path: Some("/tmp/.mount_Agent-old/usr\n".to_string()),
            ..RecordingRunner::default()
        };

        ensure_codex_app_server(&mut runner, std::path::Path::new("/repo"))
            .expect("replace stale server");

        assert!(runner
            .commands
            .iter()
            .any(|command| command_contains(command, "kill-session")));
        assert!(runner.commands.iter().any(|command| {
            command_contains(command, "new-session")
                && command_contains(command, CODEX_APP_SERVER_TMUX_SESSION)
        }));
    }

    #[test]
    fn codex_app_server_replaces_unhealthy_session_from_stable_directory() {
        let fallback = std::path::Path::new("/repo");
        let stable_path = super::codex_app_server_working_directory(fallback);
        let mut runner = RecordingRunner {
            codex_server_exists: true,
            codex_server_ready: false,
            codex_server_path: Some(format!("{}\n", stable_path.display())),
            ..RecordingRunner::default()
        };

        ensure_codex_app_server(&mut runner, fallback).expect("replace unhealthy server");

        assert!(runner.commands.iter().any(|command| {
            command_contains(command, "curl")
                && command_contains(command, CODEX_APP_SERVER_READY_URL)
        }));
        assert!(runner
            .commands
            .iter()
            .any(|command| command_contains(command, "kill-session")));
        assert!(runner.commands.iter().any(|command| {
            command_contains(command, "new-session")
                && command_contains(command, CODEX_APP_SERVER_TMUX_SESSION)
        }));
    }

    #[test]
    fn codex_app_server_preserves_healthy_session_from_stable_directory() {
        let fallback = std::path::Path::new("/repo");
        let stable_path = super::codex_app_server_working_directory(fallback);
        let mut runner = RecordingRunner {
            codex_server_exists: true,
            codex_server_ready: true,
            codex_server_path: Some(format!("{}\n", stable_path.display())),
            ..RecordingRunner::default()
        };

        ensure_codex_app_server(&mut runner, fallback).expect("preserve healthy server");

        assert!(runner.commands.iter().any(|command| {
            command_contains(command, "curl")
                && command_contains(command, CODEX_APP_SERVER_READY_URL)
        }));
        assert!(!runner
            .commands
            .iter()
            .any(|command| command_contains(command, "kill-session")));
        assert!(!runner
            .commands
            .iter()
            .any(|command| command_contains(command, "new-session")));
    }

    #[test]
    fn create_run_persists_when_tmux_window_appears_after_retry() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner {
            created_window_visible_after_list_calls: Some(2),
            ..RecordingRunner::default()
        };
        let repo_root = tempfile::tempdir().expect("repo root");
        let repo_path = repo_root.path().join("repo");

        let run = create_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            NewRunRequest {
                repo_path,
                base_ref: "HEAD".to_string(),
                tag: "default".to_string(),
                run_name: "eventually-visible".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: false,
            },
        )
        .expect("created run");

        assert_eq!(registry.list_active_runs().expect("active runs"), vec![run]);
        assert_eq!(
            runner
                .commands
                .iter()
                .filter(|command| command_contains(command, "list-windows"))
                .count(),
            2
        );
    }

    #[test]
    fn create_run_rolls_back_when_tmux_window_is_missing_after_launch() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner::default();
        let repo_root = tempfile::tempdir().expect("repo root");
        let repo_path = repo_root.path().join("repo");

        let result = create_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            NewRunRequest {
                repo_path,
                base_ref: "HEAD".to_string(),
                tag: "default".to_string(),
                run_name: "quick-exit".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: false,
            },
        );

        assert!(result.is_err());
        assert_eq!(registry.list_active_runs().expect("active runs"), vec![]);
        assert!(runner
            .commands
            .iter()
            .any(|command| command_contains(command, "kill-window")));
        assert!(runner
            .commands
            .iter()
            .any(|command| command_contains(command, "remove")));
        assert!(runner
            .commands
            .iter()
            .any(|command| command_contains(command, "branch") && command_contains(command, "-D")));
    }

    #[test]
    fn folder_session_launch_and_end_never_run_git_or_delete_folder_files() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner {
            created_window_visible_after_list_calls: Some(1),
            ..RecordingRunner::default()
        };
        let folder = tempfile::tempdir().expect("folder");
        let source = folder.path().join("notes.txt");
        std::fs::write(&source, "keep me").expect("source");

        let session = create_folder_session_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            NewFolderSessionRequest {
                folder_path: folder.path().to_path_buf(),
                tag: "local".to_string(),
                run_name: "investigate".to_string(),
                agent: AgentKind::Claude,
            },
        )
        .expect("folder session");

        assert!(session.is_folder());
        assert_eq!(
            session.worktree_path,
            folder.path().canonicalize().expect("canonical folder")
        );
        assert!(runner
            .commands
            .iter()
            .all(|command| command.first().map(String::as_str) != Some("git")));

        let opened = open_run_in_vscode_with_registry(&registry, &mut runner, session.id)
            .expect("open folder in editor")
            .expect("folder session");
        assert_eq!(opened.id, session.id);
        let merge_error = merge_run_with_registry(&registry, &mut runner, session.id)
            .expect_err("folder merge must be rejected");
        assert!(merge_error
            .to_string()
            .contains("folder sessions do not support Git merge"));
        assert!(runner
            .commands
            .iter()
            .all(|command| command.first().map(String::as_str) != Some("git")));

        close_and_delete_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            session.id,
        )
        .expect("end folder session");

        assert_eq!(
            std::fs::read_to_string(source).expect("preserved"),
            "keep me"
        );
        assert!(runner
            .commands
            .iter()
            .all(|command| command.first().map(String::as_str) != Some("git")));
        assert!(registry.list_active_sessions().expect("active").is_empty());
    }

    #[test]
    fn create_run_records_resolved_base_commit() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner {
            created_window_visible_after_list_calls: Some(1),
            rev_parse_output: "abc123def456\n".to_string(),
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
                base_ref: "feature/base".to_string(),
                tag: "default".to_string(),
                run_name: "diff-review".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: false,
            },
        )
        .expect("created run");

        assert_eq!(run.base_commit.as_deref(), Some("abc123def456"));
        assert!(runner.commands.iter().any(|command| {
            command_contains(command, "rev-parse")
                && command_contains(command, "feature/base^{commit}")
                && command_contains(command, path_str(&repo_path))
        }));
    }

    #[test]
    fn create_run_persists_when_tmux_window_exists_after_launch() {
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
                repo_path,
                base_ref: "HEAD".to_string(),
                tag: "default".to_string(),
                run_name: "active-run".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: false,
            },
        )
        .expect("created run");

        assert_eq!(registry.list_active_runs().expect("active runs"), vec![run]);
        assert!(runner
            .commands
            .iter()
            .any(|command| command_contains(command, "list-windows")));
    }

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
                copy_ignored_files: false,
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
            .find(|command| {
                command_contains(command, "worktree") && command_contains(command, "add")
            })
            .expect("git worktree add command");
        assert!(add_worktree.contains(&"feature/login".to_string()));
        assert!(add_worktree.contains(&path_str(&run.worktree_path).to_string()));
    }

    #[test]
    fn create_run_copies_nonignored_untracked_files_before_launching_agent() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner {
            created_window_visible_after_list_calls: Some(1),
            untracked_files_output: "notes/scratch.txt\0".to_string(),
            ..RecordingRunner::default()
        };
        let repo_root = tempfile::tempdir().expect("repo root");
        let repo_path = repo_root.path().join("repo");
        let source_file = repo_path.join("notes").join("scratch.txt");
        std::fs::create_dir_all(source_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&source_file, "draft").expect("write source");

        let run = create_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            NewRunRequest {
                repo_path: repo_path.clone(),
                base_ref: "HEAD".to_string(),
                tag: "default".to_string(),
                run_name: "copy-files".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: false,
            },
        )
        .expect("created run");

        assert_eq!(
            std::fs::read_to_string(run.worktree_path.join("notes").join("scratch.txt"))
                .expect("read copied file"),
            "draft"
        );

        let list_untracked_index = runner
            .commands
            .iter()
            .position(|command| command_contains(command, "ls-files"))
            .expect("list untracked command");
        let new_window_index = runner
            .commands
            .iter()
            .position(|command| command_contains(command, "new-window"))
            .expect("new-window command");
        assert!(list_untracked_index < new_window_index);

        let list_untracked = &runner.commands[list_untracked_index];
        assert!(list_untracked.contains(&path_str(&repo_path).to_string()));
        assert!(list_untracked.contains(&"--exclude-standard".to_string()));
    }

    #[test]
    fn preview_ignored_untracked_files_reports_git_candidates() {
        let mut runner = RecordingRunner {
            untracked_files_output: ".env\0generated/client.js\0".to_string(),
            ..RecordingRunner::default()
        };
        let repo_root = tempfile::tempdir().expect("repo root");
        let repo_path = repo_root.path().join("repo");
        let generated_file = repo_path.join("generated").join("client.js");
        std::fs::create_dir_all(generated_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(repo_path.join(".env"), "TOKEN=secret").expect("write env");
        std::fs::write(&generated_file, "client").expect("write generated");

        let preview = preview_ignored_untracked_files(&mut runner, &repo_path).expect("preview");

        assert_eq!(preview.file_count, 2);
        assert_eq!(preview.total_bytes, 18);
        let command = runner.commands.last().expect("git command");
        assert!(command_contains(command, "--ignored"));
        assert!(command_contains(command, "--exclude-standard"));
    }

    #[test]
    fn create_run_can_copy_ignored_untracked_files_before_launching_agent() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner {
            created_window_visible_after_list_calls: Some(1),
            untracked_files_output: ".env\0generated/client.js\0".to_string(),
            ..RecordingRunner::default()
        };
        let repo_root = tempfile::tempdir().expect("repo root");
        let repo_path = repo_root.path().join("repo");
        let generated_file = repo_path.join("generated").join("client.js");
        std::fs::create_dir_all(generated_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(repo_path.join(".env"), "TOKEN=secret").expect("write env");
        std::fs::write(&generated_file, "client").expect("write generated");

        let run = create_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            NewRunRequest {
                repo_path: repo_path.clone(),
                base_ref: "HEAD".to_string(),
                tag: "default".to_string(),
                run_name: "copy-ignored-files".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: true,
            },
        )
        .expect("created run");

        assert_eq!(
            std::fs::read_to_string(run.worktree_path.join(".env")).expect("read env"),
            "TOKEN=secret"
        );
        assert_eq!(
            std::fs::read_to_string(run.worktree_path.join("generated").join("client.js"))
                .expect("read generated"),
            "client"
        );
        let command = runner
            .commands
            .iter()
            .find(|command| command_contains(command, "ls-files"))
            .expect("list untracked command");
        assert!(!command_contains(command, "--exclude-standard"));
    }

    #[test]
    fn close_and_delete_run_deletes_nonignored_untracked_files_before_removing_worktree() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner {
            created_window_visible_after_list_calls: Some(1),
            untracked_files_output: "notes/scratch.txt\0".to_string(),
            ..RecordingRunner::default()
        };
        let repo_root = tempfile::tempdir().expect("repo root");
        let repo_path = repo_root.path().join("repo");
        let source_file = repo_path.join("notes").join("scratch.txt");
        std::fs::create_dir_all(source_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&source_file, "draft").expect("write source");
        let run = create_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            NewRunRequest {
                repo_path,
                base_ref: "HEAD".to_string(),
                tag: "default".to_string(),
                run_name: "cleanup-files".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: false,
            },
        )
        .expect("created run");
        let copied_file = run.worktree_path.join("notes").join("scratch.txt");
        assert!(copied_file.exists());

        close_and_delete_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            run.id,
        )
        .expect("ended run");

        assert!(!copied_file.exists());
        assert!(!run.worktree_path.join("notes").exists());

        let list_worktree_untracked_index = runner
            .commands
            .iter()
            .position(|command| {
                command_contains(command, "ls-files")
                    && command.contains(&path_str(&run.worktree_path).to_string())
            })
            .expect("list worktree untracked command");
        let remove_worktree_index = runner
            .commands
            .iter()
            .position(|command| {
                command_contains(command, "worktree") && command_contains(command, "remove")
            })
            .expect("remove worktree command");
        assert!(list_worktree_untracked_index < remove_worktree_index);
    }

    #[test]
    fn create_run_wraps_agent_launch_so_failures_keep_the_pane_open() {
        let registry = SqliteRegistry::in_memory().expect("registry");
        let mut runner = RecordingRunner {
            created_window_visible_after_list_calls: Some(1),
            ..RecordingRunner::default()
        };
        let repo_root = tempfile::tempdir().expect("repo root");
        let repo_path = repo_root.path().join("repo");

        create_run_with_registry(
            &registry,
            &mut runner,
            &AppConfig::for_session("agentctl-test"),
            NewRunRequest {
                repo_path,
                base_ref: "HEAD".to_string(),
                tag: "default".to_string(),
                run_name: "diagnostic-pane".to_string(),
                agent: AgentKind::Codex,
                copy_ignored_files: false,
            },
        )
        .expect("created run");

        let new_window = runner
            .commands
            .iter()
            .find(|command| command_contains(command, "new-window"))
            .expect("tmux new-window command");
        let shell_command = new_window.last().expect("tmux shell command");

        assert!(shell_command.contains("curl -fsS http://127.0.0.1:17655/readyz"));
        assert!(shell_command.contains("codex --remote ws://127.0.0.1:17655"));
        assert!(shell_command.contains("Agent command exited with status %s."));
        assert!(shell_command.contains("\"$agent_status\""));
        assert!(shell_command.contains("exec \"${SHELL:-/bin/sh}\""));

        assert!(runner.commands.iter().any(|command| {
            command_contains(command, "new-session")
                && command_contains(command, CODEX_APP_SERVER_TMUX_SESSION)
                && command.last().is_some_and(|part| {
                    part.contains("\"${SHELL:-/bin/sh}\" -lic")
                        && part.contains("codex app-server --listen")
                })
        }));
    }

    fn command_contains(command: &[String], needle: &str) -> bool {
        command.iter().any(|part| part == needle)
    }

    fn window_name_arg(command: &[String]) -> Option<String> {
        command
            .windows(2)
            .find_map(|parts| (parts[0] == "-n").then(|| parts[1].clone()))
    }
}
