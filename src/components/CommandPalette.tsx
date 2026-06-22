import { Search, X } from "lucide-react";
import { useMemo, useState } from "react";
import type { DashboardState } from "../types";

interface CommandPaletteProps {
  open: boolean;
  dashboard: DashboardState;
  onClose: () => void;
  onNewRun: () => void;
  onSelectRun: (id: string) => void;
  onRefresh: () => void;
  onCleanupStale: () => void;
}

export function CommandPalette({
  open,
  dashboard,
  onClose,
  onNewRun,
  onSelectRun,
  onRefresh,
  onCleanupStale
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const items = useMemo(() => {
    const actions = [
      { key: "new", title: "New run", subtitle: "Create a worktree and launch an agent", run: null },
      { key: "refresh", title: "Refresh", subtitle: "Reload runs and status", run: null },
      { key: "cleanup", title: "Stop stale runs", subtitle: "Preserve worktrees and branches", run: null }
    ];
    const runs = dashboard.repos.flatMap((repo) =>
      repo.runs.map((run) => ({
        key: run.id,
        title: run.runName,
        subtitle: `${repo.repoName} #${run.tag} ${run.agent}`,
        run
      }))
    );
    return [...actions, ...runs].filter((item) => {
      const text = `${item.title} ${item.subtitle}`.toLowerCase();
      return text.includes(query.toLowerCase());
    });
  }, [dashboard.repos, query]);

  if (!open) return null;

  function activate(key: string) {
    if (key === "new") onNewRun();
    else if (key === "refresh") onRefresh();
    else if (key === "cleanup") onCleanupStale();
    else onSelectRun(key);
  }

  return (
    <div className="modal-backdrop">
      <div className="palette" role="dialog" aria-label="Command palette">
        <div className="palette-search">
          <Search size={18} />
          <input value={query} onChange={(event) => setQuery(event.target.value)} autoFocus placeholder="Search actions and runs" />
          <button className="icon-button" onClick={onClose} title="Close">
            <X size={18} />
          </button>
        </div>
        <div className="palette-results">
          {items.map((item) => (
            <button key={item.key} onClick={() => activate(item.key)}>
              <strong>{item.title}</strong>
              <span>{item.subtitle}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

