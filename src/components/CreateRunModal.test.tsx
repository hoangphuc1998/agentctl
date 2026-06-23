import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createRun } from "../api";
import { CreateRunModal } from "./CreateRunModal";

vi.mock("../api", () => ({
  createRun: vi.fn()
}));

describe("CreateRunModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});
