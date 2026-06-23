# Session Progress Log

## Current State

**Last Updated:** 2026-06-23 21:53 +07
**Session ID:** fix-tmux-window-initial-redraw
**Active Feature:** feat-021 - Reliable Tmux Initial Redraw

## Status

### What's Done

- [x] Reproduced the frontend race with a red TerminalPane regression.
- [x] Confirmed the root cause: terminal output listeners were registered after `startTerminal`, so tmux attach could emit initial clear/redraw bytes before the frontend was listening.
- [x] Registered terminal output/closed listeners before starting the PTY attach.
- [x] Buffered startup output by terminal id until `startTerminal` returns the id, then flushed only the matching terminal's buffered output.
- [x] Repainted after xterm finishes writing received tmux bytes.
- [x] Updated `feature_list.json` with `feat-021`.

### What's In Progress

- [x] Verification is complete.
- [x] Changes are ready for commit.

### What's Next

1. Next session can run `./init.sh` immediately from this branch.

## Blockers / Risks

- [x] No unresolved blockers.
- [ ] The visual symptom is timing-dependent in WebKitGTK/Tauri; the automated regression covers the identified byte-loss race, while final confidence still benefits from using the packaged app normally.

## Decisions Made

- **Root cause fixed at attach boundary:** Preserve tmux's initial redraw stream instead of adding more app-level visibility repaint hooks.
- **Startup buffer scope:** Buffer only while waiting for the new terminal id, keyed by terminal id, so stale events from other sessions are discarded.

## Files Modified This Session

- `src/components/TerminalPane.tsx` - Registers listeners before starting tmux attach, buffers startup output, and repaints after xterm writes.
- `src/components/TerminalPane.test.tsx` - Adds regression coverage for tmux output emitted before `startTerminal` resolves.
- `feature_list.json` - Adds completed `feat-021` with verification evidence.
- `progress.md` - Records this session state.

## Evidence of Completion

- [x] `npm test -- src/components/TerminalPane.test.tsx` failed red with `expected [] to include 'initial tmux redraw'`.
- [x] `npm test -- src/components/TerminalPane.test.tsx` exited 0 with 9 tests passing after the fix.
- [x] `npm test` exited 0 with 8 files and 34 tests passing.
- [x] `npm run build` exited 0.
- [x] `./init.sh` exited 0 with npm test, npm run build, and cargo test.
