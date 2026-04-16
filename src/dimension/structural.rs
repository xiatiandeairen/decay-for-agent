use anyhow::{Context, Result};
use log::debug;
use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::DataStore;
use crate::diagnose::{Issue, Level};

// --- Structural thresholds ---
/// Total tracked files in the project. Exceeding this suggests modules need splitting.
/// 500 is roughly where a flat project becomes hard to navigate without tooling.
const FILE_COUNT_WARN: i64 = 500;
/// Critical file count — at 1000+ files, boundaries are almost certainly unclear.
/// Projects of this size typically need a monorepo or sub-crate decomposition.
const FILE_COUNT_CRIT: i64 = 1000;
/// Maximum directory nesting depth. Depth >5 usually means over-segmented hierarchy.
/// Industry standard (e.g. Google style guide) recommends keeping nesting shallow.
const DEPTH_WARN: i64 = 5;
/// Critical nesting depth — 8+ levels make navigation and import paths unwieldy.
/// At this point directory structure rarely reflects real module boundaries.
const DEPTH_CRIT: i64 = 8;
/// Number of distinct top-level directories. Too many indicates missing cohesion.
/// 15+ top-level entries means the root is being used as a catch-all flat namespace.
const TOP_DIRS_WARN: i64 = 15;

pub struct Structural;

impl Dimension for Structural {
    fn name(&self) -> &'static str {
        "structural"
    }

    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult> {
        let conn = store.conn();
        let snapshot_id = store.snapshot_id();
        let mut score: i32 = 100;
        let mut issues = Vec::new();
        let name = self.name().to_string();
        debug!("structural: evaluating");

        // Query once, use for both score and diagnose
        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .with_context(|| format!("structural: failed to count files for snapshot {snapshot_id}"))?;

        if file_count > FILE_COUNT_CRIT {
            score -= 40;
            issues.push(Issue::with_actions(
                Level::Critical, name.clone(), format!("{file_count} files in project"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Split,
                    target: Target::file("src/"),
                    suggestion: "split into sub-modules by responsibility".into(),
                    reason: format!("{file_count} files exceed {FILE_COUNT_CRIT} threshold"),
                    priority: Priority::Critical, effort: Effort::Large,
                    details: vec![],
                    impact: None,
                }],
            ));
        } else if file_count > FILE_COUNT_WARN {
            score -= 20;
            issues.push(Issue::with_actions(
                Level::Warning, name.clone(), format!("{file_count} files in project"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Refactor,
                    target: Target::file("src/"),
                    suggestion: "review directory structure for extractable modules".into(),
                    reason: format!("{file_count} files exceed {FILE_COUNT_WARN} threshold"),
                    priority: Priority::High, effort: Effort::Medium,
                    details: vec![],
                    impact: None,
                }],
            ));
        }

        let max_depth: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(depth), 0) FROM files WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .with_context(|| format!("structural: failed to get max depth for snapshot {snapshot_id}"))?;

        if max_depth > DEPTH_CRIT {
            score -= 30;
        } else if max_depth > DEPTH_WARN {
            score -= 15;
        }
        if max_depth > DEPTH_WARN {
            issues.push(Issue::with_actions(
                Level::Warning, name.clone(), format!("max directory depth is {max_depth}"),
                vec![Action {
                    dimension: name.clone(), action_type: ActionType::Move,
                    target: Target::file("."),
                    suggestion: "flatten nested directories".into(),
                    reason: format!("max depth {max_depth} exceeds {DEPTH_WARN} threshold"),
                    priority: Priority::Medium, effort: Effort::Medium,
                    details: vec![],
                    impact: None,
                }],
            ));
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
            .with_context(|| format!("structural: failed to count top-level dirs for snapshot {snapshot_id}"))?;

        if top_dirs > TOP_DIRS_WARN {
            score -= 15;
            issues.push(Issue::new(
                Level::Info, name, format!("{top_dirs} top-level entries"),
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

    #[test]
    fn test_healthy() -> Result<()> {
        let store = test_support::setup_db_store();
        for i in 0..10 {
            store.conn().execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 1000, 2)",
                [format!("src/file{i}.rs")],
            )?;
        }
        let dim = Structural;
        let result = dim.evaluate(&store)?;
        let score = result.score.unwrap();
        assert!(score > 80, "healthy project should score >80, got {score}");
        let issues = result.issues;
        assert!(issues.is_empty());
        Ok(())
    }

    #[test]
    fn test_unhealthy() -> Result<()> {
        let store = test_support::setup_db_store();
        for i in 0..600 {
            store.conn().execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 1000, 9)",
                [format!("a/b/c/d/e/f/g/h/i/file{i}.rs")],
            )?;
        }
        let dim = Structural;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score < 60, "unhealthy project should score <60, got {score}");
        Ok(())
    }
}
