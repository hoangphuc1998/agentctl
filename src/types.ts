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

export interface RunDiffFileView {
  path: string;
  oldPath: string | null;
  status: string;
  additions: number;
  deletions: number;
  binary: boolean;
  patch: string | null;
  message: string | null;
}

export interface RunDiffView {
  runId: string;
  baseRef: string;
  baseCommit: string | null;
  worktreePath: string;
  files: RunDiffFileView[];
  fileCount: number;
  additions: number;
  deletions: number;
  generatedAt: number;
  warning: string | null;
}

export interface CreateRunPayload {
  repoPath: string;
  baseRef: string;
  tag: string;
  runName: string;
  agent: AgentKind;
}

export interface CreateRunDefaults {
  repoPath?: string;
  baseRef?: string;
  tag?: string;
  agent?: AgentKind;
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

export interface TmuxRestoreStatus {
  configured: boolean;
  tpmInstalled: boolean;
  resurrectInstalled: boolean;
  continuumInstalled: boolean;
  autoRestoreEnabled: boolean;
  bootEnabled: boolean;
  savedStateExists: boolean;
  systemdUnitExists: boolean;
  configPath: string;
  detail: string;
}

export interface PairedDevice {
  id: string;
  name: string;
  pairedAt: number;
}

export interface MobileBridgeStatus {
  enabled: boolean;
  bind: string;
  publicUrl: string;
  pairedDevices: PairedDevice[];
  xtunnelStartCommand: string[];
}

export interface MobilePairingCode {
  code: string;
  expiresAt: number;
}
