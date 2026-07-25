import { Bot, Files, Folder, FolderGit2, FolderOpen, GitBranch, Play, Tag, Type, X } from "lucide-react";
import { useEffect, useState } from "react";
import {
  chooseDirectory,
  createFolderSession,
  createRun,
  folderSuggestions,
  ignoredFilesPreview,
  repoSuggestions
} from "../api";
import { Chip } from "./Chip";
import { ConfirmDialog } from "./ConfirmDialog";
import type {
  AgentKind,
  CreateFolderSessionPayload,
  CreateRunDefaults,
  CreateRunPayload,
  IgnoredFilesPreview,
  RunView,
  WorkspaceKind
} from "../types";
import type { Suggestion } from "../api";

interface CreateRunModalProps {
  open: boolean;
  activeRepoPath: string | null;
  activeFolderPath?: string | null;
  defaults?: CreateRunDefaults | null;
  onClose: () => void;
  onCreated: (run: RunView) => void;
  onError: (message: string | null) => void;
}

export function CreateRunModal({
  open,
  activeRepoPath,
  activeFolderPath,
  defaults,
  onClose,
  onCreated,
  onError
}: CreateRunModalProps) {
  const initialValues = createInitialValues(
    defaults,
    activeRepoPath,
    activeFolderPath ?? null
  );
  const [repoPath, setRepoPath] = useState(initialValues.repoPath);
  const [baseRef, setBaseRef] = useState(initialValues.baseRef);
  const [tag, setTag] = useState(initialValues.tag);
  const [runName, setRunName] = useState("");
  const [agent, setAgent] = useState<AgentKind>(initialValues.agent);
  const [workspaceKind, setWorkspaceKind] = useState<WorkspaceKind>(
    initialValues.workspaceKind
  );
  const [busy, setBusy] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [repoPathFocused, setRepoPathFocused] = useState(false);
  const [repoPathOptions, setRepoPathOptions] = useState<Suggestion[]>([]);
  const [activeRepoPathOption, setActiveRepoPathOption] = useState(-1);
  const [copyIgnoredFiles, setCopyIgnoredFiles] = useState(true);
  const [ignoredPreview, setIgnoredPreview] = useState<IgnoredFilesPreview | null>(null);
  const [ignoredPreviewLoading, setIgnoredPreviewLoading] = useState(false);
  const [ignoredPreviewError, setIgnoredPreviewError] = useState<string | null>(null);
  const [pendingLargeCopy, setPendingLargeCopy] = useState<{
    payload: CreateRunPayload;
    preview: IgnoredFilesPreview;
  } | null>(null);

  useEffect(() => {
    if (!open) return;
    const next = createInitialValues(defaults, activeRepoPath, activeFolderPath ?? null);
    setRepoPath(next.repoPath);
    setBaseRef(next.baseRef);
    setTag(next.tag);
    setAgent(next.agent);
    setWorkspaceKind(next.workspaceKind);
    setRunName("");
    setSubmitError(null);
    setRepoPathFocused(false);
    setRepoPathOptions([]);
    setActiveRepoPathOption(-1);
    setCopyIgnoredFiles(true);
    setIgnoredPreview(null);
    setIgnoredPreviewLoading(false);
    setIgnoredPreviewError(null);
    setPendingLargeCopy(null);
  }, [activeFolderPath, activeRepoPath, defaults, open]);

  useEffect(() => {
    if (!open || !repoPathFocused) return;
    let cancelled = false;

    const suggestions =
      workspaceKind === "folder" ? folderSuggestions(repoPath) : repoSuggestions(repoPath);
    suggestions
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
  }, [open, repoPath, repoPathFocused, workspaceKind]);

  useEffect(() => {
    if (!open || workspaceKind !== "worktree" || !copyIgnoredFiles || !repoPath.trim()) {
      setIgnoredPreview(null);
      setIgnoredPreviewLoading(false);
      setIgnoredPreviewError(null);
      return;
    }

    let cancelled = false;
    const timeout = window.setTimeout(() => {
      setIgnoredPreviewLoading(true);
      setIgnoredPreviewError(null);
      ignoredFilesPreview(repoPath.trim())
        .then((preview) => {
          if (cancelled) return;
          setIgnoredPreview(preview);
        })
        .catch((err) => {
          if (cancelled) return;
          setIgnoredPreview(null);
          setIgnoredPreviewError(`Could not inspect ignored files: ${errorMessage(err)}`);
        })
        .finally(() => {
          if (cancelled) return;
          setIgnoredPreviewLoading(false);
        });
    }, 300);

    return () => {
      cancelled = true;
      window.clearTimeout(timeout);
    };
  }, [copyIgnoredFiles, open, repoPath, workspaceKind]);

  if (!open) return null;

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const missingRequired =
      !repoPath.trim() ||
      !tag.trim() ||
      !runName.trim() ||
      (workspaceKind === "worktree" && !baseRef.trim());
    if (missingRequired) {
      const message =
        workspaceKind === "folder"
          ? "Folder, tag, and session name are required."
          : "Repo, base, tag, and run name are required.";
      setSubmitError(message);
      onError(message);
      return;
    }
    if (workspaceKind === "folder") {
      await performFolderCreate({
        folderPath: repoPath.trim(),
        tag: tag.trim(),
        runName: runName.trim(),
        agent
      });
      return;
    }
    const payload: CreateRunPayload = {
      repoPath: repoPath.trim(),
      baseRef: baseRef.trim(),
      tag: tag.trim(),
      runName: runName.trim(),
      agent,
      copyIgnoredFiles
    };

    if (copyIgnoredFiles) {
      setBusy(true);
      setIgnoredPreviewError(null);
      try {
        const preview = await ignoredFilesPreview(payload.repoPath);
        setIgnoredPreview(preview);
        if (preview.requiresConfirmation) {
          setPendingLargeCopy({ payload, preview });
          return;
        }
      } catch (err) {
        const message = `Could not inspect ignored files: ${errorMessage(err)}`;
        setIgnoredPreview(null);
        setIgnoredPreviewError(message);
        onError(message);
        return;
      } finally {
        setBusy(false);
      }
    }

    await performCreate(payload);
  }

  async function performFolderCreate(payload: CreateFolderSessionPayload) {
    setBusy(true);
    setSubmitError(null);
    try {
      const result = await createFolderSession(payload);
      if (result.run) onCreated(result.run);
    } catch (err) {
      const message = errorMessage(err);
      setSubmitError(message);
      onError(message);
    } finally {
      setBusy(false);
    }
  }

  async function performCreate(payload: CreateRunPayload) {
    setBusy(true);
    setSubmitError(null);
    try {
      const result = await createRun(payload);
      if (result.run) onCreated(result.run);
    } catch (err) {
      const message = errorMessage(err);
      setSubmitError(message);
      onError(message);
    } finally {
      setBusy(false);
    }
  }

  function confirmLargeCopy() {
    const pending = pendingLargeCopy;
    setPendingLargeCopy(null);
    if (pending) void performCreate(pending.payload);
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

  function selectWorkspaceKind(kind: WorkspaceKind) {
    setWorkspaceKind(kind);
    setRepoPath(kind === "folder" ? activeFolderPath ?? "" : activeRepoPath ?? "");
    setRepoPathFocused(false);
    setRepoPathOptions([]);
    setActiveRepoPathOption(-1);
    setSubmitError(null);
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
  const activeWorkspacePath =
    workspaceKind === "folder" ? activeFolderPath : activeRepoPath;

  return (
    <>
      <div className="modal-backdrop">
        <form className="modal" onSubmit={submit}>
        <div className="modal-header">
          <div>
            <p className="eyebrow">New Run</p>
            <h2>
              {workspaceKind === "folder"
                ? "Launch agent in existing folder"
                : "Create worktree and launch agent"}
            </h2>
            {activeWorkspacePath && (
              <div className="modal-header-chips">
                <Chip tone="neutral" icon={<Folder size={14} />} title={activeWorkspacePath}>
                  {workspaceKind === "folder" ? "active folder" : "active repo"}
                </Chip>
              </div>
            )}
          </div>
          <button type="button" className="icon-button" onClick={onClose} title="Close">
            <X size={18} />
          </button>
        </div>

        <fieldset className="workspace-kind-field">
          <legend>Workspace mode</legend>
          <div className="workspace-kind-control" role="group" aria-label="Workspace mode">
            <button
              type="button"
              className={workspaceKind === "worktree" ? "workspace-kind active" : "workspace-kind"}
              aria-pressed={workspaceKind === "worktree"}
              onClick={() => selectWorkspaceKind("worktree")}
            >
              <FolderGit2 size={16} />
              <span>Worktree</span>
            </button>
            <button
              type="button"
              className={workspaceKind === "folder" ? "workspace-kind active" : "workspace-kind"}
              aria-pressed={workspaceKind === "folder"}
              onClick={() => selectWorkspaceKind("folder")}
            >
              <Folder size={16} />
              <span>Folder</span>
            </button>
          </div>
        </fieldset>

        <label className="field-full">
          {workspaceKind === "folder" ? "Folder path" : "Repo path"}
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
                title={workspaceKind === "folder" ? "Browse folder" : "Browse repo folder"}
                aria-label={workspaceKind === "folder" ? "Browse folder" : "Browse repo folder"}
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
          {workspaceKind === "folder" ? "Session name" : "Run name"}
          <div className="input-with-icon">
            <Type size={16} />
            <input value={runName} onChange={(event) => setRunName(event.target.value)} autoFocus />
          </div>
        </label>

        <div className="modal-field-grid">
          {workspaceKind === "worktree" && (
            <label>
              Base ref
              <div className="input-with-icon">
                <GitBranch size={16} />
                <input value={baseRef} onChange={(event) => setBaseRef(event.target.value)} />
              </div>
            </label>
          )}
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
        {workspaceKind === "worktree" && <div className="ignored-copy-card">
          <label className="ignored-copy-toggle">
            <input
              type="checkbox"
              checked={copyIgnoredFiles}
              onChange={(event) => setCopyIgnoredFiles(event.target.checked)}
            />
            <Files size={17} />
            <span>
              <strong>Copy ignored files</strong>
              <small>Includes .env secrets, generated files, caches, and every Git-ignored file.</small>
            </span>
          </label>
          {copyIgnoredFiles && (
            <div className="ignored-copy-preview" aria-live="polite">
              {ignoredPreviewLoading && <span>Scanning ignored files…</span>}
              {!ignoredPreviewLoading && ignoredPreview && (
                <span>
                  {ignoredPreview.fileCount === 0
                    ? "No ignored files found."
                    : `${formatCount(ignoredPreview.fileCount)} ignored files · ${formatBytes(ignoredPreview.totalBytes)}`}
                </span>
              )}
              {!ignoredPreviewLoading && ignoredPreview?.requiresConfirmation && (
                <span className="ignored-copy-warning">Large snapshot—confirmation required.</span>
              )}
              {!ignoredPreviewLoading && ignoredPreviewError && (
                <span className="ignored-copy-error" role="alert">
                  {ignoredPreviewError}
                </span>
              )}
            </div>
          )}
        </div>}
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
      {pendingLargeCopy && (
        <ConfirmDialog
          title="Copy large snapshot?"
          body={`This will copy ${formatCount(pendingLargeCopy.preview.fileCount)} ignored files (${formatBytes(pendingLargeCopy.preview.totalBytes)}) into the new worktree.`}
          confirmLabel="Copy and create"
          onCancel={() => setPendingLargeCopy(null)}
          onConfirm={confirmLargeCopy}
        />
      )}
    </>
  );
}

const agentOptions: AgentKind[] = ["codex", "claude"];

function createInitialValues(
  defaults: CreateRunDefaults | null | undefined,
  activeRepoPath: string | null,
  activeFolderPath: string | null
) {
  const workspaceKind = defaults?.workspaceKind ?? "worktree";
  return {
    workspaceKind,
    repoPath:
      defaults?.repoPath ??
      (workspaceKind === "folder" ? activeFolderPath : activeRepoPath) ??
      "",
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

function formatCount(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const formatted = value >= 10 || Number.isInteger(value) ? value.toFixed(0) : value.toFixed(1);
  return `${formatted} ${units[unitIndex]}`;
}
