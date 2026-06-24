use crate::agent::{AgentKind, LaunchPlan};
use std::env;

const MANAGED_HISTORY_LIMIT: &str = "100000";
const MANAGED_DEFAULT_TERMINAL: &str = "tmux-256color";
const TERMINAL_COLOR_ENV_VARS: [&str; 6] = [
    "COLORTERM",
    "NO_COLOR",
    "FORCE_COLOR",
    "CLICOLOR",
    "CLICOLOR_FORCE",
    "COLORFGBG",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalColorEnvironment {
    values: Vec<(String, Option<String>)>,
}

impl TerminalColorEnvironment {
    pub fn capture() -> Self {
        Self {
            values: TERMINAL_COLOR_ENV_VARS
                .iter()
                .map(|name| ((*name).to_string(), captured_terminal_color_value(name)))
                .collect(),
        }
    }

    pub fn from_values<const N: usize>(values: [(&str, Option<&str>); N]) -> Self {
        Self {
            values: TERMINAL_COLOR_ENV_VARS
                .iter()
                .map(|name| {
                    let value = values
                        .iter()
                        .find(|(candidate, _)| candidate == name)
                        .and_then(|(_, value)| value.map(str::to_string));
                    ((*name).to_string(), value)
                })
                .collect(),
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_deref()))
    }

    pub fn uses_truecolor(&self) -> bool {
        self.values
            .iter()
            .find(|(name, _)| name == "COLORTERM")
            .and_then(|(_, value)| value.as_deref())
            .map(|value| {
                let value = value.to_ascii_lowercase();
                value == "truecolor" || value == "24bit"
            })
            .unwrap_or(false)
    }
}

fn captured_terminal_color_value(name: &str) -> Option<String> {
    let value = env::var(name).ok();
    if name == "COLORFGBG" {
        return value.filter(|value| !value.is_empty()).or_else(|| {
            // Agent Deck uses COLORFGBG as a light/dark background hint for
            // terminal-aware tools. Default to dark when the launcher does not
            // expose a terminal-specific value.
            Some("15;0".to_string())
        });
    }
    value
}

#[derive(Default)]
pub struct GitCommandBuilder;

impl GitCommandBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn add_worktree(
        &self,
        repo_path: &str,
        worktree_path: &str,
        branch: &str,
        base_ref: &str,
    ) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            branch.to_string(),
            worktree_path.to_string(),
            base_ref.to_string(),
        ]
    }

    pub fn remove_worktree(&self, repo_path: &str, worktree_path: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            worktree_path.to_string(),
        ]
    }

    pub fn delete_branch(&self, repo_path: &str, branch: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "branch".to_string(),
            "-D".to_string(),
            branch.to_string(),
        ]
    }

    pub fn status_porcelain(&self, repo_path: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "status".to_string(),
            "--porcelain".to_string(),
        ]
    }

    pub fn nonignored_untracked_files(&self, repo_path: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "ls-files".to_string(),
            "--others".to_string(),
            "--exclude-standard".to_string(),
            "-z".to_string(),
        ]
    }

    pub fn origin_head_ref(&self, repo_path: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "symbolic-ref".to_string(),
            "--short".to_string(),
            "refs/remotes/origin/HEAD".to_string(),
        ]
    }

    pub fn local_branches(&self, repo_path: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "branch".to_string(),
            "--format=%(refname:short)".to_string(),
        ]
    }

    pub fn checkout_branch(&self, repo_path: &str, branch: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "checkout".to_string(),
            branch.to_string(),
        ]
    }

    pub fn merge_branch(&self, repo_path: &str, branch: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "merge".to_string(),
            "--no-ff".to_string(),
            branch.to_string(),
        ]
    }

    pub fn merge_abort(&self, repo_path: &str) -> Vec<String> {
        vec![
            "git".to_string(),
            "-C".to_string(),
            repo_path.to_string(),
            "merge".to_string(),
            "--abort".to_string(),
        ]
    }
}

#[derive(Default)]
pub struct EditorCommandBuilder;

impl EditorCommandBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn open(&self, path: &str) -> Vec<String> {
        vec!["code".to_string(), path.to_string()]
    }
}

pub struct TmuxCommandBuilder {
    session: String,
}

impl TmuxCommandBuilder {
    pub fn new(session: impl Into<String>) -> Self {
        Self {
            session: session.into(),
        }
    }

