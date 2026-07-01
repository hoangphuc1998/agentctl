pub mod mobile_bridge;
pub mod models;
pub mod run_classification;
pub mod services;
pub mod terminal_plan;
pub mod tmux_restore;

#[cfg(any(test, feature = "tauri-app"))]
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

#[cfg(feature = "tauri-app")]
pub mod commands;
#[cfg(feature = "tauri-app")]
pub mod error;
#[cfg(feature = "tauri-app")]
pub mod mobile_bridge_server;
#[cfg(feature = "tauri-app")]
pub mod mobile_pwa;
#[cfg(feature = "tauri-app")]
pub mod state;
#[cfg(feature = "tauri-app")]
pub mod terminal;

#[cfg(feature = "tauri-app")]
use tauri::Manager;

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

#[cfg(feature = "tauri-app")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            enforce_startup_window_state(app);
            Ok(())
        })
        .manage(state::DesktopState::new())
        .invoke_handler(tauri::generate_handler![
            commands::dashboard_state,
            commands::create_run,
            commands::restore_run,
            commands::stop_run,
            commands::end_run,
            commands::merge_run,
            commands::open_in_vscode,
            commands::run_diff,
            commands::cleanup_stale_runs,
            commands::tmux_restore_status,
            commands::enable_tmux_restore,
            commands::repo_suggestions,
            commands::base_ref_suggestions,
            commands::mobile_bridge_status,
            commands::issue_mobile_pairing_code,
            commands::revoke_mobile_device,
            commands::start_mobile_bridge,
            commands::stop_mobile_bridge,
            commands::start_terminal,
            commands::terminal_input,
            commands::resize_terminal,
            commands::close_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Manager");
}
