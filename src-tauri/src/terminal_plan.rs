use std::path::{Path, PathBuf};

use agentctl_core::domain::RunRecord;
use serde::Deserialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TerminalLinkTarget {
    Url {
        url: String,
    },
    File {
        path: String,
        line: Option<u32>,
        column: Option<u32>,
    },
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

pub fn terminal_link_command(
    run: &RunRecord,
    target: &TerminalLinkTarget,
    generated_files_root: Option<&Path>,
) -> Result<TerminalCommand, String> {
    match target {
        TerminalLinkTarget::Url { url } => web_link_command(url),
        TerminalLinkTarget::File { path, line, column } => {
            file_link_command(run, path, *line, *column, generated_files_root)
        }
    }
}

fn web_link_command(url: &str) -> Result<TerminalCommand, String> {
    let authority = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| "terminal links only support HTTP and HTTPS URLs".to_string())?;
    if authority.is_empty()
        || authority.starts_with('/')
        || url
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err("terminal link URL is invalid".to_string());
    }

    Ok(TerminalCommand {
        program: "xdg-open".to_string(),
        args: vec![url.to_string()],
    })
}

fn file_link_command(
    run: &RunRecord,
    path: &str,
    line: Option<u32>,
    column: Option<u32>,
    generated_files_root: Option<&Path>,
) -> Result<TerminalCommand, String> {
    if line == Some(0) || column == Some(0) || (column.is_some() && line.is_none()) {
        return Err("terminal file link has an invalid location".to_string());
    }

    let worktree = run
        .worktree_path
        .canonicalize()
        .map_err(|err| format!("failed to resolve run worktree: {err}"))?;
    let requested = Path::new(path);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        worktree.join(requested)
    };
    let (file, scope) = canonical_terminal_file(&worktree, &candidate, generated_files_root)?;
    if scope == TerminalFileScope::Generated {
        if line.is_some() || column.is_some() {
            return Err("generated file links do not support source locations".to_string());
        }
        return Ok(TerminalCommand {
            program: "xdg-open".to_string(),
            args: vec![file.display().to_string()],
        });
    }

    let file_target = match line {
        Some(line) => match column {
            Some(column) => format!("{}:{line}:{column}", file.display()),
            None => format!("{}:{line}", file.display()),
        },
        None => file.display().to_string(),
    };

    Ok(TerminalCommand {
        program: "code".to_string(),
        args: if line.is_some() {
            vec!["--goto".to_string(), file_target]
        } else {
            vec![file_target]
        },
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalFileScope {
    Workspace,
    Generated,
}

fn canonical_terminal_file(
    worktree: &Path,
    candidate: &Path,
    generated_files_root: Option<&Path>,
) -> Result<(PathBuf, TerminalFileScope), String> {
    let file = candidate
        .canonicalize()
        .map_err(|err| format!("terminal file link does not exist: {err}"))?;
    if !file.is_file() {
        return Err("terminal file link does not point to a regular file".to_string());
    }
    if file.starts_with(worktree) {
        return Ok((file, TerminalFileScope::Workspace));
    }
    if generated_files_root
        .and_then(|root| root.canonicalize().ok())
        .is_some_and(|root| file.starts_with(root))
    {
        return Ok((file, TerminalFileScope::Generated));
    }
    Err(
        "terminal file link points outside the run worktree and generated files directory"
            .to_string(),
    )
}
