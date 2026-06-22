use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use uuid::Uuid;

use crate::{
    agent::{AgentKind, LaunchPlan},
    commands::{
        shell_join, AgentCommandBuilder, EditorCommandBuilder, GitCommandBuilder,
        TerminalColorEnvironment, TmuxCommandBuilder,
    },
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
    registry::{RegistryResult, SqliteRegistry},
    worktree::{default_branch_name, default_sibling_worktree_path, sanitize_slug},
};

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
        let new_window =
            tmux.new_window(&window, path_str(&run.worktree_path), &shell_join(&command));
        self.runner.run(&new_window)?;

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
        let git = GitCommandBuilder::new();
        runner.run(&git.remove_worktree(path_str(&run.repo_path), path_str(&run.worktree_path)))?;
        runner.run(&git.delete_branch(path_str(&run.repo_path), &run.branch))?;
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
    let branch = default_branch_name(&run_slug);
    let worktree_path = default_sibling_worktree_path(&request.repo_path, &run_slug);
    let tmux_window = window_name(&repo_name, &tag, &run_slug, id);
    let agent_session_id = match request.agent {
        AgentKind::Codex => None,
        AgentKind::Claude => Some(Uuid::new_v4()),
    };

    let tmux = TmuxCommandBuilder::new(&config.tmux_session);
    ensure_tmux_session(runner, &tmux, config.terminal_color_environment.as_ref())?;

    let git = GitCommandBuilder::new();
    let add_worktree = git.add_worktree(
        path_str(&request.repo_path),
        path_str(&worktree_path),
        &branch,
        &request.base_ref,
    );
    runner.run(&add_worktree)?;

    let agent_command = AgentCommandBuilder::new().launch(LaunchPlan {
        agent: request.agent,
        worktree_path: worktree_path.clone(),
        session_id: agent_session_id,
    });
    let new_window = tmux.new_window(
        &tmux_window,
        path_str(&worktree_path),
        &shell_join(&agent_command),
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

    let run = RunRecord {
        id,
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
