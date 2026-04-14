use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Return the database path under the XDG data directory.
///
/// Linux: ~/.local/share/decay/snapshots.db
/// macOS: ~/Library/Application Support/decay/snapshots.db
pub fn db_path() -> Result<PathBuf> {
    let data_dir = dirs::data_dir().context("could not determine data directory")?;
    let decay_dir = data_dir.join("decay");
    std::fs::create_dir_all(&decay_dir)
        .with_context(|| format!("failed to create {}", decay_dir.display()))?;
    Ok(decay_dir.join("snapshots.db"))
}

/// Open (or create) the database and ensure the schema exists.
pub fn init() -> Result<Connection> {
    let path = db_path()?;
    let conn = Connection::open(&path)
        .with_context(|| format!("failed to open database at {}", path.display()))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_path TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            version TEXT NOT NULL
        );",
    )
    .context("failed to create snapshots table")?;

    Ok(conn)
}

/// Insert a new snapshot and return its ID.
pub fn create_snapshot(conn: &Connection, project_path: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO snapshots (project_path, version) VALUES (?1, ?2)",
        rusqlite::params![project_path, env!("CARGO_PKG_VERSION")],
    )
    .context("failed to insert snapshot")?;

    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_and_create_snapshot() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                version TEXT NOT NULL
            );",
        )?;

        let id = create_snapshot(&conn, "/tmp/test-project")?;
        assert_eq!(id, 1);

        let id2 = create_snapshot(&conn, "/tmp/test-project")?;
        assert_eq!(id2, 2);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))?;
        assert_eq!(count, 2);

        Ok(())
    }

    #[test]
    fn test_db_path_is_valid() -> Result<()> {
        let path = db_path()?;
        assert!(path.ends_with("decay/snapshots.db"));
        Ok(())
    }
}
