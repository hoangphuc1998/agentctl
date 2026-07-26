import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import {
  chooseDirectory,
  createFolderSession,
  createRun,
  dashboardState,
  enableTmuxRestore,
  endRun,
  ignoredFilesPreview,
  issueMobilePairingCode,
  listenAgentAttention,
  mergeRun,
  mobileBridgeStatus,
  folderSuggestions,
  repoSuggestions,
  runDiff,
  startMobileBridge,
  restoreRun,
  stopMobileBridge,
  tmuxRestoreStatus
} from "./api";
import type { DashboardState, MobileBridgeStatus, RunDiffView, RunView, TmuxRestoreStatus } from "./types";

const tauriWindowMocks = vi.hoisted(() => ({
  setBadgeCount: vi.fn()
}));

vi.mock("./api", () => ({
  chooseDirectory: vi.fn(),
  cleanupStaleRuns: vi.fn(),
  createFolderSession: vi.fn(),
  createRun: vi.fn(),
  dashboardState: vi.fn(),
  enableTmuxRestore: vi.fn(),
  endRun: vi.fn(),
  ignoredFilesPreview: vi.fn(),
  issueMobilePairingCode: vi.fn(),
  listenAgentAttention: vi.fn(),
  mergeRun: vi.fn(),
  mobileBridgeStatus: vi.fn(),
  openInVsCode: vi.fn(),
  folderSuggestions: vi.fn(),
  repoSuggestions: vi.fn(),
  restoreRun: vi.fn(),
  runDiff: vi.fn(),
  startMobileBridge: vi.fn(),
  stopRun: vi.fn(),
  stopMobileBridge: vi.fn(),
  tmuxRestoreStatus: vi.fn()
}));

