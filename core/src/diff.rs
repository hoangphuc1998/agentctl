use std::{collections::BTreeMap, error::Error, path::Path, process::Command};

use chrono::Utc;
use serde::Serialize;

use crate::domain::RunRecord;

pub type DiffResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDiff {
    pub run_id: String,
    pub base_ref: String,
    pub base_commit: Option<String>,
    pub worktree_path: String,
    pub files: Vec<RunDiffFile>,
    pub file_count: usize,
    pub additions: usize,
    pub deletions: usize,
    pub generated_at: i64,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDiffFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
    pub binary: bool,
    pub patch: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileStat {
    additions: usize,
    deletions: usize,
    binary: bool,
}

pub fn load_run_diff(run: &RunRecord) -> DiffResult<RunDiff> {
    let (base_commit, warning) = diff_base(run)?;
    let mut files = tracked_diff_files(run.worktree_path.as_path(), &base_commit)?;

    files.sort_by(|left, right| left.path.cmp(&right.path));
    let additions = files.iter().map(|file| file.additions).sum();
    let deletions = files.iter().map(|file| file.deletions).sum();

    Ok(RunDiff {
        run_id: run.id.to_string(),
        base_ref: run.base_ref.clone(),
        base_commit: Some(base_commit),
        worktree_path: run.worktree_path.to_string_lossy().to_string(),
        file_count: files.len(),
        additions,
        deletions,
        files,
        generated_at: Utc::now().timestamp(),
        warning,
    })
}

fn diff_base(run: &RunRecord) -> DiffResult<(String, Option<String>)> {
    if let Some(base_commit) = run.base_commit.as_deref().filter(|value| !value.is_empty()) {
        return Ok((base_commit.to_string(), None));
    }

    let reference = format!("{}^{{commit}}", run.base_ref);
    let output = git_output(
        run.worktree_path.as_path(),
        &["rev-parse", "--verify", reference.as_str()],
    )?;
    Ok((
        output.trim().to_string(),
        Some(format!(
            "This run was created before exact base commits were recorded; diff uses the current `{}` commit.",
            run.base_ref
        )),
    ))
}

fn tracked_diff_files(worktree_path: &Path, base_commit: &str) -> DiffResult<Vec<RunDiffFile>> {
    let stats = parse_numstat(&git_bytes(
        worktree_path,
        &[
            "diff",
            "--find-renames",
            "--numstat",
            "-z",
            base_commit,
            "HEAD",
            "--",
        ],
    )?);
    let mut files = parse_name_status(&git_bytes(
        worktree_path,
        &[
            "diff",
            "--find-renames",
            "--name-status",
            "-z",
            base_commit,
            "HEAD",
            "--",
        ],
    )?);

    for file in &mut files {
        if let Some(stat) = stats.get(&file.path) {
            file.additions = stat.additions;
            file.deletions = stat.deletions;
            file.binary = stat.binary;
        }
        if file.binary {
            file.message = Some("Binary file not shown.".to_string());
            continue;
        }
        file.patch = Some(git_output(
            worktree_path,
            &[
                "diff",
                "--find-renames",
                base_commit,
                "HEAD",
                "--",
                file.path.as_str(),
            ],
        )?);
    }

    Ok(files)
}

fn parse_name_status(output: &[u8]) -> Vec<RunDiffFile> {
    let tokens = nul_strings(output);
    let mut files = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let status_code = &tokens[index];
        let Some(status_kind) = status_code.chars().next() else {
            index += 1;
            continue;
        };
        if matches!(status_kind, 'R' | 'C') {
            if index + 2 >= tokens.len() {
                break;
            }
            files.push(RunDiffFile {
                path: tokens[index + 2].clone(),
                old_path: Some(tokens[index + 1].clone()),
                status: status_label(status_kind).to_string(),
                additions: 0,
                deletions: 0,
                binary: false,
                patch: None,
                message: None,
            });
            index += 3;
        } else {
            if index + 1 >= tokens.len() {
                break;
            }
            files.push(RunDiffFile {
                path: tokens[index + 1].clone(),
                old_path: None,
                status: status_label(status_kind).to_string(),
                additions: 0,
                deletions: 0,
                binary: false,
                patch: None,
                message: None,
            });
            index += 2;
        }
    }
    files
}

fn parse_numstat(output: &[u8]) -> BTreeMap<String, FileStat> {
    let tokens = nul_strings(output);
    let mut stats = BTreeMap::new();
    let mut index = 0;
    while index < tokens.len() {
        let fields = tokens[index].split('\t').collect::<Vec<_>>();
        if fields.len() < 3 {
            index += 1;
            continue;
        }

        let binary = fields[0] == "-" || fields[1] == "-";
        let stat = FileStat {
            additions: fields[0].parse().unwrap_or(0),
            deletions: fields[1].parse().unwrap_or(0),
            binary,
        };

        if fields[2].is_empty() {
            if index + 2 >= tokens.len() {
                break;
            }
            stats.insert(tokens[index + 2].clone(), stat);
            index += 3;
        } else {
            stats.insert(fields[2].to_string(), stat);
            index += 1;
        }
    }
    stats
}

fn status_label(status: char) -> &'static str {
    match status {
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        'U' => "unmerged",
        _ => "modified",
    }
}

fn nul_strings(output: &[u8]) -> Vec<String> {
    output
        .split(|byte| *byte == 0)
        .filter(|token| !token.is_empty())
        .map(|token| String::from_utf8_lossy(token).to_string())
        .collect()
}

fn git_output(repo_path: &Path, args: &[&str]) -> DiffResult<String> {
    Ok(String::from_utf8_lossy(&git_bytes(repo_path, args)?).to_string())
}

fn git_bytes(repo_path: &Path, args: &[&str]) -> DiffResult<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            repo_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into())
    }
}
