use std::path::PathBuf;

use agentctl_core::{
    completion::CompletionCandidate,
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord},
};

use crate::models::{repo_tree_from_runs, DashboardState, HostToolStatus, Suggestion};

pub fn build_dashboard_state(
    active_runs: Vec<RunRecord>,
    active_repo_path: Option<PathBuf>,
    host_tools: Vec<HostToolStatus>,
    selected_run_id: Option<String>,
) -> DashboardState {
    let selected_run_id = selected_run_id
        .filter(|id| active_runs.iter().any(|run| run.id.to_string() == *id))
        .or_else(|| active_runs.first().map(|run| run.id.to_string()));
    let active_count = active_runs.len();
    let stale_count = active_runs.iter().filter(|run| is_stale_run(run)).count();

    DashboardState {
        repos: repo_tree_from_runs(&active_runs),
        selected_run_id,
        active_count,
        stale_count,
        active_repo_path: active_repo_path.map(|path| path.to_string_lossy().to_string()),
        host_tools,
    }
}

pub fn suggestions_from_candidates(candidates: Vec<CompletionCandidate>) -> Vec<Suggestion> {
    candidates
        .into_iter()
        .map(|candidate| Suggestion {
            value: candidate.value,
            detail: candidate.detail,
        })
        .collect()
}

pub fn is_stale_run(run: &RunRecord) -> bool {
    run.lifecycle == Lifecycle::Active
        && run.observed_state == ObservedState::Unknown
        && run.detection_source == DetectionSource::Unknown
}
