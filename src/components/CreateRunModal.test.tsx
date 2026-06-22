import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createRun } from "../api";
import { CreateRunModal } from "./CreateRunModal";

vi.mock("../api", () => ({
  createRun: vi.fn()
}));

describe("CreateRunModal", () => {
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
