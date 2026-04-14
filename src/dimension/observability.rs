use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use super::Dimension;
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
const UNWRAP_DENSITY_WARN: f64 = 5.0;  // per 1K lines
const UNWRAP_DENSITY_CRIT: f64 = 15.0;
const HARDCODED_CONFIG_WARN: usize = 5;

pub struct Observability;

impl Dimension for Observability {
    fn name(&self) -> &'static str {
        "observability"
    }

    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>> {
        let project_path = get_project_path(conn, snapshot_id)?;
        let files = get_source_files(conn, snapshot_id)?;
        if files.is_empty() {
            return Ok(Some(100));
        }

        let analysis = analyze(&project_path, &files);
        let mut score: i32 = 100;
        debug!("observability: {} files, {} lines", analysis.file_count, analysis.total_lines);

        // Unwrap/panic density
        if analysis.total_lines > 0 {
            let density = analysis.unwrap_panic_count as f64 / (analysis.total_lines as f64 / 1000.0);
            if density > UNWRAP_DENSITY_CRIT {
                score -= 30;
            } else if density > UNWRAP_DENSITY_WARN {
                score -= 15;
            }
        }

        // No logging framework
        if !analysis.has_logging {
            score -= 20;
        }

        // Error swallowing
        if analysis.total_catches > 0 {
            let swallow_ratio = analysis.empty_catches as f64 / analysis.total_catches as f64;
            if swallow_ratio > 0.2 {
                score -= 15;
            }
        }

        // Hardcoded config
        if analysis.hardcoded_configs > HARDCODED_CONFIG_WARN {
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

        // Report files with high unwrap/panic density
        for (path, count) in &analysis.unwrap_details {
            if *count > 5 {
                issues.push(Issue {
                    level: Level::Warning,
                    category: cat.clone(),
                    message: format!("{path} has {count} unwrap/panic calls"),
                    prescription: Some(format!("replace unwrap/panic in {path} with proper error handling")),
                });
            }
        }

        if !analysis.has_logging {
            issues.push(Issue {
                level: Level::Warning,
                category: cat.clone(),
                message: "no logging framework detected in project".to_string(),
                prescription: Some("add structured logging (e.g. log/tracing/slog for Rust, logging for Python)".to_string()),
            });
        }

        if analysis.empty_catches > 0 {
            issues.push(Issue {
                level: Level::Warning,
                category: cat.clone(),
                message: format!("{} empty catch/except blocks detected", analysis.empty_catches),
                prescription: Some("handle or log errors instead of silently swallowing them".to_string()),
            });
        }

        if analysis.hardcoded_configs > 0 {
            issues.push(Issue {
                level: Level::Info,
                category: cat,
                message: format!("{} hardcoded configuration values detected", analysis.hardcoded_configs),
                prescription: Some("externalize configuration using environment variables or config files".to_string()),
            });
        }

        Ok(issues)
    }
}

struct Analysis {
    file_count: usize,
    total_lines: usize,
    unwrap_panic_count: usize,
    has_logging: bool,
    total_catches: usize,
    empty_catches: usize,
    hardcoded_configs: usize,
    unwrap_details: Vec<(String, usize)>,
}

fn analyze(project_path: &str, rel_paths: &[String]) -> Analysis {
    let base = Path::new(project_path);
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut unwrap_panic_count = 0;
    let mut has_logging = false;
    let mut total_catches = 0;
    let mut empty_catches = 0;
    let mut hardcoded_configs = 0;
    let mut unwrap_details = Vec::new();

    let log_patterns = ["log::", "tracing::", "slog::", "env_logger", "log4", "logger.",
        "logging.", "console.log", "console.error", "console.warn", "print!", "println!",
        "eprintln!", "debug!", "info!", "warn!", "error!"];

    let unwrap_patterns = [".unwrap()", ".expect(", "panic!(", "unreachable!(", "unimplemented!(",
        "todo!("];

    let catch_patterns = ["catch", "except", "rescue"];

    for rel_path in rel_paths {
        let abs_path = base.join(rel_path);
        let Ok(content) = std::fs::read_to_string(&abs_path) else {
            continue;
        };

        file_count += 1;
        let lines: Vec<&str> = content.lines().collect();
        total_lines += lines.len();

        let mut file_unwraps = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check logging
            if !has_logging {
                for pat in &log_patterns {
                    if trimmed.contains(pat) {
                        has_logging = true;
                        break;
                    }
                }
            }

            // Unwrap/panic count
            for pat in &unwrap_patterns {
                if trimmed.contains(pat) {
                    unwrap_panic_count += 1;
                    file_unwraps += 1;
                }
            }

            // Catch blocks
            for pat in &catch_patterns {
                if trimmed.starts_with(pat) || trimmed.contains(&format!("}} {pat}")) || trimmed.contains(&format!("{pat} ")) {
                    total_catches += 1;
                    // Check if next non-empty line is just closing brace (empty catch)
                    let next_content = lines.get(i + 1).map(|l| l.trim()).unwrap_or("");
                    if next_content == "}" || next_content == "pass" || next_content.is_empty() {
                        empty_catches += 1;
                    }
                }
            }

            // Hardcoded config patterns
            if (trimmed.contains("http://") || trimmed.contains("https://"))
                && !trimmed.starts_with("//")
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("///")
                && !trimmed.contains("example.com")
                && !trimmed.contains("localhost")
            {
                hardcoded_configs += 1;
            }
        }

        if file_unwraps > 0 {
            unwrap_details.push((rel_path.clone(), file_unwraps));
        }
    }

    Analysis {
        file_count,
        total_lines,
        unwrap_panic_count,
        has_logging,
        total_catches,
        empty_catches,
        hardcoded_configs,
        unwrap_details,
    }
}

fn get_project_path(conn: &Connection, snapshot_id: i64) -> Result<String> {
    conn.query_row(
        "SELECT project_path FROM snapshots WHERE id = ?1",
        [snapshot_id],
        |row| row.get(0),
    )
    .context("failed to get project path")
}

fn get_source_files(conn: &Connection, snapshot_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT path FROM files WHERE snapshot_id = ?1")
        .context("failed to prepare file paths query")?;
    let paths: Vec<String> = stmt
        .query_map([snapshot_id], |row| row.get(0))
        .context("failed to query file paths")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect file paths")?;
    // Filter to source files only
    Ok(paths.into_iter().filter(|p| is_source_file(p)).collect())
}

fn is_source_file(path: &str) -> bool {
    let src_extensions = [".rs", ".py", ".js", ".ts", ".tsx", ".jsx", ".go", ".java",
        ".kt", ".swift", ".rb", ".php", ".c", ".cpp", ".h", ".hpp", ".cs"];
    src_extensions.iter().any(|ext| path.ends_with(ext))
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
    fn test_healthy_with_logging() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db(&dir);
        add_file(&conn, sid, &dir, "src/main.rs", "use log::info;\nfn main() {\n    info!(\"starting\");\n}\n");
        let dim = Observability;
        let score = dim.score(&conn, sid)?.unwrap();
        assert!(score > 70, "project with logging should score >70, got {score}");
        Ok(())
    }

    #[test]
    fn test_many_unwraps() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db(&dir);
        let content = (0..20).map(|i| format!("let x{i} = val.unwrap();")).collect::<Vec<_>>().join("\n");
        add_file(&conn, sid, &dir, "src/main.rs", &content);
        let dim = Observability;
        let issues = dim.diagnose(&conn, sid)?;
        assert!(issues.iter().any(|i| i.message.contains("unwrap/panic")));
        Ok(())
    }
}
