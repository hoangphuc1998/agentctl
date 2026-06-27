# Mobile Choice Mode Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `/mobile` PWA replace its free-form instruction composer with tappable choice/key controls when Codex or Claude is showing an interactive chooser.

**Architecture:** Keep the Mobile Bridge protocol unchanged and implement the feature entirely inside the embedded PWA assets owned by `src-tauri/src/mobile_pwa.rs`. Add pure JavaScript prompt-analysis helpers, reuse the existing `terminalInput` WebSocket message, and render a bottom control panel that switches between normal composer, direct choices, and fallback terminal keys.

**Tech Stack:** Rust asset tests, embedded JavaScript/CSS strings, Tauri/Axum Mobile Bridge assets.

---

### Task 1: Add Mobile PWA Choice Mode Regression Tests

**Files:**
- Modify: `src-tauri/src/mobile_pwa.rs`

- [ ] **Step 1: Add RED asset tests**

Append these focused tests inside `mod tests` in `src-tauri/src/mobile_pwa.rs` after `mobile_ui_surfaces_stream_status_and_operator_controls`:

```rust
    #[test]
    fn mobile_script_detects_choice_prompts_and_replaces_composer() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("function analyzeTerminalPrompt(text)"));
        assert!(script.body.contains("function controlPanelTemplate(prompt)"));
        assert!(script.body.contains("function choiceModeTemplate(prompt)"));
        assert!(script.body.contains("return choiceModeTemplate(prompt);"));
        assert!(script.body.contains(r#"data-terminal-choice-input"#));
    }

    #[test]
    fn mobile_script_maps_numbered_and_cursor_choices_to_terminal_input() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("function numberedChoicesFromLines(lines)"));
        assert!(script.body.contains("function cursorChoicesFromLines(lines)"));
        assert!(script.body.contains("input: `${number}\\n`"));
        assert!(script.body.contains("repeatKey(\"\\x1b[B\", index - selectedIndex)"));
        assert!(script.body.contains("repeatKey(\"\\x1b[A\", selectedIndex - index)"));
        assert!(script.body.contains("+ \"\\r\""));
    }

    #[test]
    fn mobile_script_renders_fallback_terminal_keys_for_uncertain_prompts() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");
        let styles = asset_for_path("/mobile/styles.css").expect("mobile styles should be served");

        assert!(script.body.contains("function fallbackKeyModeTemplate()"));
        assert!(script.body.contains("data-terminal-key"));
        assert!(script.body.contains(r#"{ label: "Up", data: "\x1b[A" }"#));
        assert!(script.body.contains(r#"{ label: "Down", data: "\x1b[B" }"#));
        assert!(script.body.contains(r#"{ label: "Enter", data: "\r" }"#));
        assert!(script.body.contains(r#"{ label: "Esc", data: "\x1b" }"#));
        assert!(script.body.contains(r#"{ label: "Tab", data: "\t" }"#));
        assert!(styles.body.contains(".choice-panel"));
        assert!(styles.body.contains(".key-button"));
    }

    #[test]
    fn mobile_script_keeps_normal_instruction_composer_for_regular_terminal_text() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("function normalComposerTemplate()"));
        assert!(script.body.contains(r#"<textarea name="instruction""#));
        assert!(script.body.contains("sendTerminalInput(`${text}\\n`)"));
    }

    #[test]
    fn mobile_service_worker_cache_is_bumped_for_choice_mode_assets() {
        let service_worker = asset_for_path("/mobile/sw.js").expect("mobile service worker should be served");

        assert!(service_worker
            .body
            .contains(r#"const CACHE_NAME = "agent-manager-mobile-v2";"#));
    }
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p agent-manager-desktop --features tauri-app mobile_pwa
```

Expected: fails because the new choice-mode functions, CSS selectors, and cache version do not exist yet.

### Task 2: Implement Prompt Analysis and Choice Input

**Files:**
- Modify: `src-tauri/src/mobile_pwa.rs`

- [ ] **Step 1: Add shared terminal input helper**

Replace direct `state.socket.send(JSON.stringify({ type: "terminalInput", ... }))` in `sendInstruction` with this helper and call:

```javascript
function sendTerminalInput(data) {
  if (!selectedRun() || !state.socket || !state.terminalId || !data) return;
  state.socket.send(JSON.stringify({ type: "terminalInput", terminalId: state.terminalId, data }));
}

function sendInstruction(event) {
  event.preventDefault();
  const form = new FormData(event.currentTarget);
  const text = String(form.get("instruction") || "").trimEnd();
  if (!text) return;
  sendTerminalInput(`${text}\n`);
  event.currentTarget.reset();
}
```

- [ ] **Step 2: Add prompt analyzer helpers**

Insert these functions before `selectedRunTemplate(run)`:

