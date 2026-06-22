# Session Progress Log

## Current State

**Last Updated:** 2026-06-22 19:06 +07
**Session ID:** fix-create-run-agent-exit-recovery
**Active Feature:** feat-006 - Create Run Agent Exit Recovery

## Status

### What's Done

- [x] Traced the create-run failure to the tmux launch check treating an immediately exited agent command as a missing tmux window.
- [x] Added a shell failure wrapper for agent create/restore launches so non-zero exits keep the pane open with diagnostics and a recovery shell.
- [x] Added a core regression test covering the wrapped create-run launch command.
- [x] Verified the wrapper manually in a temporary tmux session with a failing command.
- [x] Ran targeted and standard verification.

### What's In Progress

- [x] No active implementation work remains for this feature.

### What's Next

1. Commit the verified changes with a descriptive message.
2. Next session can run `./init.sh` immediately from this worktree.

## Blockers / Risks

- [x] No blockers remain.
- [ ] Dependency note: `./init.sh` skipped npm checks because `node_modules` is absent in this worktree. This session changed Rust core code only; Rust verification passed.

## Decisions Made

- **Keep failed agent panes inspectable:** A create/restore launch now runs the agent command through a small shell wrapper. If the agent exits with a non-zero status, the pane prints the exit status and execs the user's shell so the tmux window remains available.
  - Context: The previous flow rolled back the run when the agent command exited before the window verification check could observe it.
  - Detail: The wrapper uses `agent_status` instead of `status` because `status` is a read-only special parameter in `zsh`, which tmux uses on this machine.

## Files Modified This Session

- `core/src/commands.rs` - Adds the shell failure wrapper used for agent launch commands.
- `core/src/app.rs` - Uses the wrapper for create and restore tmux launches and adds a regression test.
- `feature_list.json` - Records completed feature state and verification evidence.
- `progress.md` - Records this session handoff.

## Evidence of Completion

- [x] Regression red: `cargo test -p agentctl-core create_run_wraps_agent_launch_so_failures_keep_the_pane_open` failed before implementation because create-run passed bare `codex`.
- [x] Wrapper portability red: the same test failed after tightening it to require `agent_status`, avoiding zsh's read-only `status` parameter.
- [x] Targeted test: `cargo test -p agentctl-core create_run_wraps_agent_launch_so_failures_keep_the_pane_open` passed.
- [x] Core tests: `cargo test -p agentctl-core` passed with 10 tests.
- [x] Formatting: `cargo fmt --check` exited 0.
- [x] Manual smoke: temporary tmux window running `false` stayed open, displayed `Agent command exited with status 1...`, and execed `zsh`.
- [x] Standard verification: `./init.sh` exited 0; npm checks were skipped because `node_modules` is absent, and all Rust tests passed.

## Notes for Next Session

The create-run failure shown as `tmux window was not created or exited immediately: ...` should now be avoided for non-zero agent startup exits: the run can be recorded, and the tmux pane remains open with the agent exit status and a shell for inspection.
