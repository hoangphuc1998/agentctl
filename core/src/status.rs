use chrono::Utc;

use crate::{
    agent::AgentKind,
    domain::{DetectionSource, ObservedState},
};

const ACTIVE_WORK_FRESHNESS_SECS: i64 = 15;
const RECENT_TERMINAL_LINES: usize = 20;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalSnapshot {
    pub pane_active: bool,
    pub current_command: String,
    pub pane_title: String,
    pub visible_text: String,
    pub activity_at: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StatusConfidence {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusSignal {
    ActiveWork,
    ExplicitInput,
    Completion,
    AgentPrompt,
    AgentRuntime,
    PaneUnavailable,
}

impl StatusSignal {
    fn precedence(self) -> u8 {
        match self {
            Self::ActiveWork => 50,
            Self::ExplicitInput => 40,
            Self::Completion => 30,
            Self::AgentPrompt => 20,
            Self::AgentRuntime => 10,
            Self::PaneUnavailable => 0,
        }
    }

    fn observed_state(self) -> ObservedState {
        match self {
            Self::ActiveWork | Self::AgentRuntime => ObservedState::Running,
            Self::ExplicitInput | Self::AgentPrompt => ObservedState::NeedsUser,
            Self::Completion => ObservedState::CompletedUnchecked,
            Self::PaneUnavailable => ObservedState::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusReason {
    ActiveWorkMarker,
    ExplicitInputRequest,
    CompletionMarker,
    AgentPrompt,
    AgentRuntime,
    PaneUnavailable,
    EvidenceExpired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusEvidence {
    pub signal: StatusSignal,
    pub source: DetectionSource,
    pub reason: StatusReason,
    pub confidence: StatusConfidence,
    pub observed_at: i64,
    pub valid_for_seconds: Option<i64>,
}

impl StatusEvidence {
    fn is_fresh(self, now: i64) -> bool {
        self.valid_for_seconds
            .is_none_or(|valid_for| now.saturating_sub(self.observed_at) <= valid_for)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusDecision {
    pub state: ObservedState,
    pub source: DetectionSource,
    pub reason: StatusReason,
    pub confidence: StatusConfidence,
}

pub fn observe_terminal(agent: AgentKind, pane: &TerminalSnapshot) -> StatusDecision {
    observe_terminal_at(agent, pane, Utc::now().timestamp())
}

pub fn observe_terminal_at(agent: AgentKind, pane: &TerminalSnapshot, now: i64) -> StatusDecision {
    let evidence = collect_terminal_evidence(agent, pane, now);
    reduce_status_evidence(&evidence, now)
}

pub fn collect_terminal_evidence(
    agent: AgentKind,
    pane: &TerminalSnapshot,
    now: i64,
) -> Vec<StatusEvidence> {
    if !pane.pane_active {
        return vec![StatusEvidence {
            signal: StatusSignal::PaneUnavailable,
            source: DetectionSource::Unknown,
            reason: StatusReason::PaneUnavailable,
            confidence: StatusConfidence::High,
            observed_at: now,
            valid_for_seconds: None,
        }];
    }

    let recent = recent_activity_text(&pane.visible_text);
    let text = recent.to_ascii_lowercase();
    let activity_at = pane.activity_at.unwrap_or(now);
    let mut evidence = Vec::with_capacity(4);

    if has_active_work_marker(agent, &text, &pane.pane_title) {
        evidence.push(StatusEvidence {
            signal: StatusSignal::ActiveWork,
            source: DetectionSource::Tmux,
            reason: StatusReason::ActiveWorkMarker,
            confidence: StatusConfidence::High,
            observed_at: activity_at,
            valid_for_seconds: Some(ACTIVE_WORK_FRESHNESS_SECS),
        });
    }

    if contains_any(
        &text,
        &[
            "approve command",
            "do you want to",
            "need input",
            "needs input",
            "need approval",
            "requires approval",
            "requires input",
            "waiting for input",
            "waiting for user",
            "press enter",
            "enter to submit answer",
            "[y/n]",
        ],
    ) {
        evidence.push(StatusEvidence {
            signal: StatusSignal::ExplicitInput,
            source: DetectionSource::Heuristic,
            reason: StatusReason::ExplicitInputRequest,
            confidence: StatusConfidence::High,
            observed_at: activity_at,
            valid_for_seconds: None,
        });
    }

    if has_completion_marker(&text) {
        evidence.push(StatusEvidence {
            signal: StatusSignal::Completion,
            source: DetectionSource::Heuristic,
            reason: StatusReason::CompletionMarker,
            confidence: StatusConfidence::Medium,
            observed_at: activity_at,
            valid_for_seconds: None,
        });
    }

    if has_agent_input_prompt(agent, &recent) {
        evidence.push(StatusEvidence {
            signal: StatusSignal::AgentPrompt,
            source: DetectionSource::Heuristic,
            reason: StatusReason::AgentPrompt,
            confidence: StatusConfidence::Medium,
            observed_at: activity_at,
            valid_for_seconds: None,
        });
    }

    if is_agent_runtime_command(agent, &pane.current_command) {
        evidence.push(StatusEvidence {
            signal: StatusSignal::AgentRuntime,
            source: DetectionSource::Tmux,
            reason: StatusReason::AgentRuntime,
            confidence: StatusConfidence::Medium,
            observed_at: now,
            valid_for_seconds: Some(ACTIVE_WORK_FRESHNESS_SECS),
        });
    }

    evidence
}

pub fn reduce_status_evidence(evidence: &[StatusEvidence], now: i64) -> StatusDecision {
    let selected = evidence
        .iter()
        .copied()
        .filter(|item| item.is_fresh(now))
        .max_by_key(|item| (item.signal.precedence(), item.confidence, item.observed_at));

    match selected {
        Some(item) => StatusDecision {
            state: item.signal.observed_state(),
            source: item.source,
            reason: item.reason,
            confidence: item.confidence,
        },
        None => StatusDecision {
            state: ObservedState::Unknown,
            source: DetectionSource::Unknown,
            reason: StatusReason::EvidenceExpired,
            confidence: StatusConfidence::Low,
        },
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn has_active_work_marker(agent: AgentKind, text: &str, title: &str) -> bool {
    if agent == AgentKind::Claude && contains_braille_spinner(title) {
        return true;
    }

    text.lines().any(|line| {
        let trimmed = line.trim_start();
        let marked_status = trimmed
            .strip_prefix('•')
            .or_else(|| trimmed.strip_prefix('◦'))
            .map(str::trim_start)
            .is_some_and(|status| {
                status.starts_with("running ")
                    || status.starts_with("working ")
                    || status.contains("esc to interrupt")
                    || status.contains("ctrl+c to interrupt")
            });

        marked_status
            || trimmed.contains("esc to interrupt")
            || trimmed.contains("ctrl+c to interrupt")
    })
}

fn contains_braille_spinner(text: &str) -> bool {
    text.chars()
        .any(|character| ('\u{2800}'..='\u{28ff}').contains(&character))
}

fn has_completion_marker(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        contains_any(
            line,
            &[
                "all tasks complete",
                "all tasks completed",
                "implementation complete",
                "ready for review",
                "worked for",
            ],
        ) || matches!(line, "complete" | "completed" | "done")
            || line.starts_with("✓ complete")
            || line.starts_with("✓ done")
            || line.starts_with("status: complete")
    })
}

fn has_agent_input_prompt(agent: AgentKind, text: &str) -> bool {
    let prompt = match agent {
        AgentKind::Codex => '›',
        AgentKind::Claude => '❯',
    };
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed == prompt.to_string() || trimmed.starts_with(&format!("{prompt} "))
    })
}

fn recent_activity_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .rposition(|line| is_turn_separator(line))
        .map(|index| index + 1)
        .unwrap_or(0);
    let recent = &lines[start..];
    let tail_start = recent.len().saturating_sub(RECENT_TERMINAL_LINES);
    recent[tail_start..].join("\n")
}

fn is_turn_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.chars().count() >= 20 && trimmed.chars().all(|ch| matches!(ch, '─' | '━'))
}

fn is_agent_runtime_command(agent: AgentKind, command: &str) -> bool {
    let command = command.trim();
    command == agent.as_str() || matches!(command, "node" | "nodejs" | "deno" | "bun")
}
