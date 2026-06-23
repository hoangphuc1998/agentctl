# Startup Maximized Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Agent Manager reliably open as a normal maximized desktop window without switching to true fullscreen.

**Architecture:** Keep `src-tauri/tauri.conf.json` declarative `maximized: true` as the intended startup state. Add a small Rust startup-window policy in `src-tauri/src/lib.rs` that runs during Tauri setup and requests `maximize()` followed by `set_focus()` on the main window. Test the policy separately from Tauri OS APIs with a small trait and fake window.

**Tech Stack:** Tauri 2, Rust, Vitest for config guardrail tests, existing `./init.sh` verification.

---

### Task 1: Guard Tauri Startup Window Config

**Files:**
- Modify: `src-tauri/tauriConfig.test.ts`
- Read: `src-tauri/tauri.conf.json`

- [x] **Step 1: Write the failing config test**

Add this test inside `describe("Tauri package configuration", () => { ... })` in `src-tauri/tauriConfig.test.ts`:

```ts
it("starts the main window as a normal maximized window", () => {
  const config = JSON.parse(readText("src-tauri/tauri.conf.json"));
  const [mainWindow] = config.app.windows;

  expect(mainWindow.maximized).toBe(true);
  expect(mainWindow.fullscreen ?? false).toBe(false);
});
```

- [x] **Step 2: Run the config test**

Run: `npm test -- src-tauri/tauriConfig.test.ts`

Expected: PASS, because the config already has `maximized: true` and no `fullscreen` property.

### Task 2: Add Startup Maximize Policy

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write Rust policy tests before production behavior**

Add a private startup-window policy module to `src-tauri/src/lib.rs` with tests first. Keep the pure policy module available for tests and Tauri app builds so it can be tested without enabling the Tauri app feature while staying warning-free in non-app library builds. The tests should use this fake window and assert the order `maximize`, then `set_focus`, plus non-fatal error collection:

```rust
mod startup_window {
    pub trait StartupWindow {
        fn maximize(&self) -> Result<(), String>;
        fn set_focus(&self) -> Result<(), String>;
    }

    pub fn enforce_startup_maximized_window<W: StartupWindow>(window: &W) -> Vec<String> {
        let mut errors = Vec::new();
        if let Err(err) = window.maximize() {
            errors.push(format!("failed to maximize startup window: {err}"));
        }
        if let Err(err) = window.set_focus() {
            errors.push(format!("failed to focus startup window: {err}"));
        }
        errors
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::RefCell;

        #[derive(Default)]
        struct FakeWindow {
            calls: RefCell<Vec<&'static str>>,
            maximize_error: Option<String>,
            focus_error: Option<String>,
        }

        impl StartupWindow for FakeWindow {
            fn maximize(&self) -> Result<(), String> {
                self.calls.borrow_mut().push("maximize");
                self.maximize_error.clone().map_or(Ok(()), Err)
            }

            fn set_focus(&self) -> Result<(), String> {
                self.calls.borrow_mut().push("set_focus");
                self.focus_error.clone().map_or(Ok(()), Err)
            }
        }

        #[test]
        fn startup_window_requests_maximize_before_focus() {
            let window = FakeWindow::default();

            let errors = enforce_startup_maximized_window(&window);

            assert!(errors.is_empty());
            assert_eq!(*window.calls.borrow(), vec!["maximize", "set_focus"]);
        }

        #[test]
        fn startup_window_errors_are_collected_without_stopping_later_requests() {
            let window = FakeWindow {
                maximize_error: Some("wm refused".to_string()),
                focus_error: Some("focus denied".to_string()),
                ..FakeWindow::default()
            };

            let errors = enforce_startup_maximized_window(&window);

            assert_eq!(*window.calls.borrow(), vec!["maximize", "set_focus"]);
            assert_eq!(
                errors,
                vec![
                    "failed to maximize startup window: wm refused",
                    "failed to focus startup window: focus denied"
                ]
            );
        }
    }
}
```

- [x] **Step 2: Run Rust tests and verify the policy compiles**

Run: `cargo test -p agent-manager-desktop startup_window`

Expected: PASS for the new tests after the module compiles.

- [x] **Step 3: Connect policy to Tauri setup**

Still in `src-tauri/src/lib.rs`, add an implementation of `StartupWindow` for `tauri::WebviewWindow`, plus a public helper that can be called from `.setup(...)`:

```rust
#[cfg(feature = "tauri-app")]
impl startup_window::StartupWindow for tauri::WebviewWindow {
    fn maximize(&self) -> Result<(), String> {
        self.maximize().map_err(|err| err.to_string())
    }

    fn set_focus(&self) -> Result<(), String> {
        self.set_focus().map_err(|err| err.to_string())
    }
}

#[cfg(feature = "tauri-app")]
fn enforce_startup_window_state(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        eprintln!("failed to maximize startup window: main window was not found");
        return;
    };

    for error in startup_window::enforce_startup_maximized_window(&window) {
        eprintln!("{error}");
    }
}
```

Update the Tauri builder:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_notification::init())
    .setup(|app| {
        enforce_startup_window_state(app);
        Ok(())
    })
    .manage(state::DesktopState::new())
```

- [x] **Step 4: Run targeted Rust verification**

Run: `cargo test -p agent-manager-desktop startup_window`

Expected: PASS with the policy tests.

### Task 3: Verify, Record, Commit

**Files:**
- Modify: `feature_list.json`
- Modify: `progress.md`

- [x] **Step 1: Run full verification**

Run: `npm test`

Expected: all Vitest files pass.

Run: `npm run build`

Expected: TypeScript and Vite build pass.

Run: `cargo test -p agent-manager-desktop startup_window`

Expected: startup-window tests pass.

Run: `./init.sh`

Expected: npm test, npm build, and cargo test pass.

Run: `git diff --check`

Expected: no output and exit 0.

- [x] **Step 2: Update required artifacts**

Add `feat-022` to `feature_list.json`:

```json
{
  "id": "feat-022",
  "name": "Reliable Startup Maximized Window",
  "description": "Request normal maximized window state again during Tauri startup so the app opens maximized without requiring a manual minimize/maximize cycle.",
  "dependencies": ["feat-021"],
  "status": "completed",
  "evidence": "2026-06-23: Added a Tauri setup startup-window maximize/focus policy with Rust coverage and config guardrails. npm test, npm run build, cargo test -p agent-manager-desktop startup_window, ./init.sh, and git diff --check exited 0."
}
```

Update `progress.md` with the current session state, files changed, and verification evidence.

- [x] **Step 3: Commit implementation**

Run:

```bash
git add src-tauri/src/lib.rs src-tauri/tauriConfig.test.ts feature_list.json progress.md docs/superpowers/plans/2026-06-23-startup-maximized-window.md
git commit -m "Fix startup maximized window state"
```

Expected: commit succeeds and `git status --short` is clean.
