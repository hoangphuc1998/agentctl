import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createContext, runInContext } from "node:vm";
import { describe, expect, it } from "vitest";

type PromptChoice = {
  label: string;
  input: string;
};

type PromptModel = {
  mode: "normal" | "choice" | "fallbackKeys";
  choices: PromptChoice[];
};

function embeddedAppScript() {
  const source = readFileSync(join(process.cwd(), "src-tauri/src/mobile_pwa.rs"), "utf8");
  const match = source.match(/const APP_JS: &str = r#"(.*?)"#;/s);
  if (!match) throw new Error("APP_JS asset was not found");
  return match[1];
}

function loadPromptAnalyzer() {
  const appElement = {
    innerHTML: "",
    querySelector: () => null,
    querySelectorAll: () => []
  };
  const context = createContext({
    document: { getElementById: () => appElement },
    localStorage: {
      getItem: () => null,
      setItem: () => undefined,
      removeItem: () => undefined
    },
    navigator: {},
    location: {
      protocol: "https:",
      host: "example.test",
      origin: "https://example.test"
    },
    WebSocket: function WebSocket() {}
  });

  runInContext(`${embeddedAppScript()}\nthis.__analyzeTerminalPrompt = analyzeTerminalPrompt;`, context);
  return context.__analyzeTerminalPrompt as (text: string) => PromptModel;
}

describe("mobile PWA prompt analyzer", () => {
  it("parses Codex approval options by row structure and shortcut hints", () => {
    const analyzeTerminalPrompt = loadPromptAnalyzer();

    const prompt = analyzeTerminalPrompt(`
Would you like to run the following command?

$ ssh xdep@10.30.22.49 -p 2026 "curl -s http://localhost:64000/api/answer-trace/932320144"

› 1. Yes, proceed (y)
  2. Yes, and don't ask again for commands that start with \`ssh xdep@10.30.22.49 -p 2026\` (p)
  3. No, and tell Codex what to do differently (esc)

Press enter to confirm or esc to cancel
`);

    expect(prompt.mode).toBe("choice");
    expect(prompt.choices).toEqual([
      { label: "Yes, proceed", input: "y" },
      {
        label: "Yes, and don't ask again for commands that start with `ssh xdep@10.30.22.49 -p 2026`",
        input: "p"
      },
      { label: "No, and tell Codex what to do differently", input: "\x1b" }
    ]);
  });

  it("keeps Claude selected numbered prompts as arrow-selectable choices", () => {
    const analyzeTerminalPrompt = loadPromptAnalyzer();

    const prompt = analyzeTerminalPrompt(`
Do you want to make this edit to page.tsx?
❯ 1. Yes
  2. Yes, allow all edits during this session (shift+tab)
  3. No

Esc to cancel · Tab to amend
`);

    expect(prompt.mode).toBe("choice");
    expect(prompt.choices).toEqual([
      { label: "Yes", input: "\r" },
      { label: "Yes, allow all edits during this session (shift+tab)", input: "\x1b[B\r" },
      { label: "No", input: "\x1b[B\x1b[B\r" },
      { label: "Cancel", input: "\x1b" }
    ]);
  });

  it("parses plain numbered and lettered prompts without selected markers", () => {
    const analyzeTerminalPrompt = loadPromptAnalyzer();

    expect(analyzeTerminalPrompt("Choose one:\n1. Alpha\n2. Beta").choices).toEqual([
      { label: "Alpha", input: "1\n" },
      { label: "Beta", input: "2\n" }
    ]);

    expect(analyzeTerminalPrompt("Select a mode:\nA) Fast\nB) Careful").choices).toEqual([
      { label: "Fast", input: "A\n" },
      { label: "Careful", input: "B\n" }
    ]);
  });

  it("falls back to terminal keys for interactive prompts without option rows", () => {
    const analyzeTerminalPrompt = loadPromptAnalyzer();

    expect(analyzeTerminalPrompt("Use arrow keys to select a row, then press Enter").mode).toBe("fallbackKeys");
    expect(analyzeTerminalPrompt("• Read file\n• Run pwd\nWorking...").mode).toBe("normal");
  });
});
