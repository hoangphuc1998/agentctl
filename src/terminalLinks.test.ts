import { describe, expect, it, vi } from "vitest";
import {
  createTerminalLinkProvider,
  detectTerminalLinks,
  shouldOpenTerminalLink,
  terminalLinkActivationFromUri,
  terminalLinkTargetFromUri
} from "./terminalLinks";

describe("detectTerminalLinks", () => {
  it("detects web URLs without surrounding terminal punctuation", () => {
    expect(detectTerminalLinks("Docs: (https://example.com/guide?q=tmux)."))
      .toEqual([
        expect.objectContaining({
          text: "https://example.com/guide?q=tmux",
          target: { kind: "url", url: "https://example.com/guide?q=tmux" }
        })
      ]);
  });

  it("detects worktree files with optional line and column locations", () => {
    expect(
      detectTerminalLinks("Open src/components/TerminalPane.tsx:42:7 or ./README.md:12")
        .map(({ text, target }) => ({ text, target }))
    ).toEqual([
      {
        text: "src/components/TerminalPane.tsx:42:7",
        target: {
          kind: "file",
          path: "src/components/TerminalPane.tsx",
          line: 42,
          column: 7
        }
      },
      {
        text: "./README.md:12",
        target: { kind: "file", path: "./README.md", line: 12 }
      }
    ]);
  });

  it("does not reinterpret URL fragments or version numbers as files", () => {
    const links = detectTerminalLinks("See https://example.com/app.js#L12 with xterm 5.5");

    expect(links).toHaveLength(1);
    expect(links[0].target).toEqual({ kind: "url", url: "https://example.com/app.js#L12" });
  });
});

describe("terminalLinkTargetFromUri", () => {
  it("accepts HTTP links and file URLs with locations", () => {
    expect(terminalLinkTargetFromUri("https://example.com/docs")).toEqual({
      kind: "url",
      url: "https://example.com/docs"
    });
    expect(terminalLinkTargetFromUri("file:///repo/src/main.rs#L18:4")).toEqual({
      kind: "file",
      path: "/repo/src/main.rs",
      line: 18,
      column: 4
    });
    expect(terminalLinkTargetFromUri("file:///repo/src/main.rs:21:6")).toEqual({
      kind: "file",
      path: "/repo/src/main.rs",
      line: 21,
      column: 6
    });
    expect(terminalLinkActivationFromUri("file:///repo/render.png")).toBe("direct");
    expect(terminalLinkActivationFromUri("https://example.com/docs")).toBe("modifier");
  });

  it("rejects unsupported hyperlink protocols", () => {
    expect(terminalLinkTargetFromUri("javascript:alert(1)")).toBeNull();
  });
});

describe("createTerminalLinkProvider", () => {
  it("maps detected text columns to xterm's one-based buffer range", () => {
    const activate = vi.fn();
    const provider = createTerminalLinkProvider(
      {
        buffer: {
          active: {
            getLine: () => ({
              translateToString: () => "Edit src/main.ts:9"
            })
          }
        }
      },
      { activate }
    );
    const callback = vi.fn();

    provider.provideLinks(3, callback);
    const [links] = callback.mock.calls[0];

    expect(links).toHaveLength(1);
    expect(links[0].range).toEqual({
      start: { x: 6, y: 3 },
      end: { x: 18, y: 3 }
    });
    const event = new MouseEvent("click", { button: 0, ctrlKey: true });
    links[0].activate(event, links[0].text);
    expect(activate).toHaveBeenCalledWith(
      {
        kind: "file",
        path: "src/main.ts",
        line: 9
      },
      event,
      "modifier"
    );
  });
});

describe("shouldOpenTerminalLink", () => {
  it("opens explicit file links directly and keeps other links behind Ctrl+primary-click", () => {
    expect(
      shouldOpenTerminalLink(
        new MouseEvent("click", { button: 0 }),
        "direct"
      )
    ).toBe(true);
    expect(shouldOpenTerminalLink(new MouseEvent("click", { button: 0 }), "modifier")).toBe(false);
    expect(
      shouldOpenTerminalLink(
        new MouseEvent("click", { button: 0, ctrlKey: true }),
        "modifier"
      )
    ).toBe(true);
    expect(shouldOpenTerminalLink(new MouseEvent("click", { button: 0 }))).toBe(false);
    expect(
      shouldOpenTerminalLink(
        new MouseEvent("click", { button: 1, ctrlKey: true }),
        "modifier"
      )
    ).toBe(false);
  });
});
