use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use agentctl_core::{
    agent::LaunchPlan,
    commands::{shell_join, AgentCommandBuilder},
    domain::{Lifecycle, RunRecord},
    registry::SqliteRegistry,
};
use serde::Serialize;

const PANE_LINE: &str = "pane";
const WINDOW_LINE: &str = "window";
const CONFIG_BLOCK_START: &str = "# >>> Agent Manager tmux restore >>>";
const CONFIG_BLOCK_END: &str = "# <<< Agent Manager tmux restore <<<";
const REWRITE_ARG: &str = "__tmux-resurrect-rewrite";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TmuxRestorePaths {
    config_path: PathBuf,
    tpm_dir: PathBuf,
    resurrect_dir: PathBuf,
    continuum_dir: PathBuf,
    resurrect_save_dir: PathBuf,
    last_resurrect_file: PathBuf,
    systemd_unit_file: PathBuf,
}

impl TmuxRestorePaths {
    pub fn for_home(home: impl AsRef<Path>) -> Self {
        let home = home.as_ref();
        Self::for_home_with_config(home, home.join(".tmux.conf"))
    }

    pub fn from_environment() -> Self {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let config_path = preferred_tmux_config_path(&home);
        Self::for_home_with_config(home, config_path)
    }

    fn for_home_with_config(home: impl AsRef<Path>, config_path: PathBuf) -> Self {
        let home = home.as_ref();
        let tmux_dir = home.join(".tmux");
        let plugins_dir = tmux_dir.join("plugins");
        let resurrect_save_dir = tmux_dir.join("resurrect");
        Self {
            config_path,
            tpm_dir: plugins_dir.join("tpm"),
            resurrect_dir: plugins_dir.join("tmux-resurrect"),
            continuum_dir: plugins_dir.join("tmux-continuum"),
            last_resurrect_file: resurrect_save_dir.join("last"),
            resurrect_save_dir,
            systemd_unit_file: home.join(".config/systemd/user/tmux.service"),
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn tpm_dir(&self) -> &Path {
        &self.tpm_dir
    }

    pub fn resurrect_dir(&self) -> &Path {
        &self.resurrect_dir
    }

    pub fn continuum_dir(&self) -> &Path {
        &self.continuum_dir
    }

    pub fn resurrect_save_dir(&self) -> &Path {
        &self.resurrect_save_dir
    }

    pub fn last_resurrect_file(&self) -> &Path {
        &self.last_resurrect_file
    }

    fn tpm_install_script(&self) -> PathBuf {
        self.tpm_dir.join("bin/install_plugins")
    }

    fn resurrect_save_script(&self) -> PathBuf {
        self.resurrect_dir.join("scripts/save.sh")
    }

    fn resurrect_restore_script(&self) -> PathBuf {
        self.resurrect_dir.join("scripts/restore.sh")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxRestoreStatus {
    pub configured: bool,
    pub tpm_installed: bool,
    pub resurrect_installed: bool,
    pub continuum_installed: bool,
    pub auto_restore_enabled: bool,
    pub boot_enabled: bool,
    pub saved_state_exists: bool,
    pub systemd_unit_exists: bool,
    pub config_path: String,
    pub detail: String,
}

pub fn tmux_restore_status(paths: &TmuxRestorePaths) -> TmuxRestoreStatus {
    let config = fs::read_to_string(paths.config_path()).unwrap_or_default();
    let tpm_installed = paths.tpm_dir().exists();
    let resurrect_installed = paths.resurrect_dir().exists();
    let continuum_installed = paths.continuum_dir().exists();
    let auto_restore_enabled = tmux_option_is_on(&config, "@continuum-restore");
    let boot_enabled = tmux_option_is_on(&config, "@continuum-boot");
    let hook_configured = config.contains(REWRITE_ARG);
    let configured = tpm_installed
        && resurrect_installed
        && continuum_installed
        && auto_restore_enabled
        && boot_enabled
        && hook_configured;
    let saved_state_exists = paths.last_resurrect_file().exists();
    let systemd_unit_exists = paths.systemd_unit_file.exists();

    TmuxRestoreStatus {
        configured,
        tpm_installed,
        resurrect_installed,
        continuum_installed,
        auto_restore_enabled,
        boot_enabled,
        saved_state_exists,
        systemd_unit_exists,
        config_path: paths.config_path().to_string_lossy().to_string(),
        detail: restore_status_detail(configured, saved_state_exists),
    }
}

pub fn registry_path_from_environment() -> PathBuf {
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data_home)
            .join("agentctl")
            .join("agentctl.sqlite3");
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("share")
        .join("agentctl")
        .join("agentctl.sqlite3")
}

pub fn stable_agent_manager_executable(appimage: Option<&OsStr>, current_exe: &Path) -> PathBuf {
    appimage
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| current_exe.to_path_buf())
}

pub fn agent_manager_executable_from_environment() -> io::Result<PathBuf> {
    let appimage = env::var_os("APPIMAGE");
    Ok(stable_agent_manager_executable(
        appimage.as_deref(),
        &env::current_exe()?,
    ))
}

pub fn enable_tmux_restore(
    paths: &TmuxRestorePaths,
    agent_manager_binary: &Path,
) -> io::Result<()> {
    let tpm_install_script = paths.tpm_install_script();
    if !tpm_install_script.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "TPM install script not found: {}",
                tpm_install_script.display()
            ),
        ));
    }

    let existing = fs::read_to_string(paths.config_path()).unwrap_or_default();
    let updated =
        upsert_agent_manager_tmux_config(&existing, &agent_manager_binary.to_string_lossy());
    if let Some(parent) = paths.config_path().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(paths.config_path(), updated)?;

    run_command(Command::new("tmux").arg("start-server"))?;
    run_command(
        Command::new("tmux")
            .arg("source-file")
            .arg(paths.config_path()),
    )?;
    run_command(&mut Command::new(tpm_install_script))?;
    run_command(
        Command::new("tmux")
            .arg("source-file")
            .arg(paths.config_path()),
    )?;
    save_tmux_restore_snapshot(paths)
}

