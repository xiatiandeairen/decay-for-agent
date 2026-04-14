use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use git2::Repository;
use rusqlite::Connection;

/// Summary of git history analysis.
pub struct GitSummary {
    pub files_analyzed: usize,
    pub total_commits: usize,
}

/// Analyze git history for the given number of days and write to git_changes table.
pub fn collect(
    conn: &Connection,
    snapshot_id: i64,
    project_path: &Path,
    days: u32,
) -> Result<GitSummary> {
    let repo = Repository::open(project_path).context("failed to open git repository")?;

    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk.push_head().context("failed to push HEAD")?;
    revwalk
        .set_sorting(git2::Sort::TIME)
        .context("failed to set sorting")?;

    let cutoff = chrono_cutoff(days);
    let mut total_commits = 0;

    // Aggregate per file: change_count, lines_added, lines_deleted, last_modified
    let mut file_stats: HashMap<String, FileChange> = HashMap::new();

    for oid in revwalk {
        let oid = oid.context("failed to read oid")?;
        let commit = repo.find_commit(oid).context("failed to find commit")?;
        let commit_time = commit.time().seconds();

        if commit_time < cutoff {
            break;
        }

        total_commits += 1;

        let tree = commit.tree().context("failed to get commit tree")?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .context("failed to diff trees")?;

        for delta in diff.deltas() {
            let file_path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string());

            if let Some(path) = file_path {
                let entry = file_stats.entry(path).or_insert(FileChange {
                    change_count: 0,
                    lines_added: 0,
                    lines_deleted: 0,
                    last_modified: commit_time,
                });
                entry.change_count += 1;
                // last_modified is the most recent (first seen due to TIME sort)
            }
        }

        // Get line-level stats via patch
        let mut line_stats_opts = git2::DiffOptions::new();
        let diff_with_lines = repo
            .diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&tree),
                Some(&mut line_stats_opts),
            )
            .context("failed to diff for line stats")?;

        diff_with_lines
            .foreach(
                &mut |_, _| true,
                None,
                None,
                Some(&mut |delta, _hunk, line| {
                    let file_path = delta
                        .new_file()
                        .path()
                        .or_else(|| delta.old_file().path())
                        .map(|p| p.to_string_lossy().to_string());

                    if let Some(entry) = file_path.and_then(|path| file_stats.get_mut(&path)) {
                        match line.origin() {
                            '+' => entry.lines_added += 1,
                            '-' => entry.lines_deleted += 1,
                            _ => {}
                        }
                    }
                    true
                }),
            )
            .context("failed to iterate diff lines")?;
    }

    let files_analyzed = file_stats.len();

    for (path, stats) in &file_stats {
        let last_modified = format_timestamp(stats.last_modified);
        conn.execute(
            "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                snapshot_id,
                path,
                stats.change_count,
                stats.lines_added,
                stats.lines_deleted,
                last_modified,
            ],
        )
        .context("failed to insert git_changes record")?;
    }

    Ok(GitSummary {
        files_analyzed,
        total_commits,
    })
}

struct FileChange {
    change_count: i64,
    lines_added: i64,
    lines_deleted: i64,
    last_modified: i64,
}

/// Calculate the unix timestamp for N days ago.
fn chrono_cutoff(days: u32) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    now - (days as i64 * 86400)
}

/// Format a unix timestamp as ISO 8601 string.
fn format_timestamp(ts: i64) -> String {
    // Simple UTC formatting without external crate
    let secs_per_day: i64 = 86400;
    let days = ts / secs_per_day;
    let remaining = ts % secs_per_day;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Days since epoch to year-month-day (simplified)
    let mut y = 1970;
    let mut d = days;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let months_days: &[i64] = if is_leap_year(y) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in months_days {
        if d < md {
            break;
        }
        d -= md;
        m += 1;
    }

    format!(
        "{y:04}-{m:02}-{:02}T{hours:02}:{minutes:02}:{seconds:02}Z",
        d + 1
    )
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        // 2026-01-01 00:00:00 UTC = 1767225600
        let ts = 1767225600;
        let result = format_timestamp(ts);
        assert_eq!(result, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn test_chrono_cutoff() {
        let cutoff = chrono_cutoff(90);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(cutoff < now);
        assert!(now - cutoff >= 90 * 86400);
        assert!(now - cutoff < 91 * 86400);
    }

    #[test]
    fn test_collect_on_current_repo() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE git_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                change_count INTEGER NOT NULL,
                lines_added INTEGER NOT NULL,
                lines_deleted INTEGER NOT NULL,
                last_modified TEXT NOT NULL
            );",
        )?;

        let project_path = Path::new(env!("CARGO_MANIFEST_DIR"));
        let summary = collect(&conn, 1, project_path, 90)?;

        // We should have at least some commits and files in this repo
        assert!(summary.total_commits > 0, "expected commits in repo");
        assert!(summary.files_analyzed > 0, "expected files analyzed");

        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM git_changes", [], |row| row.get(0))?;
        assert!(count > 0, "expected git_changes records");

        Ok(())
    }
}
