import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { TerminalPane, shouldForwardTerminalInput } from "./TerminalPane";
import type { RunView } from "../types";

const mocks = vi.hoisted(() => ({
  terminalOptions: [] as Array<Record<string, unknown>>,
  terminals: [] as Array<{
    emitData: (data: string) => void;
    write: ReturnType<typeof vi.fn>;
    reset: ReturnType<typeof vi.fn>;
    focus: ReturnType<typeof vi.fn>;
    dispose: ReturnType<typeof vi.fn>;
  }>,
  startTerminal: vi.fn(),
  terminalInput: vi.fn(),
  resizeTerminal: vi.fn(),
  closeTerminal: vi.fn(),
  listenTerminalOutput: vi.fn(),
  listenTerminalClosed: vi.fn(),
  resizeObservers: [] as Array<{
    observe: ReturnType<typeof vi.fn>;
    unobserve: ReturnType<typeof vi.fn>;
    disconnect: ReturnType<typeof vi.fn>;
    trigger: () => void;
  }>
}));

vi.mock("@xterm/xterm", () => {
  class Terminal {
    private dataHandler: ((data: string) => void) | null = null;
    public write = vi.fn();
    public reset = vi.fn();
    public focus = vi.fn();
    public dispose = vi.fn();

    constructor(options: Record<string, unknown>) {
      mocks.terminalOptions.push(options);
      mocks.terminals.push({
        emitData: (data: string) => this.dataHandler?.(data),
        write: this.write,
        reset: this.reset,
        focus: this.focus,
        dispose: this.dispose
      });
    }

    loadAddon() {}
    open() {}
    onData(handler: (data: string) => void) {
      this.dataHandler = handler;
      return { dispose: vi.fn() };
    }
  }

  return { Terminal };
});

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
    proposeDimensions() {
      return { cols: 120, rows: 32 };
    }
  }
}));

vi.mock("../api", () => ({
  startTerminal: mocks.startTerminal,
  terminalInput: mocks.terminalInput,
  resizeTerminal: mocks.resizeTerminal,
  closeTerminal: mocks.closeTerminal,
  listenTerminalOutput: mocks.listenTerminalOutput,
  listenTerminalClosed: mocks.listenTerminalClosed
}));

describe("TerminalPane", () => {
  beforeEach(() => {
    mocks.terminalOptions.length = 0;
    mocks.terminals.length = 0;
    mocks.resizeObservers.length = 0;
    mocks.startTerminal.mockReset().mockResolvedValue({ terminalId: "term-1", runId: "run-1" });
    mocks.terminalInput.mockReset().mockResolvedValue(undefined);
    mocks.resizeTerminal.mockReset().mockResolvedValue(undefined);
    mocks.closeTerminal.mockReset().mockResolvedValue(undefined);
    mocks.listenTerminalOutput.mockReset().mockResolvedValue(() => undefined);
    mocks.listenTerminalClosed.mockReset().mockResolvedValue(() => undefined);
    const ResizeObserverMock = class {
      public observe = vi.fn();
      public unobserve = vi.fn();
      public disconnect = vi.fn();

      constructor(callback: ResizeObserverCallback) {
        mocks.resizeObservers.push({
          observe: this.observe,
          unobserve: this.unobserve,
          disconnect: this.disconnect,
          trigger: () => callback([], this)
        });
      }
    };
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);
  });

  it("keeps the terminal attached when dashboard refresh replaces the selected run object", async () => {
    const onError = vi.fn();
    const run = runView();
    const { rerender } = render(<TerminalPane selectedRun={run} onError={onError} />);

    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    rerender(
      <TerminalPane
        selectedRun={{ ...run, observedState: "needs-user", updatedAt: run.updatedAt + 1 }}
        onError={onError}
      />
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(mocks.closeTerminal).not.toHaveBeenCalled();
    expect(mocks.startTerminal).toHaveBeenCalledTimes(1);
  });

  it("does not forward xterm device-attribute replies as command input", async () => {
    const run = runView();
    render(<TerminalPane selectedRun={run} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    mocks.terminals[0].emitData("\x1b[>0;276;0c");

    expect(mocks.terminalInput).not.toHaveBeenCalled();
  });

  it("focuses the terminal when a selected run is attached", async () => {
    const run = runView();
    render(<TerminalPane selectedRun={run} onError={vi.fn()} />);

    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    expect(mocks.terminals[0].focus).toHaveBeenCalledTimes(1);
  });

  it("uses a readable text profile for dense tmux output", async () => {
    render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);

    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    expect(mocks.terminalOptions[0]).toMatchObject({
      fontSize: 15,
      lineHeight: 1.22,
      letterSpacing: 0,
      fontWeight: 500,
      fontWeightBold: 700,
      minimumContrastRatio: 4.5
    });
  });

  it("does not attach to missing tmux targets for restorable runs", async () => {
    render(<TerminalPane selectedRun={{ ...runView(), observedState: "unknown", detectionSource: "unknown", restorable: true }} onError={vi.fn()} />);

    await screen.findByText("Resume run to attach a terminal.");

    expect(mocks.startTerminal).not.toHaveBeenCalled();
  });

  it("refits and resizes the terminal when the pane dimensions change", async () => {
    const run = runView();
    render(<TerminalPane selectedRun={run} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    expect(mocks.resizeObservers).toHaveLength(1);

    mocks.resizeObservers[0].trigger();

    await waitFor(() => expect(mocks.resizeTerminal).toHaveBeenCalledWith("term-1", 120, 32));
  });
});

describe("shouldForwardTerminalInput", () => {
  it("filters xterm capability replies while keeping user input", () => {
    expect(shouldForwardTerminalInput("\x1b[>0;276;0c")).toBe(false);
    expect(shouldForwardTerminalInput("\x1b[?1;2c")).toBe(false);
    expect(shouldForwardTerminalInput("git status\r")).toBe(true);
  });
});

function runView(): RunView {
  return {
    id: "run-1",
    repoPath: "/repo",
    repoName: "repo",
    tag: "feature",
    runName: "terminal-fix",
    agent: "codex",
    lifecycle: "active",
    observedState: "running",
    detectionSource: "tmux",
    branch: "terminal-fix",
    baseRef: "main",
    worktreePath: "/repo-worktrees/terminal-fix",
    restorable: false,
    createdAt: 1,
    updatedAt: 2
  };
}
