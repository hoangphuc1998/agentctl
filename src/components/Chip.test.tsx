import { render, screen } from "@testing-library/react";
import { GitBranch } from "lucide-react";
import { describe, expect, it } from "vitest";
import { Chip } from "./Chip";

describe("Chip", () => {
  it("renders compact metadata with tone, icon, and title", () => {
    render(
      <Chip tone="success" icon={<GitBranch size={14} aria-hidden="true" />} title="Branch">
        main -&gt; feature
      </Chip>
    );

    const chip = screen.getByText("main -> feature").closest(".chip");

    expect(chip).toHaveClass("chip", "chip-success");
    expect(chip).toHaveAttribute("title", "Branch");
    expect(chip?.querySelector("svg")).toBeInTheDocument();
  });
});
