use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use walkdir::WalkDir;

/// Summary of a file structure scan.
#[derive(serde::Serialize)]
pub struct ScanSummary {
    pub file_count: usize,
    pub dir_count: usize,
    pub max_depth: usize,
}

/// Directories to skip during scanning.
const EXCLUDED_DIRS: &[&str] = &[".git", "target", "node_modules", ".decay"];

/// Scan the project file tree and write results to the files table.
pub fn collect(conn: &Connection, snapshot_id: i64, project_path: &Path) -> Result<ScanSummary> {
    let mut file_count = 0;
    let mut dir_count = 0;
    let mut max_depth: usize = 0;

    let walker = WalkDir::new(project_path)
        .into_iter()
        .filter_entry(|entry| {
            // Skip excluded directories
            if entry.file_type().is_dir()
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| EXCLUDED_DIRS.contains(&name))
            {
                return false;
            }
            true
        });

    for entry in walker {
        let entry = entry.context("failed to read directory entry")?;
        let depth = entry.depth();

        if entry.file_type().is_dir() {
            if depth > 0 {
                dir_count += 1;
            }
            continue;
        }

        // It's a file
        let metadata = entry.metadata().context("failed to read file metadata")?;
        let rel_path = entry
            .path()
            .strip_prefix(project_path)
            .unwrap_or(entry.path());

        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                snapshot_id,
                rel_path.to_string_lossy(),
                metadata.len() as i64,
                depth
            ],
        )
        .context("failed to insert file record")?;

        file_count += 1;
        if depth > max_depth {
            max_depth = depth;
        }
    }

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
        assert_eq!(summary.dir_count, 1);
        assert_eq!(summary.max_depth, 2);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        assert_eq!(count, 2);

        Ok(())
    }

    #[test]
    fn test_scan_excludes_git() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("file1.rs"), "hello")?;
        fs::create_dir_all(dir.path().join(".git/objects"))?;
        fs::write(dir.path().join(".git/config"), "gitconfig")?;

        let conn = setup_db();
        let summary = collect(&conn, 1, dir.path())?;

        assert_eq!(summary.file_count, 1);

        Ok(())
    }
}
