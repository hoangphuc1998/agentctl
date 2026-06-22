export type AgentKind = "codex" | "claude";
export type Lifecycle = "active" | "stopped" | "ended";
export type ObservedState =
  | "running"
  | "needs-user"
  | "completed-unchecked"
  | "completed-seen"
  | "unknown";

export interface RunView {
  id: string;
  repoPath: string;
  repoName: string;
  tag: string;
  runName: string;
  agent: AgentKind;
  lifecycle: Lifecycle;
  observedState: ObservedState;
  detectionSource: string;
  branch: string;
  baseRef: string;
  worktreePath: string;
  restorable: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface RepoNode {
  repoName: string;
  repoPath: string;
  runs: RunView[];
}

export interface HostToolStatus {
  name: string;
  available: boolean;
  detail: string;
}

export interface DashboardState {
  repos: RepoNode[];
  selectedRunId: string | null;
  activeCount: number;
  attentionCount: number;
  staleCount: number;
  restorableCount: number;
  activeRepoPath: string | null;
  hostTools: HostToolStatus[];
}

export interface CreateRunPayload {
  repoPath: string;
  baseRef: string;
  tag: string;
  runName: string;
  agent: AgentKind;
}

export interface AgentAttentionEvent {
  runId: string;
  runName: string;
  repoName: string;
  agent: AgentKind;
  observedState: Extract<ObservedState, "needs-user" | "completed-unchecked">;
  title: string;
  body: string;
}
