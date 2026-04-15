use anyhow::{Context, Result};
use log::debug;

use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::DataStore;
use crate::diagnose::{Issue, Level};

// --- Complexity thresholds ---
/// Files larger than 15 KB (≈300–400 lines) are treated as complex.
/// File size is a proxy for cyclomatic complexity when AST data is unavailable.
const LARGE_FILE_BYTES: i64 = 15360;
/// Fraction of files exceeding LARGE_FILE_BYTES that triggers a score penalty.
/// 20% means 1 in 5 files is oversized — a pattern, not an outlier.
const LARGE_RATIO_WARN: f64 = 0.2;
/// Critical threshold: 40%+ oversized files means complexity is systemic.
/// At this ratio, the codebase likely needs broad refactoring.
const LARGE_RATIO_CRIT: f64 = 0.4;
/// Average file size across the project. 10 KB average signals consistently dense files.
/// A healthy average is below 5–8 KB, keeping most files focused and readable.
const AVG_SIZE_WARN: f64 = 10240.0;
/// Single-file ceiling: 50 KB (~1500+ lines) indicates a god object or generated code.
/// Files this large are rarely maintainable and almost always need splitting.
const MAX_SIZE_WARN: i64 = 51200;

pub struct Complexity;

impl Dimension for Complexity {
    fn name(&self) -> &'static str {
        "complexity"
    }

    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult> {
        let conn = store.conn();
        let snapshot_id = store.snapshot_id();
        let mut score: i32 = 100;
        let mut issues = Vec::new();
        let name = self.name().to_string();
        debug!("complexity: evaluating");

        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .with_context(|| format!("complexity: failed to count files for snapshot {snapshot_id}"))?;

        if file_count == 0 {
            return Ok(DimensionResult { name, score: Some(100), issues });
        }

        // Query large files once for both score and diagnose
        let mut stmt = conn
            .prepare(
                &format!("SELECT path, size_bytes FROM files WHERE snapshot_id = ?1 AND size_bytes > {LARGE_FILE_BYTES} ORDER BY size_bytes DESC"),
            )
            .with_context(|| format!("complexity: failed to prepare large files query for snapshot {snapshot_id}"))?;

        let large_files: Vec<(String, i64)> = stmt
            .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .with_context(|| format!("complexity: failed to query large files for snapshot {snapshot_id}"))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("complexity: failed to collect large files for snapshot {snapshot_id}"))?;

        // Score: large file ratio
        let large_ratio = large_files.len() as f64 / file_count as f64;
        if large_ratio > LARGE_RATIO_CRIT {
            score -= 45;
        } else if large_ratio > LARGE_RATIO_WARN {
            score -= 25;
        }

        // Score: avg size
        let avg_size: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(size_bytes), 0) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .with_context(|| format!("complexity: failed to get avg file size for snapshot {snapshot_id}"))?;

        if avg_size > AVG_SIZE_WARN {
            score -= 15;
        }

        // Score: max size
        let max_size: i64 = large_files.iter().map(|(_, s)| *s).max().unwrap_or(0);
        if max_size > MAX_SIZE_WARN {
            score -= 10;
        }

        // Diagnose: individual large files
        for (path, size) in &large_files {
            let size_kb = size / 1024;
            if *size > MAX_SIZE_WARN {
                issues.push(Issue::with_actions(
                    Level::Critical,
                    name.clone(),
                    format!("{path} ({size_kb}KB)"),
                    Some(format!("split {path} into smaller units")),
                    vec![Action {
                        dimension: name.clone(),
                        action_type: ActionType::Split,
                        target: Target { file: path.clone(), line_range: None, symbol: None },
                        reason: format!("{path} is {size_kb}KB, exceeds 50KB threshold"),
                        priority: Priority::Critical,
                        effort: Effort::Large,
                    }],
                ));
            } else {
                issues.push(Issue::with_actions(
                    Level::Warning,
                    name.clone(),
                    format!("{path} ({size_kb}KB)"),
                    Some(format!("extract independent logic from {path}")),
                    vec![Action {
                        dimension: name.clone(),
                        action_type: ActionType::Extract,
                        target: Target { file: path.clone(), line_range: None, symbol: None },
                        reason: format!("{path} is {size_kb}KB, extract independent logic"),
                        priority: Priority::High,
                        effort: Effort::Medium,
                    }],
                ));
            }
        }

        // Diagnose: ratio info
        if large_ratio > LARGE_RATIO_WARN {
            let pct = (large_ratio * 100.0) as i32;
            issues.push(Issue::new(
                Level::Info,
                name,
                format!("{pct}% of files exceed 15KB"),
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
    fn test_healthy() -> Result<()> {
        let store = setup_store();
        for i in 0..20 {
            store.conn().execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 3000, 2)",
                [format!("src/file{i}.rs")],
            )?;
        }
        let dim = Complexity;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score > 80, "healthy complexity should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_large_file_warning() -> Result<()> {
        let store = setup_store();
        store.conn().execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, 'big.rs', 20000, 1)",
            [],
        )?;
        let dim = Complexity;
        let issues = dim.evaluate(&store)?.issues;
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.level == Level::Warning && i.message.contains("big.rs")));
        Ok(())
    }
}
