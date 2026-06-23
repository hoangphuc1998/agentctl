import { Bot, Folder, GitBranch, Play, Tag, Type, X } from "lucide-react";
import { useEffect, useState } from "react";
import { createRun } from "../api";
import { Chip } from "./Chip";
import type { AgentKind, CreateRunDefaults, RunView } from "../types";

interface CreateRunModalProps {
  open: boolean;
  activeRepoPath: string | null;
  defaults?: CreateRunDefaults | null;
  onClose: () => void;
  onCreated: (run: RunView) => void;
  onError: (message: string | null) => void;
}

export function CreateRunModal({
  open,
  activeRepoPath,
  defaults,
  onClose,
  onCreated,
  onError
}: CreateRunModalProps) {
  const initialValues = createInitialValues(defaults, activeRepoPath);
  const [repoPath, setRepoPath] = useState(initialValues.repoPath);
  const [baseRef, setBaseRef] = useState(initialValues.baseRef);
  const [tag, setTag] = useState(initialValues.tag);
  const [runName, setRunName] = useState("");
  const [agent, setAgent] = useState<AgentKind>(initialValues.agent);
  const [busy, setBusy] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const next = createInitialValues(defaults, activeRepoPath);
    setRepoPath(next.repoPath);
    setBaseRef(next.baseRef);
    setTag(next.tag);
    setAgent(next.agent);
    setRunName("");
    setSubmitError(null);
  }, [activeRepoPath, defaults, open]);

  if (!open) return null;

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!repoPath.trim() || !baseRef.trim() || !tag.trim() || !runName.trim()) {
      const message = "Repo, base, tag, and run name are required.";
      setSubmitError(message);
      onError(message);
      return;
    }
    setBusy(true);
    setSubmitError(null);
    try {
      const result = await createRun({
        repoPath: repoPath.trim(),
        baseRef: baseRef.trim(),
        tag: tag.trim(),
        runName: runName.trim(),
        agent
      });
      if (result.run) onCreated(result.run);
    } catch (err) {
      const message = errorMessage(err);
      setSubmitError(message);
      onError(message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop">
      <form className="modal" onSubmit={submit}>
        <div className="modal-header">
          <div>
            <p className="eyebrow">New Run</p>
            <h2>Create worktree and launch agent</h2>
            {activeRepoPath && (
              <div className="modal-header-chips">
                <Chip tone="neutral" icon={<Folder size={14} />} title={activeRepoPath}>
                  active repo
                </Chip>
              </div>
            )}
          </div>
          <button type="button" className="icon-button" onClick={onClose} title="Close">
            <X size={18} />
          </button>
        </div>

        <label className="field-full">
          Repo path
          <div className="input-with-icon">
            <Folder size={16} />
            <input value={repoPath} onChange={(event) => setRepoPath(event.target.value)} />
          </div>
        </label>
        <label className="field-full">
          Run name
          <div className="input-with-icon">
            <Type size={16} />
            <input value={runName} onChange={(event) => setRunName(event.target.value)} autoFocus />
          </div>
        </label>

        <div className="modal-field-grid">
          <label>
            Base ref
            <div className="input-with-icon">
              <GitBranch size={16} />
              <input value={baseRef} onChange={(event) => setBaseRef(event.target.value)} />
            </div>
          </label>
          <label>
            Tag
            <div className="input-with-icon">
              <Tag size={16} />
              <input value={tag} onChange={(event) => setTag(event.target.value)} />
            </div>
          </label>
          <fieldset className="agent-segment-field">
            <legend>Agent</legend>
            <div className="agent-segmented-control" role="group" aria-label="Agent">
              {agentOptions.map((option) => (
                <button
                  type="button"
                  className={agent === option ? "agent-segment active" : "agent-segment"}
                  aria-pressed={agent === option}
                  onClick={() => setAgent(option)}
                  key={option}
                >
                  <Bot size={15} />
                  <span>{option}</span>
                </button>
              ))}
            </div>
          </fieldset>
        </div>
        {submitError && (
          <label className="field-full">
            Error details
            <textarea
              className="error-details-box"
              aria-label="Create run error details"
              readOnly
              value={submitError}
            />
          </label>
        )}
        <div className="modal-actions">
          <button type="button" className="button secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="button primary" disabled={busy}>
            <Play size={16} />
            Create
          </button>
        </div>
      </form>
    </div>
  );
}

const agentOptions: AgentKind[] = ["codex", "claude"];

function createInitialValues(defaults: CreateRunDefaults | null | undefined, activeRepoPath: string | null) {
  return {
    repoPath: defaults?.repoPath ?? activeRepoPath ?? "",
    baseRef: defaults?.baseRef ?? "HEAD",
    tag: defaults?.tag ?? "default",
    agent: defaults?.agent ?? "codex"
  };
}

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Create run failed.";
}
