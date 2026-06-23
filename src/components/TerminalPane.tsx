import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { Terminal as TerminalIcon } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  closeTerminal,
  listenTerminalClosed,
  listenTerminalOutput,
  resizeTerminal,
  startTerminal,
  terminalInput
} from "../api";
import { Chip } from "./Chip";
import type { RunView } from "../types";

interface TerminalPaneProps {
  selectedRun: RunView | null;
  onError: (message: string | null) => void;
}

export function TerminalPane({ selectedRun, onError }: TerminalPaneProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const terminalIdRef = useRef<string | null>(null);
  const [status, setStatus] = useState("Select a run to attach a terminal.");
  const selectedRunId = selectedRun?.id ?? null;
  const selectedRunName = selectedRun?.runName ?? null;
  const selectedRunRestorable = selectedRun?.restorable ?? false;

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
      theme: {
        background: "#030609",
        foreground: "#e6edf6",
        cursor: "#50d890",
        selectionBackground: "#24384f"
      }
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
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
      terminal.dispose();
      terminalRef.current = null;
      fitRef.current = null;
    };
  }, [fitAndResizeTerminal, onError]);

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
        fitAndResizeTerminal();
        const dims = fit.proposeDimensions();
        const started = await startTerminal(selectedRunId, dims?.cols ?? 120, dims?.rows ?? 32);
        if (disposed) return;
        terminalIdRef.current = started.terminalId;
        setStatus(`Attached to ${selectedRunName ?? "selected run"}`);
        terminal.focus();
        unlistenOutput = await listenTerminalOutput((event) => {
          if (event.payload.terminalId === terminalIdRef.current) {
            terminal.write(event.payload.data);
          }
        });
        unlistenClosed = await listenTerminalClosed((event) => {
          if (event.payload.terminalId === terminalIdRef.current) {
            setStatus("Terminal session closed.");
            terminalIdRef.current = null;
          }
        });
      } catch (err) {
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
  }, [fitAndResizeTerminal, onError, selectedRunId, selectedRunRestorable]);

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

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Terminal error.";
}
