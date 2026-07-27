use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use agentctl_core::{
    agent::AgentKind,
    codex_thread::CodexThreadSnapshot,
    completion::CompletionCandidate,
    domain::{mark_seen_on_select, DetectionSource, ObservedState, RunRecord},
    tmux::{detect_observed_state, detection_source_for, PaneSnapshot},
};
use uuid::Uuid;

use crate::{
    models::{
        repo_tree_from_runs, AgentAttentionEvent, DashboardState, HostToolStatus, Suggestion,
    },
    run_classification,
};

pub use run_classification::{is_restorable_run, is_stale_run};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunObservation {
    pub state: ObservedState,
    pub source: DetectionSource,
    pub agent_session_id: Option<Uuid>,
}

pub fn observe_run(run: &RunRecord, pane: &PaneSnapshot) -> RunObservation {
    RunObservation {
        state: detect_observed_state(pane),
        source: detection_source_for(pane),
        agent_session_id: run.agent_session_id,
    }
}

pub fn codex_thread_assignments(
    runs: &[RunRecord],
    threads: &[CodexThreadSnapshot],
) -> HashMap<Uuid, Uuid> {
    let mut assignments = HashMap::new();
    let mut claimed = HashSet::new();

    for run in runs.iter().filter(|run| run.agent == AgentKind::Codex) {
        let Some(session_id) = run.agent_session_id else {
            continue;
        };
        assignments.insert(run.id, session_id);
        claimed.insert(session_id);
    }

    let mut unbound = runs
        .iter()
        .filter(|run| run.agent == AgentKind::Codex && run.agent_session_id.is_none())
        .collect::<Vec<_>>();
    unbound.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });

    for run in unbound {
        let candidate = threads
            .iter()
            .filter(|thread| thread.cwd == run.worktree_path)
            .filter(|thread| thread.created_at >= run.created_at)
            .filter(|thread| !claimed.contains(&thread.id))
            .max_by_key(|thread| (thread.created_at, thread.updated_at));
        if let Some(thread) = candidate {
            assignments.insert(run.id, thread.id);
            claimed.insert(thread.id);
        }
    }

    assignments
}

pub fn build_dashboard_state(
    active_runs: Vec<RunRecord>,
    active_repo_path: Option<PathBuf>,
    active_folder_path: Option<PathBuf>,
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
        active_folder_path: active_folder_path.map(|path| path.to_string_lossy().to_string()),
        host_tools,
    }
}

pub fn count_attention_runs(runs: &[RunRecord]) -> usize {
    runs.iter()
        .filter(|run| is_attention_state(run.observed_state))
        .count()
}

pub fn observed_state_after_refresh(
    detected_state: ObservedState,
    notification_seen_at: Option<i64>,
) -> ObservedState {
    if detected_state == ObservedState::CompletedUnchecked && notification_seen_at.is_some() {
        ObservedState::CompletedSeen
    } else {
        detected_state
    }
}

pub fn mark_selected_run_seen(
    runs: &mut [RunRecord],
    selected_run_id: Option<&str>,
    now: i64,
) -> Option<RunRecord> {
    let selected_run_id = selected_run_id?;
    let run = runs
        .iter_mut()
        .find(|run| run.id.to_string() == selected_run_id)?;
    let previous_state = run.observed_state;
    let previous_seen_at = run.notification_seen_at;
    mark_seen_on_select(run, now);
    if run.observed_state != previous_state || run.notification_seen_at != previous_seen_at {
        Some(run.clone())
    } else {
        None
    }
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
