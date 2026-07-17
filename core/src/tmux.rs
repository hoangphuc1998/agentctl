use std::{
    env, io,
    path::Path,
    process::{Command, ExitStatus},
};

use crate::{
    commands::{shell_join, TerminalColorEnvironment, TmuxCommandBuilder},
    domain::{DetectionSource, ObservedState},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneSnapshot {
    pub pane_active: bool,
    pub current_command: String,
    pub visible_text: String,
}

pub fn detect_observed_state(snapshot: &PaneSnapshot) -> ObservedState {
    if !snapshot.pane_active {
        return ObservedState::Unknown;
    }

    let recent = recent_activity_text(&snapshot.visible_text);
    let text = recent.to_ascii_lowercase();
    if has_active_work_marker(&text) {
        return ObservedState::Running;
    }

    if contains_any(
        &text,
        &[
            "approve command",
            "do you want to",
            "need input",
            "needs input",
            "need approval",
            "requires approval",
            "requires input",
            "waiting for input",
            "waiting for user",
            "press enter",
            "enter to submit answer",
            "[y/n]",
        ],
    ) {
        return ObservedState::NeedsUser;
    }

    if contains_any(
        &text,
        &[
            "all tasks complete",
            "completed",
            "ready for review",
            "implementation complete",
            "worked for",
            "done",
        ],
    ) {
        return ObservedState::CompletedUnchecked;
    }

    if is_agent_runtime_command(&snapshot.current_command) {
        ObservedState::Running
    } else {
        ObservedState::Unknown
    }
}

pub fn detection_source_for(snapshot: &PaneSnapshot) -> DetectionSource {
    if !snapshot.pane_active {
        DetectionSource::Unknown
    } else if detect_observed_state(snapshot) == ObservedState::Running {
        DetectionSource::Tmux
    } else {
        DetectionSource::Heuristic
    }
}

pub fn window_list_contains(output: &str, window: &str) -> bool {
    output.lines().map(str::trim).any(|name| name == window)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn has_active_work_marker(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        let Some(status) = trimmed
            .strip_prefix('•')
            .or_else(|| trimmed.strip_prefix('◦'))
            .map(str::trim_start)
        else {
            return false;
        };

        status.starts_with("running ")
            || status.starts_with("working ")
            || status.contains("esc to interrupt")
    })
}

fn recent_activity_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .rposition(|line| is_turn_separator(line))
        .map(|index| index + 1)
        .unwrap_or(0);
    lines[start..].join("\n")
}

fn is_turn_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.chars().count() >= 20 && trimmed.chars().all(|ch| matches!(ch, '─' | '━'))
}