    pub fn new_window(&self, window_name: &str, cwd: &str, command: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "new-window".to_string(),
            "-t".to_string(),
            self.session_target(),
            "-n".to_string(),
            window_name.to_string(),
            "-c".to_string(),
            cwd.to_string(),
            command.to_string(),
        ]
    }

    pub fn list_windows(&self) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "list-windows".to_string(),
            "-t".to_string(),
            self.session.clone(),
            "-F".to_string(),
            "#{window_name}".to_string(),
        ]
    }

    pub fn new_dashboard_session(&self, command: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "new-session".to_string(),
            "-d".to_string(),
            "-s".to_string(),
            self.session.clone(),
            "-n".to_string(),
            "dashboard".to_string(),
            command.to_string(),
        ]
    }

    pub fn has_session(&self) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "has-session".to_string(),
            "-t".to_string(),
            self.session.clone(),
        ]
    }

    pub fn terminal_setup_commands(&self) -> Vec<Vec<String>> {
        vec![
            self.set_option("default-terminal", MANAGED_DEFAULT_TERMINAL),
            self.set_option("mouse", "on"),
            self.set_option("history-limit", MANAGED_HISTORY_LIMIT),
        ]
    }

    pub fn terminal_color_environment_commands(
        &self,
        environment: &TerminalColorEnvironment,
    ) -> Vec<Vec<String>> {
        environment
            .entries()
            .map(|(name, value)| match value {
                Some(value) => self.set_environment(name, value),
                None => self.unset_environment(name),
            })
            .collect()
    }

    fn set_option(&self, option: &str, value: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "set-option".to_string(),
            "-t".to_string(),
            self.session.clone(),
            option.to_string(),
            value.to_string(),
        ]
    }

    pub fn set_server_option(&self, option: &str, value: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "set-option".to_string(),
            "-g".to_string(),
            "-s".to_string(),
            option.to_string(),
            value.to_string(),
        ]
    }

    pub fn show_global_option(&self, option: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "show-options".to_string(),
            "-gqv".to_string(),
            option.to_string(),
        ]
    }

    fn set_environment(&self, name: &str, value: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "set-environment".to_string(),
            "-t".to_string(),
            self.session.clone(),
            name.to_string(),
            value.to_string(),
        ]
    }

    fn unset_environment(&self, name: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "set-environment".to_string(),
            "-u".to_string(),
            "-t".to_string(),
            self.session.clone(),
            name.to_string(),
        ]
    }

    pub fn terminal_overrides_with_rgb(current: &str) -> Option<String> {
        let current = current.trim();
        if terminal_overrides_include_truecolor(current) {
            return None;
        }
        if current.is_empty() {
            Some("*:RGB".to_string())
        } else {
            Some(format!("{current},*:RGB"))
        }
    }

    pub fn new_detached_session(&self) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "new-session".to_string(),
            "-d".to_string(),
            "-s".to_string(),
            self.session.clone(),
            "-n".to_string(),
            "dashboard".to_string(),
        ]
    }

    pub fn new_dashboard_window(&self, command: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "new-window".to_string(),
            "-t".to_string(),
            self.session_target(),
            "-n".to_string(),
            "dashboard".to_string(),
            command.to_string(),
        ]
    }

    pub fn respawn_dashboard(&self, command: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "respawn-pane".to_string(),
            "-k".to_string(),
            "-t".to_string(),
            format!("{}:dashboard", self.session),
            command.to_string(),
        ]
    }

    pub fn new_session(&self) -> Vec<String> {
        self.new_detached_session()
    }

    pub fn switch_client(&self, window_name: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "switch-client".to_string(),
            "-t".to_string(),
            format!("{}:{window_name}", self.session),
        ]
    }

    pub fn open_run_window(&self, window_name: &str) -> Vec<String> {
        self.switch_client(window_name)
    }

    pub fn bind_dashboard_shortcut(&self) -> Vec<String> {
        let condition = ["#{==:#{session_name},", &self.session, "}"].concat();
        vec![
            "tmux".to_string(),
            "bind-key".to_string(),
            "-n".to_string(),
            "C-q".to_string(),
            "if-shell".to_string(),
            "-F".to_string(),
            condition,
            format!("switch-client -t {}:dashboard", self.session),
            "send-keys C-q".to_string(),
        ]
    }

    pub fn detach_client(&self) -> Vec<String> {
        vec!["tmux".to_string(), "detach-client".to_string()]
    }

    pub fn dashboard_return_hint(&self, window_name: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "display-message".to_string(),
            "-t".to_string(),
            format!("{}:{window_name}", self.session),
            "Ctrl+Q returns to dashboard".to_string(),
        ]
    }

    pub fn kill_window(&self, window_name: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "kill-window".to_string(),
            "-t".to_string(),
            format!("{}:{window_name}", self.session),
        ]
    }

    pub fn attach_session(&self, window_name: &str) -> Vec<String> {
        vec![
            "tmux".to_string(),
            "attach-session".to_string(),
            "-t".to_string(),
            format!("{}:{window_name}", self.session),
        ]
    }

    fn session_target(&self) -> String {
        format!("{}:", self.session)
    }
}