vi.mock("./components/TerminalPane", () => ({
  TerminalPane: ({ selectedRun, active }: { selectedRun: RunView | null; active?: boolean }) => (
    <section aria-label="Mock terminal" data-testid="mock-terminal" data-active={String(active ?? true)}>
      {selectedRun?.runName ?? "No selected run"}
    </section>
  )
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setBadgeCount: tauriWindowMocks.setBadgeCount })
}));

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listenAgentAttention).mockResolvedValue(vi.fn());
    vi.mocked(tmuxRestoreStatus).mockResolvedValue(restoreStatus(true));
    vi.mocked(enableTmuxRestore).mockResolvedValue(restoreStatus(true));
    vi.mocked(chooseDirectory).mockResolvedValue(null);
    vi.mocked(ignoredFilesPreview).mockResolvedValue({
      fileCount: 0,
      totalBytes: 0,
      requiresConfirmation: false
    });
    vi.mocked(repoSuggestions).mockResolvedValue([]);
    vi.mocked(folderSuggestions).mockResolvedValue([]);
    vi.mocked(runDiff).mockResolvedValue(emptyRunDiff("run-1"));
    vi.mocked(mobileBridgeStatus).mockResolvedValue(mobileStatus(false));
    vi.mocked(startMobileBridge).mockResolvedValue(mobileStatus(true));
    vi.mocked(stopMobileBridge).mockResolvedValue(mobileStatus(false));
    vi.mocked(issueMobilePairingCode).mockResolvedValue({
      code: "ABCD1234",
      expiresAt: 1782367800
    });
    tauriWindowMocks.setBadgeCount.mockResolvedValue(undefined);
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

  it("does not let an older dashboard refresh temporarily restore the previous run", async () => {
    const staleRefresh = deferred<DashboardState>();
    const selectedRunRefresh = deferred<DashboardState>();
    vi.mocked(dashboardState)
      .mockResolvedValueOnce(dashboard("run-1"))
      .mockReturnValueOnce(staleRefresh.promise)
      .mockReturnValueOnce(selectedRunRefresh.promise);

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(dashboardState).toHaveBeenCalledTimes(2));

    await userEvent.click(screen.getByRole("treeitem", { name: /api-cleanup/i }));
    expect(screen.getByRole("heading", { name: "api-cleanup" })).toBeInTheDocument();
    await waitFor(() => expect(dashboardState).toHaveBeenCalledTimes(3));

    await act(async () => staleRefresh.resolve(dashboard("run-1")));

    expect(screen.getByRole("heading", { name: "api-cleanup" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "login-flow" })).not.toBeInTheDocument();

    await act(async () => selectedRunRefresh.resolve(dashboard("run-2")));
    expect(screen.getByRole("heading", { name: "api-cleanup" })).toBeInTheDocument();
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

  it("enables tmux restart restore when plugin setup is missing", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(tmuxRestoreStatus)
      .mockResolvedValueOnce(restoreStatus(false))
      .mockResolvedValue(restoreStatus(true));

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: /enable restart restore/i }));

    await waitFor(() => expect(enableTmuxRestore).toHaveBeenCalledOnce());
    expect(await screen.findByText("restart restore on")).toBeInTheDocument();
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

  it("loads and renders the selected run diff when the Diff tab opens", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(runDiff).mockResolvedValue(
      runDiffFixture("run-1", {
        warning: "This run uses a fallback base.",
        additions: 2,
        deletions: 1,
        files: [
          {
            path: "src/App.tsx",
            oldPath: null,
            status: "modified",
            additions: 2,
            deletions: 1,
            binary: false,
            patch: "diff --git a/src/App.tsx b/src/App.tsx\n@@ -1 +1 @@\n-old line\n+new line\n",
            message: null
          }
        ]
      })
    );

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("tab", { name: /diff/i }));

    await waitFor(() => expect(runDiff).toHaveBeenCalledWith("run-1"));
    expect(screen.getByRole("tab", { name: /diff/i })).toHaveAttribute("aria-selected", "true");
    expect(screen.getAllByText("1 file").length).toBeGreaterThan(0);
    expect(screen.getAllByText("+2").length).toBeGreaterThan(0);
    expect(screen.getAllByText("-1").length).toBeGreaterThan(0);
    expect(screen.getByText("This run uses a fallback base.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /src\/App\.tsx/i })).toBeInTheDocument();
    expect(screen.getByText("+new line")).toBeInTheDocument();
  });

  it("groups changed diff files by folder with filename-first rows", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(runDiff).mockResolvedValue(
      runDiffFixture("run-1", {
        files: [
          {
            path: "src/components/RunDiffPane.tsx",
            oldPath: null,
            status: "modified",
            additions: 3,
            deletions: 1,
            binary: false,
            patch:
              "diff --git a/src/components/RunDiffPane.tsx b/src/components/RunDiffPane.tsx\n@@ -1 +1 @@\n-old component\n+new component\n",
            message: null
          },
          {
            path: "README.md",
            oldPath: null,
            status: "modified",
            additions: 1,
            deletions: 0,
            binary: false,
            patch: "diff --git a/README.md b/README.md\n@@ -1 +1 @@\n+readme update\n",
            message: null
          }
        ],
        fileCount: 2,
        additions: 4,
        deletions: 1
      })
    );

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("tab", { name: /diff/i }));

    const changedFiles = await screen.findByLabelText("Changed files");
    expect(within(changedFiles).getByText("src/components")).toBeInTheDocument();
    expect(within(changedFiles).getByText("Repository root")).toBeInTheDocument();
    expect(within(changedFiles).getByText("RunDiffPane.tsx")).toBeInTheDocument();
    expect(within(changedFiles).getByText("README.md")).toBeInTheDocument();

    await userEvent.click(within(changedFiles).getByRole("button", { name: /README\.md/i }));

    expect(screen.getByText("+readme update")).toBeInTheDocument();
  });

  it("keeps the terminal mounted but inactive while reviewing the diff", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(runDiff).mockResolvedValue(runDiffFixture("run-1"));

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    expect(screen.getByTestId("mock-terminal")).toHaveAttribute("data-active", "true");

    await userEvent.click(screen.getByRole("tab", { name: /diff/i }));

    expect(screen.getByTestId("mock-terminal")).toBeInTheDocument();
    expect(screen.getByTestId("mock-terminal")).toHaveAttribute("data-active", "false");
  });

  it("clears the previous run diff while loading another selected run", async () => {
    let resolveSecondDiff: (diff: RunDiffView) => void = () => {};
    vi.mocked(dashboardState).mockImplementation((selectedRunId?: string | null) =>
      Promise.resolve(dashboard(selectedRunId ?? "run-1"))
    );
    vi.mocked(runDiff).mockImplementation((runId: string) => {
      if (runId === "run-1") {
        return Promise.resolve(
          runDiffFixture("run-1", {
            files: [
              {
                path: "old-run.md",
                oldPath: null,
                status: "modified",
                additions: 1,
                deletions: 0,
                binary: false,
                patch: "diff --git a/old-run.md b/old-run.md\n@@ -0,0 +1 @@\n+old run\n",
                message: null
              }
            ]
          })
        );
      }
      return new Promise((resolve) => {
        resolveSecondDiff = resolve;
      });
    });

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("tab", { name: /diff/i }));
    expect(await screen.findByText("+old run")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("treeitem", { name: /api-cleanup/i }));

    await waitFor(() => expect(runDiff).toHaveBeenCalledWith("run-2"));
    expect(screen.queryByText("+old run")).not.toBeInTheDocument();
    expect(screen.getByText("Loading diff...")).toBeInTheDocument();

    resolveSecondDiff(
      runDiffFixture("run-2", {
        files: [
          {
            path: "new-run.md",
            oldPath: null,
            status: "modified",
            additions: 1,
            deletions: 0,
            binary: false,
            patch: "diff --git a/new-run.md b/new-run.md\n@@ -0,0 +1 @@\n+new run\n",
            message: null
          }
        ]
      })
    );
    expect(await screen.findByText("+new run")).toBeInTheDocument();
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
    await userEvent.click(screen.getByRole("button", { name: /^new run$/i }));
    const repoPath = screen.getByLabelText(/repo path/i);
    await userEvent.clear(repoPath);
    await userEvent.type(repoPath, "/repo/agent-manager");
    await userEvent.type(screen.getByLabelText(/run name/i), "fix-ui");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() => expect(createRun).toHaveBeenCalledOnce());
    expect(screen.queryByText("Created fix-ui.")).not.toBeInTheDocument();
  });

  it("opens New Run prefilled from a repo row", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: /new run from agent-manager/i }));

    expect(screen.getByLabelText(/repo path/i)).toHaveValue("/repo/agent-manager");
    expect(screen.getByLabelText(/base ref/i)).toHaveValue("HEAD");
    expect(screen.getByLabelText(/tag/i)).toHaveValue("default");
    expect(screen.getByRole("button", { name: /codex/i })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByLabelText(/run name/i)).toHaveValue("");
  });

  it("opens New Run prefilled from an existing run and submits edited data", async () => {
    const sourceRun: RunView = {
      ...run("run-2", "api-cleanup"),
      agent: "claude",
      baseRef: "master",
      tag: "review"
    };
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1", [run("run-1", "login-flow"), sourceRun]));
    vi.mocked(createRun).mockResolvedValue({ message: "Created.", run: null });

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: /new run from api-cleanup/i }));
    expect(screen.getByLabelText(/repo path/i)).toHaveValue("/repo/agent-manager");
    expect(screen.getByLabelText(/base ref/i)).toHaveValue("master");
    expect(screen.getByLabelText(/tag/i)).toHaveValue("review");
    expect(screen.getByRole("button", { name: /claude/i })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByLabelText(/run name/i)).toHaveValue("");

    await userEvent.type(screen.getByLabelText(/run name/i), "api-followup");
    await userEvent.clear(screen.getByLabelText(/base ref/i));
    await userEvent.type(screen.getByLabelText(/base ref/i), "release");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() =>
      expect(createRun).toHaveBeenCalledWith({
        repoPath: "/repo/agent-manager",
        baseRef: "release",
        tag: "review",
        runName: "api-followup",
        agent: "claude",
        copyIgnoredFiles: true
      })
    );
  });

  it("opens the command palette with the keyboard shortcut", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.keyboard("{Control>}k{/Control}");

    expect(screen.getByRole("dialog", { name: /command palette/i })).toBeInTheDocument();
  });

  it("opens New Run with the keyboard shortcut", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.keyboard("{Control>}{Shift>}N{/Shift}{/Control}");

    expect(screen.getByRole("heading", { name: /create worktree and launch agent/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/repo path/i)).toHaveValue("/repo/agent-manager");
  });

  it("navigates between runs with keyboard shortcuts", async () => {
    vi.mocked(dashboardState).mockImplementation((selectedRunId?: string | null) =>
      Promise.resolve(dashboard(selectedRunId ?? "run-1"))
    );

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.keyboard("{Alt>}{ArrowDown}{/Alt}");

    await waitFor(() => expect(dashboardState).toHaveBeenLastCalledWith("run-2"));
    expect(screen.getByRole("heading", { name: "api-cleanup" })).toBeInTheDocument();

    await userEvent.keyboard("{Alt>}{ArrowUp}{/Alt}");

    await waitFor(() => expect(dashboardState).toHaveBeenLastCalledWith("run-1"));
    expect(screen.getByRole("heading", { name: "login-flow" })).toBeInTheDocument();
  });

  it("opens the selected run end confirmation with the keyboard shortcut", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(endRun).mockResolvedValue({ message: "Ended login-flow.", run: null });

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.keyboard("{Control>}{Shift>}E{/Shift}{/Control}");

    expect(screen.getByRole("alertdialog", { name: /end login-flow/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /end run/i })).toHaveFocus();

    await userEvent.keyboard("{Enter}");

    await waitFor(() => expect(endRun).toHaveBeenCalledWith("run-1"));
  });

  it("shows the backend attention badge count in the top bar", async () => {
    vi.mocked(dashboardState).mockResolvedValue(
      dashboard("run-1", [run("run-1", "login-flow"), run("run-2", "api-cleanup")])
    );

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });

    expect(screen.getByText("1 attention")).toBeInTheDocument();
  });

  it("sets the app icon badge count from backend attention count", async () => {
    vi.mocked(dashboardState).mockResolvedValue(
      dashboard("run-1", [run("run-1", "login-flow"), run("run-2", "api-cleanup")])
    );

    render(<App />);

    await screen.findByText("1 attention");

    await waitFor(() => expect(tauriWindowMocks.setBadgeCount).toHaveBeenLastCalledWith(1));
  });

  it("clears the app icon badge when attention count returns to zero", async () => {
    const completedDashboard = dashboard("run-1", [
      run("run-1", "login-flow"),
      run("run-2", "api-cleanup")
    ]);
    const seenDashboard = dashboard("run-2", [
      run("run-1", "login-flow"),
      { ...run("run-2", "api-cleanup"), observedState: "completed-seen" }
    ]);
    vi.mocked(dashboardState).mockResolvedValueOnce(completedDashboard).mockResolvedValueOnce(seenDashboard);

    render(<App />);

    await screen.findByText("1 attention");
    await waitFor(() => expect(tauriWindowMocks.setBadgeCount).toHaveBeenLastCalledWith(1));

    await userEvent.click(screen.getByRole("treeitem", { name: /api-cleanup/i }));

    await waitFor(() => expect(dashboardState).toHaveBeenLastCalledWith("run-2"));
    await waitFor(() => expect(tauriWindowMocks.setBadgeCount).toHaveBeenLastCalledWith(undefined));
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
    expect(screen.queryByText("Review")).not.toBeInTheDocument();
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
    expect(within(screen.getByRole("treeitem", { name: /api-cleanup/i })).getByText("Review")).toBeInTheDocument();
    expect(dashboardState).toHaveBeenCalledTimes(2);
  });

  it("refreshes a selected completed run so its row attention badge can clear", async () => {
    const completedDashboard = dashboard("run-1", [
      run("run-1", "login-flow"),
      run("run-2", "api-cleanup")
    ]);
    const seenDashboard = dashboard("run-2", [
      run("run-1", "login-flow"),
      { ...run("run-2", "api-cleanup"), observedState: "completed-seen" }
    ]);
    vi.mocked(dashboardState).mockResolvedValueOnce(completedDashboard).mockResolvedValueOnce(seenDashboard);

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    expect(within(screen.getByRole("treeitem", { name: /api-cleanup/i })).getByText("Review")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("treeitem", { name: /api-cleanup/i }));

    await waitFor(() => expect(dashboardState).toHaveBeenLastCalledWith("run-2"));
    expect(within(screen.getByRole("treeitem", { name: /api-cleanup/i })).queryByText("Review")).not.toBeInTheDocument();
    expect(screen.queryByText("1 attention")).not.toBeInTheDocument();
  });

  it("opens mobile bridge controls from the header instead of the workspace panel", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    expect(screen.getByText("mobile bridge off")).toBeInTheDocument();
    expect(
      within(screen.getByRole("complementary", { name: /workspaces/i })).queryByRole("region", {
        name: /mobile bridge/i
      })
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /start mobile bridge/i })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: /^mobile bridge$/i }));
    const dialog = screen.getByRole("dialog", { name: /mobile bridge/i });
    await userEvent.click(within(dialog).getByRole("button", { name: /start mobile bridge/i }));

    await waitFor(() => expect(startMobileBridge).toHaveBeenCalledOnce());
    expect(await screen.findByText("mobile bridge on")).toBeInTheDocument();
    expect(within(dialog).getByText("xtunnel.cmd linhmon start 17654")).toBeInTheDocument();
    expect(within(dialog).getByText("https://linhmon.linhmon.1vn.app/mobile")).toBeInTheDocument();

    await userEvent.click(within(dialog).getByRole("button", { name: /pair android/i }));

    await waitFor(() => expect(issueMobilePairingCode).toHaveBeenCalledOnce());
    expect(within(dialog).getByText("ABCD1234")).toBeInTheDocument();
  });

  it("opens mobile bridge controls from the visible status chip", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: /mobile bridge off/i }));

    const dialog = screen.getByRole("dialog", { name: /mobile bridge/i });
    await userEvent.click(within(dialog).getByRole("button", { name: /start mobile bridge/i }));

    await waitFor(() => expect(startMobileBridge).toHaveBeenCalledOnce());
    expect(await screen.findByText("mobile bridge on")).toBeInTheDocument();
  });

  it("shows mobile bridge start failures inside the bridge dialog", async () => {
    vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
    vi.mocked(startMobileBridge).mockRejectedValue("Address already in use");

    render(<App />);

    await screen.findByRole("heading", { name: "login-flow" });
    await userEvent.click(screen.getByRole("button", { name: /^mobile bridge$/i }));
    const dialog = screen.getByRole("dialog", { name: /mobile bridge/i });
    await userEvent.click(within(dialog).getByRole("button", { name: /start mobile bridge/i }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent("Address already in use");
  });

  it("shows safe folder-session controls without Git actions", async () => {
    const session = folderRun();
    vi.mocked(dashboardState).mockResolvedValue(dashboard(session.id, [session]));

    render(<App />);

    await screen.findByRole("heading", { name: "investigate" });
    expect(screen.getByText("direct folder")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /merge/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: /diff/i })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: /end/i }));
    expect(screen.getByRole("alertdialog")).toHaveTextContent(
      "/workspace/product and all files inside it will be preserved"
    );
  });
});

