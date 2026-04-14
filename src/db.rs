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
        );

        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
            path TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            depth INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS git_changes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
            path TEXT NOT NULL,
            change_count INTEGER NOT NULL,
            lines_added INTEGER NOT NULL,
            lines_deleted INTEGER NOT NULL,
            last_modified TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scores (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
            structural INTEGER NOT NULL,
            complexity INTEGER NOT NULL,
            fragility INTEGER,
            composite INTEGER NOT NULL
        );",
    )
    .context("failed to create tables")?;

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

/// Insert scores for a snapshot.
pub fn insert_scores(
    conn: &Connection,
    snapshot_id: i64,
    structural: i32,
    complexity: i32,
    fragility: Option<i32>,
    composite: i32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO scores (snapshot_id, structural, complexity, fragility, composite) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![snapshot_id, structural, complexity, fragility, composite],
    )
    .context("failed to insert scores")?;
    Ok(())
}

/// Scores from a previous snapshot.
pub struct PreviousScores {
    pub structural: i32,
    pub complexity: i32,
    pub fragility: Option<i32>,
    pub composite: i32,
}

/// Get scores from the most recent previous snapshot for the same project.
pub fn get_previous_scores(
    conn: &Connection,
    project_path: &str,
    current_snapshot_id: i64,
) -> Result<Option<PreviousScores>> {
    let mut stmt = conn
        .prepare(
            "SELECT sc.structural, sc.complexity, sc.fragility, sc.composite
         FROM scores sc
         JOIN snapshots s ON s.id = sc.snapshot_id
         WHERE s.project_path = ?1 AND sc.snapshot_id < ?2
         ORDER BY sc.snapshot_id DESC
         LIMIT 1",
        )
        .context("failed to prepare previous scores query")?;

    let result = stmt.query_row(
        rusqlite::params![project_path, current_snapshot_id],
        |row| {
            Ok(PreviousScores {
                structural: row.get(0)?,
                complexity: row.get(1)?,
                fragility: row.get(2)?,
                composite: row.get(3)?,
            })
        },
    );

    match result {
        Ok(scores) => Ok(Some(scores)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to get previous scores"),
    }
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
