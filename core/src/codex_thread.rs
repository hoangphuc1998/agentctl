use std::path::PathBuf;

use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexThreadSnapshot {
    pub id: Uuid,
    pub cwd: PathBuf,
    pub created_at: i64,
    pub updated_at: i64,
}
