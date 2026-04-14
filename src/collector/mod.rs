pub mod file_scan;
pub mod git_history;

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

/// Summary produced by a collector after data gathering.
pub struct CollectorSummary {
    pub name: String,
    pub stats: HashMap<String, String>,
}

/// A pluggable data source for code health analysis.
///
/// Collectors gather raw data and write it to DB tables.
/// Dimensions read from those tables to compute scores.
pub trait Collector: Send + Sync {
    /// Collector name, used for logging and error reporting.
    fn name(&self) -> &'static str;

    /// Ensure required DB tables exist.
    fn ensure_schema(&self, conn: &Connection) -> Result<()>;

    /// Whether this collector can run in the given project.
    fn available(&self, project_path: &Path) -> bool;

    /// Collect data and write to DB.
    fn collect(
        &self,
        conn: &Connection,
        snapshot_id: i64,
        project_path: &Path,
    ) -> Result<CollectorSummary>;
}

/// Registry of all enabled collectors.
pub fn all_collectors() -> Vec<Box<dyn Collector>> {
    vec![
        Box::new(file_scan::FileScan),
        Box::new(git_history::GitHistory),
    ]
}
