use agentctl_core::domain::RunRecord;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn tmux_attach_command(
    run: &RunRecord,
    fallback_session: &str,
) -> Result<TerminalCommand, String> {
    let session = run.tmux_session.as_deref().unwrap_or(fallback_session);
    let Some(window) = run.tmux_window.as_deref() else {
        return Err(format!(
            "run `{}` does not have a tmux window",
            run.run_name
        ));
    };

    Ok(TerminalCommand {
        program: "env".to_string(),
        args: vec![
            "TERM=xterm-256color".to_string(),
            "COLORTERM=truecolor".to_string(),
            "tmux".to_string(),
            "attach-session".to_string(),
            "-t".to_string(),
            format!("{session}:{window}"),
        ],
    })
}
