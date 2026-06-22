# Agent Attention Notifications Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native notifications and an in-app attention badge when agent runs newly need input or complete.

**Architecture:** Rust detects state transitions during `dashboard_state`, emits `agent:attention`, and includes `attentionCount` in `DashboardState`. React listens for the event, requests browser notification permission, and renders the badge count in the top bar.

**Tech Stack:** Rust/Tauri 2, TypeScript React, Vitest, Cargo tests.

---

### Task 1: Backend Attention State

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/services.rs`
- Test: `src-tauri/tests/desktop_state.rs`

- [ ] Write failing Rust tests for `attention_count` and transition filtering.
- [ ] Run `cargo test -p agent-manager-desktop dashboard_state_counts_attention_runs dashboard_attention_events_only_fire_on_new_attention_states`.
- [ ] Add `attention_count` to `DashboardState`.
- [ ] Add pure service helpers for attention states and transition events.
- [ ] Re-run the targeted Cargo tests.

### Task 2: Backend Event Emission

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/tests/desktop_state.rs`

- [ ] Keep event construction pure and tested in services.
- [ ] Update `dashboard_state` to accept `AppHandle`.
- [ ] Emit `agent:attention` after each registry update when a transition event exists.
- [ ] Run `cargo test -p agent-manager-desktop`.

### Task 3: Frontend Event Listener and Badge

**Files:**
- Modify: `src/types.ts`
- Modify: `src/api.ts`
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Modify: `src/styles.css`

- [ ] Write failing Vitest coverage for badge rendering and notification dispatch from `agent:attention`.
- [ ] Add `attentionCount` and `AgentAttentionEvent` types.
- [ ] Add `listenAgentAttention`.
- [ ] Render the top-bar badge and dispatch native browser notifications.
- [ ] Re-run targeted Vitest tests.

### Task 4: Verification and Handoff

**Files:**
- Modify: `feature_list.json`
- Modify: `progress.md`

- [ ] Run `npm install` if `node_modules` is missing.
- [ ] Run `npm test`.
- [ ] Run `npm run build`.
- [ ] Run `./init.sh`.
- [ ] Record verification evidence in `feature_list.json` and `progress.md`.
- [ ] Commit the safe final state.
