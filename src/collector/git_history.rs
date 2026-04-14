use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use super::{Collector, CollectorSummary};
use crate::git;

pub struct GitHistory;

impl Collector for GitHistory {
    fn name(&self) -> &'static str {
        "git_history"
    }

    fn ensure_schema(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS git_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
                path TEXT NOT NULL,
                change_count INTEGER NOT NULL,
                lines_added INTEGER NOT NULL,
                lines_deleted INTEGER NOT NULL,
                last_modified TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    fn available(&self, project_path: &Path) -> bool {
        project_path.join(".git").exists()
    }

    fn collect(
        &self,
        conn: &Connection,
        snapshot_id: i64,
        project_path: &Path,
    ) -> Result<CollectorSummary> {
        let summary = git::collect(conn, snapshot_id, project_path, 90)?;
        let mut stats = HashMap::new();
        stats.insert("commits".to_string(), summary.total_commits.to_string());
        stats.insert(
            "files_analyzed".to_string(),
            summary.files_analyzed.to_string(),
        );
        Ok(CollectorSummary {
            name: self.name().to_string(),
            stats,
        })
    }
}
