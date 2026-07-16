import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { Terminal as TerminalIcon } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  closeTerminal,
  listenTerminalClosed,
  listenTerminalOutput,
  openTerminalLink,
  resizeTerminal,
  startTerminal,
  terminalInput
} from "../api";
import {
  createTerminalLinkProvider,
  terminalLinkTargetFromUri,
  type TerminalLinkTarget
} from "../terminalLinks";
import { Chip } from "./Chip";
import type { RunView } from "../types";

interface TerminalPaneProps {
  selectedRun: RunView | null;
  active?: boolean;
  onError: (message: string | null) => void;
}

export function TerminalPane({ selectedRun, active = true, onError }: TerminalPaneProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const terminalIdRef = useRef<string | null>(null);
  const activeRef = useRef(active);
  const selectedRunIdRef = useRef<string | null>(null);
  const [status, setStatus] = useState("Select a run to attach a terminal.");
  const selectedRunId = selectedRun?.id ?? null;
  const selectedRunName = selectedRun?.runName ?? null;
  const selectedRunRestorable = selectedRun?.restorable ?? false;
  selectedRunIdRef.current = selectedRunId;

  const openDetectedLink = useCallback(
    (target: TerminalLinkTarget) => {
      const runId = selectedRunIdRef.current;
      if (!runId) return;
      void openTerminalLink(runId, target).catch((err) => onError(errorMessage(err)));
    },
    [onError]
  );

  const fitAndResizeTerminal = useCallback(() => {
    const fit = fitRef.current;
    if (!fit) return;
    fit.fit();
    const dims = fit.proposeDimensions();
    const terminalId = terminalIdRef.current;
    if (dims && terminalId) {
      void resizeTerminal(terminalId, dims.cols, dims.rows).catch(() => undefined);
    }
  }, []);

  const repaintTerminal = useCallback(() => {
    fitAndResizeTerminal();
    const terminal = terminalRef.current;
    if (!terminal) return;
    terminal.refresh(0, Math.max(0, terminal.rows - 1));
  }, [fitAndResizeTerminal]);

  useEffect(() => {
    activeRef.current = active;
    if (!active) return;
    repaintTerminal();
    terminalRef.current?.focus();
  }, [active, repaintTerminal]);

  useEffect(() => {
    const terminal = new Terminal({
      cursorBlink: true,
      fontFamily:
        '"Ubuntu Mono", "MesloLGS NF", "Noto Sans Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
      fontSize: 15,
      lineHeight: 1.22,
      letterSpacing: 0,
      fontWeight: 400,
      fontWeightBold: 700,
      minimumContrastRatio: 4.5,
      linkHandler: {
        allowNonHttpProtocols: true,
        activate: (_event, uri) => {
          const target = terminalLinkTargetFromUri(uri);
          if (target) openDetectedLink(target);
        }
      },
      theme: {
        background: "#030609",
        foreground: "#e6edf6",
        cursor: "#50d890",
        selectionBackground: "#24384f"
      }
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    const linkProvider = terminal.registerLinkProvider(
      createTerminalLinkProvider(terminal, openDetectedLink)
    );
    terminalRef.current = terminal;
    fitRef.current = fit;
    if (hostRef.current) {
      terminal.open(hostRef.current);
      fitAndResizeTerminal();
    }
    const disposable = terminal.onData((data) => {
      const terminalId = terminalIdRef.current;
      if (terminalId && shouldForwardTerminalInput(data)) {
        void terminalInput(terminalId, data).catch((err) => onError(errorMessage(err)));
      }
    });
    return () => {
      disposable.dispose();
      linkProvider.dispose();
      terminal.dispose();
      terminalRef.current = null;
      fitRef.current = null;
    };
  }, [fitAndResizeTerminal, onError, openDetectedLink]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const observer = new ResizeObserver(() => fitAndResizeTerminal());
    observer.observe(host);
    return () => observer.disconnect();
  }, [fitAndResizeTerminal]);

  useEffect(() => {
    function repaintIfVisible() {
      if (document.visibilityState === "hidden") return;
      repaintTerminal();
    }

    function onVisibilityChange() {
      if (document.visibilityState === "visible") {
        repaintTerminal();
      }
    }

    document.addEventListener("visibilitychange", onVisibilityChange);
    window.addEventListener("focus", repaintIfVisible);
    window.addEventListener("pageshow", repaintIfVisible);
    return () => {
      document.removeEventListener("visibilitychange", onVisibilityChange);
      window.removeEventListener("focus", repaintIfVisible);
      window.removeEventListener("pageshow", repaintIfVisible);
    };
  }, [repaintTerminal]);

  useEffect(() => {
    let disposed = false;
    let unlistenOutput: (() => void) | null = null;
    let unlistenClosed: (() => void) | null = null;
    let awaitingTerminalId = false;
    const pendingOutput = new Map<string, string[]>();
    const pendingClosed = new Set<string>();

    async function attach() {
      const terminal = terminalRef.current;
      const fit = fitRef.current;
      if (!terminal || !fit) return;
      terminal.reset();
      if (!selectedRunId) {
        setStatus("Select a run to attach a terminal.");
        terminalIdRef.current = null;
        return;
      }
      if (selectedRunRestorable) {
        setStatus("Resume run to attach a terminal.");
        terminalIdRef.current = null;
        return;
      }

      try {
        const runId = selectedRunId;
        const [outputUnlisten, closedUnlisten] = await Promise.all([
          listenTerminalOutput((event) => {
            if (disposed) return;
            const activeTerminalId = terminalIdRef.current;
            if (event.payload.terminalId === activeTerminalId) {
              writeTerminalData(terminal, event.payload.data, repaintTerminal);
              return;
            }
            if (awaitingTerminalId && !activeTerminalId && event.payload.runId === runId) {
              const buffered = pendingOutput.get(event.payload.terminalId) ?? [];
              buffered.push(event.payload.data);
              pendingOutput.set(event.payload.terminalId, buffered);
            }
          }),
          listenTerminalClosed((event) => {
            if (disposed) return;
            const activeTerminalId = terminalIdRef.current;
            if (event.payload.terminalId === activeTerminalId) {
              setStatus("Terminal session closed.");
              terminalIdRef.current = null;
              return;
            }
            if (awaitingTerminalId && !activeTerminalId && event.payload.runId === runId) {
              pendingClosed.add(event.payload.terminalId);
            }
          })
        ]);
        if (disposed) {
          outputUnlisten();
          closedUnlisten();
          return;
        }
        unlistenOutput = outputUnlisten;
        unlistenClosed = closedUnlisten;

        fitAndResizeTerminal();
        const dims = fit.proposeDimensions();
        awaitingTerminalId = true;
        const started = await startTerminal(selectedRunId, dims?.cols ?? 120, dims?.rows ?? 32);
        if (disposed) return;
        terminalIdRef.current = started.terminalId;
        awaitingTerminalId = false;
        setStatus(`Attached to ${selectedRunName ?? "selected run"}`);
        for (const data of pendingOutput.get(started.terminalId) ?? []) {
          writeTerminalData(terminal, data, repaintTerminal);
        }
        pendingOutput.clear();
        if (pendingClosed.has(started.terminalId)) {
          setStatus("Terminal session closed.");
          terminalIdRef.current = null;
          return;
        }
        if (activeRef.current) {
          terminal.focus();
        }
      } catch (err) {
        awaitingTerminalId = false;
        if (unlistenOutput) {
          unlistenOutput();
          unlistenOutput = null;
        }
        if (unlistenClosed) {
          unlistenClosed();
          unlistenClosed = null;
        }
        setStatus("Terminal unavailable.");
        onError(errorMessage(err));
      }
    }

    void attach();

    function onResize() {
      fitAndResizeTerminal();
    }

    window.addEventListener("resize", onResize);
    return () => {
      disposed = true;
      window.removeEventListener("resize", onResize);
      if (unlistenOutput) unlistenOutput();
      if (unlistenClosed) unlistenClosed();
      const terminalId = terminalIdRef.current;
      if (terminalId) void closeTerminal(terminalId).catch(() => undefined);
      terminalIdRef.current = null;
    };
  }, [
    fitAndResizeTerminal,
    onError,
    repaintTerminal,
    selectedRunId,
    selectedRunName,
    selectedRunRestorable
  ]);

  return (
    <section className="terminal-shell">
      <div className="terminal-toolbar">
        <span className="terminal-title">
          <TerminalIcon size={15} />
          <span>Embedded tmux terminal</span>
        </span>
        <span className="terminal-status">
          <Chip tone={selectedRunId && !selectedRunRestorable ? "success" : "neutral"}>
            {selectedRunId && !selectedRunRestorable ? "tmux attached" : "idle"}
          </Chip>
          <Chip tone="info" title={status}>
            {status}
          </Chip>
        </span>
      </div>
      <div className="terminal-host" ref={hostRef} />
    </section>
  );
}

export function shouldForwardTerminalInput(data: string): boolean {
  return !/^\x1b\[(?:\?\d+(?:;\d+)*|>0;\d+;0)c$/.test(data);
}

function writeTerminalData(terminal: Terminal, data: string, afterWrite: () => void): void {
  terminal.write(data, afterWrite);
}

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Terminal error.";
}
