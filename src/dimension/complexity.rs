use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use super::Dimension;
use crate::diagnose::{Issue, Level};

// --- Complexity thresholds ---
const LARGE_FILE_BYTES: i64 = 15360;
const LARGE_RATIO_WARN: f64 = 0.2;
const LARGE_RATIO_CRIT: f64 = 0.4;
const AVG_SIZE_WARN: f64 = 10240.0;
const MAX_SIZE_WARN: i64 = 51200;

pub struct Complexity;

impl Dimension for Complexity {
    fn name(&self) -> &'static str {
        "complexity"
    }

    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>> {
        let mut score: i32 = 100;
        debug!("complexity: scoring starting");

        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count files")?;

        if file_count == 0 {
            return Ok(Some(100));
        }

        let large_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1 AND size_bytes > ?2",
                rusqlite::params![snapshot_id, LARGE_FILE_BYTES],
                |row| row.get(0),
            )
            .context("failed to count large files")?;

        let large_ratio = large_count as f64 / file_count as f64;
        if large_ratio > LARGE_RATIO_CRIT {
            score -= 45;
        } else if large_ratio > LARGE_RATIO_WARN {
            score -= 25;
        }

        let avg_size: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(size_bytes), 0) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to get avg file size")?;

        if avg_size > AVG_SIZE_WARN {
            score -= 15;
        }

        let max_size: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(size_bytes), 0) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to get max file size")?;

        if max_size > MAX_SIZE_WARN {
            score -= 10;
        }

        Ok(Some(score.max(0)))
    }

    fn diagnose(&self, conn: &Connection, snapshot_id: i64) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();
        let name = self.name().to_string();

        let mut stmt = conn
            .prepare(
                "SELECT path, size_bytes FROM files WHERE snapshot_id = ?1 AND size_bytes > 15360 ORDER BY size_bytes DESC",
            )
            .context("failed to prepare large files query")?;

        let large_files: Vec<(String, i64)> = stmt
            .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .context("failed to query large files")?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to collect large files")?;

        for (path, size) in &large_files {
            let size_kb = size / 1024;
            if *size > 51200 {
                issues.push(Issue {
                    level: Level::Critical,
                    category: name.clone(),
                    message: format!("{path} ({size_kb}KB)"),
                    prescription: Some(format!("split {path} into smaller units")),
                });
            } else {
                issues.push(Issue {
                    level: Level::Warning,
                    category: name.clone(),
                    message: format!("{path} ({size_kb}KB)"),
                    prescription: Some(format!("extract independent logic from {path}")),
                });
            }
        }

        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count files")?;

        if file_count > 0 {
            let ratio = large_files.len() as f64 / file_count as f64;
            if ratio > 0.2 {
                let pct = (ratio * 100.0) as i32;
                issues.push(Issue {
                    level: Level::Info,
                    category: name,
                    message: format!("{pct}% of files exceed 15KB"),
                    prescription: None,
                });
            }
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_healthy() -> Result<()> {
        let conn = setup_db();
        for i in 0..20 {
            conn.execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 3000, 2)",
                [format!("src/file{i}.rs")],
            )?;
        }
        let dim = Complexity;
        let score = dim.score(&conn, 1)?.unwrap();
        assert!(score > 80, "healthy complexity should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_large_file_warning() -> Result<()> {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, 'big.rs', 20000, 1)",
            [],
        )?;
        let dim = Complexity;
        let issues = dim.diagnose(&conn, 1)?;
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.level == Level::Warning && i.message.contains("big.rs")));
        Ok(())
    }
}
