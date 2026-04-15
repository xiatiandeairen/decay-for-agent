use anyhow::Result;
use log::debug;

use super::{Dimension, DimensionResult};
use crate::action::{Action, ActionType, Effort, Priority, Target};
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
/// Unsafe/eval occurrences per 1,000 lines triggering a warning.
/// 2+ per 1K lines means unsafe usage is habitual rather than exceptional.
const UNSAFE_DENSITY_WARN: f64 = 2.0;
/// Critical unsafe density: 8+ per 1K lines indicates safety guarantees are systematically bypassed.
/// At this level, memory safety and sandboxing assumptions can no longer be trusted.
const UNSAFE_DENSITY_CRIT: f64 = 8.0;
/// Direct dependency count above which supply-chain risk becomes significant.
/// 50+ direct deps dramatically increases the attack surface and update burden.
const DEP_COUNT_WARN: usize = 50;
/// Critical dependency count. 100+ direct deps signals transitive exposure is very likely unaudited.
const DEP_COUNT_CRIT: usize = 100;
/// Unsafe/eval occurrences per file before flagging it individually in diagnosis.
/// More than 3 in one file suggests that file specifically is a reliability weak point.
const UNSAFE_PER_FILE_WARN: usize = 3;

pub struct Reliability;

impl Dimension for Reliability {
    fn name(&self) -> &'static str {
        "reliability"
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
        debug!("reliability: {} files, {} lines", analysis.file_count, analysis.total_lines);

        // Unsafe/eval density
        if analysis.total_lines > 0 {
            let density = analysis.unsafe_count as f64 / (analysis.total_lines as f64 / 1000.0);
            if density > UNSAFE_DENSITY_CRIT {
                score -= 30;
            } else if density > UNSAFE_DENSITY_WARN {
                score -= 15;
            }
        }
        for (path, count) in &analysis.unsafe_details {
            if *count > UNSAFE_PER_FILE_WARN {
                issues.push(Issue::with_actions(
                    Level::Warning,
                    name.clone(),
                    format!("{path} has {count} unsafe/eval occurrences"),
                    Some(format!("minimize unsafe code in {path}, prefer safe abstractions")),
                    vec![Action {
                        dimension: name.clone(),
                        action_type: ActionType::Replace,
                        target: Target { file: path.clone(), line_range: None, symbol: None },
                        reason: format!("{path} has {count} unsafe/eval, replace with safe abstractions"),
                        priority: Priority::High,
                        effort: Effort::Medium,
                    }],
                ));
            }
        }

        // SQL/shell injection patterns
        let injection_penalty = (analysis.injection_patterns * 20).min(40) as i32;
        score -= injection_penalty;
        for (path, pattern, line_no) in &analysis.injection_details {
            issues.push(Issue::with_actions(
                Level::Critical,
                name.clone(),
                format!("{path}:{line_no}: potential {pattern}"),
                Some("use parameterized queries or safe command execution".to_string()),
                vec![Action {
                    dimension: name.clone(),
                    action_type: ActionType::Replace,
                    target: Target { file: path.clone(), line_range: Some((*line_no, *line_no)), symbol: None },
                    reason: format!("{path}:{line_no}: potential {pattern}, use parameterized queries"),
                    priority: Priority::Critical,
                    effort: Effort::Small,
                }],
            ));
        }

        // Hardcoded secrets
        let secret_penalty = (analysis.hardcoded_secrets * 15).min(30) as i32;
        score -= secret_penalty;
        for (path, kind, line_no) in &analysis.secret_details {
            issues.push(Issue::with_actions(
                Level::Critical,
                name.clone(),
                format!("{path}:{line_no}: {kind}"),
                Some("use environment variables or secret management for credentials".to_string()),
                vec![Action {
                    dimension: name.clone(),
                    action_type: ActionType::Replace,
                    target: Target { file: path.clone(), line_range: Some((*line_no, *line_no)), symbol: None },
                    reason: format!("{path}:{line_no}: {kind}, use env vars or secret management"),
                    priority: Priority::Critical,
                    effort: Effort::Small,
                }],
            ));
        }

