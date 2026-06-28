import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createContext, runInContext } from "node:vm";
import { describe, expect, it, vi } from "vitest";

type PromptChoice = {
  label: string;
  input: string;
};

type PromptModel = {
  mode: "normal" | "choice" | "fallbackKeys";
  choices: PromptChoice[];
};

type MobilePwaHarness = {
  state: {
    terminalId: string | null;
    terminalOutput: string;
    selectedRunId: string | null;
    dashboard: unknown;
    socket: { send: (data: string) => void } | null;
    composerOverridePromptSignature?: string;
  };
  analyzeTerminalPrompt: (text: string) => PromptModel;
  controlPanelTemplate: (prompt: PromptModel) => string;
  sendInstruction: ((event: { preventDefault: () => void; currentTarget: unknown }) => void) | null;
  showComposerForCurrentPrompt: (() => void) | null;
  showPromptControls: (() => void) | null;
  terminalPromptSignature: (text: string) => string;
};

function embeddedAppScript() {
  const source = readFileSync(join(process.cwd(), "src-tauri/src/mobile_pwa.rs"), "utf8");
  const match = source.match(/const APP_JS: &str = r#"(.*?)"#;/s);
  if (!match) throw new Error("APP_JS asset was not found");
  return match[1];
}

function loadMobilePwaHarness() {
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
    WebSocket: function WebSocket() {},
    FormData: class FormData {
      private readonly form: { instructionValue?: string };

      constructor(form: { instructionValue?: string }) {
        this.form = form;
      }

      get(name: string) {
        return name === "instruction" ? this.form.instructionValue || "" : null;
      }
    }
  });

  runInContext(
    `${embeddedAppScript()}
this.__mobilePwaHarness = {
  state,
  analyzeTerminalPrompt,
  controlPanelTemplate,
  terminalPromptSignature,
  sendInstruction: typeof sendInstruction === "function" ? sendInstruction : null,
  showComposerForCurrentPrompt: typeof showComposerForCurrentPrompt === "function" ? showComposerForCurrentPrompt : null,
  showPromptControls: typeof showPromptControls === "function" ? showPromptControls : null
};`,
    context
  );
  return context.__mobilePwaHarness as MobilePwaHarness;
}

function loadPromptAnalyzer() {
  return loadMobilePwaHarness().analyzeTerminalPrompt;
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

  it("can reveal the textbox for the current choice prompt and return to choices", () => {
    const harness = loadMobilePwaHarness();
    harness.state.terminalId = "terminal-1";
    harness.state.terminalOutput = "Choose one:\n1. Alpha\n2. Beta";
    const prompt = harness.analyzeTerminalPrompt(harness.state.terminalOutput);

    expect(harness.controlPanelTemplate(prompt)).toContain('data-action="show-composer"');
    expect(harness.controlPanelTemplate(prompt)).not.toContain("<textarea");

    expect(harness.showComposerForCurrentPrompt).not.toBeNull();
    harness.showComposerForCurrentPrompt?.();

    expect(harness.state.composerOverridePromptSignature).toBe(
      harness.terminalPromptSignature(harness.state.terminalOutput)
    );
    expect(harness.controlPanelTemplate(prompt)).toContain("<textarea");
    expect(harness.controlPanelTemplate(prompt)).toContain('data-action="show-prompt-controls"');
    expect(harness.controlPanelTemplate(prompt)).not.toContain("data-choice-mode");

    expect(harness.showPromptControls).not.toBeNull();
    harness.showPromptControls?.();
    expect(harness.state.composerOverridePromptSignature).toBe("");
    expect(harness.controlPanelTemplate(prompt)).toContain("data-choice-mode");
  });

  it("submits composer instructions with carriage return as terminal Enter", () => {
    const harness = loadMobilePwaHarness();
    const sentMessages: string[] = [];
    const reset = vi.fn();
    harness.state.dashboard = {
      repos: [
        {
          runs: [
            {
              id: "run-1"
            }
          ]
        }
      ]
    };
    harness.state.selectedRunId = "run-1";
    harness.state.terminalId = "terminal-1";
    harness.state.socket = { send: (message) => sentMessages.push(message) };

    expect(harness.sendInstruction).not.toBeNull();
    harness.sendInstruction?.({
      preventDefault: vi.fn(),
      currentTarget: {
        instructionValue: "pwd",
        reset
      }
    });

    expect(sentMessages).toHaveLength(1);
    expect(JSON.parse(sentMessages[0])).toMatchObject({
      type: "terminalInput",
      terminalId: "terminal-1",
      data: "pwd\r"
    });
    expect(reset).toHaveBeenCalledOnce();
  });
});