pub fn refresh_tmux_restore_hook(
    paths: &TmuxRestorePaths,
    agent_manager_binary: &Path,
) -> io::Result<bool> {
    let existing = fs::read_to_string(paths.config_path()).unwrap_or_default();
    if !agent_manager_config_block_exists(&existing) {
        return Ok(false);
    }

    let updated =
        upsert_agent_manager_tmux_config(&existing, &agent_manager_binary.to_string_lossy());
    if updated == existing {
        return Ok(false);
    }

    if let Some(parent) = paths.config_path().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(paths.config_path(), updated)?;
    Ok(true)
}

pub fn save_tmux_restore_snapshot(paths: &TmuxRestorePaths) -> io::Result<()> {
    let save_script = paths.resurrect_save_script();
    if save_script.exists() {
        run_command(Command::new(save_script).arg("quiet"))?;
    }
    Ok(())
}

pub fn save_tmux_restore_snapshot_best_effort() {
    let _ = save_tmux_restore_snapshot(&TmuxRestorePaths::from_environment());
}

pub fn restore_tmux_session_if_missing(
    paths: &TmuxRestorePaths,
    managed_session: &str,
) -> io::Result<()> {
    let status = tmux_restore_status(paths);
    if !status.configured || !status.saved_state_exists {
        return Ok(());
    }

    if !tmux_restore_needed(&status, tmux_has_session(managed_session)?) {
        return Ok(());
    }

    run_command(Command::new("tmux").arg("start-server"))?;
    run_command(
        Command::new("tmux")
            .arg("source-file")
            .arg(paths.config_path()),
    )?;
    let restore_script = paths.resurrect_restore_script();
    if restore_script.exists() {
        run_command(Command::new("tmux").arg("run-shell").arg(restore_script))?;
        wait_for_tmux_session(managed_session)?;
    }
    Ok(())
}

pub fn tmux_restore_needed(status: &TmuxRestoreStatus, has_session: bool) -> bool {
    status.configured && status.saved_state_exists && !has_session
}

pub fn restore_tmux_session_best_effort(managed_session: &str) {
    let paths = TmuxRestorePaths::from_environment();
    if let Ok(executable) = agent_manager_executable_from_environment() {
        let _ = refresh_tmux_restore_hook(&paths, &executable);
    }
    let _ = restore_tmux_session_if_missing(&paths, managed_session);
}

pub fn rewrite_saved_resurrect_state_from_environment(
    managed_session: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let paths = TmuxRestorePaths::from_environment();
    let registry_path = registry_path_from_environment();
    rewrite_saved_resurrect_state(&paths, &registry_path, managed_session)
}

pub fn rewrite_saved_resurrect_state(
    paths: &TmuxRestorePaths,
    registry_path: &Path,
    managed_session: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !paths.last_resurrect_file().exists() {
        return Ok(());
    }
    let registry = SqliteRegistry::open(registry_path)?;
    let runs = registry.list_active_runs()?;
    let current = fs::read_to_string(paths.last_resurrect_file())?;
    let rewritten = rewrite_resurrect_state(&current, &runs, managed_session);
    if rewritten != current {
        fs::write(paths.last_resurrect_file(), rewritten)?;
    }
    Ok(())
}

pub fn upsert_agent_manager_tmux_config(existing: &str, agent_manager_binary: &str) -> String {
    let without_existing_block = remove_agent_manager_config_block(existing);
    let mut config = without_existing_block.trim_end().to_string();
    if !config.is_empty() {
        config.push_str("\n\n");
    }
    config.push_str(&agent_manager_tmux_config_block(agent_manager_binary));
    config
}

