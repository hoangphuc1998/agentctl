use std::{
    collections::HashMap,
    io::{Read, Write},
    thread,
};

use agentctl_core::domain::RunRecord;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::{
    error::{DesktopError, DesktopResult},
    terminal_plan::tmux_attach_command,
};

#[derive(Default)]
pub struct TerminalManager {
    sessions: HashMap<String, TerminalSession>,
}

struct TerminalSession {
    run_id: String,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutputEvent {
    terminal_id: String,
    run_id: String,
    data: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalClosedEvent {
    terminal_id: String,
    run_id: String,
}

impl TerminalManager {
    pub fn start(
        &mut self,
        app: AppHandle,
        run: &RunRecord,
        fallback_session: &str,
        cols: u16,
        rows: u16,
    ) -> DesktopResult<String> {
        self.close_for_run(&run.id.to_string());
        let plan = tmux_attach_command(run, fallback_session).map_err(DesktopError::Message)?;
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| DesktopError::Message(err.to_string()))?;

        let mut command = CommandBuilder::new(plan.program);
        command.args(plan.args);
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| DesktopError::Message(err.to_string()))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| DesktopError::Message(err.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| DesktopError::Message(err.to_string()))?;

        let terminal_id = Uuid::new_v4().to_string();
        let run_id = run.id.to_string();
        spawn_reader(app, terminal_id.clone(), run_id.clone(), reader);
        self.sessions.insert(
            terminal_id.clone(),
            TerminalSession {
                run_id,
                master: pair.master,
                writer,
                child,
            },
        );
        Ok(terminal_id)
    }

    pub fn input(&mut self, terminal_id: &str, data: &str) -> DesktopResult<()> {
        let Some(session) = self.sessions.get_mut(terminal_id) else {
            return Err(DesktopError::Message(format!(
                "terminal session not found: {terminal_id}"
            )));
        };
        session.writer.write_all(data.as_bytes())?;
        session.writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, terminal_id: &str, cols: u16, rows: u16) -> DesktopResult<()> {
        let Some(session) = self.sessions.get(terminal_id) else {
            return Err(DesktopError::Message(format!(
                "terminal session not found: {terminal_id}"
            )));
        };
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| DesktopError::Message(err.to_string()))
    }

    pub fn close(&mut self, terminal_id: &str) -> DesktopResult<()> {
        if let Some(mut session) = self.sessions.remove(terminal_id) {
            let _ = session.child.kill();
        }
        Ok(())
    }

    fn close_for_run(&mut self, run_id: &str) {
        let terminal_ids = self
            .sessions
            .iter()
            .filter_map(|(terminal_id, session)| {
                (session.run_id == run_id).then(|| terminal_id.clone())
            })
            .collect::<Vec<_>>();
        for terminal_id in terminal_ids {
            let _ = self.close(&terminal_id);
        }
    }
}

fn spawn_reader(
    app: AppHandle,
    terminal_id: String,
    run_id: String,
    mut reader: Box<dyn Read + Send>,
) {
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    let data = String::from_utf8_lossy(&buffer[..size]).to_string();
                    let _ = app.emit(
                        "terminal:output",
                        TerminalOutputEvent {
                            terminal_id: terminal_id.clone(),
                            run_id: run_id.clone(),
                            data,
                        },
                    );
                }
                Err(_) => break,
            }
        }
        let _ = app.emit(
            "terminal:closed",
            TerminalClosedEvent {
                terminal_id,
                run_id,
            },
        );
    });
}
