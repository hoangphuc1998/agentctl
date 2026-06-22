import {
  AlertTriangle,
  CheckCircle2,
  Command,
  GitMerge,
  Monitor,
  Play,
  Plus,
  RefreshCw,
  Search,
  Square,
  Trash2
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  cleanupStaleRuns,
  dashboardState,
  endRun,
  mergeRun,
  openInVsCode,
  stopRun
} from "./api";
import { CommandPalette } from "./components/CommandPalette";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { CreateRunModal } from "./components/CreateRunModal";
import { RepoRunTree } from "./components/RepoRunTree";
import { StatusBadge } from "./components/StatusBadge";
import { TerminalPane } from "./components/TerminalPane";
import type { DashboardState, RunView } from "./types";

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
  activeRepoPath: null,
  hostTools: []
};

export function App() {
  const [dashboard, setDashboard] = useState<DashboardState>(emptyDashboard);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const selectedRun = useMemo(
    () =>
      dashboard.repos
        .flatMap((repo) => repo.runs)
        .find((run) => run.id === selectedRunId) ?? null,
    [dashboard.repos, selectedRunId]
  );

  const loadDashboard = useCallback(
    async (nextSelectedRunId = selectedRunId) => {
      setRefreshing(true);
      try {
        const next = await dashboardState(nextSelectedRunId);
        setDashboard(next);
        setSelectedRunId(next.selectedRunId);
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setRefreshing(false);
      }
    },
    [selectedRunId]
  );

  useEffect(() => {
    void loadDashboard(null);
    const interval = window.setInterval(() => void loadDashboard(selectedRunId), 3000);
    return () => window.clearInterval(interval);
  }, [loadDashboard, selectedRunId]);

  async function runAction(action: PendingAction) {
    try {
      if (action.kind === "stop") {
        const result = await stopRun(action.run.id);
        setNotice(result.message);
      } else if (action.kind === "end") {
        const result = await endRun(action.run.id);
        setNotice(result.message);
      } else if (action.kind === "merge") {
        const result = await mergeRun(action.run.id);
        setNotice(result?.message ?? "Run not found.");
      } else {
        const result = await cleanupStaleRuns();
        setNotice(result.message);
      }
      setPendingAction(null);
      await loadDashboard(selectedRunId);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function openCode() {
    if (!selectedRun) return;
    try {
      const result = await openInVsCode(selectedRun.id);
      setNotice(result.message);
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
        <div>
          <p className="eyebrow">Agent Manager</p>
          <h1>Run control</h1>
        </div>
        <div className="top-actions">
          <button className="icon-button" onClick={() => void loadDashboard(selectedRunId)} title="Refresh">
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

      <section className="status-strip" aria-label="System status">
        <div className="metric">
          <CheckCircle2 size={18} />
          <span>{dashboard.activeCount} active</span>
        </div>
        <div className="metric">
          <AlertTriangle size={18} />
          <span>{dashboard.staleCount} stale</span>
        </div>
        {dashboard.hostTools.map((tool) => (
          <span className={tool.available ? "tool available" : "tool missing"} key={tool.name}>
            {tool.name}
          </span>
        ))}
      </section>

      {(notice || error) && (
        <section className={error ? "notice error" : "notice"}>{error ?? notice}</section>
      )}

      <section className="workspace">
        <aside className="left-panel">
          <div className="panel-title">
            <Search size={16} />
            <span>Workspaces</span>
          </div>
          <RepoRunTree repos={dashboard.repos} selectedRunId={selectedRunId} onSelectRun={setSelectedRunId} />
        </aside>

        <section className="run-surface">
          <div className="run-header">
            {selectedRun ? (
              <>
                <div>
                  <StatusBadge state={selectedRun.observedState} />
                  <h2>{selectedRun.runName}</h2>
                  <p>
                    {selectedRun.repoName} / #{selectedRun.tag} / {selectedRun.agent}
                  </p>
                </div>
                <div className="run-actions">
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
              <div>
                <StatusBadge state="unknown" />
                <h2>No run selected</h2>
                <p>Create or select a run from the left panel.</p>
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
          setSelectedRunId(run.id);
          setNotice(`Created ${run.runName}.`);
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
          setSelectedRunId(id);
        }}
        onRefresh={() => {
          setPaletteOpen(false);
          void loadDashboard(selectedRunId);
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

