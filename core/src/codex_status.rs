use std::path::{Path, PathBuf};

use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ObservedState;

pub const CODEX_THREAD_LIST_RESPONSE_ID: u64 = 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CodexThreadActiveFlag {
    WaitingOnApproval,
    WaitingOnUserInput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CodexThreadStatus {
    NotLoaded,
    Idle,
    SystemError,
    Active {
        #[serde(default, rename = "activeFlags")]
        active_flags: Vec<CodexThreadActiveFlag>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CodexThreadSnapshot {
    pub id: Uuid,
    pub cwd: PathBuf,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
    pub status: CodexThreadStatus,
}

#[derive(Deserialize)]
struct ThreadListRpcResponse {
    id: Option<u64>,
    result: Option<ThreadListResult>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ThreadListResult {
    data: Vec<CodexThreadSnapshot>,
}

pub fn parse_thread_list_response(
    message: &str,
) -> Result<Option<Vec<CodexThreadSnapshot>>, String> {
    let response: ThreadListRpcResponse =
        serde_json::from_str(message).map_err(|err| format!("invalid Codex RPC message: {err}"))?;
    if response.id != Some(CODEX_THREAD_LIST_RESPONSE_ID) {
        return Ok(None);
    }
    if let Some(error) = response.error {
        return Err(format!("Codex thread/list failed: {error}"));
    }
    response
        .result
        .map(|result| Some(result.data))
        .ok_or_else(|| "Codex thread/list response had no result".to_string())
}

pub fn observed_state_from_codex_status(status: &CodexThreadStatus) -> Option<ObservedState> {
    match status {
        CodexThreadStatus::NotLoaded => None,
        CodexThreadStatus::Idle => Some(ObservedState::NeedsUser),
        CodexThreadStatus::SystemError => Some(ObservedState::Unknown),
        CodexThreadStatus::Active { active_flags } => {
            if active_flags.iter().any(|flag| {
                matches!(
                    flag,
                    CodexThreadActiveFlag::WaitingOnApproval
                        | CodexThreadActiveFlag::WaitingOnUserInput
                )
            }) {
                Some(ObservedState::NeedsUser)
            } else {
                Some(ObservedState::Running)
            }
        }
    }
}

pub fn select_thread_for_run<'a>(
    threads: &'a [CodexThreadSnapshot],
    session_id: Option<Uuid>,
    worktree_path: &Path,
) -> Option<&'a CodexThreadSnapshot> {
    if let Some(session_id) = session_id {
        if let Some(thread) = threads.iter().find(|thread| {
            thread.id == session_id && observed_state_from_codex_status(&thread.status).is_some()
        }) {
            return Some(thread);
        }
    }

    threads
        .iter()
        .filter(|thread| thread.cwd == worktree_path)
        .filter(|thread| observed_state_from_codex_status(&thread.status).is_some())
        .max_by_key(|thread| thread.updated_at)
}
