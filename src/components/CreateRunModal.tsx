import { Folder, Play, X } from "lucide-react";
import { useState } from "react";
import { createRun } from "../api";
import type { AgentKind, RunView } from "../types";

interface CreateRunModalProps {
  open: boolean;
  activeRepoPath: string | null;
  onClose: () => void;
  onCreated: (run: RunView) => void;
  onError: (message: string | null) => void;
}

export function CreateRunModal({ open, activeRepoPath, onClose, onCreated, onError }: CreateRunModalProps) {
  const [repoPath, setRepoPath] = useState(activeRepoPath ?? "");
  const [baseRef, setBaseRef] = useState("HEAD");
  const [tag, setTag] = useState("default");
  const [runName, setRunName] = useState("");
  const [agent, setAgent] = useState<AgentKind>("codex");
  const [busy, setBusy] = useState(false);

  if (!open) return null;

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!repoPath.trim() || !baseRef.trim() || !tag.trim() || !runName.trim()) {
      onError("Repo, base, tag, and run name are required.");
      return;
    }
    setBusy(true);
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
      onError(errorMessage(err));
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
          </div>
          <button type="button" className="icon-button" onClick={onClose} title="Close">
            <X size={18} />
          </button>
        </div>

        <label>
          Repo path
          <div className="input-with-icon">
            <Folder size={16} />
            <input value={repoPath} onChange={(event) => setRepoPath(event.target.value)} />
          </div>
        </label>
        <label>
          Base ref
          <input value={baseRef} onChange={(event) => setBaseRef(event.target.value)} />
        </label>
        <label>
          Tag
          <input value={tag} onChange={(event) => setTag(event.target.value)} />
        </label>
        <label>
          Run name
          <input value={runName} onChange={(event) => setRunName(event.target.value)} autoFocus />
        </label>
        <label>
          Agent
          <select value={agent} onChange={(event) => setAgent(event.target.value as AgentKind)}>
            <option value="codex">codex</option>
            <option value="claude">claude</option>
          </select>
        </label>
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

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Create run failed.";
}

