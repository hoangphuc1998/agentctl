import {
  Activity,
  AlertTriangle,
  Bell,
  Bot,
  Code2,
  Command,
  Copy,
  FileDiff,
  Folder,
  FolderGit2,
  GitBranch,
  GitMerge,
  Monitor,
  Play,
  Plus,
  RefreshCw,
  RotateCcw,
  Search,
  ShieldCheck,
  Smartphone,
  Square,
  Tag,
  Terminal,
  Trash2,
  Wrench,
  X
} from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  cleanupStaleRuns,
  dashboardState,
  enableTmuxRestore,
  endRun,
  issueMobilePairingCode,
  listenAgentAttention,
  mergeRun,
  mobileBridgeStatus,
  openInVsCode,
  startMobileBridge,
  restoreRun,
  stopMobileBridge,
  stopRun,
  tmuxRestoreStatus
} from "./api";
import { Chip, type ChipTone } from "./components/Chip";
import { CommandPalette } from "./components/CommandPalette";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { CreateRunModal } from "./components/CreateRunModal";
import { RepoRunTree } from "./components/RepoRunTree";
import { RunDiffPane } from "./components/RunDiffPane";
import { StatusBadge } from "./components/StatusBadge";
import { TerminalPane } from "./components/TerminalPane";
import { appShortcutFromEvent } from "./keyboardShortcuts";
import type {
  CreateRunDefaults,
  DashboardState,
  HostToolStatus,
  MobileBridgeStatus,
  MobilePairingCode,
  RepoNode,
  RunView,
  TmuxRestoreStatus
} from "./types";

type PendingAction =
  | { kind: "stop"; run: RunView }
  | { kind: "end"; run: RunView }
  | { kind: "merge"; run: RunView }
  | { kind: "cleanup-stale" };

type RunViewMode = "terminal" | "diff";

const emptyDashboard: DashboardState = {
  repos: [],
  selectedRunId: null,
  activeCount: 0,
  attentionCount: 0,
  staleCount: 0,
  restorableCount: 0,
  activeRepoPath: null,
  activeFolderPath: null,
  hostTools: []
};

