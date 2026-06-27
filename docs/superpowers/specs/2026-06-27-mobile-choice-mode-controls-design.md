# Mobile Choice Mode Controls Design

## Goal

Make the Chrome `/mobile` PWA friendly for Codex and Claude chooser prompts.
When the agent replaces the editor with a list of choices, the mobile UI should
hide the instruction textbox and show large tappable choices instead.

## Problem

The current mobile PWA has a terminal text area plus an instruction composer.
That works for free-form messages, but Codex and Claude often present prompt
screens where the only valid interaction is selecting an option. On a phone,
typing arrow keys or hidden terminal commands is awkward and easy to get wrong.

## Scope

In scope:

- Detect obvious choice prompts from the latest visible terminal text.
- Enter a dedicated Choice Mode when choices are detected.
- Hide the free-form instruction textarea and Send button while Choice Mode is
  active.
- Render detected choices as large tap targets.
- Send the terminal input needed to choose a tapped option.
- Fall back to a compact key bar when no reliable choices are detected.

Out of scope:

- Full terminal keyboard emulation.
- Desktop embedded terminal changes.
- Native Android app changes.
- Parsing every possible terminal UI format.

## User Experience

The terminal remains the primary surface. The bottom control area changes based
on detected state:

- Normal Mode: show the existing instruction textbox and Send button.
- Choice Mode: replace the textbox with option buttons only.
- Fallback Key Mode: show compact controls such as Up, Down, Enter, Esc, and
  Tab when a prompt looks interactive but choices cannot be parsed reliably.

Choice buttons should be large enough for thumb taps and use the visible option
label from the terminal prompt. A Cancel/Esc option should appear when the
prompt text indicates cancellation is available.

## Detection

Choice detection runs on the current mobile terminal snapshot text. It should be
conservative: false negatives are acceptable because fallback keys remain
available, but false positives can send the wrong terminal input.

Supported patterns:

- Numbered choices: `1. Yes`, `2) No`, `[1] Allow`.
- Cursor-highlighted choices: lines prefixed by `❯`, `>`, or similar visible
  selection markers.
- Cancel hints: visible text such as `Esc to cancel` or `press Esc`.

The detector returns a structured prompt model:

- `mode`: `normal`, `choice`, or `fallbackKeys`.
- `choices`: label plus terminal input sequence for each option.
- `fallbackKeys`: key controls to show when choices are not reliable.

## Input Mapping

For numbered choices, tapping a choice sends the number plus Enter, for example
`"1\n"`.

For cursor-highlighted choices, tapping a choice sends the minimal arrow
sequence from the currently highlighted option to the tapped option, then Enter.
If the current index cannot be determined, the UI should not create direct
choice buttons for that prompt and should use fallback keys instead.

Fallback key mappings:

- Up: `\x1b[A`
- Down: `\x1b[B`
- Enter: `\r`
- Esc: `\x1b`
- Tab: `\t`

## Components

`mobile_pwa.rs` remains the owner of the embedded PWA assets.

Suggested internal JS boundaries:

- `analyzeTerminalPrompt(text)` returns the prompt mode and choices.
- `sendTerminalKey(data)` sends raw terminal input for direct choices or key
  buttons.
- `controlPanelTemplate(promptModel)` renders Normal, Choice, or Fallback Key
  controls.

These functions should stay pure where practical so they can be validated with
asset-string regression tests.

## Data Flow

1. Mobile bridge sends terminal snapshots over the existing stream.
2. The PWA updates `state.terminalOutput`.
3. The PWA analyzes the current terminal text.
4. Render chooses the control panel:
   - normal composer for ordinary terminal text,
   - direct option buttons for detected choices,
   - fallback key bar for uncertain interactive prompts.
5. Button taps send `terminalInput` WebSocket messages using the active
   `terminalId`.

No Mobile Bridge protocol change is required.

## Error Handling

- If no terminal is attached, all choice/key controls are disabled.
- If the WebSocket closes, Choice Mode should disappear with the existing
  terminal closed state.
- If parsing is uncertain, use fallback key controls instead of guessing.

## Testing

Add focused PWA asset tests for:

- Choice Mode hides the instruction textarea and Send button.
- Numbered choices render direct buttons and send number plus Enter.
- Cursor-highlighted choices render direct buttons only when the selected row is
  identifiable.
- Fallback keys render when no reliable choices are detected.
- Normal instruction composer remains unchanged for ordinary terminal text.

Run the standard project verification after implementation:

- `cargo test -p agent-manager-desktop --features tauri-app mobile_pwa`
- `npm test`
- `npm run build`
- `cargo check -p agent-manager-desktop --features tauri-app`
- `./init.sh`
