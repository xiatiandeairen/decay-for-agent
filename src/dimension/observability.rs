use anyhow::Result;
use log::debug;

use super::{Dimension, DimensionResult};
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
/// Unwrap/panic calls per 1,000 lines. These mask errors and make failures opaque.
/// 5 per 1K lines is the point where panic-driven flow starts dominating error paths.
const UNWRAP_DENSITY_WARN: f64 = 5.0;  // per 1K lines
/// Critical unwrap density: 15+ per 1K lines means error handling is essentially absent.
/// At this level any unexpected input will likely crash the process with no useful context.
const UNWRAP_DENSITY_CRIT: f64 = 15.0;
/// Number of hardcoded URLs in non-comment source lines.
/// More than 5 suggests configuration is embedded in code instead of being externalized.
const HARDCODED_CONFIG_WARN: usize = 5;

pub struct Observability;

impl Dimension for Observability {
    fn name(&self) -> &'static str {
        "observability"
    }

    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult> {
        let source_files = store.source_files();
        let name = self.name().to_string();

        if source_files.is_empty() {
            return Ok(DimensionResult { name, score: Some(100), issues: vec![] });
        }

        let analysis = analyze(source_files);
        let mut score: i32 = 100;
        let mut issues = Vec::new();
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
        for (path, count) in &analysis.unwrap_details {
            if *count > 5 {
                issues.push(Issue {
                    level: Level::Warning,
                    category: name.clone(),
                    message: format!("{path} has {count} unwrap/panic calls"),
                    prescription: Some(format!("replace unwrap/panic in {path} with proper error handling")),
                });
            }
        }

        // No logging framework
        if !analysis.has_logging {
            score -= 20;
            issues.push(Issue {
                level: Level::Warning,
                category: name.clone(),
                message: "no logging framework detected in project".to_string(),
                prescription: Some("add structured logging (e.g. log/tracing/slog for Rust, logging for Python)".to_string()),
            });
        }

        // Error swallowing
        if analysis.total_catches > 0 {
            let swallow_ratio = analysis.empty_catches as f64 / analysis.total_catches as f64;
            if swallow_ratio > 0.2 {
                score -= 15;
            }
        }
        if analysis.empty_catches > 0 {
            issues.push(Issue {
                level: Level::Warning,
                category: name.clone(),
                message: format!("{} empty catch/except blocks detected", analysis.empty_catches),
                prescription: Some("handle or log errors instead of silently swallowing them".to_string()),
            });
        }

        // Hardcoded config
        if analysis.hardcoded_configs > HARDCODED_CONFIG_WARN {
            score -= 10;
        }
        if analysis.hardcoded_configs > 0 {
            issues.push(Issue {
                level: Level::Info,
                category: name,
                message: format!("{} hardcoded configuration values detected", analysis.hardcoded_configs),
                prescription: Some("externalize configuration using environment variables or config files".to_string()),
            });
        }

        Ok(DimensionResult {
            name: self.name().to_string(),
            score: Some(score.max(0)),
            issues,
        })
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

fn analyze(source_files: &[SourceFile]) -> Analysis {
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

    for sf in source_files {
        file_count += 1;
        total_lines += sf.line_count;

        let mut file_unwraps = 0;

        for (i, line) in sf.lines.iter().enumerate() {
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
                    let next_content = sf.lines.get(i + 1).map(|l| l.trim()).unwrap_or("");
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
            unwrap_details.push((sf.path.clone(), file_unwraps));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_store::DataStore;
    use rusqlite::Connection;
    use std::fs;
    use tempfile::TempDir;

    fn setup_store(dir: &TempDir) -> DataStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, project_path TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), version TEXT NOT NULL);
             CREATE TABLE files (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, size_bytes INTEGER NOT NULL, depth INTEGER NOT NULL);
             CREATE TABLE git_changes (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, change_count INTEGER NOT NULL, lines_added INTEGER NOT NULL, lines_deleted INTEGER NOT NULL, last_modified TEXT NOT NULL);",
        ).unwrap();
        conn.execute("INSERT INTO snapshots (project_path, version) VALUES (?1, '0.1.0')", [dir.path().to_string_lossy().to_string()]).unwrap();
        let sid = conn.last_insert_rowid();
        DataStore::new(conn, sid, dir.path().to_string_lossy().to_string())
    }

    fn add_file(store: &DataStore, dir: &TempDir, path: &str, content: &str) {
        fs::create_dir_all(dir.path().join(path).parent().unwrap()).unwrap();
        fs::write(dir.path().join(path), content).unwrap();
        store.conn().execute("INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, ?2, ?3, 1)", rusqlite::params![store.snapshot_id(), path, content.len()]).unwrap();
    }

    #[test]
    fn test_healthy_with_logging() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        add_file(&store, &dir, "src/main.rs", "use log::info;\nfn main() {\n    info!(\"starting\");\n}\n");
        let dim = Observability;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score > 70, "project with logging should score >70, got {score}");
        Ok(())
    }

    #[test]
    fn test_many_unwraps() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        let content = (0..20).map(|i| format!("let x{i} = val.unwrap();")).collect::<Vec<_>>().join("\n");
        add_file(&store, &dir, "src/main.rs", &content);
        let dim = Observability;
        let issues = dim.evaluate(&store)?.issues;
        assert!(issues.iter().any(|i| i.message.contains("unwrap/panic")));
        Ok(())
    }
}
