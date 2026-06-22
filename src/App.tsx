import {
  Activity,
  AlertTriangle,
  Bot,
  Code2,
  Command,
  FolderGit2,
  GitBranch,
  GitMerge,
  Monitor,
  Play,
  Plus,
  RefreshCw,
  RotateCcw,
  Search,
  Square,
  Tag,
  Terminal,
  Trash2,
  Wrench
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  cleanupStaleRuns,
  dashboardState,
  endRun,
  mergeRun,
  openInVsCode,
  restoreRun,
  stopRun
} from "./api";
import { Chip, type ChipTone } from "./components/Chip";
import { CommandPalette } from "./components/CommandPalette";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { CreateRunModal } from "./components/CreateRunModal";
import { RepoRunTree } from "./components/RepoRunTree";
import { StatusBadge } from "./components/StatusBadge";
import { TerminalPane } from "./components/TerminalPane";
import type { DashboardState, HostToolStatus, RunView } from "./types";

type PendingAction =
  | { kind: "stop"; run: RunView }
  | { kind: "end"; run: RunView }
  | { kind: "merge"; run: RunView }
  | { kind: "cleanup-stale" };

const emptyDashboard: DashboardState = {
  repos: [],
  selectedRunId: null,
  activeCount: 0,
  staleCount: 0,
  restorableCount: 0,
  activeRepoPath: null,
  hostTools: []
};

export function App() {
  const [dashboard, setDashboard] = useState<DashboardState>(emptyDashboard);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const selectedRunIdRef = useRef<string | null>(null);

  const selectedRun = useMemo(
    () =>
      dashboard.repos
        .flatMap((repo) => repo.runs)
        .find((run) => run.id === selectedRunId) ?? null,
    [dashboard.repos, selectedRunId]
  );

  const selectRun = useCallback((runId: string | null) => {
    selectedRunIdRef.current = runId;
    setSelectedRunId(runId);
  }, []);

  const loadDashboard = useCallback(
    async (nextSelectedRunId?: string | null) => {
      setRefreshing(true);
      try {
        const requestedRunId =
          nextSelectedRunId === undefined ? selectedRunIdRef.current : nextSelectedRunId;
        const next = await dashboardState(requestedRunId);
        setDashboard(next);
        selectRun(next.selectedRunId);
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setRefreshing(false);
      }
    },
    [selectRun]
  );

  useEffect(() => {
    void loadDashboard(null);
    const interval = window.setInterval(() => void loadDashboard(), 3000);
    return () => window.clearInterval(interval);
  }, [loadDashboard]);

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
      return `This removes ${action.run.worktreePath} and deletes branch ${action.run.branch}.`;
    }
    if (action.kind === "merge") {
      return `This merges branch ${action.run.branch} into the repository default branch. Both trees must be clean.`;
    }
    return "This stops the tmux window and keeps the worktree and branch restorable.";
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
          <Chip tone={dashboard.staleCount > 0 ? "warning" : "neutral"} icon={<AlertTriangle size={14} />}>
            {dashboard.staleCount} stale
          </Chip>
          <Chip tone={dashboard.restorableCount > 0 ? "info" : "neutral"} icon={<RotateCcw size={14} />}>
            {dashboard.restorableCount} restorable
          </Chip>
          {dashboard.hostTools.map((tool) => (
            <Chip tone={hostToolTone(tool)} icon={hostToolIcon(tool.name)} title={tool.detail || tool.name} key={tool.name}>
              {tool.name}
            </Chip>
          ))}
        </div>

        <div className="top-actions">
          <button className="icon-button" onClick={() => void loadDashboard()} title="Refresh">
            <RefreshCw size={18} className={refreshing ? "spin" : ""} />
          </button>
          <button className="button secondary" onClick={() => setPaletteOpen(true)}>
            <Command size={18} />
            Palette
          </button>
          <button className="button primary" onClick={() => setCreateOpen(true)}>
            <Plus size={18} />
            New Run
          </button>
        </div>
      </header>

      {error && <section className="notice error">{error}</section>}

      <section className="workspace">
        <aside className="left-panel">
          <div className="panel-title">
            <span className="panel-title-label">
              <Search size={16} />
              <span>Workspaces</span>
            </span>
            <Chip tone="neutral">{dashboard.repos.length} repos</Chip>
          </div>
          <RepoRunTree repos={dashboard.repos} selectedRunId={selectedRunId} onSelectRun={selectRun} />
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
                    <Chip tone="neutral" icon={<FolderGit2 size={14} />} title={selectedRun.repoPath}>
                      {selectedRun.repoName}
                    </Chip>
                    <Chip tone="info" icon={<Tag size={14} />}>
                      #{selectedRun.tag}
                    </Chip>
                    <Chip tone="neutral" icon={agentIcon(selectedRun.agent)}>
                      {selectedRun.agent}
                    </Chip>
                    <Chip tone="neutral" icon={<GitBranch size={14} />} title={selectedRun.worktreePath}>
                      {selectedRun.baseRef} -&gt; {selectedRun.branch}
                    </Chip>
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
                  <button
                    className="icon-button"
                    onClick={() => setPendingAction({ kind: "merge", run: selectedRun })}
                    title="Merge"
                  >
                    <GitMerge size={18} />
                  </button>
                  <button
                    className="icon-button"
                    onClick={() => setPendingAction({ kind: "stop", run: selectedRun })}
                    title="Stop"
                  >
                    <Square size={18} />
                  </button>
                  <button
                    className="icon-button danger"
                    onClick={() => setPendingAction({ kind: "end", run: selectedRun })}
                    title="End"
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

          <TerminalPane selectedRun={selectedRun} onError={setError} />
        </section>
      </section>

      <CreateRunModal
        open={createOpen}
        activeRepoPath={dashboard.activeRepoPath}
        onClose={() => setCreateOpen(false)}
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
          setCreateOpen(true);
        }}
        onSelectRun={(id) => {
          setPaletteOpen(false);
          selectRun(id);
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
        <button className="floating-run-button" onClick={() => setCreateOpen(true)}>
          <Play size={18} />
          Create first run
        </button>
      )}
    </main>
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
