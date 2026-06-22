import { describe, expect, it } from "vitest";
import css from "./styles.css?raw";

describe("app shell layout CSS", () => {
  it("lets the workspace consume remaining fullscreen height", () => {
    expect(css).toContain(".app-shell {");
    expect(css).toContain("min-height: 100vh;");
    expect(css).toContain("display: flex;");
    expect(css).toContain(".workspace {\n  flex: 1 1 auto;");
  });

  it("keeps page chrome fixed while the terminal pane owns overflow", () => {
    expect(css).toContain("html,\nbody,\n#root {\n  height: 100%;\n  overflow: hidden;");
    expect(css).toContain("height: 100vh;");
    expect(css).toContain(".terminal-host {\n  min-width: 0;\n  min-height: 0;\n  overflow: hidden;");
  });

  it("uses compact dashboard chrome and dark segmented modal controls", () => {
    expect(css).toContain("grid-template-columns: 320px minmax(0, 1fr);");
    expect(css).toContain("min-height: 68px;");
    expect(css).toContain(".agent-segmented-control {");
    expect(css).toContain(".agent-segment.active {");
  });
});
