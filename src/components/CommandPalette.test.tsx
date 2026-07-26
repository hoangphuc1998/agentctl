import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CommandPalette } from "./CommandPalette";
import type { DashboardState, RunView } from "../types";

describe("CommandPalette", () => {
  it("activates the highlighted result with arrow keys and Enter", async () => {
    const onSelectRun = vi.fn();
    render(
      <CommandPalette
        open
        dashboard={dashboard()}
        onClose={vi.fn()}
        onNewRun={vi.fn()}
        onSelectRun={onSelectRun}
        onRefresh={vi.fn()}
        onCleanupStale={vi.fn()}
      />
    );

    await userEvent.keyboard("{ArrowDown}{ArrowDown}{ArrowDown}{Enter}");

    expect(onSelectRun).toHaveBeenCalledWith("run-1");
  });

  it("closes with Escape", async () => {
    const onClose = vi.fn();
    render(
      <CommandPalette
        open
        dashboard={dashboard()}
        onClose={onClose}
        onNewRun={vi.fn()}
        onSelectRun={vi.fn()}
        onRefresh={vi.fn()}
        onCleanupStale={vi.fn()}
      />
    );

    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });
});

function dashboard(): DashboardState {
  return {
    repos: [
      {
        workspaceKind: "worktree",
        workspacePath: "/repo/agent-manager",
        repoName: "agent-manager",
        repoPath: "/repo/agent-manager",
        runs: [run("run-1", "login-flow"), run("run-2", "api-cleanup")]
      }
    ],
    selectedRunId: "run-1",
    activeCount: 2,
    attentionCount: 0,
    staleCount: 0,
    restorableCount: 0,
    activeRepoPath: "/repo/agent-manager",
    activeFolderPath: null,
    hostTools: []
  };
}

function run(id: string, runName: string): RunView {
  return {
    id,
    workspaceKind: "worktree",
    workspacePath: `/repo/worktrees/${runName}`,
    repoPath: "/repo/agent-manager",
    repoName: "agent-manager",
    tag: "default",
    runName,
    agent: "codex",
    lifecycle: "active",
    observedState: "running",
    detectionSource: "tmux",
    branch: runName,
    baseRef: "HEAD",
    worktreePath: `/repo/worktrees/${runName}`,
    restorable: false,
    createdAt: 1,
    updatedAt: 2
  };
}