        // Dependency count
        let dep_count = store.dependencies().direct_count;
        if dep_count > DEP_COUNT_CRIT {
            score -= 20;
        } else if dep_count > DEP_COUNT_WARN {
            score -= 10;
        }
        if dep_count > DEP_COUNT_WARN {
            issues.push(Issue::with_actions(
                Level::Info,
                name,
                format!("{dep_count} direct dependencies"),
                Some("audit dependencies for necessity, remove unused ones".to_string()),
                vec![Action {
                    dimension: "reliability".into(),
                    action_type: ActionType::Remove,
                    target: Target { file: ".".into(), line_range: None, symbol: None },
                    reason: format!("{dep_count} direct dependencies, audit and remove unused"),
                    priority: Priority::Medium,
                    effort: Effort::Small,
                }],
            ));
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
    unsafe_count: usize,
    injection_patterns: usize,
    hardcoded_secrets: usize,
    unsafe_details: Vec<(String, usize)>,
    injection_details: Vec<(String, String, u32)>, // (path, pattern, line_no)
    secret_details: Vec<(String, String, u32)>,   // (path, kind, line_no)
}

fn analyze(source_files: &[SourceFile]) -> Analysis {
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut unsafe_count = 0;
    let mut injection_patterns = 0;
    let mut hardcoded_secrets = 0;
    let mut unsafe_details = Vec::new();
    let mut injection_details = Vec::new();
    let mut secret_details: Vec<(String, String, u32)> = Vec::new();

    let unsafe_patterns = ["unsafe {", "unsafe{", "eval(", "exec(", "Function("];

    for sf in source_files {
        file_count += 1;
        total_lines += sf.line_count;
        let mut file_unsafe = 0;

        for (i, line) in sf.lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            let line_no = (i + 1) as u32;

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
                injection_details.push((sf.path.clone(), "SQL string concatenation".to_string(), line_no));
            }

            // Shell injection
            if (trimmed.contains("Command::new") || trimmed.contains("subprocess") || trimmed.contains("os.system"))
                && (trimmed.contains("format!(") || trimmed.contains("f\"") || trimmed.contains("+ "))
            {
                injection_patterns += 1;
                injection_details.push((sf.path.clone(), "shell command injection risk".to_string(), line_no));
            }

            // Hardcoded secrets
            let lower = trimmed.to_lowercase();
            if (lower.contains("password") || lower.contains("secret") || lower.contains("api_key") || lower.contains("apikey"))
                && (lower.contains("= \"") || lower.contains("= '"))
                && !lower.contains("env") && !lower.contains("config") && !lower.contains("example")
            {
                hardcoded_secrets += 1;
                secret_details.push((sf.path.clone(), "hardcoded credential detected".to_string(), line_no));
            }
        }

        if file_unsafe > 0 {
            unsafe_details.push((sf.path.clone(), file_unsafe));
        }
    }

    Analysis {
        file_count,
        total_lines,
        unsafe_count,
        injection_patterns,
        hardcoded_secrets,
        unsafe_details,
        injection_details,
        secret_details,
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
    fn test_safe_project() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        add_file(&store, &dir, "src/main.rs", "fn main() {\n    let x = 42;\n}\n");
        let dim = Reliability;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score > 80, "safe project should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_unsafe_code() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        let content = (0..10).map(|_| "unsafe { std::ptr::null() };").collect::<Vec<_>>().join("\n");
        add_file(&store, &dir, "src/main.rs", &content);
        let dim = Reliability;
        let score = dim.evaluate(&store)?.score.unwrap();
        assert!(score < 80, "unsafe project should score <80, got {score}");
        Ok(())
    }
}
