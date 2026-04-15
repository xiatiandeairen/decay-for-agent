use anyhow::{Context, Result};
use log::debug;

use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::DataStore;
use crate::diagnose::{Issue, Level};

// --- Fragility thresholds ---
/// Fraction of total line churn concentrated in the top 10% of files.
/// 50%+ concentration means a small set of files absorbs most change risk.
const CHURN_CONCENTRATION_WARN: f64 = 0.5;
/// Critical concentration: 70%+ of churn in top 10% of files signals hotspot fragility.
/// These files are high-blast-radius; any bug there affects the whole project disproportionately.
const CHURN_CONCENTRATION_CRIT: f64 = 0.7;
/// Total lines added+deleted for a single file across history.
/// 500+ lines of churn indicates a persistently unstable file that warrants isolation.
const MAX_CHURN_WARN: i64 = 500;

pub struct Fragility;

impl Dimension for Fragility {
    fn name(&self) -> &'static str {
        "fragility"
    }

    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult> {
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
            .with_context(|| format!("fragility: failed to count git_changes for snapshot {snapshot_id}"))?;

        if file_count == 0 {
            return Ok(DimensionResult { name, score: None, issues });
        }

        let mut score: i32 = 100;
        debug!("fragility: evaluating");

        // Total churn (shared by score + diagnose)
        let total_churn: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(lines_added + lines_deleted), 0) FROM git_changes WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .with_context(|| format!("fragility: failed to sum churn for snapshot {snapshot_id}"))?;

        if total_churn == 0 {
            return Ok(DimensionResult { name, score: Some(100), issues });
        }

        // Churn concentration (shared by score + diagnose)
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
            .with_context(|| format!("fragility: failed to get top churn for snapshot {snapshot_id}"))?;

        let concentration = top_churn as f64 / total_churn as f64;
        if concentration > CHURN_CONCENTRATION_CRIT {
            score -= 45;
        } else if concentration > CHURN_CONCENTRATION_WARN {
            score -= 25;
        }
        if concentration > CHURN_CONCENTRATION_WARN {
            let pct = (concentration * 100.0) as i32;
            issues.push(Issue::with_actions(
                Level::Warning,
                name.clone(),
                format!("top 10% files account for {pct}% of churn"),
                Some("distribute changes across more files".into()),
                vec![Action {
                    dimension: name.clone(),
                    action_type: ActionType::Refactor,
                    target: Target { file: ".".into(), line_range: None, symbol: None },
                    reason: format!("top 10% files account for {pct}% of churn, distribute changes"),
                    priority: Priority::High,
                    effort: Effort::Large,
                }],
            ));
        }

        // High churn files (>MAX_CHURN_WARN lines), excluding lock files
        let mut stmt = conn
            .prepare(
                &format!("SELECT path, (lines_added + lines_deleted) as churn FROM git_changes WHERE snapshot_id = ?1 AND (lines_added + lines_deleted) > {MAX_CHURN_WARN} AND path NOT LIKE '%.lock' AND path NOT LIKE '%lock.json' ORDER BY churn DESC"),
            )
            .with_context(|| format!("fragility: failed to prepare churn query for snapshot {snapshot_id}"))?;

        let high_churn: Vec<(String, i64)> = stmt
            .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .with_context(|| format!("fragility: failed to query high churn for snapshot {snapshot_id}"))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("fragility: failed to collect high churn for snapshot {snapshot_id}"))?;

        if !high_churn.is_empty() {
            score -= 15;
        }

        for (path, churn) in &high_churn {
            issues.push(Issue::with_actions(
                Level::Critical,
                name.clone(),
                format!("{path} has {churn} lines churn"),
                Some(format!("split {path} to isolate unstable logic")),
                vec![Action {
                    dimension: name.clone(),
                    action_type: ActionType::Split,
                    target: Target { file: path.clone(), line_range: None, symbol: None },
                    reason: format!("{path} has {churn} lines churn, isolate unstable logic"),
                    priority: Priority::Critical,
                    effort: Effort::Medium,
                }],
            ));
        }

        // Frequently changed files (>10 changes), excluding lock files
        let mut freq_stmt = conn
            .prepare(
                "SELECT path, change_count FROM git_changes WHERE snapshot_id = ?1 AND change_count > 10 AND path NOT LIKE '%.lock' AND path NOT LIKE '%lock.json' ORDER BY change_count DESC",
            )
            .with_context(|| format!("fragility: failed to prepare freq query for snapshot {snapshot_id}"))?;

        let frequent: Vec<(String, i64)> = freq_stmt
            .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .with_context(|| format!("fragility: failed to query frequent for snapshot {snapshot_id}"))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("fragility: failed to collect frequent for snapshot {snapshot_id}"))?;

        for (path, count) in &frequent {
            issues.push(Issue::new(
                Level::Info,
                name.clone(),
                format!("{path} changed {count} times"),
                None,
            ));
        }

        Ok(DimensionResult {
            name: self.name().to_string(),
            score: Some(score.max(0)),
            issues,
        })
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
        let score = dim.evaluate(&store)?.score;
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
        let score = dim.evaluate(&store)?.score.unwrap();
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
        let issues = dim.evaluate(&store)?.issues;
        assert!(issues.iter().any(|i| i.level == Level::Critical && i.message.contains("hot.rs")));
        Ok(())
    }
}
