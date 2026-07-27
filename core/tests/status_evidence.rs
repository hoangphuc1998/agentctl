use agentctl_core::{
    agent::AgentKind,
    domain::{DetectionSource, ObservedState},
    status::{
        observe_terminal_at, reduce_status_evidence, StatusConfidence, StatusEvidence,
        StatusReason, StatusSignal,
    },
    tmux::PaneSnapshot,
};

fn snapshot(command: &str, title: &str, visible_text: &str, activity_at: i64) -> PaneSnapshot {
    PaneSnapshot {
        pane_active: true,
        current_command: command.to_string(),
        pane_title: title.to_string(),
        visible_text: visible_text.to_string(),
        activity_at: Some(activity_at),
    }
}

#[test]
fn claude_title_spinner_is_strong_work_evidence_even_during_shell_tools() {
    let pane = snapshot("bash", "⠹ Refactoring status handling", "", 100);

    let decision = observe_terminal_at(AgentKind::Claude, &pane, 100);

    assert_eq!(decision.state, ObservedState::Running);
    assert_eq!(decision.source, DetectionSource::Tmux);
    assert_eq!(decision.reason, StatusReason::ActiveWorkMarker);
    assert_eq!(decision.confidence, StatusConfidence::High);
}

#[test]
fn claude_prompt_is_agent_specific_user_attention_evidence() {
    let pane = snapshot(
        "claude",
        "Claude Code",
        "I need a choice before continuing.\n❯ ",
        100,
    );

    let decision = observe_terminal_at(AgentKind::Claude, &pane, 100);

    assert_eq!(decision.state, ObservedState::NeedsUser);
    assert_eq!(decision.reason, StatusReason::AgentPrompt);
}

#[test]
fn stale_work_marker_falls_back_to_the_live_runtime_process() {
    let pane = snapshot(
        "node",
        "Codex CLI",
        "◦ Working (10s • esc to interrupt)",
        10,
    );

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::Running);
    assert_eq!(decision.reason, StatusReason::AgentRuntime);
    assert_eq!(decision.confidence, StatusConfidence::Medium);
}

#[test]
fn ordinary_prose_containing_done_does_not_report_completion() {
    let pane = snapshot(
        "node",
        "Codex CLI",
        "I have not done the migration yet.\n",
        100,
    );

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::Running);
    assert_eq!(decision.reason, StatusReason::AgentRuntime);
}

#[test]
fn old_attention_text_outside_the_recent_terminal_tail_is_ignored() {
    let mut text = String::from("Need input before continuing.\n");
    for index in 0..21 {
        text.push_str(&format!("ordinary output line {index}\n"));
    }
    let pane = snapshot("node", "Codex CLI", &text, 100);

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::Running);
    assert_eq!(decision.reason, StatusReason::AgentRuntime);
}

#[test]
fn active_work_outranks_completion_and_input_words_in_the_same_snapshot() {
    let pane = snapshot(
        "node",
        "Codex CLI",
        concat!(
            "Need input before declaring the implementation complete and ready for review.\n",
            "◦ Running cargo test\n",
            "◦ Working (3m 09s • esc to interrupt)\n",
        ),
        100,
    );

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::Running);
    assert_eq!(decision.reason, StatusReason::ActiveWorkMarker);
}

#[test]
fn final_completion_marker_outranks_the_idle_codex_prompt() {
    let pane = snapshot(
        "codex",
        "Codex CLI",
        concat!(
            "Implementation complete.\n",
            "Ready for review.\n",
            "─ Worked for 2m 14s ─\n",
            "› Find and fix a bug in @filename\n",
        ),
        100,
    );

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::CompletedUnchecked);
    assert_eq!(decision.source, DetectionSource::Heuristic);
    assert_eq!(decision.reason, StatusReason::CompletionMarker);
}

#[test]
fn codex_prompt_reports_user_attention_without_attention_words() {
    let pane = snapshot(
        "node",
        "Codex CLI",
        concat!(
            "The implementation has two viable approaches. Which approach should I take?\n",
            "› Use the safer approach\n",
            "gpt-5.6-sol high · ~/project\n",
        ),
        100,
    );

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::NeedsUser);
    assert_eq!(decision.reason, StatusReason::AgentPrompt);
}

#[test]
fn interruptible_background_work_outranks_the_codex_prompt() {
    let pane = snapshot(
        "node",
        "Codex CLI",
        concat!(
            "• Waiting for background terminal (16m 42s • esc to interrupt)\n",
            "› Use /skills to list available skills\n",
            "gpt-5.6-sol high · ~/project\n",
        ),
        100,
    );

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::Running);
    assert_eq!(decision.reason, StatusReason::ActiveWorkMarker);
}

#[test]
fn explicit_input_request_reports_user_attention() {
    let pane = snapshot(
        "codex",
        "Codex CLI",
        "Need input\nReview the command before continuing.\n",
        100,
    );

    let decision = observe_terminal_at(AgentKind::Codex, &pane, 100);

    assert_eq!(decision.state, ObservedState::NeedsUser);
    assert_eq!(decision.reason, StatusReason::ExplicitInputRequest);
    assert_eq!(decision.confidence, StatusConfidence::High);
}

#[test]
fn reducer_ignores_expired_terminal_activity_in_favor_of_fresh_provider_evidence() {
    let evidence = [
        StatusEvidence {
            signal: StatusSignal::ActiveWork,
            source: DetectionSource::Tmux,
            reason: StatusReason::ActiveWorkMarker,
            confidence: StatusConfidence::High,
            observed_at: 10,
            valid_for_seconds: Some(15),
        },
        StatusEvidence {
            signal: StatusSignal::Completion,
            source: DetectionSource::Provider,
            reason: StatusReason::CompletionMarker,
            confidence: StatusConfidence::High,
            observed_at: 100,
            valid_for_seconds: Some(30),
        },
    ];

    let decision = reduce_status_evidence(&evidence, 100);

    assert_eq!(decision.state, ObservedState::CompletedUnchecked);
    assert_eq!(decision.source, DetectionSource::Provider);
}