function dashboard(selectedRunId: string, runs: RunView[] = [run("run-1", "login-flow"), run("run-2", "api-cleanup")]): DashboardState {
  const firstRun = runs[0];
  const workspaceKind = firstRun?.workspaceKind ?? "worktree";
  const workspacePath =
    workspaceKind === "folder"
      ? firstRun.workspacePath
      : firstRun?.repoPath ?? "/repo/agent-manager";
  return {
    repos: [
      {
        workspaceKind,
        workspacePath,
        repoName: firstRun?.repoName ?? "agent-manager",
        repoPath: firstRun?.repoPath ?? "/repo/agent-manager",
        runs
      }
    ],
    selectedRunId,
    activeCount: runs.length,
    attentionCount: runs.filter((run) => run.observedState === "needs-user" || run.observedState === "completed-unchecked").length,
    staleCount: 0,
    restorableCount: runs.filter((run) => run.restorable).length,
    activeRepoPath: "/repo/agent-manager",
    activeFolderPath: workspaceKind === "folder" ? workspacePath : null,
    hostTools: [
      { name: "git", available: true, detail: "available" },
      { name: "tmux", available: true, detail: "available" }
    ]
  };
}

function folderRun(): RunView {
  return {
    id: "folder-1",
    workspaceKind: "folder",
    workspacePath: "/workspace/product",
    repoPath: "/workspace/product",
    repoName: "product",
    tag: "local",
    runName: "investigate",
    agent: "claude",
    lifecycle: "active",
    observedState: "running",
    detectionSource: "tmux",
    branch: "",
    baseRef: "",
    worktreePath: "/workspace/product",
    restorable: false,
    createdAt: 3,
    updatedAt: 4
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });
  return { promise, resolve };
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
    workspaceKind: "worktree",
    workspacePath: `/repo/worktrees/${runName}`,
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

