pub mod complexity;
pub mod fragility;
pub mod structural;

use anyhow::Result;
use rusqlite::Connection;

use crate::diagnose::Issue;

/// Result of evaluating a single dimension.
pub struct DimensionResult {
    pub name: String,
    /// None = data source unavailable (e.g. no git history for fragility).
    pub score: Option<i32>,
    pub issues: Vec<Issue>,
}

/// A measurable dimension of code health.
pub trait Dimension: Send + Sync {
    /// Dimension name, used for output and persistence.
    fn name(&self) -> &'static str;

    /// Compute health score (0-100, deduction-based).
    /// Returns None when the required data source is unavailable.
    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>>;

    /// Diagnose issues and generate prescriptions.
    fn diagnose(&self, conn: &Connection, snapshot_id: i64) -> Result<Vec<Issue>>;

    /// Run score + diagnose in one call.
    fn evaluate(&self, conn: &Connection, snapshot_id: i64) -> Result<DimensionResult> {
        Ok(DimensionResult {
            name: self.name().to_string(),
            score: self.score(conn, snapshot_id)?,
            issues: self.diagnose(conn, snapshot_id)?,
        })
    }
}

/// Registry of all enabled dimensions.
pub fn all_dimensions() -> Vec<Box<dyn Dimension>> {
    vec![
        Box::new(structural::Structural),
        Box::new(complexity::Complexity),
        Box::new(fragility::Fragility),
    ]
}