fn terminal_overrides_include_truecolor(current: &str) -> bool {
    current.split(',').any(|entry| {
        entry
            .split_once(':')
            .map(|(_, features)| {
                features
                    .split(':')
                    .any(|feature| matches!(feature, "RGB" | "Tc"))
            })
            .unwrap_or(false)
    })
}

#[derive(Default)]
pub struct AgentCommandBuilder;

impl AgentCommandBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn launch(&self, plan: LaunchPlan) -> Vec<String> {
        match plan.agent {
            AgentKind::Codex => vec!["codex".to_string()],
            AgentKind::Claude => {
                let mut command = vec!["claude".to_string()];
                if let Some(session_id) = plan.session_id {
                    command.push("--session-id".to_string());
                    command.push(session_id.to_string());
                }
                command
            }
        }
    }

    pub fn restore(&self, plan: LaunchPlan) -> Vec<String> {
        match plan.agent {
            AgentKind::Codex => {
                let mut command = vec!["codex".to_string(), "resume".to_string()];
                if let Some(session_id) = plan.session_id {
                    command.push(session_id.to_string());
                } else {
                    command.push("--last".to_string());
                }
                command
            }
            AgentKind::Claude => {
                let mut command = vec!["claude".to_string()];
                if let Some(session_id) = plan.session_id {
                    command.push("--resume".to_string());
                    command.push(session_id.to_string());
                } else {
                    command.push("--continue".to_string());
                }
                command
            }
        }
    }
}

pub fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn shell_command_with_failure_diagnostics(args: &[String]) -> String {
    format!(
        "{}; agent_status=$?; if [ \"$agent_status\" -ne 0 ]; then printf '\\nAgent command exited with status %s. Starting a shell so this pane stays open.\\n' \"$agent_status\"; exec \"${{SHELL:-/bin/sh}}\"; fi",
        shell_join(args)
    )
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '='))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::agent::{AgentKind, LaunchPlan};

    use super::{AgentCommandBuilder, GitCommandBuilder};

    #[test]
    fn untracked_files_command_excludes_ignored_files_and_uses_null_output() {
        let command = GitCommandBuilder::new().nonignored_untracked_files("/repo");

        assert_eq!(
            command,
            vec![
                "git".to_string(),
                "-C".to_string(),
                "/repo".to_string(),
                "ls-files".to_string(),
                "--others".to_string(),
                "--exclude-standard".to_string(),
                "-z".to_string(),
            ]
        );
    }

    #[test]
    fn codex_restore_uses_exact_session_id_when_available() {
        let session_id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();

        let command = AgentCommandBuilder::new().restore(LaunchPlan {
            agent: AgentKind::Codex,
            worktree_path: "/repo-worktree".into(),
            session_id: Some(session_id),
        });

        assert_eq!(
            command,
            vec![
                "codex".to_string(),
                "resume".to_string(),
                session_id.to_string()
            ]
        );
    }

    #[test]
    fn codex_restore_falls_back_to_latest_worktree_session() {
        let command = AgentCommandBuilder::new().restore(LaunchPlan {
            agent: AgentKind::Codex,
            worktree_path: "/repo-worktree".into(),
            session_id: None,
        });

        assert_eq!(
            command,
            vec![
                "codex".to_string(),
                "resume".to_string(),
                "--last".to_string()
            ]
        );
    }

    #[test]
    fn claude_restore_uses_resume_with_exact_session_id() {
        let session_id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();

        let command = AgentCommandBuilder::new().restore(LaunchPlan {
            agent: AgentKind::Claude,
            worktree_path: "/repo-worktree".into(),
            session_id: Some(session_id),
        });

        assert_eq!(
            command,
            vec![
                "claude".to_string(),
                "--resume".to_string(),
                session_id.to_string()
            ]
        );
    }

    #[test]
    fn claude_restore_falls_back_to_current_directory_conversation() {
        let command = AgentCommandBuilder::new().restore(LaunchPlan {
            agent: AgentKind::Claude,
            worktree_path: "/repo-worktree".into(),
            session_id: None,
        });

        assert_eq!(
            command,
            vec!["claude".to_string(), "--continue".to_string()]
        );
    }
}
