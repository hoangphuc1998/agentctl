import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RepoRunTree } from "./RepoRunTree";
import type { RepoNode } from "../types";

const repos: RepoNode[] = [
  {
    workspaceKind: "worktree",
    workspacePath: "/home/me/agent-manager",
    repoName: "agent-manager",
    repoPath: "/home/me/agent-manager",
    runs: [
      {
        id: "run-1",
        workspaceKind: "worktree",
        workspacePath: "/home/me/agent-manager-worktrees/login-flow",
        repoPath: "/home/me/agent-manager",
        repoName: "agent-manager",
        tag: "feature",
        runName: "login-flow",
        agent: "codex",
        lifecycle: "active",
        observedState: "running",
        detectionSource: "tmux",
        branch: "login-flow",
        baseRef: "main",
        worktreePath: "/home/me/agent-manager-worktrees/login-flow",
        restorable: false,
        createdAt: 1,
        updatedAt: 2
      },
      {
        id: "run-2",
        workspaceKind: "worktree",
        workspacePath: "/home/me/agent-manager-worktrees/api-cleanup",
        repoPath: "/home/me/agent-manager",
        repoName: "agent-manager",
        tag: "review",
        runName: "api-cleanup",
        agent: "claude",
        lifecycle: "active",
        observedState: "needs-user",
        detectionSource: "heuristic",
        branch: "api-cleanup",
        baseRef: "master",
        worktreePath: "/home/me/agent-manager-worktrees/api-cleanup",
        restorable: false,
        createdAt: 3,
        updatedAt: 4
      }
    ]
  }
];

describe("RepoRunTree", () => {
  it("renders repositories and child runs in one combined left panel", () => {
    render(<RepoRunTree repos={repos} selectedRunId="run-1" onSelectRun={() => {}} />);

    expect(screen.getByRole("tree", { name: "Repositories and runs" })).toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /agent-manager 2 runs/i })).toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /login-flow/i })).toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /api-cleanup/i })).toBeInTheDocument();
    expect(screen.getByText("#feature")).toBeInTheDocument();
    expect(screen.getByText("main -> login-flow")).toBeInTheDocument();
  });

  it("selects child runs without a separate run-list column", async () => {
    const onSelectRun = vi.fn();
    render(<RepoRunTree repos={repos} selectedRunId={null} onSelectRun={onSelectRun} />);

    await userEvent.click(screen.getByRole("treeitem", { name: /api-cleanup/i }));

    expect(onSelectRun).toHaveBeenCalledWith("run-2");
  });

  it("collapses and expands repository groups from the repo header", async () => {
    render(<RepoRunTree repos={repos} selectedRunId={null} onSelectRun={() => {}} />);

    await userEvent.click(screen.getByRole("button", { name: /collapse agent-manager/i }));

    expect(screen.queryByRole("treeitem", { name: /login-flow/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("treeitem", { name: /api-cleanup/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /expand agent-manager/i })).toHaveAttribute(
      "aria-expanded",
      "false"
    );

    await userEvent.click(screen.getByRole("button", { name: /expand agent-manager/i }));

    expect(screen.getByRole("treeitem", { name: /login-flow/i })).toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /api-cleanup/i })).toBeInTheDocument();
  });

  it("requests a new run from a repo row", async () => {
    const onCreateRunFromRepo = vi.fn();
    render(
      <RepoRunTree
        repos={repos}
        selectedRunId={null}
        onSelectRun={vi.fn()}
        onCreateRunFromRepo={onCreateRunFromRepo}
        onCreateRunFromRun={vi.fn()}
      />
    );

    await userEvent.click(screen.getByRole("button", { name: /new run from agent-manager/i }));

    expect(onCreateRunFromRepo).toHaveBeenCalledWith(repos[0]);
  });

  it("requests a new run from an existing run without selecting it", async () => {
    const onCreateRunFromRun = vi.fn();
    const onSelectRun = vi.fn();
    render(
      <RepoRunTree
        repos={repos}
        selectedRunId={null}
        onSelectRun={onSelectRun}
        onCreateRunFromRepo={vi.fn()}
        onCreateRunFromRun={onCreateRunFromRun}
      />
    );

    await userEvent.click(screen.getByRole("button", { name: /new run from api-cleanup/i }));

    expect(onCreateRunFromRun).toHaveBeenCalledWith(repos[0].runs[1]);
    expect(onSelectRun).not.toHaveBeenCalled();
  });

  it("shows notification badges on runs that need attention", () => {
    const attentionRepos: RepoNode[] = [
      {
        ...repos[0],
        runs: [
          repos[0].runs[0],
          repos[0].runs[1],
          {
            ...repos[0].runs[0],
            id: "run-3",
            runName: "docs-review",
            observedState: "completed-unchecked"
          }
        ]
      }
    ];

    render(<RepoRunTree repos={attentionRepos} selectedRunId={null} onSelectRun={() => {}} />);

    expect(
      within(screen.getByRole("treeitem", { name: /api-cleanup/i })).getByText("Input")
    ).toBeInTheDocument();
    expect(
      within(screen.getByRole("treeitem", { name: /docs-review/i })).getByText("Review")
    ).toBeInTheDocument();
    expect(
      within(screen.getByRole("treeitem", { name: /login-flow/i })).queryByText(/input|review/i)
    ).not.toBeInTheDocument();
  });
});
