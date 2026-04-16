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
/// 1000+ lines of churn indicates a persistently unstable file that warrants isolation.
/// Young projects with rapid iteration naturally have high churn; 1000 filters to true hotspots.
const MAX_CHURN_WARN: i64 = 1000;
/// Maximum number of high-churn files to report individually.
const MAX_CHURN_ISSUES: usize = 5;
/// Minimum number of changes to a file before flagging it as frequently changed.
const FREQ_CHANGE_WARN: i64 = 20;

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
                Level::Warning, name.clone(), format!("top 10% files account for {pct}% of churn"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Refactor,
                    target: Target::file("."),
                    suggestion: "distribute changes across more files".into(),
                    reason: format!("top 10% files account for {pct}% of churn"),
                    priority: Priority::High, effort: Effort::Large,
                    details: vec![],
                    impact: None,
                    verify: String::new(),
                }],
            ));
        }

        // High churn files (>MAX_CHURN_WARN lines), excluding lock files
        let mut stmt = conn
            .prepare(
                "SELECT path, (lines_added + lines_deleted) as churn FROM git_changes WHERE snapshot_id = ?1 AND (lines_added + lines_deleted) > ?2 AND path NOT LIKE '%.lock' AND path NOT LIKE '%lock.json' ORDER BY churn DESC",
            )
            .with_context(|| format!("fragility: failed to prepare churn query for snapshot {snapshot_id}"))?;

        let high_churn: Vec<(String, i64)> = stmt
            .query_map(rusqlite::params![snapshot_id, MAX_CHURN_WARN], |row| Ok((row.get(0)?, row.get(1)?)))
            .with_context(|| format!("fragility: failed to query high churn for snapshot {snapshot_id}"))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("fragility: failed to collect high churn for snapshot {snapshot_id}"))?;

        if !high_churn.is_empty() {
            score -= 15;
        }

        // Check how many of the last 3 snapshots (same project) each high-churn file appears in
        let recent_snapshot_ids: Vec<i64> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM snapshots WHERE project_path = ?1 AND id < ?2 ORDER BY id DESC LIMIT 3",
                )
                .with_context(|| "fragility: failed to prepare recent snapshots query")?;
            stmt.query_map(rusqlite::params![store.project_path(), snapshot_id], |row| row.get(0))
                .with_context(|| "fragility: failed to query recent snapshots")?
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| "fragility: failed to collect recent snapshots")?
        };

        for (path, churn) in high_churn.iter().take(MAX_CHURN_ISSUES) {
            // Count how many recent snapshots also contain this file in git_changes
            let consecutive_count = if recent_snapshot_ids.is_empty() {
                0usize
            } else {
                let placeholders: String = recent_snapshot_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                let query = format!(
                    "SELECT COUNT(DISTINCT snapshot_id) FROM git_changes WHERE snapshot_id IN ({placeholders}) AND path = ?",
                );
                let mut stmt = conn.prepare(&query)
                    .with_context(|| "fragility: failed to prepare consecutive query")?;
                let mut param_idx = 1;
                for sid in &recent_snapshot_ids {
                    stmt.raw_bind_parameter(param_idx, sid)?;
                    param_idx += 1;
                }
                stmt.raw_bind_parameter(param_idx, path.as_str())?;
                let count: i64 = stmt.raw_query().next()?.unwrap().get(0)?;
                count as usize
            };

            let is_active_dev = consecutive_count >= 3;

            if is_active_dev {
                // +1 for current snapshot
                let n = consecutive_count + 1;
                issues.push(Issue::new(
                    Level::Info, name.clone(),
                    format!("{path} has {churn} lines churn (active development — changed in {n} consecutive snapshots)"),
                ));
            } else {
                // Generate split suggestions from source file function analysis
                let details = store.source_files()
                    .iter()
                    .find(|sf| sf.path == *path)
                    .map(|sf| crate::dimension::helpers::suggest_split_details(&sf.lines, path))
                    .unwrap_or_default();
                issues.push(Issue::with_actions(
                    Level::Critical, name.clone(), format!("{path} has {churn} lines churn"),
                    vec![Action {
                        dimension: name.clone(), action_type: ActionType::Split,
                        target: Target::file(path),
                        suggestion: format!("split {path} to isolate unstable logic"),
                        reason: format!("{path} has {churn} lines churn"),
                        priority: Priority::Critical, effort: Effort::Medium,
                        details,
                        impact: None,
                        verify: String::new(),
                    }],
                ));
            }
        }

        // Frequently changed files (>10 changes), excluding lock files
        let mut freq_stmt = conn
            .prepare(
                "SELECT path, change_count FROM git_changes WHERE snapshot_id = ?1 AND change_count > ?2 AND path NOT LIKE '%.lock' AND path NOT LIKE '%lock.json' ORDER BY change_count DESC",
            )
            .with_context(|| format!("fragility: failed to prepare freq query for snapshot {snapshot_id}"))?;

        let frequent: Vec<(String, i64)> = freq_stmt
            .query_map(rusqlite::params![snapshot_id, FREQ_CHANGE_WARN], |row| Ok((row.get(0)?, row.get(1)?)))
            .with_context(|| format!("fragility: failed to query frequent for snapshot {snapshot_id}"))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("fragility: failed to collect frequent for snapshot {snapshot_id}"))?;

        for (path, count) in &frequent {
            issues.push(Issue::new(
                Level::Info, name.clone(), format!("{path} changed {count} times"),
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
    use crate::dimension::test_support;
    use rusqlite::Connection;

    #[test]
    fn test_no_git() -> Result<()> {
        let store = test_support::setup_db_store();
        let dim = Fragility;
        let score = dim.evaluate(&store)?.score;
        assert_eq!(score, None);
        Ok(())
    }

    #[test]
    fn test_even_churn() -> Result<()> {
        let store = test_support::setup_db_store();
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
        let store = test_support::setup_db_store();
        store.conn().execute(
            "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (1, 'hot.rs', 20, 700, 500, '2026-04-01')",
            [],
        )?;
        let dim = Fragility;
        let issues = dim.evaluate(&store)?.issues;
        assert!(issues.iter().any(|i| i.level == Level::Critical && i.message.contains("hot.rs")));
        Ok(())
    }

    #[test]
    fn test_active_dev_downgrades_to_info() -> Result<()> {
        let store = test_support::setup_db_store();
        let conn = store.conn();

        // Create 3 older snapshots for the same project_path ('/tmp')
        for _ in 0..3 {
            conn.execute(
                "INSERT INTO snapshots (project_path, version) VALUES ('/tmp', '0.1.0')",
                [],
            )?;
        }
        // Snapshot IDs: 1 (current), 2, 3, 4 (historical)
        // The query fetches snapshots with id < current (1), so ids 2,3,4 won't work.
        // We need current snapshot_id to be the largest. Let's create a new store.
        drop(store);

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, project_path TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), version TEXT NOT NULL);
             CREATE TABLE files (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, size_bytes INTEGER NOT NULL, depth INTEGER NOT NULL);
             CREATE TABLE git_changes (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, change_count INTEGER NOT NULL, lines_added INTEGER NOT NULL, lines_deleted INTEGER NOT NULL, last_modified TEXT NOT NULL);",
        )?;

        // Create 3 historical snapshots + 1 current (ids 1,2,3 historical, 4 current)
        for _ in 0..3 {
            conn.execute(
                "INSERT INTO snapshots (project_path, version) VALUES ('/tmp', '0.1.0')",
                [],
            )?;
        }
        conn.execute(
            "INSERT INTO snapshots (project_path, version) VALUES ('/tmp', '0.1.0')",
            [],
        )?;
        let current_id: i64 = 4;

        // Add high-churn file to all 4 snapshots
        for sid in 1..=4 {
            conn.execute(
                "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (?1, 'active.rs', 20, 700, 500, '2026-04-01')",
                [sid],
            )?;
        }

        let store = DataStore::new(conn, current_id, "/tmp".to_string());
        let dim = Fragility;
        let result = dim.evaluate(&store)?;

        // Should be Info (active development), not Critical
        let active_issue = result.issues.iter().find(|i| i.message.contains("active.rs"));
        assert!(active_issue.is_some(), "should have an issue for active.rs");
        let issue = active_issue.unwrap();
        assert_eq!(issue.level, Level::Info, "active development file should be Info, got {:?}", issue.level);
        assert!(issue.message.contains("active development"), "message should mention active development: {}", issue.message);
        assert!(issue.message.contains("4 consecutive snapshots"), "message should show 4 consecutive snapshots: {}", issue.message);

        // Should NOT have a Critical issue for this file
        assert!(!result.issues.iter().any(|i| i.level == Level::Critical && i.message.contains("active.rs")));

        Ok(())
    }

    #[test]
    fn test_high_churn_stays_critical_without_history() -> Result<()> {
        // File with high churn but only in 1 snapshot — should remain Critical
        let store = test_support::setup_db_store();
        store.conn().execute(
            "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (1, 'new_hot.rs', 20, 800, 400, '2026-04-01')",
            [],
        )?;
        let dim = Fragility;
        let result = dim.evaluate(&store)?;
        assert!(result.issues.iter().any(|i| i.level == Level::Critical && i.message.contains("new_hot.rs")),
            "file without historical presence should remain Critical");
        Ok(())
    }
}
