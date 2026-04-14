use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use super::{Collector, CollectorSummary};
use crate::scan;

pub struct FileScan;

impl Collector for FileScan {
    fn name(&self) -> &'static str {
        "file_scan"
    }

    fn ensure_schema(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
                path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                depth INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    fn available(&self, _project_path: &Path) -> bool {
        true // file scan always available
    }

    fn collect(
        &self,
        conn: &Connection,
        snapshot_id: i64,
        project_path: &Path,
    ) -> Result<CollectorSummary> {
        let summary = scan::collect(conn, snapshot_id, project_path)?;
        let mut stats = HashMap::new();
        stats.insert("files".to_string(), summary.file_count.to_string());
        stats.insert("dirs".to_string(), summary.dir_count.to_string());
        stats.insert("max_depth".to_string(), summary.max_depth.to_string());
        Ok(CollectorSummary {
            name: self.name().to_string(),
            stats,
        })
    }
}