export function App() {
  const [dashboard, setDashboard] = useState<DashboardState>(emptyDashboard);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createDefaults, setCreateDefaults] = useState<CreateRunDefaults | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [activeRunView, setActiveRunView] = useState<RunViewMode>("terminal");
  const [refreshing, setRefreshing] = useState(false);
  const [restoreStatus, setRestoreStatus] = useState<TmuxRestoreStatus | null>(null);
  const [mobileStatus, setMobileStatus] = useState<MobileBridgeStatus | null>(null);
  const [mobilePairingCode, setMobilePairingCode] = useState<MobilePairingCode | null>(null);
  const [mobileBridgeOpen, setMobileBridgeOpen] = useState(false);
  const [mobileBridgeError, setMobileBridgeError] = useState<string | null>(null);
  const selectedRunIdRef = useRef<string | null>(null);
  const dashboardRequestIdRef = useRef(0);

  const selectedRun = useMemo(
    () =>
      dashboard.repos
        .flatMap((repo) => repo.runs)
        .find((run) => run.id === selectedRunId) ?? null,
    [dashboard.repos, selectedRunId]
  );
  const runsInDisplayOrder = useMemo(
    () => dashboard.repos.flatMap((repo) => repo.runs),
    [dashboard.repos]
  );

  const selectRun = useCallback((runId: string | null) => {
    selectedRunIdRef.current = runId;
    setSelectedRunId(runId);
  }, []);

  const loadDashboard = useCallback(
    async (nextSelectedRunId?: string | null) => {
      const requestId = ++dashboardRequestIdRef.current;
      setRefreshing(true);
      try {
        const requestedRunId =
          nextSelectedRunId === undefined ? selectedRunIdRef.current : nextSelectedRunId;
        const next = await dashboardState(requestedRunId);
        if (requestId !== dashboardRequestIdRef.current) return;
        setDashboard(next);
        selectRun(next.selectedRunId);
        setError(null);
      } catch (err) {
        if (requestId === dashboardRequestIdRef.current) {
          setError(errorMessage(err));
        }
      } finally {
        if (requestId === dashboardRequestIdRef.current) {
          setRefreshing(false);
        }
      }
    },
    [selectRun]
  );

  const loadTmuxRestoreStatus = useCallback(async () => {
    try {
      setRestoreStatus(await tmuxRestoreStatus());
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  const loadMobileBridgeStatus = useCallback(async () => {
    try {
      setMobileStatus(await mobileBridgeStatus());
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  const selectRunAndLoad = useCallback(
    (runId: string) => {
      selectRun(runId);
      void loadDashboard(runId);
    },
    [loadDashboard, selectRun]
  );

  const openCreateRun = useCallback((defaults: CreateRunDefaults | null = null) => {
    setCreateDefaults(defaults);
    setCreateOpen(true);
  }, []);

  const closeCreateRun = useCallback(() => {
    setCreateOpen(false);
  }, []);

  const openMobileBridge = useCallback(() => {
    setMobileBridgeError(null);
    setMobileBridgeOpen(true);
  }, []);

  const openCreateRunFromRepo = useCallback(
    (repo: RepoNode) => {
      openCreateRun({
        workspaceKind: repo.workspaceKind,
        repoPath: repo.workspacePath,
        baseRef: "HEAD",
        tag: "default",
        agent: "codex"
      });
    },
    [openCreateRun]
  );

  const openCreateRunFromRun = useCallback(
    (run: RunView) => {
      openCreateRun({
        workspaceKind: run.workspaceKind,
        repoPath:
          run.workspaceKind === "folder" ? run.workspacePath : run.repoPath,
        baseRef: run.baseRef,
        tag: run.tag,
        agent: run.agent
      });
    },
    [openCreateRun]
  );

  const selectAdjacentRun = useCallback(
    (direction: 1 | -1) => {
      if (runsInDisplayOrder.length === 0) return false;

      const currentIndex = runsInDisplayOrder.findIndex((run) => run.id === selectedRunId);
      const fallbackIndex = direction === 1 ? 0 : runsInDisplayOrder.length - 1;
      const nextIndex =
        currentIndex === -1
          ? fallbackIndex
          : (currentIndex + direction + runsInDisplayOrder.length) % runsInDisplayOrder.length;
      selectRunAndLoad(runsInDisplayOrder[nextIndex].id);
      return true;
    },
    [runsInDisplayOrder, selectRunAndLoad, selectedRunId]
  );

  useEffect(() => {
    void loadDashboard(null);
    void loadTmuxRestoreStatus();
    void loadMobileBridgeStatus();
    const interval = window.setInterval(() => void loadDashboard(), 3000);
    return () => window.clearInterval(interval);
  }, [loadDashboard, loadMobileBridgeStatus, loadTmuxRestoreStatus]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    void listenAgentAttention(() => {
      void loadDashboard();
    })
      .then((nextUnlisten) => {
        if (active) {
          unlisten = nextUnlisten;
        } else {
          nextUnlisten();
        }
      })
      .catch((err) => setError(errorMessage(err)));

    return () => {
      active = false;
      unlisten?.();
    };
  }, [loadDashboard]);

  useEffect(() => {
    const badgeCount = dashboard.attentionCount > 0 ? dashboard.attentionCount : undefined;
    void getCurrentWindow()
      .setBadgeCount(badgeCount)
      .catch((err) => {
        console.warn("Failed to update app icon badge count.", err);
      });
  }, [dashboard.attentionCount]);

  useEffect(() => {
    if (selectedRun?.workspaceKind === "folder" && activeRunView === "diff") {
      setActiveRunView("terminal");
    }
  }, [activeRunView, selectedRun?.workspaceKind]);

  useEffect(() => {
    function handleGlobalShortcut(event: KeyboardEvent) {
      const shortcut = appShortcutFromEvent(event);
      if (!shortcut || createOpen || paletteOpen || pendingAction || mobileBridgeOpen) return;

      let handled = true;
      if (shortcut === "open-palette") {
        setPaletteOpen(true);
      } else if (shortcut === "new-run") {
        openCreateRun();
      } else if (shortcut === "previous-run") {
        handled = selectAdjacentRun(-1);
      } else if (shortcut === "next-run") {
        handled = selectAdjacentRun(1);
      } else if (selectedRun) {
        setPendingAction({ kind: "end", run: selectedRun });
      } else {
        handled = false;
      }

      if (handled) {
        event.preventDefault();
        event.stopPropagation();
      }
    }

    window.addEventListener("keydown", handleGlobalShortcut, true);
    return () => window.removeEventListener("keydown", handleGlobalShortcut, true);
  }, [
    createOpen,
    mobileBridgeOpen,
    openCreateRun,
    paletteOpen,
    pendingAction,
    selectAdjacentRun,
    selectedRun
  ]);

  async function runAction(action: PendingAction) {
    try {
      if (action.kind === "stop") {
        await stopRun(action.run.id);
      } else if (action.kind === "end") {
        await endRun(action.run.id);
      } else if (action.kind === "merge") {
        const result = await mergeRun(action.run.id);
        if (!result) {
          setPendingAction(null);
          await loadDashboard();
          setError("Run not found.");
          return;
        }
      } else {
        await cleanupStaleRuns();
      }
      setError(null);
      setPendingAction(null);
      await loadDashboard();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function openCode() {
    if (!selectedRun) return;
    try {
      await openInVsCode(selectedRun.id);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function resumeRun() {
    if (!selectedRun) return;
    try {
      const result = await restoreRun(selectedRun.id);
      setError(null);
      const nextSelectedRunId = result.run?.id ?? selectedRun.id;
      selectRun(nextSelectedRunId);
      await loadDashboard(nextSelectedRunId);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function enableRestartRestore() {
    try {
      setRestoreStatus(await enableTmuxRestore());
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
      void loadTmuxRestoreStatus();
    }
  }

  async function startBridge() {
    try {
      setMobileStatus(await startMobileBridge());
      setMobileBridgeError(null);
      setError(null);
    } catch (err) {
      const message = errorMessage(err);
      setMobileBridgeError(message);
      setError(message);
      void loadMobileBridgeStatus();
    }
  }

  async function stopBridge() {
    try {
      setMobileStatus(await stopMobileBridge());
      setMobilePairingCode(null);
      setMobileBridgeError(null);
      setError(null);
    } catch (err) {
      const message = errorMessage(err);
      setMobileBridgeError(message);
      setError(message);
      void loadMobileBridgeStatus();
    }
  }

  async function pairAndroid() {
    try {
      setMobilePairingCode(await issueMobilePairingCode());
      setMobileBridgeError(null);
      setError(null);
    } catch (err) {
      const message = errorMessage(err);
      setMobileBridgeError(message);
      setError(message);
    }
  }

  function actionTitle(action: PendingAction) {
    if (action.kind === "cleanup-stale") return "Stop stale runs?";
    return `${action.kind === "end" ? "End" : action.kind === "merge" ? "Merge" : "Stop"} ${
      action.run.runName
    }?`;
  }

  function actionBody(action: PendingAction) {
    if (action.kind === "cleanup-stale") {
      return "This preserves worktrees and branches while hiding active runs whose tmux windows are unavailable.";
    }
    if (action.kind === "end") {
      if (action.run.workspaceKind === "folder") {
        return `This stops and forgets the managed session. ${action.run.workspacePath} and all files inside it will be preserved.`;
      }
      return `This removes ${action.run.worktreePath} and deletes branch ${action.run.branch}.`;
    }
    if (action.kind === "merge") {
      return `This merges branch ${action.run.branch} into the repository default branch. Both trees must be clean.`;
    }
    return action.run.workspaceKind === "folder"
      ? "This stops the tmux window and preserves the folder and all files."
      : "This stops the tmux window and keeps the worktree and branch restorable.";
  }

  return (
    <main className="app-shell">
      <header className="top-bar">
        <div className="brand-block">
          <p className="eyebrow">Agent Manager</p>
          <h1>Run control</h1>
        </div>

        <div className="top-meta" aria-label="System status">
          <Chip tone="success" icon={<Activity size={14} />}>
            {dashboard.activeCount} active
          </Chip>
          {dashboard.attentionCount > 0 && (
            <Chip tone="warning" icon={<Bell size={14} />} title="Runs needing attention">
              {dashboard.attentionCount} attention
            </Chip>
          )}
          <Chip tone={dashboard.staleCount > 0 ? "warning" : "neutral"} icon={<AlertTriangle size={14} />}>
            {dashboard.staleCount} stale
          </Chip>
          <Chip tone={dashboard.restorableCount > 0 ? "info" : "neutral"} icon={<RotateCcw size={14} />}>
            {dashboard.restorableCount} restorable
          </Chip>
          {restoreStatus && (
            <Chip
              tone={restoreStatus.configured ? "success" : "warning"}
              icon={<RotateCcw size={14} />}
              title={`${restoreStatus.detail} Config: ${restoreStatus.configPath}`}
            >
              {restoreStatus.configured ? "restart restore on" : "restart restore off"}
            </Chip>
          )}
          {mobileStatus && (
            <button
              type="button"
              className={`chip mobile-status-chip chip-${mobileStatus.enabled ? "success" : "warning"}`}
              title={`${mobileStatus.publicUrl} via ${mobileStatus.bind}`}
              onClick={openMobileBridge}
            >
              <span className="chip-icon" aria-hidden="true">
                <Smartphone size={14} />
              </span>
              <span className="chip-label">{mobileStatus.enabled ? "mobile bridge on" : "mobile bridge off"}</span>
            </button>
          )}
          {dashboard.hostTools.map((tool) => (
            <Chip tone={hostToolTone(tool)} icon={hostToolIcon(tool.name)} title={tool.detail || tool.name} key={tool.name}>
              {tool.name}
            </Chip>
          ))}
        </div>

        <div className="top-actions">
          {restoreStatus && !restoreStatus.configured && (
            <button className="button secondary" onClick={enableRestartRestore}>
              <RotateCcw size={18} />
              Enable restart restore
            </button>
          )}
          <button className="icon-button" onClick={() => void loadDashboard()} title="Refresh">
            <RefreshCw size={18} className={refreshing ? "spin" : ""} />
          </button>
          <button
            className="icon-button"
            aria-label="Mobile Bridge"
            onClick={openMobileBridge}
            title="Mobile Bridge"
          >
            <Smartphone size={18} />
          </button>
          <button
            className="button secondary"
            aria-keyshortcuts="Control+K Meta+K"
            onClick={() => setPaletteOpen(true)}
            title="Palette (Ctrl+K)"
          >
            <Command size={18} />
            Palette
          </button>
          <button
            className="button primary"
            aria-keyshortcuts="Control+Shift+N Meta+Shift+N"
            onClick={() => openCreateRun()}
            title="New Run (Ctrl+Shift+N)"
          >
            <Plus size={18} />
            New Run
          </button>
        </div>
      </header>

      {error && <section className="notice error">{error}</section>}

      <section className="workspace">
        <aside className="left-panel" aria-label="Workspaces">
          <div className="panel-title">
            <span className="panel-title-label">
              <Search size={16} />
              <span>Workspaces</span>
            </span>
            <Chip tone="neutral">{dashboard.repos.length} workspaces</Chip>
          </div>
          <div className="repo-run-tree-scroll">
            <RepoRunTree
              repos={dashboard.repos}
              selectedRunId={selectedRunId}
              onSelectRun={selectRunAndLoad}
              onCreateRunFromRepo={openCreateRunFromRepo}
              onCreateRunFromRun={openCreateRunFromRun}
            />
          </div>
        </aside>

        <section className="run-surface">
          <div className="run-header">
            {selectedRun ? (
              <>
                <div className="run-title-block">
                  <div className="run-title-line">
                    <StatusBadge state={selectedRun.observedState} />
                    <h2>{selectedRun.runName}</h2>
                  </div>
                  <div className="run-chip-row" aria-label="Selected run metadata">
                    <Chip
                      tone="neutral"
                      icon={
                        selectedRun.workspaceKind === "folder" ? (
                          <Folder size={14} />
                        ) : (
                          <FolderGit2 size={14} />
                        )
                      }
                      title={selectedRun.workspacePath}
                    >
                      {selectedRun.repoName}
                    </Chip>
                    <Chip tone="info" icon={<Tag size={14} />}>
                      #{selectedRun.tag}
                    </Chip>
                    <Chip tone="neutral" icon={agentIcon(selectedRun.agent)}>
                      {selectedRun.agent}
                    </Chip>
                    {selectedRun.workspaceKind === "worktree" ? (
                      <Chip
                        tone="neutral"
                        icon={<GitBranch size={14} />}
                        title={selectedRun.worktreePath}
                      >
                        {selectedRun.baseRef} -&gt; {selectedRun.branch}
                      </Chip>
                    ) : (
                      <Chip tone="neutral" icon={<Folder size={14} />} title={selectedRun.workspacePath}>
                        direct folder
                      </Chip>
                    )}
                  </div>
                </div>
                <div className="run-actions">
                  {selectedRun.restorable && (
                    <button className="icon-button" onClick={resumeRun} title="Resume">
                      <RotateCcw size={18} />
                    </button>
                  )}
                  <button className="icon-button" onClick={openCode} title="Open in VS Code">
                    <Monitor size={18} />
                  </button>
                  {selectedRun.workspaceKind === "worktree" && (
                    <button
                      className="icon-button"
                      onClick={() => setPendingAction({ kind: "merge", run: selectedRun })}
                      title="Merge"
                    >
                      <GitMerge size={18} />
                    </button>
                  )}
                  <button
                    className="icon-button"
                    onClick={() => setPendingAction({ kind: "stop", run: selectedRun })}
                    title="Stop"
                  >
                    <Square size={18} />
                  </button>
                  <button
                    className="icon-button danger"
                    aria-keyshortcuts="Control+Shift+E Meta+Shift+E"
                    onClick={() => setPendingAction({ kind: "end", run: selectedRun })}
                    title="End (Ctrl+Shift+E)"
                  >
                    <Trash2 size={18} />
                  </button>
                </div>
              </>
            ) : (
              <div className="run-title-block">
                <div className="run-title-line">
                  <StatusBadge state="unknown" />
                  <h2>No run selected</h2>
                </div>
                <div className="run-chip-row">
                  <Chip tone="neutral" icon={<Terminal size={14} />}>
                    terminal idle
                  </Chip>
                  <Chip tone="info" icon={<Plus size={14} />}>
                    create or select a run
                  </Chip>
                </div>
              </div>
            )}
          </div>

          {selectedRun && (
            <div className="run-view-tabs" role="tablist" aria-label="Run views">
              <button
                type="button"
                role="tab"
                aria-selected={activeRunView === "terminal"}
                className={activeRunView === "terminal" ? "selected" : ""}
                onClick={() => setActiveRunView("terminal")}
              >
                <Terminal size={15} />
                Terminal
              </button>
              {selectedRun.workspaceKind === "worktree" && (
                <button
                  type="button"
                  role="tab"
                  aria-selected={activeRunView === "diff"}
                  className={activeRunView === "diff" ? "selected" : ""}
                  onClick={() => setActiveRunView("diff")}
                >
                  <FileDiff size={15} />
                  Diff
                </button>
              )}
            </div>
          )}

          <div className="run-view-stack">
            <div
              className="run-view-panel"
              hidden={
                activeRunView !== "terminal" && selectedRun?.workspaceKind !== "folder"
              }
            >
              <TerminalPane
                selectedRun={selectedRun}
                active={activeRunView === "terminal" || selectedRun?.workspaceKind === "folder"}
                onError={setError}
              />
            </div>
            <div
              className="run-view-panel"
              hidden={activeRunView !== "diff" || selectedRun?.workspaceKind === "folder"}
            >
              <RunDiffPane
                selectedRun={selectedRun}
                active={activeRunView === "diff"}
                onError={setError}
              />
            </div>
          </div>
        </section>
      </section>

      <CreateRunModal
        open={createOpen}
        activeRepoPath={dashboard.activeRepoPath}
        activeFolderPath={dashboard.activeFolderPath}
        defaults={createDefaults}
        onClose={closeCreateRun}
        onCreated={(run) => {
          setCreateOpen(false);
          selectRun(run.id);
          setError(null);
          void loadDashboard(run.id);
        }}
        onError={setError}
      />

      <CommandPalette
        open={paletteOpen}
        dashboard={dashboard}
        onClose={() => setPaletteOpen(false)}
        onNewRun={() => {
          setPaletteOpen(false);
          openCreateRun();
        }}
        onSelectRun={(id) => {
          setPaletteOpen(false);
          selectRunAndLoad(id);
        }}
        onRefresh={() => {
          setPaletteOpen(false);
          void loadDashboard();
        }}
        onCleanupStale={() => {
          setPaletteOpen(false);
          setPendingAction({ kind: "cleanup-stale" });
        }}
      />

      {mobileBridgeOpen && mobileStatus && (
        <div className="modal-backdrop">
          <section className="modal mobile-bridge-dialog" role="dialog" aria-modal="true" aria-label="Mobile Bridge">
            <div className="modal-header">
              <div>
                <p className="eyebrow">Mobile Bridge</p>
                <h2>Android access</h2>
              </div>
              <button
                type="button"
                className="icon-button"
                aria-label="Close Mobile Bridge"
                title="Close Mobile Bridge"
                onClick={() => setMobileBridgeOpen(false)}
              >
                <X size={18} />
              </button>
            </div>
            <MobileBridgePanel
              status={mobileStatus}
              pairingCode={mobilePairingCode}
              error={mobileBridgeError}
              onStart={startBridge}
              onStop={stopBridge}
              onPair={pairAndroid}
            />
          </section>
        </div>
      )}

      {pendingAction && (
        <ConfirmDialog
          title={actionTitle(pendingAction)}
          body={actionBody(pendingAction)}
          confirmLabel={pendingAction.kind === "end" ? "End Run" : "Confirm"}
          destructive={pendingAction.kind === "end"}
          onCancel={() => setPendingAction(null)}
          onConfirm={() => void runAction(pendingAction)}
        />
      )}

      {!selectedRun && (
        <button className="floating-run-button" onClick={() => openCreateRun()}>
          <Play size={18} />
          Create first run
        </button>
      )}
    </main>
  );
}

interface MobileBridgePanelProps {
  status: MobileBridgeStatus | null;
  pairingCode: MobilePairingCode | null;
  error?: string | null;
  onStart: () => void;
  onStop: () => void;
  onPair: () => void;
}

function MobileBridgePanel({
  status,
  pairingCode,
  error,
  onStart,
  onStop,
  onPair
}: MobileBridgePanelProps) {
  if (!status) return null;
  const xtunnelCommand = status.xtunnelStartCommand.join(" ");
  const mobileWebUrl = `${status.publicUrl.replace(/\/$/, "")}/mobile`;

  return (
    <section className="mobile-bridge-panel" aria-label="Mobile Bridge">
      <div className="mobile-bridge-title">
        <span>
          <Smartphone size={15} />
          <span>Mobile Bridge</span>
        </span>
        <Chip tone={status.enabled ? "success" : "warning"}>
          {status.enabled ? "on" : "off"}
        </Chip>
      </div>
      <div className="mobile-bridge-lines">
        <span title={status.bind}>{status.bind}</span>
        <span title={status.publicUrl}>{status.publicUrl}</span>
        <span title="Open this URL in Android Chrome">{mobileWebUrl}</span>
      </div>
      <code className="mobile-bridge-command">{xtunnelCommand}</code>
      {error && (
        <div className="mobile-bridge-error" role="alert">
          {error}
        </div>
      )}
      {pairingCode && (
        <div className="mobile-pairing-code" aria-label="Android pairing code">
          <ShieldCheck size={15} />
          <strong>{pairingCode.code}</strong>
        </div>
      )}
      <div className="mobile-bridge-actions">
        <button className="button secondary" onClick={status.enabled ? onStop : onStart}>
          {status.enabled ? <Square size={16} /> : <Smartphone size={16} />}
          {status.enabled ? "Stop mobile bridge" : "Start mobile bridge"}
        </button>
        <button
          className="icon-button"
          onClick={onPair}
          title="Pair Android"
          aria-label="Pair Android"
          disabled={!status.enabled}
        >
          <Copy size={16} />
        </button>
      </div>
    </section>
  );
}

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Unexpected error.";
}

function hostToolTone(tool: HostToolStatus): ChipTone {
  return tool.available ? "success" : "danger";
}

function hostToolIcon(toolName: string) {
  const normalized = toolName.toLowerCase();
  if (normalized.includes("git")) return <GitBranch size={14} />;
  if (normalized.includes("tmux")) return <Terminal size={14} />;
  if (normalized.includes("code")) return <Code2 size={14} />;
  if (normalized.includes("codex") || normalized.includes("claude")) return <Bot size={14} />;
  return <Wrench size={14} />;
}

function agentIcon(agent: string) {
  if (agent === "codex" || agent === "claude") return <Bot size={14} />;
  return <Terminal size={14} />;
}
