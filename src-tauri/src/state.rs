use std::{
    env,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use crate::{error::DesktopError, terminal::TerminalManager};

pub struct DesktopState {
    registry_path: PathBuf,
    selected_run_id: Mutex<Option<String>>,
    terminals: Mutex<TerminalManager>,
}

impl DesktopState {
    pub fn new() -> Self {
        Self {
            registry_path: registry_path(),
            selected_run_id: Mutex::new(None),
            terminals: Mutex::new(TerminalManager::default()),
        }
    }

    pub fn registry_path(&self) -> &PathBuf {
        &self.registry_path
    }

    pub fn selected_run_id(&self) -> Result<Option<String>, DesktopError> {
        Ok(self.selected_guard()?.clone())
    }

    pub fn set_selected_run_id(&self, id: Option<String>) -> Result<(), DesktopError> {
        *self.selected_guard()? = id;
        Ok(())
    }

    pub fn terminals(&self) -> Result<MutexGuard<'_, TerminalManager>, DesktopError> {
        self.terminals
            .lock()
            .map_err(|_| DesktopError::Message("terminal manager lock poisoned".to_string()))
    }

    fn selected_guard(&self) -> Result<MutexGuard<'_, Option<String>>, DesktopError> {
        self.selected_run_id
            .lock()
            .map_err(|_| DesktopError::Message("selected run lock poisoned".to_string()))
    }
}

fn registry_path() -> PathBuf {
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data_home)
            .join("agentctl")
            .join("agentctl.sqlite3");
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("share")
        .join("agentctl")
        .join("agentctl.sqlite3")
}
