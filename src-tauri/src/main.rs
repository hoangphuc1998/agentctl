const TMUX_RESURRECT_REWRITE_ARG: &str = "__tmux-resurrect-rewrite";
const TMUX_SESSION: &str = "agentctl";

fn main() {
    if std::env::args().nth(1).as_deref() == Some(TMUX_RESURRECT_REWRITE_ARG) {
        if let Err(err) =
            agent_manager_desktop::tmux_restore::rewrite_saved_resurrect_state_from_environment(
                TMUX_SESSION,
            )
        {
            eprintln!("failed to rewrite tmux-resurrect state: {err}");
            std::process::exit(1);
        }
        return;
    }

    run_desktop_app();
}

#[cfg(feature = "tauri-app")]
fn run_desktop_app() {
    agent_manager_desktop::run();
}

#[cfg(not(feature = "tauri-app"))]
fn run_desktop_app() {
    eprintln!("Build with --features tauri-app to run the Agent Manager desktop app.");
}
