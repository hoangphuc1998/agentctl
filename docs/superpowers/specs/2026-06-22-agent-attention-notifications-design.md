# Agent Attention Notifications Design

## Goal

Notify the user when an agent run newly needs user input or has just completed, and show a persistent in-app badge count for current attention-worthy runs.

## Architecture

The Rust dashboard refresh remains the source of truth because it owns tmux snapshot inspection and registry updates. During each `dashboard_state` call, the backend compares each active run's previous stored observed state with the newly detected state. It emits an `agent:attention` event only when a run transitions into `needs-user` or `completed-unchecked`.

The frontend listens for `agent:attention` and displays a native desktop notification. The top bar renders `attentionCount`, computed from backend dashboard state, for all current `needs-user` and `completed-unchecked` runs.

## Event Payload

`agent:attention` includes the run id, run name, repo name, agent kind, observed state, and notification title/body. This keeps React presentation simple and avoids re-deriving notification text in multiple places.

## Scope

This feature does not change persistent seen semantics. Selecting a run does not mark it seen, and notification events are only emitted on state transitions discovered during refresh.

## Testing

Rust tests cover attention counting and transition filtering. React tests cover badge rendering and native notification dispatch from a backend event.
