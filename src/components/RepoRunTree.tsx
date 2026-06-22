import { ChevronDown } from "lucide-react";
import type { RepoNode } from "../types";
import { StatusBadge } from "./StatusBadge";

interface RepoRunTreeProps {
  repos: RepoNode[];
  selectedRunId: string | null;
  onSelectRun: (runId: string) => void;
}

export function RepoRunTree({ repos, selectedRunId, onSelectRun }: RepoRunTreeProps) {
  if (repos.length === 0) {
    return <div className="empty-tree">No runs yet.</div>;
  }

  return (
    <div className="repo-run-tree" role="tree" aria-label="Repositories and runs">
      {repos.map((repo) => (
        <div className="repo-group" key={repo.repoPath}>
          <div className="repo-row" role="treeitem" aria-expanded="true" aria-label={`${repo.repoName} ${repo.runs.length} runs`}>
            <ChevronDown size={16} />
            <span>{repo.repoName}</span>
            <span className="repo-count">{repo.runs.length}</span>
          </div>
          <div className="run-children" role="group">
            {repo.runs.map((run) => (
              <button
                className={run.id === selectedRunId ? "run-row selected" : "run-row"}
                key={run.id}
                role="treeitem"
                aria-selected={run.id === selectedRunId}
                aria-label={`${run.runName} ${run.agent} ${run.observedState}`}
                onClick={() => onSelectRun(run.id)}
              >
                <StatusBadge state={run.observedState} compact />
                <span className="run-row-main">
                  <strong>{run.runName}</strong>
                  <span>
                    <span className="tag">#{run.tag}</span>
                    <span>{run.baseRef} -&gt; {run.branch}</span>
                  </span>
                </span>
                <span className="agent-pill">{run.agent}</span>
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

