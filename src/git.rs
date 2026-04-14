use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use git2::Repository;

/// Summary of git history analysis.
#[derive(serde::Serialize)]
pub struct GitSummary {
    pub files_analyzed: usize,
    pub total_commits: usize,
}

/// A single file's change statistics from git history.
pub struct GitChange {
    pub path: String,
    pub change_count: i64,
    pub lines_added: i64,
    pub lines_deleted: i64,
    pub last_modified: String,
}

/// Analyze git history for the given number of days.
/// Returns (changes, summary) without writing to DB.
pub fn collect(project_path: &Path, days: u32) -> Result<(Vec<GitChange>, GitSummary)> {
    let repo = Repository::open(project_path).context("failed to open git repository")?;

    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk.push_head().context("failed to push HEAD")?;
    revwalk
        .set_sorting(git2::Sort::TIME)
        .context("failed to set sorting")?;

    let cutoff = chrono_cutoff(days);
    let mut total_commits = 0;

    // Aggregate per file
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
            if let Some(path) = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
            {
                let entry = file_stats.entry(path).or_insert(FileChange {
                    change_count: 0,
                    lines_added: 0,
                    lines_deleted: 0,
                    last_modified: commit_time,
                });
                entry.change_count += 1;
            }
        }

        diff.foreach(
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
        .context("failed to count line changes")?;
    }

    let files_analyzed = file_stats.len();

    let changes: Vec<GitChange> = file_stats
        .into_iter()
        .map(|(path, stats)| GitChange {
            path,
            change_count: stats.change_count,
            lines_added: stats.lines_added,
            lines_deleted: stats.lines_deleted,
            last_modified: crate::util::format_timestamp(stats.last_modified),
        })
        .collect();

    Ok((
        changes,
        GitSummary {
            files_analyzed,
            total_commits,
        },
    ))
}

struct FileChange {
    change_count: i64,
    lines_added: i64,
    lines_deleted: i64,
    last_modified: i64,
}

fn chrono_cutoff(days: u32) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    now - (days as i64 * 86400)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let project_path = Path::new(env!("CARGO_MANIFEST_DIR"));
        let (changes, summary) = collect(project_path, 90)?;

        assert!(summary.total_commits > 0, "expected commits in repo");
        assert!(summary.files_analyzed > 0, "expected files analyzed");
        assert!(!changes.is_empty(), "expected git changes");

        Ok(())
    }
}
