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
    BlockingApproval,
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
            Self::BlockingApproval => 60,
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
            Self::BlockingApproval | Self::ExplicitInput | Self::AgentPrompt => {
                ObservedState::NeedsUser
            }
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

    let recent = if agent == AgentKind::Claude {
        terminal_tail_text(&pane.visible_text)
    } else {
        recent_activity_text(&pane.visible_text)
    };
    let text = recent.to_ascii_lowercase();
    let activity_at = pane.activity_at.unwrap_or(now);
    let mut evidence = Vec::with_capacity(5);

    if has_blocking_approval_prompt(agent, &text) {
        evidence.push(StatusEvidence {
            signal: StatusSignal::BlockingApproval,
            source: DetectionSource::Heuristic,
            reason: StatusReason::ExplicitInputRequest,
            confidence: StatusConfidence::High,
            observed_at: activity_at,
            valid_for_seconds: None,
        });
    }

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

    if has_completion_marker(agent, &text) {
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

fn has_blocking_approval_prompt(agent: AgentKind, text: &str) -> bool {
    if agent != AgentKind::Codex {
        return false;
    }

    let has_question = text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("would you like to ") && trimmed.ends_with('?')
    });
    let has_numbered_affirmative_choice = text.lines().any(|line| {
        line.trim_start()
            .strip_prefix('›')
            .unwrap_or(line.trim_start())
            .trim_start()
            .starts_with("1. yes")
    });

    has_question && has_numbered_affirmative_choice
}

fn has_completion_marker(agent: AgentKind, text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        (agent == AgentKind::Claude && is_claude_completion_footer(line))
            || contains_any(
                line,
                &[
                    "all tasks complete",
                    "all tasks completed",
                    "implementation complete",
                    "ready for review",
                    "worked for",
                ],
            )
            || matches!(line, "complete" | "completed" | "done")
            || line.starts_with("✓ complete")
            || line.starts_with("✓ done")
            || line.starts_with("status: complete")
    })
}

fn is_claude_completion_footer(line: &str) -> bool {
    let Some(summary) = line.strip_prefix('✻').map(str::trim_start) else {
        return false;
    };
    let Some((verb, duration)) = summary.split_once(" for ") else {
        return false;
    };
    !verb.is_empty() && duration.chars().any(|character| character.is_ascii_digit())
}

fn has_agent_input_prompt(agent: AgentKind, text: &str) -> bool {
    let prompt = match agent {
        AgentKind::Codex => '›',
        AgentKind::Claude => '❯',
    };
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.strip_prefix(prompt).is_some_and(|remainder| {
            remainder.is_empty() || remainder.chars().next().is_some_and(char::is_whitespace)
        })
    })
}

fn recent_activity_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .rposition(|line| is_turn_separator(line))
        .map(|index| index + 1)
        .unwrap_or(0);
    terminal_tail_lines(&lines[start..])
}

fn terminal_tail_text(text: &str) -> String {
    terminal_tail_lines(&text.lines().collect::<Vec<_>>())
}

fn terminal_tail_lines(lines: &[&str]) -> String {
    let tail_start = lines.len().saturating_sub(RECENT_TERMINAL_LINES);
    lines[tail_start..].join("\n")
}

fn is_turn_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.chars().count() >= 20 && trimmed.chars().all(|ch| matches!(ch, '─' | '━'))
}

fn is_agent_runtime_command(agent: AgentKind, command: &str) -> bool {
    let command = command.trim();
    command == agent.as_str() || matches!(command, "node" | "nodejs" | "deno" | "bun")
}
