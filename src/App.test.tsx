import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { dashboardState } from "./api";
import type { DashboardState, RunView } from "./types";

vi.mock("./api", () => ({
  cleanupStaleRuns: vi.fn(),
  dashboardState: vi.fn(),
  endRun: vi.fn(),
  mergeRun: vi.fn(),
  openInVsCode: vi.fn(),
  stopRun: vi.fn()
}));

vi.mock("./components/TerminalPane", () => ({
  TerminalPane: ({ selectedRun }: { selectedRun: RunView | null }) => (
    <section aria-label="Mock terminal">{selectedRun?.runName ?? "No selected run"}</section>
  )
}));

describe("App", () => {
  it("keeps the run chosen by the user instead of returning to the first run", async () => {
    vi.mocked(dashboardState).mockImplementation((selectedRunId?: string | null) =>
      Promise.resolve(dashboard(selectedRunId ?? "run-1"))
    );

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });

    await userEvent.click(screen.getByRole("treeitem", { name: /api-cleanup/i }));

    await waitFor(() => expect(dashboardState).toHaveBeenCalled());
    await Promise.resolve();
    await Promise.resolve();

    expect(screen.getByRole("heading", { name: "api-cleanup" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "login-flow" })).not.toBeInTheDocument();
  });
});

function dashboard(selectedRunId: string): DashboardState {
  return {
    repos: [
      {
        repoName: "agent-manager",
        repoPath: "/repo/agent-manager",
        runs: [run("run-1", "login-flow"), run("run-2", "api-cleanup")]
      }
    ],
    selectedRunId,
    activeCount: 2,
    staleCount: 0,
    activeRepoPath: "/repo/agent-manager",
    hostTools: [
      { name: "git", available: true, detail: "available" },
      { name: "tmux", available: true, detail: "available" }
    ]
  };
}

function run(id: string, runName: string): RunView {
  return {
    id,
    repoPath: "/repo/agent-manager",
    repoName: "agent-manager",
    tag: "default",
    runName,
    agent: "codex",
    lifecycle: "active",
    observedState: id === "run-1" ? "running" : "completed-unchecked",
    detectionSource: "tmux",
    branch: runName,
    baseRef: "HEAD",
    worktreePath: `/repo/worktrees/${runName}`,
    createdAt: 1,
    updatedAt: 2
  };
}
