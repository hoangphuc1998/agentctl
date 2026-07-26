import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { TerminalPane, shouldForwardTerminalInput } from "./TerminalPane";
import type { RunView } from "../types";

type TerminalOutputHandler = (event: {
  payload: { terminalId: string; runId: string; data: string };
}) => void;

type TerminalClosedHandler = (event: {
  payload: { terminalId: string; runId: string };
}) => void;

const mocks = vi.hoisted(() => ({
  terminalOptions: [] as Array<Record<string, unknown>>,
  terminals: [] as Array<{
    emitData: (data: string) => void;
    getLine: ReturnType<typeof vi.fn>;
    registerLinkProvider: ReturnType<typeof vi.fn>;
    write: ReturnType<typeof vi.fn>;
    reset: ReturnType<typeof vi.fn>;
    refresh: ReturnType<typeof vi.fn>;
    focus: ReturnType<typeof vi.fn>;
    dispose: ReturnType<typeof vi.fn>;
  }>,
  startTerminal: vi.fn(),
  terminalInput: vi.fn(),
  openTerminalLink: vi.fn(),
  resizeTerminal: vi.fn(),
  closeTerminal: vi.fn(),
  listenTerminalOutput: vi.fn(),
  listenTerminalClosed: vi.fn(),
  outputHandler: null as TerminalOutputHandler | null,
  closedHandler: null as TerminalClosedHandler | null,
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
    public refresh = vi.fn();
    public focus = vi.fn();
    public dispose = vi.fn();
    public registerLinkProvider = vi.fn(() => ({ dispose: vi.fn() }));
    public rows = 32;
    public getLine = vi.fn();
    public buffer = { active: { getLine: this.getLine } };

    constructor(options: Record<string, unknown>) {
      mocks.terminalOptions.push(options);
      mocks.terminals.push({
        emitData: (data: string) => this.dataHandler?.(data),
        getLine: this.getLine,
        registerLinkProvider: this.registerLinkProvider,
        write: this.write,
        reset: this.reset,
        refresh: this.refresh,
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
  openTerminalLink: mocks.openTerminalLink,
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
    mocks.openTerminalLink.mockReset().mockResolvedValue(undefined);
    mocks.resizeTerminal.mockReset().mockResolvedValue(undefined);
    mocks.closeTerminal.mockReset().mockResolvedValue(undefined);
    mocks.outputHandler = null;
    mocks.closedHandler = null;
    mocks.listenTerminalOutput.mockReset().mockImplementation((handler: TerminalOutputHandler) => {
      mocks.outputHandler = handler;
      return Promise.resolve(() => undefined);
    });
    mocks.listenTerminalClosed.mockReset().mockImplementation((handler: TerminalClosedHandler) => {
      mocks.closedHandler = handler;
      return Promise.resolve(() => undefined);
    });
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

  it("does not show a browser tooltip over the terminal", async () => {
    const { container } = render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    expect(container.querySelector(".terminal-host")).not.toHaveAttribute("title");
  });

  it("does not forward xterm device-attribute replies as command input", async () => {
    const run = runView();
    render(<TerminalPane selectedRun={run} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    mocks.terminals[0].emitData("\x1b[>0;276;0c");

    expect(mocks.terminalInput).not.toHaveBeenCalled();
  });

  it("opens explicit OSC file hyperlinks with a primary click", async () => {
    const { container } = render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    expect(mocks.terminals[0].registerLinkProvider).toHaveBeenCalledTimes(1);
    const linkHandler = mocks.terminalOptions[0].linkHandler as {
      allowNonHttpProtocols: boolean;
      activate: (event: MouseEvent, text: string) => void;
      hover: (event: MouseEvent, text: string) => void;
    };
    expect(linkHandler.allowNonHttpProtocols).toBe(true);

    linkHandler.activate(new MouseEvent("click", { button: 0 }), "file:///repo/src/main.ts#L7:2");
    expect(mocks.openTerminalLink).toHaveBeenCalledWith("run-1", {
      kind: "file",
      path: "/repo/src/main.ts",
      line: 7,
      column: 2
    });

    mocks.openTerminalLink.mockClear();
    linkHandler.hover(new MouseEvent("mousemove"), "file:///repo/src/main.ts#L7:2");
    const host = container.querySelector(".terminal-host");
    expect(host).not.toBeNull();
    fireEvent.mouseDown(host as Element, { button: 0 });

    expect(mocks.openTerminalLink).toHaveBeenCalledWith("run-1", {
      kind: "file",
      path: "/repo/src/main.ts",
      line: 7,
      column: 2
    });
  });

  it("opens detected file references only with Ctrl+primary-click", async () => {
    const { container } = render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));
    mocks.terminals[0].getLine.mockReturnValue({
      translateToString: () => "Edit src/main.ts:9"
    });
    const provider = mocks.terminals[0].registerLinkProvider.mock.calls[0][0] as {
      provideLinks: (line: number, callback: (links: Array<{
        text: string;
        activate: (event: MouseEvent, text: string) => void;
        hover?: (event: MouseEvent, text: string) => void;
      }> | undefined) => void) => void;
    };
    let links: Array<{
      text: string;
      activate: (event: MouseEvent, text: string) => void;
      hover?: (event: MouseEvent, text: string) => void;
    }> | undefined;
    provider.provideLinks(1, (provided) => {
      links = provided;
    });
    const link = links?.[0];
    expect(link).toBeDefined();

    link?.activate(new MouseEvent("click", { button: 0 }), link.text);
    expect(mocks.openTerminalLink).not.toHaveBeenCalled();

    link?.activate(new MouseEvent("click", { button: 0, ctrlKey: true }), link.text);
    expect(mocks.openTerminalLink).toHaveBeenCalledWith("run-1", {
      kind: "file",
      path: "src/main.ts",
      line: 9
    });

    mocks.openTerminalLink.mockClear();
    link?.hover?.(new MouseEvent("mousemove"), link.text);
    const host = container.querySelector(".terminal-host");
    expect(host).not.toBeNull();
    fireEvent.mouseDown(host as Element, { button: 0, ctrlKey: true });
    expect(mocks.openTerminalLink).toHaveBeenCalledWith("run-1", {
      kind: "file",
      path: "src/main.ts",
      line: 9
    });
  });

  it("opens detected file URLs with a primary click", async () => {
    const { container } = render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));
    mocks.terminals[0].getLine.mockReturnValue({
      translateToString: () => "Saved to: file:///home/user/.codex/generated_images/render.png"
    });
    const provider = mocks.terminals[0].registerLinkProvider.mock.calls[0][0] as {
      provideLinks: (line: number, callback: (links: Array<{
        text: string;
        hover?: (event: MouseEvent, text: string) => void;
      }> | undefined) => void) => void;
    };
    let links: Array<{
      text: string;
      hover?: (event: MouseEvent, text: string) => void;
    }> | undefined;
    provider.provideLinks(1, (provided) => {
      links = provided;
    });
    const link = links?.[0];
    expect(link).toBeDefined();

    link?.hover?.(new MouseEvent("mousemove"), link.text);
    const host = container.querySelector(".terminal-host");
    expect(host).not.toBeNull();
    fireEvent.mouseDown(host as Element, { button: 0 });

    expect(mocks.openTerminalLink).toHaveBeenCalledWith("run-1", {
      kind: "file",
      path: "/home/user/.codex/generated_images/render.png"
    });
  });

  it("keeps tmux initial redraw output emitted before startTerminal resolves", async () => {
    mocks.startTerminal.mockImplementation(async () => {
      mocks.outputHandler?.({
        payload: {
          terminalId: "term-1",
          runId: "run-1",
          data: "initial tmux redraw"
        }
      });
      return { terminalId: "term-1", runId: "run-1" };
    });

    render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);

    await waitFor(() => {
      expect(mocks.terminals[0].write.mock.calls.map(([data]) => data)).toContain(
        "initial tmux redraw"
      );
    });
  });

  it("focuses the terminal when a selected run is attached", async () => {
    const run = runView();
    render(<TerminalPane selectedRun={run} onError={vi.fn()} />);

    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    expect(mocks.terminals[0].focus).toHaveBeenCalledTimes(1);
  });

  it("uses the native terminal font stack for dense tmux output", async () => {
    render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);

    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));

    expect(mocks.terminalOptions[0]).toMatchObject({
      fontFamily:
        '"Ubuntu Mono", "MesloLGS NF", "Noto Sans Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
      fontSize: 15,
      lineHeight: 1.22,
      letterSpacing: 0,
      fontWeight: 400,
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

  it("repaints the terminal when the app becomes visible again", async () => {
    render(<TerminalPane selectedRun={runView()} onError={vi.fn()} />);
    await waitFor(() => expect(mocks.startTerminal).toHaveBeenCalledTimes(1));
    mocks.terminals[0].refresh.mockClear();

    document.dispatchEvent(new Event("visibilitychange"));

    expect(mocks.terminals[0].refresh).toHaveBeenCalledWith(0, 31);
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
    workspaceKind: "worktree",
    workspacePath: "/repo-worktrees/terminal-fix",
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
