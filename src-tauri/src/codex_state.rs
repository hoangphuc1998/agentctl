use std::{
    env,
    path::{Path, PathBuf},
};

use agentctl_core::codex_thread::CodexThreadSnapshot;
use rusqlite::{Connection, OpenFlags};
use uuid::Uuid;

const THREAD_LIMIT: usize = 100;

pub fn codex_home_from_environment() -> Option<PathBuf> {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
}

pub fn load_codex_threads() -> Result<Vec<CodexThreadSnapshot>, String> {
    let codex_home = codex_home_from_environment()
        .ok_or_else(|| "neither CODEX_HOME nor HOME is configured".to_string())?;
    load_codex_threads_from_db(&codex_home.join("state_5.sqlite"))
}

pub fn load_codex_threads_from_db(
    database_path: &Path,
) -> Result<Vec<CodexThreadSnapshot>, String> {
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| {
        format!(
            "failed to open Codex state database {}: {err}",
            database_path.display()
        )
    })?;
    let mut statement = connection
        .prepare(
            "SELECT id, cwd, created_at, updated_at
             FROM threads
             WHERE archived = 0
               AND source = 'cli'
               AND (thread_source IS NULL OR thread_source = 'user')
             ORDER BY updated_at DESC, id DESC
             LIMIT ?1",
        )
        .map_err(|err| format!("failed to prepare Codex thread query: {err}"))?;
    let mut rows = statement
        .query([THREAD_LIMIT])
        .map_err(|err| format!("failed to query Codex threads: {err}"))?;
    let mut threads = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read Codex thread row: {err}"))?
    {
        let id = row
            .get::<_, String>(0)
            .map_err(|err| format!("failed to read Codex thread id: {err}"))?;
        threads.push(CodexThreadSnapshot {
            id: Uuid::parse_str(&id)
                .map_err(|err| format!("invalid Codex thread id {id}: {err}"))?,
            cwd: PathBuf::from(
                row.get::<_, String>(1)
                    .map_err(|err| format!("failed to read Codex thread cwd: {err}"))?,
            ),
            created_at: row
                .get(2)
                .map_err(|err| format!("failed to read Codex thread created_at: {err}"))?,
            updated_at: row
                .get(3)
                .map_err(|err| format!("failed to read Codex thread updated_at: {err}"))?,
        });
    }
    Ok(threads)
}
