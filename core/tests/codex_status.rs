use std::path::Path;

use agentctl_core::{
    codex_status::{
        observed_state_from_codex_status, parse_thread_list_response, select_thread_for_run,
        CodexThreadActiveFlag, CodexThreadStatus,
    },
    domain::ObservedState,
};
use uuid::Uuid;

const RUN_THREAD_ID: &str = "019f6e53-0293-7ae3-bc7b-20d4a5cde0b4";
const OLD_THREAD_ID: &str = "019f694c-17a4-7942-bd3f-b1c43e5f36f0";

#[test]
fn parses_official_codex_thread_status_response() {
    let response = format!(
        r#"{{
            "id": 2,
            "result": {{
                "data": [
                    {{
                        "id": "{RUN_THREAD_ID}",
                        "cwd": "/repos/agent-manager-worktrees/feat/status",
                        "createdAt": 40,
                        "updatedAt": 42,
                        "status": {{
                            "type": "active",
                            "activeFlags": ["waitingOnUserInput"]
                        }}
                    }},
                    {{
                        "id": "{OLD_THREAD_ID}",
                        "cwd": "/repos/agent-manager-worktrees/feat/status",
                        "createdAt": 39,
                        "updatedAt": 41,
                        "status": {{ "type": "notLoaded" }}
                    }}
                ],
                "nextCursor": null,
                "backwardsCursor": null
            }}
        }}"#
    );

    let threads = parse_thread_list_response(&response)
        .expect("valid thread list response")
        .expect("matching response id");

    assert_eq!(threads.len(), 2);
    assert_eq!(threads[0].id, Uuid::parse_str(RUN_THREAD_ID).unwrap());
    assert_eq!(threads[0].created_at, 40);
    assert_eq!(
        threads[0].status,
        CodexThreadStatus::Active {
            active_flags: vec![CodexThreadActiveFlag::WaitingOnUserInput]
        }
    );
}

#[test]
fn maps_official_codex_status_without_terminal_text() {
    assert_eq!(
        observed_state_from_codex_status(&CodexThreadStatus::Active {
            active_flags: vec![]
        }),
        Some(ObservedState::Running)
    );
    assert_eq!(
        observed_state_from_codex_status(&CodexThreadStatus::Active {
            active_flags: vec![CodexThreadActiveFlag::WaitingOnApproval]
        }),
        Some(ObservedState::NeedsUser)
    );
    assert_eq!(
        observed_state_from_codex_status(&CodexThreadStatus::Idle),
        Some(ObservedState::NeedsUser)
    );
    assert_eq!(
        observed_state_from_codex_status(&CodexThreadStatus::SystemError),
        Some(ObservedState::Unknown)
    );
    assert_eq!(
        observed_state_from_codex_status(&CodexThreadStatus::NotLoaded),
        None
    );
}

#[test]
fn selects_loaded_thread_by_session_or_latest_worktree() {
    let response = format!(
        r#"{{"id":2,"result":{{"data":[
            {{"id":"{OLD_THREAD_ID}","cwd":"/repos/worktree","createdAt":39,"updatedAt":41,"status":{{"type":"idle"}}}},
            {{"id":"{RUN_THREAD_ID}","cwd":"/repos/worktree","createdAt":40,"updatedAt":42,"status":{{"type":"active","activeFlags":[]}}}}
        ],"nextCursor":null,"backwardsCursor":null}}}}"#
    );
    let threads = parse_thread_list_response(&response).unwrap().unwrap();

    let exact = select_thread_for_run(
        &threads,
        Some(Uuid::parse_str(OLD_THREAD_ID).unwrap()),
        Path::new("/repos/worktree"),
    )
    .expect("exact loaded session");
    assert_eq!(exact.id, Uuid::parse_str(OLD_THREAD_ID).unwrap());

    let latest = select_thread_for_run(&threads, None, Path::new("/repos/worktree"))
        .expect("latest loaded worktree session");
    assert_eq!(latest.id, Uuid::parse_str(RUN_THREAD_ID).unwrap());
}
