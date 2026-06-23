use std::path::PathBuf;

use agentctl_core::{
    completion::CompletionCandidate,
    domain::{ObservedState, RunRecord},
};

use crate::{
    models::{
        repo_tree_from_runs, AgentAttentionEvent, DashboardState, HostToolStatus, Suggestion,
    },
    run_classification,
};

pub use run_classification::{is_restorable_run, is_stale_run};

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
    let attention_count = count_attention_runs(&active_runs);
    let stale_count = active_runs.iter().filter(|run| is_stale_run(run)).count();
    let restorable_count = active_runs
        .iter()
        .filter(|run| is_restorable_run(run))
        .count();

    DashboardState {
        repos: repo_tree_from_runs(&active_runs),
        selected_run_id,
        active_count,
        attention_count,
        stale_count,
        restorable_count,
        active_repo_path: active_repo_path.map(|path| path.to_string_lossy().to_string()),
        host_tools,
    }
}

pub fn count_attention_runs(runs: &[RunRecord]) -> usize {
    runs.iter()
        .filter(|run| is_attention_state(run.observed_state))
        .count()
}

pub fn agent_attention_event_for_transition(
    previous_state: ObservedState,
    run: &RunRecord,
) -> Option<AgentAttentionEvent> {
    if previous_state == run.observed_state || !is_attention_state(run.observed_state) {
        return None;
    }

    let (title, body) = match run.observed_state {
        ObservedState::NeedsUser => (
            "Agent needs input",
            format!("{} in {} is waiting for you.", run.run_name, run.repo_name),
        ),
        ObservedState::CompletedUnchecked => (
            "Agent completed",
            format!("{} in {} is ready for review.", run.run_name, run.repo_name),
        ),
        _ => return None,
    };

    Some(AgentAttentionEvent {
        run_id: run.id.to_string(),
        run_name: run.run_name.clone(),
        repo_name: run.repo_name.clone(),
        agent: run.agent.as_str().to_string(),
        observed_state: run.observed_state.as_str().to_string(),
        title: title.to_string(),
        body,
    })
}

pub fn agent_system_notification_for_event(event: &AgentAttentionEvent) -> (&str, &str) {
    (&event.title, &event.body)
}

fn is_attention_state(state: ObservedState) -> bool {
    matches!(
        state,
        ObservedState::NeedsUser | ObservedState::CompletedUnchecked
    )
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