```javascript
function analyzeTerminalPrompt(text) {
  const lines = recentNonEmptyLines(text);
  const numberedChoices = numberedChoicesFromLines(lines);
  if (numberedChoices.length >= 2) {
    return { mode: "choice", choices: withCancelChoice(numberedChoices, text) };
  }
  const cursorChoices = cursorChoicesFromLines(lines);
  if (cursorChoices.length >= 2) {
    return { mode: "choice", choices: withCancelChoice(cursorChoices, text) };
  }
  if (looksInteractivePrompt(text)) {
    return { mode: "fallbackKeys", choices: [] };
  }
  return { mode: "normal", choices: [] };
}

function recentNonEmptyLines(text) {
  return String(text || "")
    .split(/\r?\n/)
    .map((line) => line.replace(/\s+$/, ""))
    .filter((line) => line.trim())
    .slice(-28);
}

function numberedChoicesFromLines(lines) {
  const choices = [];
  for (const line of lines) {
    const match = line.match(/^\s*(?:\[(\d{1,2})\]|(\d{1,2})[.)])\s+(.+?)\s*$/);
    if (!match) continue;
    const number = match[1] || match[2];
    const label = match[3].trim();
    if (!label) continue;
    choices.push({ label, input: `${number}\n` });
  }
  return choices;
}

function cursorChoicesFromLines(lines) {
  const selectedIndex = lines.findIndex((line) => /^\s*(?:❯|>)\s+\S/.test(line));
  if (selectedIndex < 0) return [];
  const start = cursorChoiceBlockStart(lines, selectedIndex);
  const end = cursorChoiceBlockEnd(lines, selectedIndex);
  const rows = lines.slice(start, end + 1)
    .map((line) => cursorChoiceLabel(line))
    .filter(Boolean);
  const relativeSelectedIndex = selectedIndex - start;
  if (relativeSelectedIndex < 0 || relativeSelectedIndex >= rows.length || rows.length < 2) return [];
  return rows.map((label, index) => ({
    label,
    input: cursorChoiceInput(relativeSelectedIndex, index)
  }));
}

function cursorChoiceBlockStart(lines, selectedIndex) {
  let index = selectedIndex;
  while (index > 0 && cursorChoiceLabel(lines[index - 1])) index -= 1;
  return index;
}

function cursorChoiceBlockEnd(lines, selectedIndex) {
  let index = selectedIndex;
  while (index + 1 < lines.length && cursorChoiceLabel(lines[index + 1])) index += 1;
  return index;
}

function cursorChoiceLabel(line) {
  const selected = line.match(/^\s*(?:❯|>)\s+(.+?)\s*$/);
  if (selected) return selected[1].trim();
  const plain = line.match(/^\s{2,}([^\s].+?)\s*$/);
  return plain ? plain[1].trim() : "";
}

function cursorChoiceInput(selectedIndex, index) {
  if (index > selectedIndex) return repeatKey("\x1b[B", index - selectedIndex) + "\r";
  if (index < selectedIndex) return repeatKey("\x1b[A", selectedIndex - index) + "\r";
  return "\r";
}

function repeatKey(key, count) {
  return Array.from({ length: count }, () => key).join("");
}

function withCancelChoice(choices, text) {
  if (!/\besc\b|\bcancel\b/i.test(text)) return choices;
  return [...choices, { label: "Cancel", input: "\x1b" }];
}

function looksInteractivePrompt(text) {
  return /\b(choose|select|approve|confirm|press enter|press esc|esc to cancel|use (the )?(arrow|up|down)|\[y\/n\]|\(y\/n\))\b/i.test(text);
}
```

- [ ] **Step 3: Render the mode-specific control panel**

Replace the hardcoded composer form in `selectedRunTemplate(run)` with a prompt-aware control panel:

```javascript
function selectedRunTemplate(run) {
  const prompt = analyzeTerminalPrompt(state.terminalOutput);
  return `
    ...
    <pre class="terminal" data-terminal-output aria-label="Terminal output">${escapeHtml(terminalText())}</pre>
    ${controlPanelTemplate(prompt)}
  `;
}

function controlPanelTemplate(prompt) {
  if (prompt.mode === "choice") {
    return choiceModeTemplate(prompt);
  }
  if (prompt.mode === "fallbackKeys") {
    return fallbackKeyModeTemplate();
  }
  return normalComposerTemplate();
}
```

- [ ] **Step 4: Add concrete panel templates**

Insert these template functions after `statusPillTemplate()`:

