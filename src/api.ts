import { invoke } from "@tauri-apps/api/core";
import type { EventCallback, UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import type { AgentAttentionEvent, CreateRunPayload, DashboardState, RunView } from "./types";

export interface ActionResult {
  message: string;
  run: RunView | null;
}

export interface MergeActionResult {
  message: string;
  targetBranch: string;
  run: RunView;
}

export interface Suggestion {
  value: string;
  detail: string;
}

export interface TerminalStarted {
  terminalId: string;
  runId: string;
}

export interface TerminalOutputEvent {
  terminalId: string;
  runId: string;
  data: string;
}

export interface TerminalClosedEvent {
  terminalId: string;
  runId: string;
}

export function dashboardState(selectedRunId?: string | null): Promise<DashboardState> {
  return invoke("dashboard_state", { selectedRunId });
}

export function createRun(payload: CreateRunPayload): Promise<ActionResult> {
  return invoke("create_run", { payload });
}

export function restoreRun(runId: string): Promise<ActionResult> {
  return invoke("restore_run", { runId });
}

export function stopRun(runId: string): Promise<ActionResult> {
  return invoke("stop_run", { runId });
}

export function endRun(runId: string): Promise<ActionResult> {
  return invoke("end_run", { runId });
}

export function mergeRun(runId: string): Promise<MergeActionResult | null> {
  return invoke("merge_run", { runId });
}

export function openInVsCode(runId: string): Promise<ActionResult> {
  return invoke("open_in_vscode", { runId });
}

export function cleanupStaleRuns(): Promise<ActionResult> {
  return invoke("cleanup_stale_runs");
}

export function repoSuggestions(input: string): Promise<Suggestion[]> {
  return invoke("repo_suggestions", { input });
}

export function baseRefSuggestions(repoPath: string, input: string): Promise<Suggestion[]> {
  return invoke("base_ref_suggestions", { repoPath, input });
}

export function startTerminal(runId: string, cols: number, rows: number): Promise<TerminalStarted> {
  return invoke("start_terminal", { runId, cols, rows });
}

export function terminalInput(terminalId: string, data: string): Promise<void> {
  return invoke("terminal_input", { terminalId, data });
}

export function resizeTerminal(terminalId: string, cols: number, rows: number): Promise<void> {
  return invoke("resize_terminal", { terminalId, cols, rows });
}

export function closeTerminal(terminalId: string): Promise<void> {
  return invoke("close_terminal", { terminalId });
}

export function listenTerminalOutput(
  callback: EventCallback<TerminalOutputEvent>
): Promise<UnlistenFn> {
  return listen<TerminalOutputEvent>("terminal:output", callback);
}

export function listenTerminalClosed(
  callback: EventCallback<TerminalClosedEvent>
): Promise<UnlistenFn> {
  return listen<TerminalClosedEvent>("terminal:closed", callback);
}

export function listenAgentAttention(
  callback: EventCallback<AgentAttentionEvent>
): Promise<UnlistenFn> {
  return listen<AgentAttentionEvent>("agent:attention", callback);
}
