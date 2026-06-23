import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { createRun, dashboardState, listenAgentAttention, mergeRun, restoreRun } from "./api";
import type { DashboardState, RunView } from "./types";

vi.mock("./api", () => ({
  cleanupStaleRuns: vi.fn(),
  createRun: vi.fn(),
  dashboardState: vi.fn(),
  endRun: vi.fn(),
  listenAgentAttention: vi.fn(),
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
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listenAgentAttention).mockResolvedValue(vi.fn());
  });

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

  it("keeps dashboard errors visible in the notice area", async () => {
    vi.mocked(dashboardState).mockRejectedValue("Dashboard unavailable");

    render(<App />);

    const notice = await screen.findByText("Dashboard unavailable");
    expect(notice).toHaveClass("notice", "error");
  });

  it("shows missing merge results as an error notice", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(mergeRun).mockResolvedValue(null);

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: /merge/i }));
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    await waitFor(() => expect(mergeRun).toHaveBeenCalledWith("run-1"));
    const notice = await screen.findByText("Run not found.");
    expect(notice).toHaveClass("notice", "error");
  });

  it("does not reserve dashboard space for successful create-run messages", async () => {
    const createdRun = run("run-3", "fix-ui");
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(createRun).mockResolvedValue({
      message: "Created fix-ui.",
      run: createdRun
    });

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: /new run/i }));
    const repoPath = screen.getByLabelText(/repo path/i);
    await userEvent.clear(repoPath);
    await userEvent.type(repoPath, "/repo/agent-manager");
    await userEvent.type(screen.getByLabelText(/run name/i), "fix-ui");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() => expect(createRun).toHaveBeenCalledOnce());
    expect(screen.queryByText("Created fix-ui.")).not.toBeInTheDocument();
  });

  it("shows the backend attention badge count in the top bar", async () => {
    vi.mocked(dashboardState).mockResolvedValue(
      dashboard("run-1", [run("run-1", "login-flow"), run("run-2", "api-cleanup")])
    );

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });

    expect(screen.getByText("1 attention")).toBeInTheDocument();
  });

  it("does not dispatch browser notifications when the backend emits an attention event", async () => {
    const attentionListener: { current: Parameters<typeof listenAgentAttention>[0] | null } = {
      current: null
    };
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(listenAgentAttention).mockImplementation(async (callback) => {
      attentionListener.current = callback;
      return vi.fn();
    });
    const notificationSpy = installNotificationMock("granted");

    render(<App />);

    await waitFor(() => expect(listenAgentAttention).toHaveBeenCalledOnce());
    expect(attentionListener.current).not.toBeNull();
    attentionListener.current?.({
      event: "agent:attention",
      id: 1,
      payload: {
        runId: "run-2",
        runName: "api-cleanup",
        repoName: "agent-manager",
        agent: "codex",
        observedState: "completed-unchecked",
        title: "Agent completed",
        body: "api-cleanup in agent-manager is ready for review."
      }
    });

    await waitFor(() => expect(dashboardState).toHaveBeenCalledTimes(2));
    expect(notificationSpy).not.toHaveBeenCalled();
  });

  it("refreshes the badge count when the backend emits an attention event", async () => {
    const attentionListener: { current: Parameters<typeof listenAgentAttention>[0] | null } = {
      current: null
    };
    const quietDashboard = dashboard("run-1", [run("run-1", "login-flow")]);
    const attentionDashboard = dashboard("run-1", [run("run-1", "login-flow"), run("run-2", "api-cleanup")]);
    vi.mocked(dashboardState)
      .mockResolvedValueOnce(quietDashboard)
      .mockResolvedValueOnce(attentionDashboard);
    vi.mocked(listenAgentAttention).mockImplementation(async (callback) => {
      attentionListener.current = callback;
      return vi.fn();
    });
    installNotificationMock("granted");

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    expect(screen.queryByText("1 attention")).not.toBeInTheDocument();
    attentionListener.current?.({
      event: "agent:attention",
      id: 1,
      payload: {
        runId: "run-2",
        runName: "api-cleanup",
        repoName: "agent-manager",
        agent: "codex",
        observedState: "completed-unchecked",
        title: "Agent completed",
        body: "api-cleanup in agent-manager is ready for review."
      }
    });

    expect(await screen.findByText("1 attention")).toBeInTheDocument();
    expect(dashboardState).toHaveBeenCalledTimes(2);
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
    attentionCount: runs.filter((run) => run.observedState === "needs-user" || run.observedState === "completed-unchecked").length,
    staleCount: 0,
    restorableCount: runs.filter((run) => run.restorable).length,
    activeRepoPath: "/repo/agent-manager",
    hostTools: [
      { name: "git", available: true, detail: "available" },
      { name: "tmux", available: true, detail: "available" }
    ]
  };
}

function installNotificationMock(permission: NotificationPermission) {
  const notificationSpy = vi.fn();

  class MockNotification {
    static permission = permission;
    static requestPermission = vi.fn().mockResolvedValue(permission);

    constructor(title: string, options?: NotificationOptions) {
      notificationSpy(title, options);
    }
  }

  Object.defineProperty(window, "Notification", {
    configurable: true,
    value: MockNotification
  });

  return notificationSpy;
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
