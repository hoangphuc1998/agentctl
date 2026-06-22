import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { Terminal as TerminalIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
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

  useEffect(() => {
    const terminal = new Terminal({
      cursorBlink: true,
      fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, monospace",
      fontSize: 13,
      theme: {
        background: "#030609",
        foreground: "#d6e2ef",
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
      fit.fit();
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
  }, [onError]);

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

      try {
        fit.fit();
        const dims = fit.proposeDimensions();
        const started = await startTerminal(selectedRunId, dims?.cols ?? 120, dims?.rows ?? 32);
        if (disposed) return;
        terminalIdRef.current = started.terminalId;
        setStatus(`Attached to ${selectedRunName ?? "selected run"}`);
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
      const fit = fitRef.current;
      const terminalId = terminalIdRef.current;
      if (!fit || !terminalId) return;
      fit.fit();
      const dims = fit.proposeDimensions();
      if (dims) void resizeTerminal(terminalId, dims.cols, dims.rows).catch(() => undefined);
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
  }, [onError, selectedRunId]);

  return (
    <section className="terminal-shell">
      <div className="terminal-toolbar">
        <span className="terminal-title">
          <TerminalIcon size={15} />
          <span>Embedded tmux terminal</span>
        </span>
        <span className="terminal-status">
          <Chip tone={selectedRunId ? "success" : "neutral"}>{selectedRunId ? "tmux attached" : "idle"}</Chip>
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
