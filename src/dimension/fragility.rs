use anyhow::{Context, Result};
use log::debug;

use super::Dimension;
use crate::data_store::DataStore;
use crate::diagnose::{Issue, Level};

// --- Fragility thresholds ---
const CHURN_CONCENTRATION_WARN: f64 = 0.5;
const CHURN_CONCENTRATION_CRIT: f64 = 0.7;
const MAX_CHURN_WARN: i64 = 500;

pub struct Fragility;

impl Dimension for Fragility {
    fn name(&self) -> &'static str {
        "fragility"
    }

    fn score(&self, store: &DataStore) -> Result<Option<i32>> {
        let conn = store.conn();
        let snapshot_id = store.snapshot_id();

        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM git_changes WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count git_changes")?;

        if file_count == 0 {
            return Ok(None);
        }

        let mut score: i32 = 100;
        debug!("fragility: scoring starting");

        let total_churn: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(lines_added + lines_deleted), 0) FROM git_changes WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to sum churn")?;

        if total_churn == 0 {
            return Ok(Some(100));
        }

        let top_n = (file_count as f64 * 0.1).ceil().max(1.0) as i64;
        let top_churn: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(churn), 0) FROM (
                    SELECT (lines_added + lines_deleted) as churn
                    FROM git_changes WHERE snapshot_id = ?1
                    ORDER BY churn DESC LIMIT ?2
                )",
                rusqlite::params![snapshot_id, top_n],
                |row| row.get(0),
            )
            .context("failed to get top churn")?;

        let concentration = top_churn as f64 / total_churn as f64;
        if concentration > CHURN_CONCENTRATION_CRIT {
            score -= 45;
        } else if concentration > CHURN_CONCENTRATION_WARN {
            score -= 25;
        }

        let max_churn: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(lines_added + lines_deleted), 0) FROM git_changes WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to get max churn")?;

        if max_churn > MAX_CHURN_WARN {
            score -= 15;
        }

        Ok(Some(score.max(0)))
    }

    fn diagnose(&self, store: &DataStore) -> Result<Vec<Issue>> {
        let conn = store.conn();
        let snapshot_id = store.snapshot_id();
        let mut issues = Vec::new();
        let name = self.name().to_string();

        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM git_changes WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to count git_changes")?;

        if file_count == 0 {
            return Ok(issues);
        }

        // High churn files (>500 lines), excluding lock files
        let mut stmt = conn
            .prepare(
                "SELECT path, (lines_added + lines_deleted) as churn FROM git_changes WHERE snapshot_id = ?1 AND (lines_added + lines_deleted) > 500 AND path NOT LIKE '%.lock' AND path NOT LIKE '%lock.json' ORDER BY churn DESC",
            )
            .context("failed to prepare churn query")?;

        let high_churn: Vec<(String, i64)> = stmt
            .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .context("failed to query high churn")?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to collect high churn")?;

        for (path, churn) in &high_churn {
            issues.push(Issue {
                level: Level::Critical,
                category: name.clone(),
                message: format!("{path} has {churn} lines churn"),
                prescription: Some(format!("split {path} to isolate unstable logic")),
            });
        }

        // Churn concentration
        let total_churn: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(lines_added + lines_deleted), 0) FROM git_changes WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .context("failed to sum churn")?;

        if total_churn > 0 {
            let top_n = (file_count as f64 * 0.1).ceil().max(1.0) as i64;
            let top_churn: i64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(churn), 0) FROM (
                        SELECT (lines_added + lines_deleted) as churn
                        FROM git_changes WHERE snapshot_id = ?1
                        ORDER BY churn DESC LIMIT ?2
                    )",
                    rusqlite::params![snapshot_id, top_n],
                    |row| row.get(0),
                )
                .context("failed to get top churn")?;

            let concentration = top_churn as f64 / total_churn as f64;
            if concentration > 0.5 {
                let pct = (concentration * 100.0) as i32;
                issues.push(Issue {
                    level: Level::Warning,
                    category: name.clone(),
                    message: format!("top 10% files account for {pct}% of churn"),
                    prescription: Some("distribute changes across more files".into()),
                });
            }
        }

        // Frequently changed files (>10 changes), excluding lock files
        let mut freq_stmt = conn
            .prepare(
                "SELECT path, change_count FROM git_changes WHERE snapshot_id = ?1 AND change_count > 10 AND path NOT LIKE '%.lock' AND path NOT LIKE '%lock.json' ORDER BY change_count DESC",
            )
            .context("failed to prepare freq query")?;

        let frequent: Vec<(String, i64)> = freq_stmt
            .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .context("failed to query frequent")?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to collect frequent")?;

        for (path, count) in &frequent {
            issues.push(Issue {
                level: Level::Info,
                category: name.clone(),
                message: format!("{path} changed {count} times"),
                prescription: None,
            });
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_store::DataStore;
    use rusqlite::Connection;

    fn setup_store() -> DataStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, project_path TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), version TEXT NOT NULL);
             CREATE TABLE files (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, size_bytes INTEGER NOT NULL, depth INTEGER NOT NULL);
             CREATE TABLE git_changes (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, change_count INTEGER NOT NULL, lines_added INTEGER NOT NULL, lines_deleted INTEGER NOT NULL, last_modified TEXT NOT NULL);",
        ).unwrap();
        conn.execute("INSERT INTO snapshots (project_path, version) VALUES ('/tmp', '0.1.0')", []).unwrap();
        DataStore::new(conn, 1, "/tmp".to_string())
    }

    #[test]
    fn test_no_git() -> Result<()> {
        let store = setup_store();
        let dim = Fragility;
        let score = dim.score(&store)?;
        assert_eq!(score, None);
        Ok(())
    }

    #[test]
    fn test_even_churn() -> Result<()> {
        let store = setup_store();
        for i in 0..10 {
            store.conn().execute(
                "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (1, ?1, 5, 50, 30, '2026-04-01')",
                [format!("src/file{i}.rs")],
            )?;
        }
        let dim = Fragility;
        let score = dim.score(&store)?.unwrap();
        assert!(score > 70, "evenly spread churn should score >70, got {score}");
        Ok(())
    }

    #[test]
    fn test_high_churn_critical() -> Result<()> {
        let store = setup_store();
        store.conn().execute(
            "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (1, 'hot.rs', 20, 400, 200, '2026-04-01')",
            [],
        )?;
        let dim = Fragility;
        let issues = dim.diagnose(&store)?;
        assert!(issues.iter().any(|i| i.level == Level::Critical && i.message.contains("hot.rs")));
        Ok(())
    }
}
