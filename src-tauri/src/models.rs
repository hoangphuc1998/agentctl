use std::{collections::BTreeMap, path::PathBuf, process::Command};

use agentctl_core::{
    agent::AgentKind,
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
};
use serde::{Deserialize, Serialize};

use crate::run_classification::is_restorable_run;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub id: String,
    pub repo_path: String,
    pub repo_name: String,
    pub tag: String,
    pub run_name: String,
    pub agent: String,
    pub lifecycle: String,
    pub observed_state: String,
    pub detection_source: String,
    pub branch: String,
    pub base_ref: String,
    pub worktree_path: String,
    pub tmux_session: Option<String>,
    pub tmux_window: Option<String>,
    pub restorable: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<RunRecord> for RunView {
    fn from(run: RunRecord) -> Self {
        let restorable = is_restorable_run(&run);
        Self {
            id: run.id.to_string(),
            repo_path: path_string(run.repo_path),
            repo_name: run.repo_name,
            tag: run.tag,
            run_name: run.run_name,
            agent: agent_string(run.agent),
            lifecycle: lifecycle_string(run.lifecycle),
            observed_state: observed_state_string(run.observed_state),
            detection_source: detection_source_string(run.detection_source),
            branch: run.branch,
            base_ref: run.base_ref,
            worktree_path: path_string(run.worktree_path),
            tmux_session: run.tmux_session,
            tmux_window: run.tmux_window,
            restorable,
            created_at: run.created_at,
            updated_at: run.updated_at,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoNode {
    pub repo_name: String,
    pub repo_path: String,
    pub runs: Vec<RunView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostToolStatus {
    pub name: String,
    pub available: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardState {
    pub repos: Vec<RepoNode>,
    pub selected_run_id: Option<String>,
    pub active_count: usize,
    pub stale_count: usize,
    pub restorable_count: usize,
    pub active_repo_path: Option<String>,
    pub host_tools: Vec<HostToolStatus>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunPayload {
    pub repo_path: String,
    pub base_ref: String,
    pub tag: String,
    pub run_name: String,
    pub agent: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub value: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub message: String,
    pub run: Option<RunView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeActionResult {
    pub message: String,
    pub target_branch: String,
    pub run: RunView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStarted {
    pub terminal_id: String,
    pub run_id: String,
}

pub fn repo_tree_from_runs(runs: &[RunRecord]) -> Vec<RepoNode> {
    let mut grouped: BTreeMap<(String, String), Vec<RunView>> = BTreeMap::new();
    for run in runs {
        grouped
            .entry((
                run.repo_name.clone(),
                run.repo_path.to_string_lossy().to_string(),
            ))
            .or_default()
            .push(run.clone().into());
    }

    grouped
        .into_iter()
        .map(|((repo_name, repo_path), mut runs)| {
            runs.sort_by(|left, right| {
                left.run_name
                    .cmp(&right.run_name)
                    .then_with(|| left.tag.cmp(&right.tag))
            });
            RepoNode {
                repo_name,
                repo_path,
                runs,
            }
        })
        .collect()
}

pub fn host_tool_statuses() -> Vec<HostToolStatus> {
    ["git", "tmux", "codex", "claude", "code"]
        .into_iter()
        .map(host_tool_status)
        .collect()
}

fn host_tool_status(name: &str) -> HostToolStatus {
    let available = Command::new("sh")
        .args(["-lc", &format!("command -v {name}")])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    let required = matches!(name, "git" | "tmux" | "codex");
    let detail = match (required, available) {
        (true, true) => "required".to_string(),
        (true, false) => "required but missing".to_string(),
        (false, true) => "optional".to_string(),
        (false, false) => "optional and missing".to_string(),
    };
    HostToolStatus {
        name: name.to_string(),
        available,
        detail,
    }
}

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

fn agent_string(agent: AgentKind) -> String {
    agent.as_str().to_string()
}

fn lifecycle_string(lifecycle: Lifecycle) -> String {
    lifecycle.as_str().to_string()
}

fn observed_state_string(state: ObservedState) -> String {
    state.as_str().to_string()
}

fn detection_source_string(source: DetectionSource) -> String {
    source.as_str().to_string()
}
