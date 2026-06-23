pub mod models;
pub mod run_classification;
pub mod services;
pub mod terminal_plan;
pub mod tmux_restore;

#[cfg(feature = "tauri-app")]
pub mod commands;
#[cfg(feature = "tauri-app")]
pub mod error;
#[cfg(feature = "tauri-app")]
pub mod state;
#[cfg(feature = "tauri-app")]
pub mod terminal;

#[cfg(feature = "tauri-app")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::DesktopState::new())
        .invoke_handler(tauri::generate_handler![
            commands::dashboard_state,
            commands::create_run,
            commands::restore_run,
            commands::stop_run,
            commands::end_run,
            commands::merge_run,
            commands::open_in_vscode,
            commands::cleanup_stale_runs,
            commands::tmux_restore_status,
            commands::enable_tmux_restore,
            commands::repo_suggestions,
            commands::base_ref_suggestions,
            commands::start_terminal,
            commands::terminal_input,
            commands::resize_terminal,
            commands::close_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Manager");
}
