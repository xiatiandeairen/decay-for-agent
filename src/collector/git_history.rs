use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use super::{Collector, CollectorSummary};
use crate::git;
use crate::git_pipeline::{self, GitFilterContext};

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
        // Collect raw git changes
        let (changes, summary) = git::collect(project_path, 90)?;
        debug!(
            "git_history: {} raw changes from {} commits",
            changes.len(),
            summary.total_commits
        );

        // Detect primary languages from already-scanned files
        let primary_languages = detect_primary_languages(conn, snapshot_id);
        debug!("git_history: primary languages for filter: {primary_languages:?}");

        // Filter through git pipeline
        let ctx = GitFilterContext { primary_languages };
        let filtered = git_pipeline::run_pipeline(changes, &ctx);
        debug!("git_history: {} changes after filtering", filtered.len());

        // Write filtered changes to DB
        let mut stmt = conn
            .prepare(
                "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .context("failed to prepare git_changes insert")?;

        for change in &filtered {
            stmt.execute(rusqlite::params![
                snapshot_id,
                change.path,
                change.change_count,
                change.lines_added,
                change.lines_deleted,
                change.last_modified,
            ])
            .context("failed to insert git_changes record")?;
        }

        let mut stats = HashMap::new();
        stats.insert("commits".to_string(), summary.total_commits.to_string());
        stats.insert("files_analyzed".to_string(), filtered.len().to_string());
        Ok(CollectorSummary {
            name: self.name().to_string(),
            stats,
        })
    }
}

/// Detect primary languages from files already in the DB (from file_scan collector).
fn detect_primary_languages(conn: &Connection, snapshot_id: i64) -> Vec<String> {
    let mut stmt = match conn.prepare("SELECT path FROM files WHERE snapshot_id = ?1") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let paths: Vec<String> = stmt
        .query_map([snapshot_id], |row| row.get(0))
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    // Count extensions by language group
    let mut group_counts: HashMap<&str, usize> = HashMap::new();
    let mut total = 0;

    for path in &paths {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        for group in crate::filter_pipeline::LANGUAGE_GROUPS {
            if group.extensions.contains(&ext.as_str()) {
                *group_counts.entry(group.name).or_default() += 1;
                total += 1;
                break;
            }
        }
    }

    if total == 0 {
        return vec![];
    }

    group_counts
        .iter()
        .filter(|(_, count)| **count as f64 / total as f64 >= 0.10)
        .map(|(name, _)| name.to_string())
        .collect()
}
