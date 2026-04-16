/// Before/after snapshot comparison for feedback verification.
///
/// Compares dimension scores between two snapshots, showing improvement,
/// regression, and unchanged dimensions.

use std::collections::HashMap;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

/// Score change for a single dimension.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreChange {
    pub dimension: String,
    pub before: i32,
    pub after: i32,
    pub change: i32,
    pub status: ChangeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    Improved,
    Regressed,
    Unchanged,
}

impl std::fmt::Display for ChangeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeStatus::Improved => write!(f, "✅ improved"),
            ChangeStatus::Regressed => write!(f, "⚠ regressed"),
            ChangeStatus::Unchanged => write!(f, "→ unchanged"),
        }
    }
}

/// Full comparison report between two snapshots.
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonReport {
    pub before_snapshot: i64,
    pub after_snapshot: i64,
    pub changes: Vec<ScoreChange>,
    pub summary: String,
}

/// Compare dimension scores between two snapshots.
pub fn compare_snapshots(
    conn: &Connection,
    before_id: i64,
    after_id: i64,
) -> Result<ComparisonReport> {
    let before_scores = load_scores(conn, before_id)?;
    let after_scores = load_scores(conn, after_id)?;

    let mut changes = Vec::new();
    let mut improved = 0;
    let mut regressed = 0;

    // Compare all dimensions present in either snapshot
    let mut all_dims: Vec<&String> = after_scores.keys().collect();
    all_dims.sort();

    for dim in all_dims {
        let after = after_scores.get(dim).copied().flatten().unwrap_or(0);
        let before = before_scores.get(dim).copied().flatten().unwrap_or(0);
        let change = after - before;
        let status = if change > 0 {
            improved += 1;
            ChangeStatus::Improved
        } else if change < 0 {
            regressed += 1;
            ChangeStatus::Regressed
        } else {
            ChangeStatus::Unchanged
        };

        changes.push(ScoreChange {
            dimension: dim.clone(),
            before,
            after,
            change,
            status,
        });
    }

    let summary = format!(
        "{improved} improved, {regressed} regressed, {} unchanged",
        changes.len() - improved - regressed,
    );

    Ok(ComparisonReport {
        before_snapshot: before_id,
        after_snapshot: after_id,
        changes,
        summary,
    })
}

fn load_scores(conn: &Connection, snapshot_id: i64) -> Result<HashMap<String, Option<i32>>> {
    let mut stmt = conn
        .prepare("SELECT dimension, score FROM dimension_scores WHERE snapshot_id = ?1")
        .context("failed to prepare scores query")?;

    let rows: Vec<(String, Option<i32>)> = stmt
        .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .context("failed to query scores")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect scores")?;

    Ok(rows.into_iter().collect())
}

/// Print comparison report to terminal.
pub fn print_comparison(report: &ComparisonReport) {
    println!(
        "Comparing snapshot #{} → #{}",
        report.before_snapshot, report.after_snapshot
    );
    println!();
    for c in &report.changes {
        let sign = if c.change > 0 { "+" } else { "" };
        println!(
            "  {:<20} {} → {} ({sign}{}) {}",
            c.dimension, c.before, c.after, c.change, c.status
        );
    }
    println!();
    println!("Summary: {}", report.summary);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE snapshots (id INTEGER PRIMARY KEY, project_path TEXT NOT NULL, created_at TEXT DEFAULT '', version TEXT DEFAULT '');
             CREATE TABLE dimension_scores (id INTEGER PRIMARY KEY, snapshot_id INTEGER NOT NULL, dimension TEXT NOT NULL, score INTEGER);",
        ).unwrap();
        conn
    }

    #[test]
    fn test_compare_improvement() -> Result<()> {
        let conn = setup_db();
        conn.execute("INSERT INTO snapshots (id, project_path) VALUES (1, '/test')", [])?;
        conn.execute("INSERT INTO snapshots (id, project_path) VALUES (2, '/test')", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (1, 'structural', 70)", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (2, 'structural', 85)", [])?;

        let report = compare_snapshots(&conn, 1, 2)?;
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].change, 15);
        assert_eq!(report.changes[0].status, ChangeStatus::Improved);
        Ok(())
    }

    #[test]
    fn test_compare_regression() -> Result<()> {
        let conn = setup_db();
        conn.execute("INSERT INTO snapshots (id, project_path) VALUES (1, '/test')", [])?;
        conn.execute("INSERT INTO snapshots (id, project_path) VALUES (2, '/test')", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (1, 'quality', 90)", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (2, 'quality', 75)", [])?;

        let report = compare_snapshots(&conn, 1, 2)?;
        assert_eq!(report.changes[0].status, ChangeStatus::Regressed);
        Ok(())
    }

    #[test]
    fn test_compare_summary() -> Result<()> {
        let conn = setup_db();
        conn.execute("INSERT INTO snapshots (id, project_path) VALUES (1, '/test')", [])?;
        conn.execute("INSERT INTO snapshots (id, project_path) VALUES (2, '/test')", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (1, 'a', 70)", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (1, 'b', 80)", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (2, 'a', 85)", [])?;
        conn.execute("INSERT INTO dimension_scores (snapshot_id, dimension, score) VALUES (2, 'b', 80)", [])?;

        let report = compare_snapshots(&conn, 1, 2)?;
        assert!(report.summary.contains("1 improved"));
        assert!(report.summary.contains("1 unchanged"));
        Ok(())
    }
}
