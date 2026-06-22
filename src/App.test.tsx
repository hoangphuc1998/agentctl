import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { dashboardState, restoreRun } from "./api";
import type { DashboardState, RunView } from "./types";

vi.mock("./api", () => ({
  cleanupStaleRuns: vi.fn(),
  dashboardState: vi.fn(),
  endRun: vi.fn(),
  mergeRun: vi.fn(),
  openInVsCode: vi.fn(),
  restoreRun: vi.fn(),
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

  it("resumes a restorable run instead of treating it as stale", async () => {
    const restorableDashboard = dashboard("run-2", [run("run-1", "login-flow"), restorableRun()]);
    vi.mocked(dashboardState).mockResolvedValue(restorableDashboard);
    vi.mocked(restoreRun).mockResolvedValue({
      message: "Resumed `feat-score-view`.",
      run: { ...restorableRun(), observedState: "running", detectionSource: "tmux", restorable: false }
    });

    render(<App />);

    await screen.findByRole("heading", { name: "feat-score-view" });
    await userEvent.click(screen.getByRole("button", { name: /resume/i }));

    await waitFor(() => expect(restoreRun).toHaveBeenCalledWith("run-2"));
    expect(screen.getByText("1 restorable")).toBeInTheDocument();
    expect(screen.getByText("0 stale")).toBeInTheDocument();
  });
});

function dashboard(selectedRunId: string, runs: RunView[] = [run("run-1", "login-flow"), run("run-2", "api-cleanup")]): DashboardState {
  return {
    repos: [
      {
        repoName: "agent-manager",
        repoPath: "/repo/agent-manager",
        runs
      }
    ],
    selectedRunId,
    activeCount: runs.length,
    staleCount: 0,
    restorableCount: runs.filter((run) => run.restorable).length,
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
    restorable: false,
    createdAt: 1,
    updatedAt: 2
  };
}

function restorableRun(): RunView {
  return {
    ...run("run-2", "feat-score-view"),
    observedState: "unknown",
    detectionSource: "unknown",
    restorable: true
  };
}
