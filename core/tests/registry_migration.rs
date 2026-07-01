use agentctl_core::registry::SqliteRegistry;
use rusqlite::{params, Connection};
use uuid::Uuid;

#[test]
fn registry_migration_adds_nullable_base_commit_to_existing_runs() {
    let dir = tempfile::tempdir().expect("dir");
    let db_path = dir.path().join("agentctl.sqlite3");
    {
        let conn = Connection::open(&db_path).expect("open old db");
        conn.execute_batch(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE TABLE runs (
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
                worktree_path TEXT NOT NULL,
                tmux_session TEXT,
                tmux_window TEXT,
                tmux_pane TEXT,
                agent_session_id TEXT,
                notification_seen_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )
        .expect("create old schema");
        conn.execute(
            r#"
            INSERT INTO runs (
                id, repo_path, repo_name, tag, run_name, agent, lifecycle,
                observed_state, detection_source, branch, base_ref, worktree_path,
                tmux_session, tmux_window, tmux_pane, agent_session_id,
                notification_seen_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
            "#,
            params![
                Uuid::new_v4().to_string(),
                "/repo/agent-manager",
                "agent-manager",
                "feature",
                "diff-review",
                "codex",
                "active",
                "running",
                "tmux",
                "diff-review",
                "HEAD",
                "/repo/agent-manager-worktrees/diff-review",
                "agentctl",
                "agent-manager__feature__diff-review",
                Option::<String>::None,
                Option::<String>::None,
                Option::<i64>::None,
                1,
                2,
            ],
        )
        .expect("insert old run");
    }

    let registry = SqliteRegistry::open(&db_path).expect("migrated registry");
    let runs = registry.list_runs().expect("runs");

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_name, "diff-review");
    assert_eq!(runs[0].base_commit, None);
}