fn is_agent_runtime_command(command: &str) -> bool {
    matches!(
        command.trim(),
        "codex" | "claude" | "node" | "nodejs" | "deno" | "bun"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_work_marker_stays_running_when_recent_text_mentions_completion() {
        let snapshot = PaneSnapshot {
            pane_active: true,
            current_command: "codex".to_string(),
            visible_text: "• running tests\nImplemented the requested fix.\nAll tasks completed.\n"
                .to_string(),
        };

        assert_eq!(detect_observed_state(&snapshot), ObservedState::Running);
        assert_eq!(detection_source_for(&snapshot), DetectionSource::Tmux);
    }

    #[test]
    fn current_codex_work_marker_stays_running_when_transcript_mentions_completion() {
        let snapshot = PaneSnapshot {
            pane_active: true,
            current_command: "node".to_string(),
            visible_text: concat!(
                "Need input before declaring the implementation complete and ready for review.\n",
                "◦ Running cargo test\n",
                "◦ Working (3m 09s • esc to interrupt)\n",
            )
            .to_string(),
        };

        assert_eq!(detect_observed_state(&snapshot), ObservedState::Running);
        assert_eq!(detection_source_for(&snapshot), DetectionSource::Tmux);
    }

    #[test]
    fn live_agent_runtime_reports_completed_when_recent_text_is_final() {
        let snapshot = PaneSnapshot {
            pane_active: true,
            current_command: "codex".to_string(),
            visible_text: "Implementation complete.\nReady for review.\n".to_string(),
        };

        assert_eq!(
            detect_observed_state(&snapshot),
            ObservedState::CompletedUnchecked
        );
        assert_eq!(detection_source_for(&snapshot), DetectionSource::Heuristic);
    }

    #[test]
    fn visible_need_input_text_reports_needs_user() {
        let snapshot = PaneSnapshot {
            pane_active: true,
            current_command: "codex".to_string(),
            visible_text: "Need input\nReview the command before continuing.\n".to_string(),
        };

        assert_eq!(detect_observed_state(&snapshot), ObservedState::NeedsUser);
        assert_eq!(detection_source_for(&snapshot), DetectionSource::Heuristic);
    }
}

pub struct Tmux {
    session: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardRepairAction {
    CreateSession,
    CreateDashboardWindow,
    RespawnDashboard,
    Ready,
}

pub fn dashboard_repair_action(
    has_session: bool,
    has_dashboard_window: bool,
    dashboard_is_running: bool,
    terminal_is_configured: bool,
) -> DashboardRepairAction {
    if !has_session {
        DashboardRepairAction::CreateSession
    } else if !has_dashboard_window {
        DashboardRepairAction::CreateDashboardWindow
    } else if !dashboard_is_running || !terminal_is_configured {
        DashboardRepairAction::RespawnDashboard
    } else {
        DashboardRepairAction::Ready
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpenRunOutcome {
    SwitchedWindow(String),
}

impl OpenRunOutcome {
    pub fn window_name(&self) -> &str {
        match self {
            Self::SwitchedWindow(window_name) => window_name,
        }
    }
}

impl Tmux {
    pub fn new(session: impl Into<String>) -> Self {
        Self {
            session: session.into(),
        }
    }

    pub fn ensure_session(&self) -> io::Result<()> {
        let builder = TmuxCommandBuilder::new(&self.session);
        let terminal_color_environment = TerminalColorEnvironment::capture();
        if !self.has_session()? {
            run_command(&builder.new_detached_session())?;
        }
        run_terminal_setup(&builder, Some(&terminal_color_environment))
    }

    pub fn open_dashboard(&self, executable: &Path) -> io::Result<()> {
        let builder = TmuxCommandBuilder::new(&self.session);
        let command = dashboard_command(executable);
        let terminal_color_environment = TerminalColorEnvironment::capture();
        let has_session = self.has_session()?;
        let has_dashboard_window = has_session && self.has_window("dashboard")?;
        let dashboard_is_running = has_dashboard_window && self.dashboard_is_running(executable)?;
        let terminal_is_configured =
            has_session && self.terminal_is_configured(&terminal_color_environment)?;
        match dashboard_repair_action(
            has_session,
            has_dashboard_window,
            dashboard_is_running,
            terminal_is_configured,
        ) {
            DashboardRepairAction::CreateSession => {
                run_command(&builder.new_detached_session())?;
                run_terminal_setup(&builder, Some(&terminal_color_environment))?;
                run_command(&builder.respawn_dashboard(&command))?
            }
            DashboardRepairAction::CreateDashboardWindow => {
                run_terminal_setup(&builder, Some(&terminal_color_environment))?;
                run_command(&builder.new_dashboard_window(&command))?
            }
            DashboardRepairAction::RespawnDashboard => {
                run_terminal_setup(&builder, Some(&terminal_color_environment))?;
                run_command(&builder.respawn_dashboard(&command))?
            }
            DashboardRepairAction::Ready => {
                run_terminal_setup(&builder, Some(&terminal_color_environment))?
            }
        }
        run_command(&builder.bind_dashboard_shortcut())?;
        if env::var_os("TMUX").is_some() {
            run_command(&builder.switch_client("dashboard"))
        } else {
            run_command(&builder.attach_session("dashboard"))
        }
    }

    pub fn new_window(&self, window: &str, cwd: &Path, command: &[String]) -> io::Result<()> {
        let shell_command = shell_join(command);
        run_command(&TmuxCommandBuilder::new(&self.session).new_window(
            window,
            &cwd.to_string_lossy(),
            &shell_command,
        ))
    }

    pub fn kill_window(&self, window: &str) -> io::Result<()> {
        run_status(
            Command::new("tmux")
                .arg("kill-window")
                .arg("-t")
                .arg(format!("{}:{window}", self.session)),
        )
    }

    pub fn open_run(&self, window: &str) -> io::Result<OpenRunOutcome> {
        let builder = TmuxCommandBuilder::new(&self.session);
        if !self.has_window(window)? {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("tmux window not found: {}:{window}", self.session),
            ));
        }
        run_command(&builder.open_run_window(window))?;
        let _ = run_command(&builder.dashboard_return_hint(window));
        Ok(OpenRunOutcome::SwitchedWindow(window.to_string()))
    }

    pub fn detach_client(&self) -> io::Result<()> {
        if env::var_os("TMUX").is_none() {
            return Ok(());
        }
        run_command(&TmuxCommandBuilder::new(&self.session).detach_client())
    }

    pub fn snapshot_window(&self, window: &str) -> io::Result<PaneSnapshot> {
        let target = format!("{}:{window}", self.session);
        let command = Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                &target,
                "#{pane_current_command}",
            ])
            .output()?;
        if !command.status.success() {
            return Ok(PaneSnapshot {
                pane_active: false,
                current_command: String::new(),
                visible_text: String::new(),
            });
        }

        let text = Command::new("tmux")
            .args(["capture-pane", "-p", "-J", "-S", "-120", "-t", &target])
            .output()?;
        Ok(PaneSnapshot {
            pane_active: text.status.success(),
            current_command: String::from_utf8_lossy(&command.stdout).trim().to_string(),
            visible_text: String::from_utf8_lossy(&text.stdout).to_string(),
        })
    }

    fn has_session(&self) -> io::Result<bool> {
        let status = Command::new("tmux")
            .args(["has-session", "-t", &self.session])
            .status()?;
        Ok(status.success())
    }

    fn has_window(&self, window: &str) -> io::Result<bool> {
        let output = Command::new("tmux")
            .args(["list-windows", "-t", &self.session, "-F", "#{window_name}"])
            .output()?;
        Ok(output.status.success()
            && window_list_contains(&String::from_utf8_lossy(&output.stdout), window))
    }

    fn dashboard_is_running(&self, executable: &Path) -> io::Result<bool> {
        let output = Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                &format!("{}:dashboard", self.session),
                "#{pane_current_command}",
            ])
            .output()?;
        if !output.status.success() {
            return Ok(false);
        }
        let expected = executable
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agentctl");
        let current = String::from_utf8_lossy(&output.stdout);
        Ok(current.trim() == expected)
    }

    fn terminal_is_configured(&self, environment: &TerminalColorEnvironment) -> io::Result<bool> {
        let output = Command::new("tmux")
            .args([
                "show-options",
                "-qv",
                "-t",
                &self.session,
                "default-terminal",
            ])
            .output()?;
        Ok(output.status.success()
            && String::from_utf8_lossy(&output.stdout).trim() == "tmux-256color"
            && self.terminal_color_environment_is_configured(environment)?
            && self.terminal_overrides_are_configured(environment)?)
    }

    fn terminal_color_environment_is_configured(
        &self,
        environment: &TerminalColorEnvironment,
    ) -> io::Result<bool> {
        for (name, expected) in environment.entries() {
            let output = Command::new("tmux")
                .args(["show-environment", "-t", &self.session, name])
                .output()?;
            let current = String::from_utf8_lossy(&output.stdout);
            match expected {
                Some(expected) => {
                    if !output.status.success()
                        || current.trim_end() != format!("{name}={expected}")
                    {
                        return Ok(false);
                    }
                }
                None => {
                    if output.status.success() && current.trim_end() != format!("-{name}") {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    fn terminal_overrides_are_configured(
        &self,
        environment: &TerminalColorEnvironment,
    ) -> io::Result<bool> {
        if !environment.uses_truecolor() {
            return Ok(true);
        }
        let output = Command::new("tmux")
            .args(["show-options", "-gqv", "terminal-overrides"])
            .output()?;
        Ok(output.status.success()
            && TmuxCommandBuilder::terminal_overrides_with_rgb(&String::from_utf8_lossy(
                &output.stdout,
            ))
            .is_none())
    }
}

fn dashboard_command(executable: &Path) -> String {
    shell_join(&[
        executable.to_string_lossy().to_string(),
        "__dashboard".to_string(),
    ])
}

fn run_status(command: &mut Command) -> io::Result<()> {
    let status = command.status()?;
    ensure_success(status)
}

fn run_command(command: &[String]) -> io::Result<()> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty command"))?;
    run_status(Command::new(program).args(args))
}

fn run_commands(commands: Vec<Vec<String>>) -> io::Result<()> {
    for command in commands {
        run_command(&command)?;
    }
    Ok(())
}

fn run_terminal_setup(
    builder: &TmuxCommandBuilder,
    environment: Option<&TerminalColorEnvironment>,
) -> io::Result<()> {
    run_commands(builder.terminal_setup_commands())?;
    if let Some(environment) = environment {
        run_commands(builder.terminal_color_environment_commands(environment))?;
        if environment.uses_truecolor() {
            let output = command_output(&builder.show_global_option("terminal-overrides"))?;
            if !output.status.success() {
                return Err(io::Error::other("failed to read tmux terminal-overrides"));
            }
            if let Some(updated) = TmuxCommandBuilder::terminal_overrides_with_rgb(
                &String::from_utf8_lossy(&output.stdout),
            ) {
                run_command(&builder.set_server_option("terminal-overrides", &updated))?;
            }
        }
    }
    Ok(())
}

fn command_output(command: &[String]) -> io::Result<std::process::Output> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty command"))?;
    Command::new(program).args(args).output()
}

fn ensure_success(status: ExitStatus) -> io::Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("command failed with {status}")))
    }
}
