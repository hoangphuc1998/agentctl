#[cfg(feature = "tauri-app")]
fn main() {
    agent_manager_desktop::run();
}

#[cfg(not(feature = "tauri-app"))]
fn main() {
    eprintln!("Build with --features tauri-app to run the Agent Manager desktop app.");
}
