use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use super::Dimension;
use crate::diagnose::{Issue, Level};

// --- Structural thresholds ---
const FILE_COUNT_WARN: i64 = 500;
const FILE_COUNT_CRIT: i64 = 1000;
const DEPTH_WARN: i64 = 5;
const DEPTH_CRIT: i64 = 8;
const TOP_DIRS_WARN: i64 = 15;

pub struct Structural;

impl Dimension for Structural {
    fn name(&self) -> &'static str {
        "structural"
    }

    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>> {
        let mut score: i32 = 100;
        debug!("structural: file_count query starting");

        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count files")?;

        if file_count > FILE_COUNT_CRIT {
            score -= 40;
        } else if file_count > FILE_COUNT_WARN {
            score -= 20;
        }

        let max_depth: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(depth), 0) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to get max depth")?;

        if max_depth > DEPTH_CRIT {
            score -= 30;
        } else if max_depth > DEPTH_WARN {
            score -= 15;
        }

        let top_dirs: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT CASE
                    WHEN INSTR(path, '/') > 0 THEN SUBSTR(path, 1, INSTR(path, '/') - 1)
                    ELSE path
                 END) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count top-level dirs")?;

        if top_dirs > TOP_DIRS_WARN {
            score -= 15;
        }

        Ok(Some(score.max(0)))
    }

    fn diagnose(&self, conn: &Connection, snapshot_id: i64) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();
        let name = self.name().to_string();

        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count files")?;

        if file_count > 1000 {
            issues.push(Issue {
                level: Level::Critical,
                category: name.clone(),
                message: format!("{file_count} files in project"),
                prescription: Some("split into sub-modules by responsibility".into()),
            });
        } else if file_count > 500 {
            issues.push(Issue {
                level: Level::Warning,
                category: name.clone(),
                message: format!("{file_count} files in project"),
                prescription: Some("review directory structure for extractable modules".into()),
            });
        }

        let max_depth: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(depth), 0) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to get max depth")?;

        if max_depth > 5 {
            issues.push(Issue {
                level: Level::Warning,
                category: name.clone(),
                message: format!("max directory depth is {max_depth}"),
                prescription: Some("flatten nested directories".into()),
            });
        }

        let top_dirs: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT CASE
                    WHEN INSTR(path, '/') > 0 THEN SUBSTR(path, 1, INSTR(path, '/') - 1)
                    ELSE path
                 END) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count top-level dirs")?;

        if top_dirs > 15 {
            issues.push(Issue {
                level: Level::Info,
                category: name,
                message: format!("{top_dirs} top-level entries"),
                prescription: None,
            });
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
        for i in 0..10 {
            conn.execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 1000, 2)",
                [format!("src/file{i}.rs")],
            )?;
        }
        let dim = Structural;
        let score = dim.score(&conn, 1)?.unwrap();
        assert!(score > 80, "healthy project should score >80, got {score}");
        let issues = dim.diagnose(&conn, 1)?;
        assert!(issues.is_empty());
        Ok(())
    }

    #[test]
    fn test_unhealthy() -> Result<()> {
        let conn = setup_db();
        for i in 0..600 {
            conn.execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 1000, 9)",
                [format!("a/b/c/d/e/f/g/h/i/file{i}.rs")],
            )?;
        }
        let dim = Structural;
        let score = dim.score(&conn, 1)?.unwrap();
        assert!(score < 60, "unhealthy project should score <60, got {score}");
        Ok(())
    }
}