function emptyRunDiff(runId: string): RunDiffView {
  return runDiffFixture(runId, { files: [] });
}

function runDiffFixture(
  runId: string,
  overrides: Partial<RunDiffView> = {}
): RunDiffView {
  return {
    runId,
    baseRef: "HEAD",
    baseCommit: "abc123",
    worktreePath: "/repo/worktrees/login-flow",
    files: [
      {
        path: "README.md",
        oldPath: null,
        status: "modified",
        additions: 1,
        deletions: 0,
        binary: false,
        patch: "diff --git a/README.md b/README.md\n@@ -1 +1 @@\n+updated\n",
        message: null
      }
    ],
    fileCount: overrides.files?.length ?? 1,
    additions: 1,
    deletions: 0,
    generatedAt: 42,
    warning: null,
    ...overrides
  };
}

function restoreStatus(configured: boolean): TmuxRestoreStatus {
  return {
    configured,
    tpmInstalled: configured,
    resurrectInstalled: configured,
    continuumInstalled: configured,
    autoRestoreEnabled: configured,
    bootEnabled: configured,
    savedStateExists: configured,
    systemdUnitExists: configured,
    configPath: "/home/me/.tmux.conf",
    detail: configured
      ? "tmux restart restore is configured and has a saved session."
      : "tmux restart restore is not configured."
  };
}

function mobileStatus(enabled: boolean): MobileBridgeStatus {
  return {
    enabled,
    bind: "127.0.0.1:17654",
    publicUrl: "https://linhmon.linhmon.1vn.app",
    pairedDevices: [],
    xtunnelStartCommand: ["xtunnel.cmd", "linhmon", "start", "17654"]
  };
}