```javascript
function choiceModeTemplate(prompt) {
  const disabled = state.terminalId ? "" : "disabled";
  return `
    <div class="choice-panel" data-choice-mode aria-label="Terminal choices">
      <div class="choice-list">
        ${prompt.choices.map((choice) => `
          <button class="choice-button" data-terminal-choice-input="${escapeHtml(choice.input)}" ${disabled}>
            ${escapeHtml(choice.label)}
          </button>
        `).join("")}
      </div>
    </div>
  `;
}

function fallbackKeyModeTemplate() {
  const keys = [
    { label: "Up", data: "\x1b[A" },
    { label: "Down", data: "\x1b[B" },
    { label: "Enter", data: "\r" },
    { label: "Esc", data: "\x1b" },
    { label: "Tab", data: "\t" }
  ];
  const disabled = state.terminalId ? "" : "disabled";
  return `
    <div class="choice-panel key-panel" aria-label="Terminal keys">
      <div class="key-bar">
        ${keys.map((key) => `
          <button class="key-button" data-terminal-key="${escapeHtml(key.data)}" ${disabled}>
            ${escapeHtml(key.label)}
          </button>
        `).join("")}
      </div>
    </div>
  `;
}

function normalComposerTemplate() {
  return `
    <form class="composer-bar" data-form="instruction">
      <label>
        <span class="muted">Instruction</span>
        <textarea name="instruction" placeholder="Send instructions to the selected agent"></textarea>
      </label>
      <button data-send-instruction ${state.terminalId ? "" : "disabled"}>Send</button>
    </form>
  `;
}
```

- [ ] **Step 5: Bind choice/key buttons**

Add these click bindings in `bindEvents()` after the instruction form listener:

```javascript
  app.querySelectorAll("[data-terminal-choice-input]").forEach((button) => {
    button.addEventListener("click", () => sendTerminalInput(button.getAttribute("data-terminal-choice-input")));
  });
  app.querySelectorAll("[data-terminal-key]").forEach((button) => {
    button.addEventListener("click", () => sendTerminalInput(button.getAttribute("data-terminal-key")));
  });
```

- [ ] **Step 6: Run tests to verify GREEN for JS behavior**

Run:

```bash
cargo test -p agent-manager-desktop --features tauri-app mobile_pwa
```

Expected: tests for JavaScript choice behavior pass except any CSS/cache assertions still waiting on Task 3.

### Task 3: Style Choice Controls and Bust PWA Cache

**Files:**
- Modify: `src-tauri/src/mobile_pwa.rs`

- [ ] **Step 1: Add bottom choice/key control styles**

Insert these CSS rules after `.composer-bar textarea`:

```css
.choice-panel {
  border-top: 1px solid var(--line);
  background: rgba(234, 240, 237, 0.98);
  padding: 10px;
}

.choice-list {
  display: grid;
  gap: 8px;
}

.choice-button,
.key-button {
  min-height: 46px;
  border: 1px solid rgba(15, 118, 110, 0.28);
  background: #f7fff9;
  color: var(--ink);
  font-weight: 700;
}

.choice-button {
  justify-content: start;
  text-align: left;
  padding: 10px 12px;
}

.key-bar {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 7px;
}

.key-button {
  padding: 0 8px;
}
```

- [ ] **Step 2: Bump service worker cache version**

Change:

```javascript
const CACHE_NAME = "agent-manager-mobile-v1";
```

to:

```javascript
const CACHE_NAME = "agent-manager-mobile-v2";
```

- [ ] **Step 3: Run tests to verify GREEN for the full PWA asset suite**

Run:

```bash
cargo test -p agent-manager-desktop --features tauri-app mobile_pwa
```

Expected: all `mobile_pwa` tests pass.

### Task 4: Record Evidence and Verify Repository

**Files:**
- Modify: `feature_list.json`
- Modify: `progress.md`

- [ ] **Step 1: Add feature tracker entry**

Append a completed feature entry with ID `feat-042`, name `Mobile PWA Choice Mode Controls`, dependency `feat-041`, and evidence listing the RED test, focused `mobile_pwa` test, npm/build/Rust verification, and final `./init.sh`.

- [ ] **Step 2: Update progress log**

Replace the current state in `progress.md` with this session, marking the active feature as `feat-042 - Mobile PWA Choice Mode Controls`, listing implementation steps, risks, decisions, modified files, and verification evidence.

- [ ] **Step 3: Run final verification**

Run:

```bash
cargo fmt --check
cargo test -p agent-manager-desktop --features tauri-app mobile_pwa
npm test
npm run build
cargo check -p agent-manager-desktop --features tauri-app
./init.sh
```

Expected: all commands exit 0.

- [ ] **Step 4: Commit the safe state**

Run:

```bash
git add src-tauri/src/mobile_pwa.rs docs/superpowers/plans/2026-06-27-mobile-choice-mode-controls.md feature_list.json progress.md
git commit -m "feat: add mobile choice mode controls"
```

Expected: commit succeeds and `git status --short` is clean.

## Self-Review

- Spec coverage: the plan covers conservative prompt detection, Choice Mode, composer hiding, direct input mapping, fallback keys, no bridge protocol change, cache refresh, tests, tracker updates, and final verification.
- Placeholder scan: the plan contains no unfinished placeholders.
- Type consistency: prompt model fields are `mode` and `choices`; terminal input attributes are `data-terminal-choice-input` and `data-terminal-key`; key mappings match the design spec.
