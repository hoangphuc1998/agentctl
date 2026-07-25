use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use rusqlite::{params, types::Type, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::{
    agent::AgentKind,
    domain::{DetectionSource, Lifecycle, ObservedState, RunRecord, WorkspaceKind},
};

pub type RegistryResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub struct SqliteRegistry {
    conn: Connection,
}

impl SqliteRegistry {
    pub fn open(path: &Path) -> RegistryResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let registry = Self { conn };
        registry.migrate()?;
        Ok(registry)
    }

    pub fn in_memory() -> RegistryResult<Self> {
        let registry = Self {
            conn: Connection::open_in_memory()?,
        };
        registry.migrate()?;
        Ok(registry)
    }

    pub fn upsert_run(&self, run: &RunRecord) -> RegistryResult<()> {
        if run.is_folder() {
            return self.upsert_folder_session(run);
        }
        self.conn.execute(
            r#"
            INSERT INTO runs (
                id, repo_path, repo_name, tag, run_name, agent, lifecycle,
                observed_state, detection_source, branch, base_ref, base_commit, worktree_path,
                tmux_session, tmux_window, tmux_pane, agent_session_id,
                notification_seen_at, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18, ?19, ?20
            )
            ON CONFLICT(id) DO UPDATE SET
                repo_path = excluded.repo_path,
                repo_name = excluded.repo_name,
                tag = excluded.tag,
                run_name = excluded.run_name,
                agent = excluded.agent,
                lifecycle = excluded.lifecycle,
                observed_state = excluded.observed_state,
                detection_source = excluded.detection_source,
                branch = excluded.branch,
                base_ref = excluded.base_ref,
                base_commit = excluded.base_commit,
                worktree_path = excluded.worktree_path,
                tmux_session = excluded.tmux_session,
                tmux_window = excluded.tmux_window,
                tmux_pane = excluded.tmux_pane,
                agent_session_id = excluded.agent_session_id,
                notification_seen_at = excluded.notification_seen_at,
                updated_at = excluded.updated_at
            "#,
            params![
                run.id.to_string(),
                path_to_string(&run.repo_path),
                run.repo_name,
                run.tag,
                run.run_name,
                run.agent.as_str(),
                run.lifecycle.as_str(),
                run.observed_state.as_str(),
                run.detection_source.as_str(),
                run.branch,
                run.base_ref,
                run.base_commit,
                path_to_string(&run.worktree_path),
                run.tmux_session,
                run.tmux_window,
                run.tmux_pane,
                run.agent_session_id.map(|id| id.to_string()),
                run.notification_seen_at,
                run.created_at,
                run.updated_at,
            ],
        )?;
        Ok(())
    }

    fn upsert_folder_session(&self, run: &RunRecord) -> RegistryResult<()> {
        self.conn.execute(
            r#"
            INSERT INTO folder_sessions (
                id, folder_path, folder_name, tag, run_name, agent, lifecycle,
                observed_state, detection_source, tmux_session, tmux_window, tmux_pane,
                agent_session_id, notification_seen_at, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
            )
            ON CONFLICT(id) DO UPDATE SET
                folder_path = excluded.folder_path,
                folder_name = excluded.folder_name,
                tag = excluded.tag,
                run_name = excluded.run_name,
                agent = excluded.agent,
                lifecycle = excluded.lifecycle,
                observed_state = excluded.observed_state,
                detection_source = excluded.detection_source,
                tmux_session = excluded.tmux_session,
                tmux_window = excluded.tmux_window,
                tmux_pane = excluded.tmux_pane,
                agent_session_id = excluded.agent_session_id,
                notification_seen_at = excluded.notification_seen_at,
                updated_at = excluded.updated_at
            "#,
            params![
                run.id.to_string(),
                path_to_string(&run.worktree_path),
                run.repo_name,
                run.tag,
                run.run_name,
                run.agent.as_str(),
                run.lifecycle.as_str(),
                run.observed_state.as_str(),
                run.detection_source.as_str(),
                run.tmux_session,
                run.tmux_window,
                run.tmux_pane,
                run.agent_session_id.map(|id| id.to_string()),
                run.notification_seen_at,
                run.created_at,
                run.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_runs(&self) -> RegistryResult<Vec<RunRecord>> {
        self.query_runs(
            "SELECT * FROM runs ORDER BY repo_name ASC, tag ASC, run_name ASC",
            [],
        )
    }

    pub fn list_active_runs(&self) -> RegistryResult<Vec<RunRecord>> {
        self.query_runs(
            "SELECT * FROM runs WHERE lifecycle = 'active' ORDER BY repo_name ASC, tag ASC, run_name ASC",
            [],
        )
    }

    pub fn list_active_sessions(&self) -> RegistryResult<Vec<RunRecord>> {
        let mut sessions = self.list_active_runs()?;
        sessions.extend(self.query_folder_sessions(
            "SELECT * FROM folder_sessions WHERE lifecycle = 'active' ORDER BY folder_name ASC, tag ASC, run_name ASC",
            [],
        )?);
        sessions.sort_by(|left, right| {
            left.repo_name
                .cmp(&right.repo_name)
                .then_with(|| left.workspace_kind.cmp(&right.workspace_kind))
                .then_with(|| left.tag.cmp(&right.tag))
                .then_with(|| left.run_name.cmp(&right.run_name))
        });
        Ok(sessions)
    }

    pub fn list_restore_candidates(&self) -> RegistryResult<Vec<RunRecord>> {
        self.query_runs(
            "SELECT * FROM runs WHERE lifecycle = 'stopped' ORDER BY updated_at DESC",
            [],
        )
    }

    pub fn get_run(&self, id: Uuid) -> RegistryResult<Option<RunRecord>> {
        let mut stmt = self.conn.prepare("SELECT * FROM runs WHERE id = ?1")?;
        let run = stmt
            .query_row(params![id.to_string()], map_run)
            .optional()?;
        if run.is_some() {
            return Ok(run);
        }
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM folder_sessions WHERE id = ?1")?;
        Ok(stmt
            .query_row(params![id.to_string()], map_folder_session)
            .optional()?)
    }

    pub fn set_lifecycle(&self, id: Uuid, lifecycle: Lifecycle, now: i64) -> RegistryResult<()> {
        self.conn.execute(
            "UPDATE runs SET lifecycle = ?1, updated_at = ?2 WHERE id = ?3",
            params![lifecycle.as_str(), now, id.to_string()],
        )?;
        self.conn.execute(
            "UPDATE folder_sessions SET lifecycle = ?1, updated_at = ?2 WHERE id = ?3",
            params![lifecycle.as_str(), now, id.to_string()],
        )?;
        Ok(())
    }

    pub fn set_observed_state(
        &self,
        id: Uuid,
        state: ObservedState,
        source: DetectionSource,
        notification_seen_at: Option<i64>,
        now: i64,
    ) -> RegistryResult<()> {
        self.conn.execute(
            r#"
            UPDATE runs
            SET observed_state = ?1,
                detection_source = ?2,
                notification_seen_at = ?3,
                updated_at = ?4
            WHERE id = ?5
            "#,
            params![
                state.as_str(),
                source.as_str(),
                notification_seen_at,
                now,
                id.to_string()
            ],
        )?;
        self.conn.execute(
            r#"
            UPDATE folder_sessions
            SET observed_state = ?1,
                detection_source = ?2,
                notification_seen_at = ?3,
                updated_at = ?4
            WHERE id = ?5
            "#,
            params![
                state.as_str(),
                source.as_str(),
                notification_seen_at,
                now,
                id.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn active_repo_path(&self) -> RegistryResult<Option<PathBuf>> {
        let value = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'active_repo_path'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value.map(PathBuf::from))
    }

    pub fn set_active_repo_path(&self, path: &Path) -> RegistryResult<()> {
        self.conn.execute(
            r#"
            INSERT INTO settings (key, value) VALUES ('active_repo_path', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![path_to_string(path)],
        )?;
        Ok(())
    }

    pub fn active_folder_path(&self) -> RegistryResult<Option<PathBuf>> {
        let value = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'active_folder_path'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value.map(PathBuf::from))
    }

    pub fn set_active_folder_path(&self, path: &Path) -> RegistryResult<()> {
        self.conn.execute(
            r#"
            INSERT INTO settings (key, value) VALUES ('active_folder_path', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![path_to_string(path)],
        )?;
        Ok(())
    }

    pub fn recent_repo_paths(&self) -> RegistryResult<Vec<PathBuf>> {
        let mut paths = Vec::new();
        if let Some(path) = self.active_repo_path()? {
            paths.push(path);
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT repo_path, MAX(updated_at) AS last_seen
            FROM runs
            GROUP BY repo_path
            ORDER BY last_seen DESC
            "#,
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            let path = PathBuf::from(row?);
            if !paths.iter().any(|existing| existing == &path) {
                paths.push(path);
            }
        }

        Ok(paths)
    }

    pub fn recent_folder_paths(&self) -> RegistryResult<Vec<PathBuf>> {
        let mut paths = Vec::new();
        if let Some(path) = self.active_folder_path()? {
            paths.push(path);
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT folder_path, MAX(updated_at) AS last_seen
            FROM folder_sessions
            GROUP BY folder_path
            ORDER BY last_seen DESC
            "#,
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            let path = PathBuf::from(row?);
            if !paths.iter().any(|existing| existing == &path) {
                paths.push(path);
            }
        }

        Ok(paths)
    }

    fn migrate(&self) -> RegistryResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY NOT NULL,
                repo_path TEXT NOT NULL,
                repo_name TEXT NOT NULL,
                tag TEXT NOT NULL,
                run_name TEXT NOT NULL,
                agent TEXT NOT NULL,
                lifecycle TEXT NOT NULL,
                observed_state TEXT NOT NULL,
                detection_source TEXT NOT NULL,
                branch TEXT NOT NULL,
                base_ref TEXT NOT NULL,
                base_commit TEXT,
                worktree_path TEXT NOT NULL,
                tmux_session TEXT,
                tmux_window TEXT,
                tmux_pane TEXT,
                agent_session_id TEXT,
                notification_seen_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS folder_sessions (
                id TEXT PRIMARY KEY NOT NULL,
                folder_path TEXT NOT NULL,
                folder_name TEXT NOT NULL,
                tag TEXT NOT NULL,
                run_name TEXT NOT NULL,
                agent TEXT NOT NULL,
                lifecycle TEXT NOT NULL,
                observed_state TEXT NOT NULL,
                detection_source TEXT NOT NULL,
                tmux_session TEXT,
                tmux_window TEXT,
                tmux_pane TEXT,
                agent_session_id TEXT,
                notification_seen_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_runs_lifecycle ON runs(lifecycle);
            CREATE INDEX IF NOT EXISTS idx_runs_grouping ON runs(repo_name, tag, run_name);
            CREATE INDEX IF NOT EXISTS idx_folder_sessions_lifecycle
                ON folder_sessions(lifecycle);
            CREATE INDEX IF NOT EXISTS idx_folder_sessions_grouping
                ON folder_sessions(folder_name, tag, run_name);
            "#,
        )?;
        if !self.column_exists("runs", "base_commit")? {
            self.conn
                .execute("ALTER TABLE runs ADD COLUMN base_commit TEXT", [])?;
        }
        Ok(())
    }

    fn column_exists(&self, table: &str, column: &str) -> RegistryResult<bool> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for candidate in columns {
            if candidate? == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn query_runs<P>(&self, sql: &str, params: P) -> RegistryResult<Vec<RunRecord>>
    where
        P: rusqlite::Params,
    {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params, map_run)?;
        let mut runs = Vec::new();
        for row in rows {
            runs.push(row?);
        }
        Ok(runs)
    }

    fn query_folder_sessions<P>(&self, sql: &str, params: P) -> RegistryResult<Vec<RunRecord>>
    where
        P: rusqlite::Params,
    {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params, map_folder_session)?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }
}

fn map_run(row: &Row<'_>) -> rusqlite::Result<RunRecord> {
    let id: String = row.get("id")?;
    let agent: String = row.get("agent")?;
    let lifecycle: String = row.get("lifecycle")?;
    let observed_state: String = row.get("observed_state")?;
    let detection_source: String = row.get("detection_source")?;
    let agent_session_id: Option<String> = row.get("agent_session_id")?;

    Ok(RunRecord {
        id: parse_uuid(&id, 0)?,
        workspace_kind: WorkspaceKind::Worktree,
        repo_path: PathBuf::from(row.get::<_, String>("repo_path")?),
        repo_name: row.get("repo_name")?,
        tag: row.get("tag")?,
        run_name: row.get("run_name")?,
        agent: parse_agent(&agent, 5)?,
        lifecycle: parse_lifecycle(&lifecycle, 6)?,
        observed_state: parse_observed_state(&observed_state, 7)?,
        detection_source: parse_detection_source(&detection_source, 8)?,
        branch: row.get("branch")?,
        base_ref: row.get("base_ref")?,
        base_commit: row.get("base_commit")?,
        worktree_path: PathBuf::from(row.get::<_, String>("worktree_path")?),
        tmux_session: row.get("tmux_session")?,
        tmux_window: row.get("tmux_window")?,
        tmux_pane: row.get("tmux_pane")?,
        agent_session_id: agent_session_id
            .as_deref()
            .map(|value| parse_uuid(value, 15))
            .transpose()?,
        notification_seen_at: row.get("notification_seen_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_folder_session(row: &Row<'_>) -> rusqlite::Result<RunRecord> {
    let id: String = row.get("id")?;
    let agent: String = row.get("agent")?;
    let lifecycle: String = row.get("lifecycle")?;
    let observed_state: String = row.get("observed_state")?;
    let detection_source: String = row.get("detection_source")?;
    let agent_session_id: Option<String> = row.get("agent_session_id")?;
    let folder_path = PathBuf::from(row.get::<_, String>("folder_path")?);

    Ok(RunRecord {
        id: parse_uuid(&id, 0)?,
        workspace_kind: WorkspaceKind::Folder,
        repo_path: folder_path.clone(),
        repo_name: row.get("folder_name")?,
        tag: row.get("tag")?,
        run_name: row.get("run_name")?,
        agent: parse_agent(&agent, 5)?,
        lifecycle: parse_lifecycle(&lifecycle, 6)?,
        observed_state: parse_observed_state(&observed_state, 7)?,
        detection_source: parse_detection_source(&detection_source, 8)?,
        branch: String::new(),
        base_ref: String::new(),
        base_commit: None,
        worktree_path: folder_path,
        tmux_session: row.get("tmux_session")?,
        tmux_window: row.get("tmux_window")?,
        tmux_pane: row.get("tmux_pane")?,
        agent_session_id: agent_session_id
            .as_deref()
            .map(|value| parse_uuid(value, 12))
            .transpose()?,
        notification_seen_at: row.get("notification_seen_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn parse_uuid(value: &str, column: usize) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(value).map_err(|err| conversion_error(column, err))
}

fn parse_agent(value: &str, column: usize) -> rusqlite::Result<AgentKind> {
    AgentKind::from_str(value).map_err(|err| conversion_error(column, io_error(err)))
}

fn parse_lifecycle(value: &str, column: usize) -> rusqlite::Result<Lifecycle> {
    Lifecycle::from_storage(value).map_err(|err| conversion_error(column, io_error(err)))
}

fn parse_observed_state(value: &str, column: usize) -> rusqlite::Result<ObservedState> {
    ObservedState::from_storage(value).map_err(|err| conversion_error(column, io_error(err)))
}

fn parse_detection_source(value: &str, column: usize) -> rusqlite::Result<DetectionSource> {
    DetectionSource::from_storage(value).map_err(|err| conversion_error(column, io_error(err)))
}

fn conversion_error<E>(column: usize, err: E) -> rusqlite::Error
where
    E: Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err))
}

fn io_error(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
