use agent_manager_desktop::codex_state::load_codex_threads_from_db;
use rusqlite::Connection;
use uuid::Uuid;

#[test]
fn loads_only_active_user_cli_threads_from_codex_state() {
    let directory = tempfile::tempdir().expect("state directory");
    let database_path = directory.path().join("state_5.sqlite");
    let connection = Connection::open(&database_path).expect("state database");
    connection
        .execute_batch(
            "CREATE TABLE threads (
                id TEXT PRIMARY KEY,
                cwd TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                archived INTEGER NOT NULL,
                source TEXT NOT NULL,
                thread_source TEXT
            );",
        )
        .expect("thread schema");
    let active_id = Uuid::new_v4();
    let archived_id = Uuid::new_v4();
    let subagent_id = Uuid::new_v4();
    let remote_id = Uuid::new_v4();
    for (id, archived, source, updated_at) in [
        (active_id, 0, "user", 30),
        (archived_id, 1, "user", 40),
        (subagent_id, 0, "subagent", 50),
    ] {
        connection
            .execute(
                "INSERT INTO threads
                 (id, cwd, created_at, updated_at, archived, source, thread_source)
                 VALUES (?1, '/workspace', 10, ?2, ?3, 'cli', ?4)",
                (id.to_string(), updated_at, archived, source),
            )
            .expect("thread row");
    }
    connection
        .execute(
            "INSERT INTO threads
             (id, cwd, created_at, updated_at, archived, source, thread_source)
             VALUES (?1, '/workspace', 10, 60, 0, 'vscode', 'user')",
            [remote_id.to_string()],
        )
        .expect("remote thread row");
    drop(connection);

    let threads = load_codex_threads_from_db(&database_path).expect("load threads");

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].id, active_id);
    assert_eq!(threads[0].cwd.to_string_lossy(), "/workspace");
    assert_eq!(threads[0].created_at, 10);
    assert_eq!(threads[0].updated_at, 30);
}
