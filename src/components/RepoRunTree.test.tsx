import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RepoRunTree } from "./RepoRunTree";
import type { RepoNode } from "../types";

const repos: RepoNode[] = [
  {
    repoName: "agent-manager",
    repoPath: "/home/me/agent-manager",
    runs: [
      {
        id: "run-1",
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
});
