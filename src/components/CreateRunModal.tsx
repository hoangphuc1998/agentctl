import { Bot, Folder, FolderOpen, GitBranch, Play, Tag, Type, X } from "lucide-react";
import { useEffect, useState } from "react";
import { chooseDirectory, createRun, repoSuggestions } from "../api";
import { Chip } from "./Chip";
import type { AgentKind, CreateRunDefaults, RunView } from "../types";
import type { Suggestion } from "../api";

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
  const [repoPathFocused, setRepoPathFocused] = useState(false);
  const [repoPathOptions, setRepoPathOptions] = useState<Suggestion[]>([]);
  const [activeRepoPathOption, setActiveRepoPathOption] = useState(-1);

  useEffect(() => {
    if (!open) return;
    const next = createInitialValues(defaults, activeRepoPath);
    setRepoPath(next.repoPath);
    setBaseRef(next.baseRef);
    setTag(next.tag);
    setAgent(next.agent);
    setRunName("");
    setSubmitError(null);
    setRepoPathFocused(false);
    setRepoPathOptions([]);
    setActiveRepoPathOption(-1);
  }, [activeRepoPath, defaults, open]);

  useEffect(() => {
    if (!open || !repoPathFocused) return;
    let cancelled = false;

    repoSuggestions(repoPath)
      .then((suggestions) => {
        if (cancelled) return;
        setRepoPathOptions(suggestions);
        setActiveRepoPathOption(-1);
      })
      .catch(() => {
        if (cancelled) return;
        setRepoPathOptions([]);
        setActiveRepoPathOption(-1);
      });

    return () => {
      cancelled = true;
    };
  }, [open, repoPath, repoPathFocused]);

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

  async function browseRepoFolder() {
    try {
      const selected = await chooseDirectory();
      if (!selected) return;
      setRepoPath(selected);
      setRepoPathFocused(false);
      setRepoPathOptions([]);
      setActiveRepoPathOption(-1);
    } catch {
      setRepoPathFocused(false);
    }
  }

  function selectRepoPathOption(option: Suggestion) {
    setRepoPath(option.value);
    setRepoPathFocused(false);
    setRepoPathOptions([]);
    setActiveRepoPathOption(-1);
  }

  function handleRepoPathKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (!repoPathFocused || repoPathOptions.length === 0) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveRepoPathOption((index) => (index + 1) % repoPathOptions.length);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveRepoPathOption((index) =>
        index <= 0 ? repoPathOptions.length - 1 : index - 1
      );
      return;
    }

    if (event.key === "Enter" && activeRepoPathOption >= 0) {
      event.preventDefault();
      selectRepoPathOption(repoPathOptions[activeRepoPathOption]);
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      setRepoPathFocused(false);
      setActiveRepoPathOption(-1);
    }
  }

  const repoPathListId = "create-run-repo-path-suggestions";
  const showRepoPathOptions = repoPathFocused && repoPathOptions.length > 0;

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
          <div className="repo-path-picker">
            <div className="input-with-icon repo-path-input">
              <Folder size={16} />
              <input
                value={repoPath}
                onChange={(event) => {
                  setRepoPath(event.target.value);
                  setRepoPathFocused(true);
                }}
                onFocus={() => setRepoPathFocused(true)}
                onBlur={() => setRepoPathFocused(false)}
                onKeyDown={handleRepoPathKeyDown}
                role="combobox"
                aria-autocomplete="list"
                aria-controls={showRepoPathOptions ? repoPathListId : undefined}
                aria-expanded={showRepoPathOptions}
                aria-haspopup="listbox"
                aria-activedescendant={
                  activeRepoPathOption >= 0
                    ? `create-run-repo-path-option-${activeRepoPathOption}`
                    : undefined
                }
              />
              <button
                type="button"
                className="icon-button repo-browse-button"
                onClick={browseRepoFolder}
                title="Browse repo folder"
                aria-label="Browse repo folder"
              >
                <FolderOpen size={16} />
              </button>
            </div>
            {showRepoPathOptions && (
              <div className="repo-path-suggestions" id={repoPathListId} role="listbox">
                {repoPathOptions.map((option, index) => (
                  <button
                    type="button"
                    className={
                      index === activeRepoPathOption
                        ? "repo-path-suggestion active"
                        : "repo-path-suggestion"
                    }
                    id={`create-run-repo-path-option-${index}`}
                    role="option"
                    aria-selected={index === activeRepoPathOption}
                    key={`${option.value}-${option.detail}`}
                    onMouseDown={(event) => event.preventDefault()}
                    onMouseEnter={() => setActiveRepoPathOption(index)}
                    onClick={() => selectRepoPathOption(option)}
                  >
                    <span className="repo-path-suggestion-value">{option.value}</span>
                    <span className="repo-path-suggestion-detail">{option.detail}</span>
                  </button>
                ))}
              </div>
            )}
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
