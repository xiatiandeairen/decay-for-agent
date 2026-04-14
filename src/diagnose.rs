use std::fmt;

use anyhow::{Context, Result};
use rusqlite::Connection;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Critical,
    Warning,
    Info,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Critical => write!(f, "CRITICAL"),
            Level::Warning => write!(f, "WARNING"),
            Level::Info => write!(f, "INFO"),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Structural,
    Complexity,
    Fragility,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Structural => write!(f, "structural"),
            Category::Complexity => write!(f, "complexity"),
            Category::Fragility => write!(f, "fragility"),
        }
    }
}

#[derive(serde::Serialize)]
pub struct Issue {
    pub level: Level,
    pub category: Category,
    pub message: String,
    pub prescription: Option<String>,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  [{}] {}: {}", self.level, self.category, self.message)?;
        if let Some(rx) = &self.prescription {
            write!(f, " — {rx}")?;
        }
        Ok(())
    }
}

/// Run all diagnosis rules and return issues sorted by severity.
pub fn run(conn: &Connection, snapshot_id: i64) -> Result<Vec<Issue>> {
    let mut issues = Vec::new();

    diagnose_structural(conn, snapshot_id, &mut issues)?;
    diagnose_complexity(conn, snapshot_id, &mut issues)?;
    diagnose_fragility(conn, snapshot_id, &mut issues)?;

    issues.sort_by_key(|i| i.level);
    Ok(issues)
}

fn diagnose_structural(conn: &Connection, snapshot_id: i64, issues: &mut Vec<Issue>) -> Result<()> {
    let file_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to count files")?;

    if file_count > 1000 {
        issues.push(Issue {
            level: Level::Critical,
            category: Category::Structural,
            message: format!("{file_count} files in project"),
            prescription: Some("split into sub-modules by responsibility".into()),
        });
    } else if file_count > 500 {
        issues.push(Issue {
            level: Level::Warning,
            category: Category::Structural,
            message: format!("{file_count} files in project"),
            prescription: Some("review directory structure for extractable modules".into()),
        });
    }

    let max_depth: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(depth), 0) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to get max depth")?;

    if max_depth > 5 {
        issues.push(Issue {
            level: Level::Warning,
            category: Category::Structural,
            message: format!("max directory depth is {max_depth}"),
            prescription: Some("flatten nested directories".into()),
        });
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
        .context("failed to count top-level dirs")?;

    if top_dirs > 15 {
        issues.push(Issue {
            level: Level::Info,
            category: Category::Structural,
            message: format!("{top_dirs} top-level entries"),
            prescription: None,
        });
    }

    Ok(())
}

fn diagnose_complexity(conn: &Connection, snapshot_id: i64, issues: &mut Vec<Issue>) -> Result<()> {
    // Large files (>50KB critical, >15KB warning) — report specific files
    let mut stmt = conn
        .prepare(
            "SELECT path, size_bytes FROM files WHERE snapshot_id = ?1 AND size_bytes > 15360 ORDER BY size_bytes DESC",
        )
        .context("failed to prepare large files query")?;

    let large_files: Vec<(String, i64)> = stmt
        .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .context("failed to query large files")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect large files")?;

    for (path, size) in &large_files {
        let size_kb = size / 1024;
        if *size > 51200 {
            issues.push(Issue {
                level: Level::Critical,
                category: Category::Complexity,
                message: format!("{path} ({size_kb}KB)"),
                prescription: Some(format!("split {path} into smaller units")),
            });
        } else {
            issues.push(Issue {
                level: Level::Warning,
                category: Category::Complexity,
                message: format!("{path} ({size_kb}KB)"),
                prescription: Some(format!("extract independent logic from {path}")),
            });
        }
    }

    // Large file ratio
    let file_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to count files")?;

    if file_count > 0 {
        let ratio = large_files.len() as f64 / file_count as f64;
        if ratio > 0.2 {
            let pct = (ratio * 100.0) as i32;
            issues.push(Issue {
                level: Level::Info,
                category: Category::Complexity,
                message: format!("{pct}% of files exceed 15KB"),
                prescription: None,
            });
        }
    }

    Ok(())
}

