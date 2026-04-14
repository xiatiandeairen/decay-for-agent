use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use super::Dimension;
use crate::diagnose::{Issue, Level};

pub struct Reliability;

impl Dimension for Reliability {
    fn name(&self) -> &'static str {
        "reliability"
    }

    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>> {
        let project_path = get_project_path(conn, snapshot_id)?;
        let files = get_source_files(conn, snapshot_id)?;
        if files.is_empty() {
            return Ok(Some(100));
        }

        let analysis = analyze(&project_path, &files);
        let mut score: i32 = 100;
        debug!("reliability: {} files, {} lines", analysis.file_count, analysis.total_lines);

        // Unsafe/eval density
        if analysis.total_lines > 0 {
            let density = analysis.unsafe_count as f64 / (analysis.total_lines as f64 / 1000.0);
            if density > 8.0 {
                score -= 30;
            } else if density > 2.0 {
                score -= 15;
            }
        }

        // SQL/shell injection patterns
        let injection_penalty = (analysis.injection_patterns * 20).min(40) as i32;
        score -= injection_penalty;

        // Hardcoded secrets
        let secret_penalty = (analysis.hardcoded_secrets * 15).min(30) as i32;
        score -= secret_penalty;

        // Dependency count
        if analysis.dependency_count > 100 {
            score -= 20;
        } else if analysis.dependency_count > 50 {
            score -= 10;
        }

        Ok(Some(score.max(0)))
    }

    fn diagnose(&self, conn: &Connection, snapshot_id: i64) -> Result<Vec<Issue>> {
        let project_path = get_project_path(conn, snapshot_id)?;
        let files = get_source_files(conn, snapshot_id)?;
        if files.is_empty() {
            return Ok(vec![]);
        }

        let analysis = analyze(&project_path, &files);
        let mut issues = Vec::new();
        let cat = self.name().to_string();

        // Report files with unsafe/eval
        for (path, count) in &analysis.unsafe_details {
            if *count > 3 {
                issues.push(Issue {
                    level: Level::Warning,
                    category: cat.clone(),
                    message: format!("{path} has {count} unsafe/eval occurrences"),
                    prescription: Some(format!("minimize unsafe code in {path}, prefer safe abstractions")),
                });
            }
        }

        // Injection patterns
        for (path, pattern) in &analysis.injection_details {
            issues.push(Issue {
                level: Level::Critical,
                category: cat.clone(),
                message: format!("{path}: potential {pattern}"),
                prescription: Some("use parameterized queries or safe command execution".to_string()),
            });
        }

        // Hardcoded secrets
        for (path, kind) in &analysis.secret_details {
            issues.push(Issue {
                level: Level::Critical,
                category: cat.clone(),
                message: format!("{path}: {kind}"),
                prescription: Some("use environment variables or secret management for credentials".to_string()),
            });
        }

        // Dependency count
        if analysis.dependency_count > 50 {
            issues.push(Issue {
                level: Level::Info,
                category: cat,
                message: format!("{} direct dependencies", analysis.dependency_count),
                prescription: Some("audit dependencies for necessity, remove unused ones".to_string()),
            });
        }

        Ok(issues)
    }
}

struct Analysis {
    file_count: usize,
    total_lines: usize,
    unsafe_count: usize,
    injection_patterns: usize,
    hardcoded_secrets: usize,
    dependency_count: usize,
    unsafe_details: Vec<(String, usize)>,
    injection_details: Vec<(String, String)>,
    secret_details: Vec<(String, String)>,
}

