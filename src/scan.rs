use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use crate::filter;

/// Summary of a file structure scan.
#[derive(serde::Serialize)]
pub struct ScanSummary {
    pub file_count: usize,
    pub dir_count: usize,
    pub max_depth: usize,
}

/// Scan the project using the three-layer filter and write results to the files table.
pub fn collect(conn: &Connection, snapshot_id: i64, project_path: &Path) -> Result<ScanSummary> {
    let files = filter::resolve_files(project_path)?;

    let mut max_depth: usize = 0;

    for f in &files {
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                snapshot_id,
                f.rel_path.to_string_lossy(),
                f.size as i64,
                f.depth
            ],
        )
        .context("failed to insert file record")?;

        if f.depth > max_depth {
            max_depth = f.depth;
        }
    }

    let file_count = files.len();
    let dir_count = filter::count_dirs(&files);

    debug!("scan: {file_count} files, {dir_count} dirs after filtering");

    Ok(ScanSummary {
        file_count,
        dir_count,
        max_depth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                depth INTEGER NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_scan_basic() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("file1.rs"), "hello")?;
        fs::create_dir_all(dir.path().join("sub"))?;
        fs::write(dir.path().join("sub/file2.rs"), "world")?;

        let conn = setup_db();
        let summary = collect(&conn, 1, dir.path())?;

        assert_eq!(summary.file_count, 2);
        assert!(summary.dir_count >= 1);
        assert!(summary.max_depth >= 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        assert_eq!(count, 2);

        Ok(())
    }

    #[test]
    fn test_scan_excludes_build_artifacts() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("main.rs"), "fn main() {}")?;
        fs::create_dir_all(dir.path().join(".build/debug"))?;
        fs::write(dir.path().join(".build/debug/app"), "binary")?;
        fs::create_dir_all(dir.path().join(".git/objects"))?;
        fs::write(dir.path().join(".git/config"), "gitconfig")?;
        fs::write(dir.path().join("icon.png"), "image")?;

        let conn = setup_db();
        let summary = collect(&conn, 1, dir.path())?;

        assert_eq!(summary.file_count, 1); // only main.rs
        Ok(())
    }
}
