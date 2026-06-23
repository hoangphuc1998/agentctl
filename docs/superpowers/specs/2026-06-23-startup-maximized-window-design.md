# Startup Maximized Window Design

## Goal

Agent Manager should open as a normal maximized desktop window on launch. It should not enter true OS fullscreen, hide window chrome, or require the user to minimize and maximize the window manually before the app occupies the available screen area.

## Root Cause Hypothesis

The current Tauri window config already sets `maximized: true`, but on the affected Linux desktop the first window-manager maximize state is not reliably applied at startup. The user's manual minimize/maximize cycle fixes the state, which indicates a later native maximize request works after the window exists.

## Architecture

Keep the declarative Tauri config with `maximized: true` so packaged and development windows still express the intended startup state. Add a small Rust startup-window policy that runs during Tauri setup, gets the main window, calls `maximize()`, and then calls `set_focus()`.

This keeps native window lifecycle behavior in the Tauri backend instead of React. The frontend layout remains unchanged because it already uses viewport-height flex/grid rules and is not the source of the maximize state.

## Error Handling

If the startup maximize or focus request fails, the app should continue launching. The backend should log the failure to stderr instead of surfacing an in-app error, because this is a best-effort startup-window correction and should not block app use.

## Scope

This feature does not switch to `fullscreen: true`, does not remove normal window decorations, and does not add user preferences for launch mode. It only makes the existing maximized startup intent reliable.

## Testing

Add focused Rust coverage for the startup-window policy so the intended order and non-fatal error handling are explicit. Extend the package-config test to keep `maximized: true` and `fullscreen` unset or false.