fn diagnose_fragility(conn: &Connection, snapshot_id: i64, issues: &mut Vec<Issue>) -> Result<()> {
    let file_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM git_changes WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to count git_changes")?;

    if file_count == 0 {
        return Ok(());
    }

    // High churn files (>500 lines)
    let mut stmt = conn
        .prepare(
            "SELECT path, (lines_added + lines_deleted) as churn FROM git_changes WHERE snapshot_id = ?1 AND (lines_added + lines_deleted) > 500 ORDER BY churn DESC",
        )
        .context("failed to prepare churn query")?;

    let high_churn: Vec<(String, i64)> = stmt
        .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .context("failed to query high churn")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect high churn")?;

    for (path, churn) in &high_churn {
        issues.push(Issue {
            level: Level::Critical,
            category: Category::Fragility,
            message: format!("{path} has {churn} lines churn"),
            prescription: Some(format!("split {path} to isolate unstable logic")),
        });
    }

    // Churn concentration
    let total_churn: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(lines_added + lines_deleted), 0) FROM git_changes WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to sum churn")?;

    if total_churn > 0 {
        let top_n = (file_count as f64 * 0.1).ceil().max(1.0) as i64;
        let top_churn: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(churn), 0) FROM (
                    SELECT (lines_added + lines_deleted) as churn
                    FROM git_changes WHERE snapshot_id = ?1
                    ORDER BY churn DESC LIMIT ?2
                )",
                rusqlite::params![snapshot_id, top_n],
                |row| row.get(0),
            )
            .context("failed to get top churn")?;

        let concentration = top_churn as f64 / total_churn as f64;
        if concentration > 0.5 {
            let pct = (concentration * 100.0) as i32;
            issues.push(Issue {
                level: Level::Warning,
                category: Category::Fragility,
                message: format!("top 10% files account for {pct}% of churn"),
                prescription: Some("distribute changes across more files".into()),
            });
        }
    }

    // Frequently changed files (>10 changes)
    let mut freq_stmt = conn
        .prepare(
            "SELECT path, change_count FROM git_changes WHERE snapshot_id = ?1 AND change_count > 10 ORDER BY change_count DESC",
        )
        .context("failed to prepare freq query")?;

    let frequent: Vec<(String, i64)> = freq_stmt
        .query_map([snapshot_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .context("failed to query frequent")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect frequent")?;

    for (path, count) in &frequent {
        issues.push(Issue {
            level: Level::Info,
            category: Category::Fragility,
            message: format!("{path} changed {count} times"),
            prescription: None,
        });
    }

    Ok(())
}

/// Format and print the issues list.
pub fn print_issues(issues: &[Issue]) {
    if issues.is_empty() {
        println!("No issues found.");
        return;
    }

    let critical = issues.iter().filter(|i| i.level == Level::Critical).count();
    let warning = issues.iter().filter(|i| i.level == Level::Warning).count();
    let info = issues.iter().filter(|i| i.level == Level::Info).count();

    let mut parts = Vec::new();
    if critical > 0 {
        parts.push(format!("{critical} critical"));
    }
    if warning > 0 {
        parts.push(format!("{warning} warning"));
    }
    if info > 0 {
        parts.push(format!("{info} info"));
    }

    println!("Issues ({}):", parts.join(", "));
    for issue in issues {
        println!("{issue}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                depth INTEGER NOT NULL
            );
            CREATE TABLE git_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                change_count INTEGER NOT NULL,
                lines_added INTEGER NOT NULL,
                lines_deleted INTEGER NOT NULL,
                last_modified TEXT NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_no_issues_healthy_project() -> Result<()> {
        let conn = setup_db();
        for i in 0..10 {
            conn.execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 3000, 2)",
                [format!("src/file{i}.rs")],
            )?;
        }
        let issues = run(&conn, 1)?;
        assert!(issues.is_empty(), "healthy project should have no issues");
        Ok(())
    }

    #[test]
    fn test_large_file_warning() -> Result<()> {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, 'big.rs', 20000, 1)",
            [],
        )?;
        let issues = run(&conn, 1)?;
        assert!(!issues.is_empty());
        assert!(
            issues
                .iter()
                .any(|i| i.level == Level::Warning && i.message.contains("big.rs"))
        );
        Ok(())
    }

    #[test]
    fn test_high_churn_critical() -> Result<()> {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (1, 'hot.rs', 20, 400, 200, '2026-04-01')",
            [],
        )?;
        let issues = run(&conn, 1)?;
        assert!(
            issues
                .iter()
                .any(|i| i.level == Level::Critical && i.message.contains("hot.rs"))
        );
        Ok(())
    }

    #[test]
    fn test_issues_sorted_by_level() -> Result<()> {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, 'big.rs', 60000, 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, 'med.rs', 20000, 1)",
            [],
        )?;
        let issues = run(&conn, 1)?;
        assert!(issues.len() >= 2);
        // Critical should come before Warning
        let first_critical = issues.iter().position(|i| i.level == Level::Critical);
        let first_warning = issues.iter().position(|i| i.level == Level::Warning);
        if let (Some(c), Some(w)) = (first_critical, first_warning) {
            assert!(c < w, "critical should come before warning");
        }
        Ok(())
    }
}
