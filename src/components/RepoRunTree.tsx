import { useState } from "react";
import { Bell, Bot, ChevronDown, ChevronRight, GitBranch, Plus, Tag } from "lucide-react";
import type { RepoNode, RunView } from "../types";
import { Chip } from "./Chip";
import { StatusBadge } from "./StatusBadge";

interface RepoRunTreeProps {
  repos: RepoNode[];
  selectedRunId: string | null;
  onSelectRun: (runId: string) => void;
  onCreateRunFromRepo?: (repo: RepoNode) => void;
  onCreateRunFromRun?: (run: RunView) => void;
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

export function RepoRunTree({
  repos,
  selectedRunId,
  onSelectRun,
  onCreateRunFromRepo,
  onCreateRunFromRun
}: RepoRunTreeProps) {
  const [collapsedRepos, setCollapsedRepos] = useState<Set<string>>(() => new Set());

  if (repos.length === 0) {
    return <div className="empty-tree">No runs yet.</div>;
  }

  function toggleRepo(repoPath: string) {
    setCollapsedRepos((previous) => {
      const next = new Set(previous);
      if (next.has(repoPath)) {
        next.delete(repoPath);
      } else {
        next.add(repoPath);
      }
      return next;
    });
  }

  return (
    <div className="repo-run-tree" role="tree" aria-label="Repositories and runs">
      {repos.map((repo) => {
        const expanded = !collapsedRepos.has(repo.repoPath);

        return (
          <div className="repo-group" key={repo.repoPath}>
            <div
              className="repo-row"
              role="treeitem"
              aria-expanded={expanded}
              aria-label={`${repo.repoName} ${repo.runs.length} runs`}
            >
              <button
                type="button"
                className="tree-collapse-button"
                aria-expanded={expanded}
                aria-label={`${expanded ? "Collapse" : "Expand"} ${repo.repoName}`}
                title={`${expanded ? "Collapse" : "Expand"} ${repo.repoName}`}
                onClick={() => toggleRepo(repo.repoPath)}
              >
                {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
              </button>
              <span>{repo.repoName}</span>
              <span className="repo-count">{repo.runs.length}</span>
              <button
                type="button"
                className="tree-create-button"
                aria-label={`New run from ${repo.repoName}`}
                title={`New run from ${repo.repoName}`}
                onClick={() => onCreateRunFromRepo?.(repo)}
              >
                <Plus size={14} />
              </button>
            </div>
            {expanded && (
              <div className="run-children" role="group">
                {repo.runs.map((run) => {
                  const attentionBadge = attentionBadgeForState(run.observedState);

                  return (
                    <div
                      className={run.id === selectedRunId ? "run-row selected" : "run-row"}
                      key={run.id}
                      role="treeitem"
                      tabIndex={0}
                      aria-selected={run.id === selectedRunId}
                      aria-label={`${run.runName} ${run.agent} ${run.observedState}`}
                      onClick={() => onSelectRun(run.id)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          onSelectRun(run.id);
                        }
                      }}
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
                      <button
                        type="button"
                        className="tree-create-button"
                        aria-label={`New run from ${run.runName}`}
                        title={`New run from ${run.runName}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          onCreateRunFromRun?.(run);
                        }}
                      >
                        <Plus size={14} />
                      </button>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
