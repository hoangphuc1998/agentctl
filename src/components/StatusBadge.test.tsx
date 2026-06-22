import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  it("renders an accessible icon badge with visible text by default", () => {
    render(<StatusBadge state="completed-unchecked" />);

    const badge = screen.getByLabelText("Complete");

    expect(badge).toHaveClass("status-badge", "completed-unchecked");
    expect(screen.getByText("Complete")).toBeVisible();
    expect(badge.querySelector("svg")).toBeInTheDocument();
  });

  it("keeps compact badges accessible without rendering visible label text", () => {
    render(<StatusBadge state="needs-user" compact />);

    const badge = screen.getByLabelText("Needs user");

    expect(badge).toHaveClass("status-badge", "needs-user");
    expect(screen.queryByText("Needs user")).not.toBeInTheDocument();
    expect(badge.querySelector("svg")).toBeInTheDocument();
  });
});