fn analyze(project_path: &str, rel_paths: &[String]) -> Analysis {
    let base = Path::new(project_path);
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut unsafe_count = 0;
    let mut injection_patterns = 0;
    let mut hardcoded_secrets = 0;
    let mut unsafe_details = Vec::new();
    let mut injection_details = Vec::new();
    let mut secret_details: Vec<(String, String)> = Vec::new();

    let unsafe_patterns = ["unsafe {", "unsafe{", "eval(", "exec(", "Function("];

    for rel_path in rel_paths {
        let abs_path = base.join(rel_path);
        let Ok(content) = std::fs::read_to_string(&abs_path) else {
            continue;
        };

        file_count += 1;
        let lines: Vec<&str> = content.lines().collect();
        total_lines += lines.len();
        let mut file_unsafe = 0;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            // Unsafe/eval
            for pat in &unsafe_patterns {
                if trimmed.contains(pat) {
                    unsafe_count += 1;
                    file_unsafe += 1;
                }
            }

            // SQL injection: string concatenation in SQL context
            if (trimmed.contains("format!(") || trimmed.contains("f\""))
                && (trimmed.to_uppercase().contains("SELECT ")
                    || trimmed.to_uppercase().contains("INSERT ")
                    || trimmed.to_uppercase().contains("DELETE ")
                    || trimmed.to_uppercase().contains("UPDATE "))
            {
                injection_patterns += 1;
                injection_details.push((rel_path.clone(), "SQL string concatenation".to_string()));
            }

            // Shell injection
            if (trimmed.contains("Command::new") || trimmed.contains("subprocess") || trimmed.contains("os.system"))
                && (trimmed.contains("format!(") || trimmed.contains("f\"") || trimmed.contains("+ "))
            {
                injection_patterns += 1;
                injection_details.push((rel_path.clone(), "shell command injection risk".to_string()));
            }

            // Hardcoded secrets
            let lower = trimmed.to_lowercase();
            if (lower.contains("password") || lower.contains("secret") || lower.contains("api_key") || lower.contains("apikey"))
                && (lower.contains("= \"") || lower.contains("= '"))
                && !lower.contains("env") && !lower.contains("config") && !lower.contains("example")
            {
                hardcoded_secrets += 1;
                secret_details.push((rel_path.clone(), "hardcoded credential detected".to_string()));
            }
        }

        if file_unsafe > 0 {
            unsafe_details.push((rel_path.clone(), file_unsafe));
        }
    }

    // Count dependencies
    let dependency_count = count_dependencies(base);

    Analysis {
        file_count,
        total_lines,
        unsafe_count,
        injection_patterns,
        hardcoded_secrets,
        dependency_count,
        unsafe_details,
        injection_details,
        secret_details,
    }
}

fn count_dependencies(project_path: &Path) -> usize {
    // Rust: count [dependencies] entries in Cargo.toml
    if let Ok(content) = std::fs::read_to_string(project_path.join("Cargo.toml")) {
        let mut in_deps = false;
        let mut count = 0;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" || trimmed == "[dev-dependencies]" {
                in_deps = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_deps = false;
                continue;
            }
            if in_deps && trimmed.contains('=') && !trimmed.starts_with('#') {
                count += 1;
            }
        }
        return count;
    }

    // Node: count dependencies in package.json
    if let Ok(content) = std::fs::read_to_string(project_path.join("package.json")) {
        return content.matches("\":").count().saturating_sub(5); // rough estimate
    }

    0
}

fn get_project_path(conn: &Connection, snapshot_id: i64) -> Result<String> {
    conn.query_row("SELECT project_path FROM snapshots WHERE id = ?1", [snapshot_id], |row| row.get(0))
        .context("failed to get project path")
}

fn get_source_files(conn: &Connection, snapshot_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM files WHERE snapshot_id = ?1")
        .context("failed to prepare file paths query")?;
    let paths: Vec<String> = stmt.query_map([snapshot_id], |row| row.get(0))
        .context("failed to query file paths")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect file paths")?;
    let src_ext = [".rs", ".py", ".js", ".ts", ".tsx", ".jsx", ".go", ".java", ".kt", ".swift", ".rb", ".php", ".c", ".cpp", ".cs"];
    Ok(paths.into_iter().filter(|p| src_ext.iter().any(|e| p.ends_with(e))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_db(dir: &TempDir) -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, project_path TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), version TEXT NOT NULL);
             CREATE TABLE files (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, size_bytes INTEGER NOT NULL, depth INTEGER NOT NULL);",
        ).unwrap();
        conn.execute("INSERT INTO snapshots (project_path, version) VALUES (?1, '0.1.0')", [dir.path().to_string_lossy().to_string()]).unwrap();
        let sid = conn.last_insert_rowid();
        (conn, sid)
    }

    fn add_file(conn: &Connection, sid: i64, dir: &TempDir, path: &str, content: &str) {
        fs::create_dir_all(dir.path().join(path).parent().unwrap()).unwrap();
        fs::write(dir.path().join(path), content).unwrap();
        conn.execute("INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, ?2, ?3, 1)", rusqlite::params![sid, path, content.len()]).unwrap();
    }

    #[test]
    fn test_safe_project() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db(&dir);
        add_file(&conn, sid, &dir, "src/main.rs", "fn main() {\n    let x = 42;\n}\n");
        let dim = Reliability;
        let score = dim.score(&conn, sid)?.unwrap();
        assert!(score > 80, "safe project should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_unsafe_code() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db(&dir);
        let content = (0..10).map(|_| "unsafe { std::ptr::null() };").collect::<Vec<_>>().join("\n");
        add_file(&conn, sid, &dir, "src/main.rs", &content);
        let dim = Reliability;
        let score = dim.score(&conn, sid)?.unwrap();
        assert!(score < 80, "unsafe project should score <80, got {score}");
        Ok(())
    }
}
