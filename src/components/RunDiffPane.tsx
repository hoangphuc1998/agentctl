import { FileText, GitCompareArrows, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { runDiff } from "../api";
import type { RunDiffFileView, RunDiffView, RunView } from "../types";
import { Chip } from "./Chip";

interface RunDiffPaneProps {
  selectedRun: RunView | null;
  active: boolean;
  onError: (message: string | null) => void;
}

export function RunDiffPane({ selectedRun, active, onError }: RunDiffPaneProps) {
  const selectedRunId = selectedRun?.id ?? null;
  const selectedRunIdRef = useRef<string | null>(selectedRunId);
  const [diff, setDiff] = useState<RunDiffView | null>(null);
  const [loadedRunId, setLoadedRunId] = useState<string | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    selectedRunIdRef.current = selectedRunId;
  }, [selectedRunId]);

  useEffect(() => {
    setDiff(null);
    setLoadedRunId(null);
    setSelectedPath(null);
    setLoadError(null);
  }, [selectedRunId]);

  const load = useCallback(async () => {
    if (!selectedRunId) return;
    const runId = selectedRunId;
    setLoading(true);
    setLoadError(null);
    try {
      const next = await runDiff(runId);
      if (selectedRunIdRef.current !== runId) return;
      if (!next) {
        throw new Error("Run not found.");
      }
      setDiff(next);
      setLoadedRunId(runId);
      setSelectedPath((current) =>
        current && next.files.some((file) => file.path === current)
          ? current
          : next.files[0]?.path ?? null
      );
    } catch (err) {
      if (selectedRunIdRef.current !== runId) return;
      const message = errorMessage(err);
      setLoadError(message);
      onError(message);
    } finally {
      if (selectedRunIdRef.current === runId) {
        setLoading(false);
      }
    }
  }, [onError, selectedRunId]);

  useEffect(() => {
    if (!active || !selectedRunId || loadedRunId === selectedRunId) return;
    void load();
  }, [active, load, loadedRunId, selectedRunId]);

  const selectedFile = useMemo(
    () => diff?.files.find((file) => file.path === selectedPath) ?? diff?.files[0] ?? null,
    [diff, selectedPath]
  );

  return (
    <section className="diff-shell" aria-label="Run diff">
      <div className="diff-toolbar">
        <span className="terminal-title">
          <GitCompareArrows size={15} />
          <span>File diff</span>
        </span>
        <span className="terminal-status">
          {diff && (
            <>
              <Chip tone="neutral">{pluralize(diff.fileCount, "file")}</Chip>
              <Chip tone="success">+{diff.additions}</Chip>
              <Chip tone="danger">-{diff.deletions}</Chip>
            </>
          )}
          <button
            className="icon-button"
            onClick={() => void load()}
            disabled={!selectedRunId || loading}
            aria-label="Refresh diff"
            title="Refresh diff"
          >
            <RefreshCw size={16} />
          </button>
        </span>
      </div>

      {!selectedRun && <div className="diff-empty">Select a run to review its file diff.</div>}
      {selectedRun && loading && !diff && <div className="diff-empty">Loading diff...</div>}
      {selectedRun && loadError && (
        <div className="diff-empty diff-error" role="alert">
          {loadError}
        </div>
      )}
      {selectedRun && diff && !loadError && (
        <div className="diff-review">
          <aside className="diff-file-list" aria-label="Changed files">
            {diff.warning && <div className="diff-warning">{diff.warning}</div>}
            {diff.files.length === 0 ? (
              <div className="diff-empty compact">No file changes.</div>
            ) : (
              diff.files.map((file) => (
                <button
                  key={`${file.oldPath ?? ""}:${file.path}`}
                  className={`diff-file-row${file.path === selectedFile?.path ? " selected" : ""}`}
                  onClick={() => setSelectedPath(file.path)}
                  type="button"
                >
                  <FileText size={15} />
                  <span className="diff-file-name">{file.path}</span>
                  <span className={`diff-file-status status-${file.status}`}>{file.status}</span>
                  <span className="diff-file-counts">
                    <span>+{file.additions}</span>
                    <span>-{file.deletions}</span>
                  </span>
                </button>
              ))
            )}
          </aside>
          <DiffFilePanel file={selectedFile} />
        </div>
      )}
    </section>
  );
}

function DiffFilePanel({ file }: { file: RunDiffFileView | null }) {
  if (!file) {
    return <div className="diff-empty">No file selected.</div>;
  }
  return (
    <div className="diff-file-panel">
      <div className="diff-file-header">
        <div>
          <h3>{file.path}</h3>
          {file.oldPath && <p className="muted">renamed from {file.oldPath}</p>}
        </div>
        <span className={`diff-file-status status-${file.status}`}>{file.status}</span>
      </div>
      {file.message && <div className="diff-warning">{file.message}</div>}
      {file.patch ? (
        <pre className="diff-code" aria-label={`Patch for ${file.path}`}>
          {file.patch.split("\n").map((line, index) => (
            <span className={diffLineClass(line)} key={`${index}:${line}`}>
              {line || " "}
            </span>
          ))}
        </pre>
      ) : (
        <div className="diff-empty compact">No text patch available.</div>
      )}
    </div>
  );
}

function diffLineClass(line: string): string {
  if (line.startsWith("@@")) return "diff-line hunk";
  if (line.startsWith("+") && !line.startsWith("+++")) return "diff-line added";
  if (line.startsWith("-") && !line.startsWith("---")) return "diff-line deleted";
  if (
    line.startsWith("diff --git") ||
    line.startsWith("index ") ||
    line.startsWith("---") ||
    line.startsWith("+++")
  ) {
    return "diff-line meta";
  }
  return "diff-line";
}

function pluralize(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Failed to load run diff.";
}
