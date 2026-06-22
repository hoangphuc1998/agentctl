use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::AgentKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Lifecycle {
    Active,
    Stopped,
    Ended,
}

impl Lifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Stopped => "stopped",
            Self::Ended => "ended",
        }
    }

    pub fn from_storage(value: &str) -> Result<Self, String> {
        match value {
            "active" => Ok(Self::Active),
            "stopped" => Ok(Self::Stopped),
            "ended" => Ok(Self::Ended),
            other => Err(format!("unknown lifecycle `{other}`")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ObservedState {
    Running,
    NeedsUser,
    CompletedUnchecked,
    CompletedSeen,
    Unknown,
}

impl ObservedState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::NeedsUser => "needs-user",
            Self::CompletedUnchecked => "completed-unchecked",
            Self::CompletedSeen => "completed-seen",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_storage(value: &str) -> Result<Self, String> {
        match value {
            "running" => Ok(Self::Running),
            "needs-user" => Ok(Self::NeedsUser),
            "completed-unchecked" => Ok(Self::CompletedUnchecked),
            "completed-seen" => Ok(Self::CompletedSeen),
            "unknown" => Ok(Self::Unknown),
            other => Err(format!("unknown observed state `{other}`")),
        }
    }

    pub fn indicator(self) -> &'static str {
        match self {
            Self::Running => "●",
            Self::NeedsUser => "◐",
            Self::CompletedUnchecked => "✓",
            Self::CompletedSeen => "✓",
            Self::Unknown => "?",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum DetectionSource {
    Provider,
    Tmux,
    Heuristic,
    Unknown,
}

impl DetectionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Tmux => "tmux",
            Self::Heuristic => "heuristic",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_storage(value: &str) -> Result<Self, String> {
        match value {
            "provider" => Ok(Self::Provider),
            "tmux" => Ok(Self::Tmux),
            "heuristic" => Ok(Self::Heuristic),
            "unknown" => Ok(Self::Unknown),
            other => Err(format!("unknown detection source `{other}`")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: Uuid,
    pub repo_path: PathBuf,
    pub repo_name: String,
    pub tag: String,
    pub run_name: String,
    pub agent: AgentKind,
    pub lifecycle: Lifecycle,
    pub observed_state: ObservedState,
    pub detection_source: DetectionSource,
    pub branch: String,
    pub base_ref: String,
    pub worktree_path: PathBuf,
    pub tmux_session: Option<String>,
    pub tmux_window: Option<String>,
    pub tmux_pane: Option<String>,
    pub agent_session_id: Option<Uuid>,
    pub notification_seen_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoGroup {
    pub repo_name: String,
    pub tags: Vec<TagGroup>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagGroup {
    pub tag: String,
    pub runs: Vec<RunRecord>,
}

pub fn group_active_runs(runs: &[RunRecord]) -> Vec<RepoGroup> {
    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<RunRecord>>> = BTreeMap::new();
    for run in runs.iter().filter(|run| run.lifecycle == Lifecycle::Active) {
        grouped
            .entry(run.repo_name.clone())
            .or_default()
            .entry(run.tag.clone())
            .or_default()
            .push(run.clone());
    }

    grouped
        .into_iter()
        .map(|(repo_name, tags)| RepoGroup {
            repo_name,
            tags: tags
                .into_iter()
                .map(|(tag, mut runs)| {
                    runs.sort_by(|left, right| left.run_name.cmp(&right.run_name));
                    TagGroup { tag, runs }
                })
                .collect(),
        })
        .collect()
}

pub fn mark_seen_on_select(run: &mut RunRecord, now: i64) {
    if run.observed_state == ObservedState::CompletedUnchecked {
        run.observed_state = ObservedState::CompletedSeen;
        run.notification_seen_at = Some(now);
        run.updated_at = now;
    }
}