pub fn agent_manager_tmux_config_block(agent_manager_binary: &str) -> String {
    let hook = shell_join(&[agent_manager_binary.to_string(), REWRITE_ARG.to_string()]);
    format!(
        "{CONFIG_BLOCK_START}\n\
set -g status on\n\
set -g @plugin 'tmux-plugins/tmux-resurrect'\n\
set -g @plugin 'tmux-plugins/tmux-continuum'\n\
set -g @continuum-restore 'on'\n\
set -g @continuum-boot 'on'\n\
set -g @resurrect-processes 'codex claude'\n\
set -g @resurrect-hook-pre-restore-pane-processes {}\n\
run '~/.tmux/plugins/tpm/tpm'\n\
{CONFIG_BLOCK_END}\n",
        tmux_double_quoted(&hook)
    )
}

fn remove_agent_manager_config_block(existing: &str) -> String {
    let mut output = Vec::new();
    let mut skipping = false;

    for line in existing.lines() {
        if line.trim() == CONFIG_BLOCK_START {
            skipping = true;
            continue;
        }
        if line.trim() == CONFIG_BLOCK_END {
            skipping = false;
            continue;
        }
        if !skipping {
            output.push(line);
        }
    }

    let mut config = output.join("\n");
    if existing.ends_with('\n') && !config.is_empty() {
        config.push('\n');
    }
    config
}

fn agent_manager_config_block_exists(existing: &str) -> bool {
    existing
        .lines()
        .any(|line| line.trim() == CONFIG_BLOCK_START)
}

fn tmux_double_quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn tmux_option_is_on(config: &str, option: &str) -> bool {
    config.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("set -g")
            && trimmed.contains(option)
            && (trimmed.ends_with("'on'")
                || trimmed.ends_with("\"on\"")
                || trimmed.ends_with(" on"))
    })
}

fn restore_status_detail(configured: bool, saved_state_exists: bool) -> String {
    match (configured, saved_state_exists) {
        (true, true) => "tmux restart restore is configured and has a saved session.".to_string(),
        (true, false) => {
            "tmux restart restore is configured but has not saved a session yet.".to_string()
        }
        (false, _) => "tmux restart restore is not configured.".to_string(),
    }
}

fn preferred_tmux_config_path(home: &Path) -> PathBuf {
    if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg_config_home).join("tmux/tmux.conf");
        if path.exists() {
            return path;
        }
    }

    let xdg_default = home.join(".config/tmux/tmux.conf");
    if xdg_default.exists() {
        return xdg_default;
    }

    home.join(".tmux.conf")
}

fn run_command(command: &mut Command) -> io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("command failed with {status}")))
    }
}

fn tmux_has_session(session: &str) -> io::Result<bool> {
    let status = Command::new("tmux")
        .args(["has-session", "-t", session])
        .status()?;
    Ok(status.success())
}

fn wait_for_tmux_session(session: &str) -> io::Result<()> {
    for _ in 0..20 {
        if tmux_has_session(session)? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

pub fn rewrite_resurrect_state(input: &str, runs: &[RunRecord], managed_session: &str) -> String {
    let windows = window_names_by_index(input);
    let runs_by_window = active_runs_by_window(runs, managed_session);
    let mut output = String::new();

    for line in input.lines() {
        output.push_str(&rewrite_line(
            line,
            managed_session,
            &windows,
            &runs_by_window,
        ));
        output.push('\n');
    }

    if !input.ends_with('\n') {
        output.pop();
    }

    output
}

fn rewrite_line(
    line: &str,
    managed_session: &str,
    windows: &HashMap<(String, String), String>,
    runs_by_window: &HashMap<String, String>,
) -> String {
    let mut fields = line.split('\t').collect::<Vec<_>>();
    if fields.first() != Some(&PANE_LINE) || fields.len() < 11 || fields[1] != managed_session {
        return line.to_string();
    }

    let window_name = windows
        .get(&(fields[1].to_string(), fields[2].to_string()))
        .map(String::as_str);
    let Some(command) = window_name.and_then(|window| runs_by_window.get(window)) else {
        return line.to_string();
    };

    fields[10] = command;
    fields.join("\t")
}

fn window_names_by_index(input: &str) -> HashMap<(String, String), String> {
    input
        .lines()
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.first() == Some(&WINDOW_LINE) && fields.len() >= 4 {
                Some((
                    (fields[1].to_string(), fields[2].to_string()),
                    remove_resurrect_prefix(fields[3]).to_string(),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn active_runs_by_window(runs: &[RunRecord], managed_session: &str) -> HashMap<String, String> {
    runs.iter()
        .filter(|run| run.lifecycle == Lifecycle::Active)
        .filter(|run| run.tmux_session.as_deref().unwrap_or(managed_session) == managed_session)
        .filter_map(|run| {
            run.tmux_window
                .as_deref()
                .map(|window| (window.to_string(), resurrect_command(run)))
        })
        .collect()
}

fn resurrect_command(run: &RunRecord) -> String {
    let command = AgentCommandBuilder::new().restore(LaunchPlan {
        agent: run.agent,
        worktree_path: run.worktree_path.clone(),
        session_id: run.agent_session_id,
    });
    format!(":{}", shell_join(&command))
}

fn remove_resurrect_prefix(value: &str) -> &str {
    value.strip_prefix(':').unwrap_or(value)
}
