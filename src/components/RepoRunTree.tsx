import { Bell, Bot, ChevronDown, GitBranch, Tag } from "lucide-react";
import type { RepoNode, RunView } from "../types";
import { Chip } from "./Chip";
import { StatusBadge } from "./StatusBadge";

interface RepoRunTreeProps {
  repos: RepoNode[];
  selectedRunId: string | null;
  onSelectRun: (runId: string) => void;
}

function attentionBadgeForState(state: RunView["observedState"]) {
  if (state === "needs-user") {
    return { label: "Input", title: "Needs input" };
  }
  if (state === "completed-unchecked") {
    return { label: "Review", title: "Ready for review" };
  }
  return null;
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
            {repo.runs.map((run) => {
              const attentionBadge = attentionBadgeForState(run.observedState);

              return (
                <button
                  className={run.id === selectedRunId ? "run-row selected" : "run-row"}
                  key={run.id}
                  role="treeitem"
                  aria-selected={run.id === selectedRunId}
                  aria-label={`${run.runName} ${run.agent} ${run.observedState}`}
                  onClick={() => onSelectRun(run.id)}
                >
                  <span className="run-row-status">
                    <StatusBadge state={run.observedState} compact />
                  </span>
                  <span className="run-row-main">
                    <span className="run-name-line">
                      <strong>{run.runName}</strong>
                      {attentionBadge && (
                        <Chip tone="warning" icon={<Bell size={13} />} title={attentionBadge.title}>
                          {attentionBadge.label}
                        </Chip>
                      )}
                    </span>
                    <span className="run-row-meta">
                      <Chip tone="info" icon={<Tag size={13} />}>
                        #{run.tag}
                      </Chip>
                      <Chip tone="neutral" icon={<GitBranch size={13} />}>
                        {run.baseRef} -&gt; {run.branch}
                      </Chip>
                    </span>
                  </span>
                  <Chip tone="neutral" icon={<Bot size={13} />}>
                    {run.agent}
                  </Chip>
                </button>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
