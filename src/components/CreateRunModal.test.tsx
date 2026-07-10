import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createRun, ignoredFilesPreview, repoSuggestions } from "../api";
import { CreateRunModal } from "./CreateRunModal";

const { chooseDirectoryMock } = vi.hoisted(() => ({
  chooseDirectoryMock: vi.fn()
}));

vi.mock("../api", () => ({
  chooseDirectory: chooseDirectoryMock,
  createRun: vi.fn(),
  ignoredFilesPreview: vi.fn(),
  repoSuggestions: vi.fn()
}));

describe("CreateRunModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ignoredFilesPreview).mockResolvedValue({
      fileCount: 0,
      totalBytes: 0,
      requiresConfirmation: false
    });
  });

  it("prefills editable create fields from defaults", () => {
    render(
      <CreateRunModal
        open
        activeRepoPath={null}
        defaults={{
          repoPath: "/repo/agent-manager",
          baseRef: "main",
          tag: "review",
          agent: "claude"
        }}
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    expect(screen.getByLabelText(/repo path/i)).toHaveValue("/repo/agent-manager");
    expect(screen.getByLabelText(/base ref/i)).toHaveValue("main");
    expect(screen.getByLabelText(/tag/i)).toHaveValue("review");
    expect(screen.getByRole("button", { name: /claude/i })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByLabelText(/run name/i)).toHaveValue("");
  });

  it("uses segmented agent controls and submits the selected agent", async () => {
    vi.mocked(createRun).mockResolvedValue({
      message: "Created fix-ui.",
      run: null
    });

    render(
      <CreateRunModal
        open
        activeRepoPath="/repo/agent-manager"
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.type(screen.getByLabelText(/run name/i), "fix-ui");
    await userEvent.click(screen.getByRole("button", { name: /claude/i }));
    await userEvent.click(screen.getByRole("button", { name: /create/i }));

    await waitFor(() =>
      expect(createRun).toHaveBeenCalledWith(
        expect.objectContaining({
          agent: "claude",
          runName: "fix-ui"
        })
      )
    );
  });

  it("shows create-run errors in a read-only textbox", async () => {
    vi.mocked(createRun).mockRejectedValue(
      "tmux window was not created or exited immediately: agentctl__default__fix-ui-color__dcf191f7"
    );

    render(
      <CreateRunModal
        open
        activeRepoPath="/repo/agent-manager"
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.type(screen.getByLabelText(/run name/i), "fix-ui-color");
    await userEvent.click(screen.getByRole("button", { name: /create/i }));

    const errorDetails = await screen.findByRole("textbox", {
      name: /create run error details/i
    });

    expect(errorDetails).toHaveValue(
      "tmux window was not created or exited immediately: agentctl__default__fix-ui-color__dcf191f7"
    );
    expect(errorDetails).toHaveAttribute("readonly");
    await waitFor(() => expect(createRun).toHaveBeenCalledOnce());
  });

  it("shows repo path suggestions and applies a clicked folder", async () => {
    vi.mocked(repoSuggestions).mockResolvedValue([
      { value: "/repo/agent-manager/", detail: "recent repo" },
      { value: "/repo/agent-manager-mobile/", detail: "directory" }
    ]);

    render(
      <CreateRunModal
        open
        activeRepoPath={null}
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.type(screen.getByLabelText(/repo path/i), "/repo");

    const suggestion = await screen.findByRole("option", {
      name: /\/repo\/agent-manager\/ recent repo/i
    });
    await userEvent.click(suggestion);

    expect(screen.getByLabelText(/repo path/i)).toHaveValue("/repo/agent-manager/");
  });

  it("supports keyboard selection for repo path suggestions", async () => {
    vi.mocked(repoSuggestions).mockResolvedValue([
      { value: "/repo/alpha/", detail: "directory" },
      { value: "/repo/beta/", detail: "directory" }
    ]);

    render(
      <CreateRunModal
        open
        activeRepoPath={null}
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    const repoPath = screen.getByLabelText(/repo path/i);
    await userEvent.type(repoPath, "/repo");
    await screen.findByRole("option", { name: /\/repo\/alpha\/ directory/i });

    await userEvent.keyboard("{ArrowDown}{ArrowDown}{Enter}");

    expect(repoPath).toHaveValue("/repo/beta/");
  });

  it("hides repo path suggestions when focus leaves the field", async () => {
    vi.mocked(repoSuggestions).mockResolvedValue([
      { value: "/repo/agent-manager/", detail: "recent repo" }
    ]);

    render(
      <CreateRunModal
        open
        activeRepoPath={null}
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.type(screen.getByLabelText(/repo path/i), "/repo");
    await screen.findByRole("option", { name: /\/repo\/agent-manager\/ recent repo/i });

    await userEvent.click(screen.getByLabelText(/run name/i));

    expect(
      screen.queryByRole("option", { name: /\/repo\/agent-manager\/ recent repo/i })
    ).not.toBeInTheDocument();
  });

  it("fills repo path from the native folder picker", async () => {
    chooseDirectoryMock.mockResolvedValue("/home/me/projects/agent-manager");

    render(
      <CreateRunModal
        open
        activeRepoPath={null}
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.click(screen.getByRole("button", { name: /browse repo folder/i }));

    expect(screen.getByLabelText(/repo path/i)).toHaveValue("/home/me/projects/agent-manager");
  });

  it("previews ignored files and copies them by default", async () => {
    vi.mocked(ignoredFilesPreview).mockResolvedValue({
      fileCount: 3,
      totalBytes: 1536,
      requiresConfirmation: false
    });
    vi.mocked(createRun).mockResolvedValue({ message: "Created copy-files.", run: null });

    render(
      <CreateRunModal
        open
        activeRepoPath="/repo/agent-manager"
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    expect(screen.getByRole("checkbox", { name: /copy ignored files/i })).toBeChecked();
    expect(await screen.findByText(/3 ignored files · 1.5 KiB/i)).toBeInTheDocument();

    await userEvent.type(screen.getByLabelText(/run name/i), "copy-files");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() =>
      expect(createRun).toHaveBeenCalledWith(
        expect.objectContaining({ copyIgnoredFiles: true, runName: "copy-files" })
      )
    );
  });

  it("can preserve the existing nonignored-only copy behavior", async () => {
    vi.mocked(createRun).mockResolvedValue({ message: "Created lean-run.", run: null });

    render(
      <CreateRunModal
        open
        activeRepoPath="/repo/agent-manager"
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.click(screen.getByRole("checkbox", { name: /copy ignored files/i }));
    await userEvent.type(screen.getByLabelText(/run name/i), "lean-run");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() =>
      expect(createRun).toHaveBeenCalledWith(
        expect.objectContaining({ copyIgnoredFiles: false, runName: "lean-run" })
      )
    );
  });

  it("requires confirmation before creating a large ignored-file snapshot", async () => {
    vi.mocked(ignoredFilesPreview).mockResolvedValue({
      fileCount: 10_000,
      totalBytes: 104_857_600,
      requiresConfirmation: true
    });
    vi.mocked(createRun).mockResolvedValue({ message: "Created large-run.", run: null });

    render(
      <CreateRunModal
        open
        activeRepoPath="/repo/agent-manager"
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.type(screen.getByLabelText(/run name/i), "large-run");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    const dialog = await screen.findByRole("alertdialog", { name: /copy large snapshot/i });
    expect(dialog).toHaveTextContent(/10,000 ignored files/i);
    expect(dialog).toHaveTextContent(/100 MiB/i);
    expect(createRun).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: /copy and create/i }));

    await waitFor(() => expect(createRun).toHaveBeenCalledOnce());
  });

  it("blocks creation when the ignored-file preview fails", async () => {
    vi.mocked(ignoredFilesPreview).mockRejectedValue({
      code: "io",
      message: "repository is unavailable"
    });

    render(
      <CreateRunModal
        open
        activeRepoPath="/repo/missing"
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await userEvent.type(screen.getByLabelText(/run name/i), "blocked-run");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    expect(await screen.findByText(/could not inspect ignored files: repository is unavailable/i))
      .toBeInTheDocument();
    expect(createRun).not.toHaveBeenCalled();
  });

  it("does not show a stale preview after the repository path changes", async () => {
    let resolveOld!: (preview: {
      fileCount: number;
      totalBytes: number;
      requiresConfirmation: boolean;
    }) => void;
    let resolveNew!: typeof resolveOld;
    vi.mocked(repoSuggestions).mockResolvedValue([]);
    vi.mocked(ignoredFilesPreview).mockImplementation(
      (repoPath) =>
        new Promise((resolve) => {
          if (repoPath === "/repo/old") resolveOld = resolve;
          if (repoPath === "/repo/new") resolveNew = resolve;
        })
    );

    render(
      <CreateRunModal
        open
        activeRepoPath="/repo/old"
        onClose={vi.fn()}
        onCreated={vi.fn()}
        onError={vi.fn()}
      />
    );

    await waitFor(() => expect(ignoredFilesPreview).toHaveBeenCalledWith("/repo/old"));
    const repoPath = screen.getByLabelText(/repo path/i);
    await userEvent.clear(repoPath);
    await userEvent.type(repoPath, "/repo/new");
    await waitFor(() => expect(ignoredFilesPreview).toHaveBeenCalledWith("/repo/new"));

    await act(async () => {
      resolveNew({ fileCount: 2, totalBytes: 2048, requiresConfirmation: false });
    });
    expect(await screen.findByText(/2 ignored files · 2 KiB/i)).toBeInTheDocument();

    await act(async () => {
      resolveOld({ fileCount: 99, totalBytes: 99, requiresConfirmation: false });
    });
    expect(screen.queryByText(/99 ignored files/i)).not.toBeInTheDocument();
    expect(screen.getByText(/2 ignored files · 2 KiB/i)).toBeInTheDocument();
  });
});
