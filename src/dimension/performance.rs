use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use super::Dimension;
use crate::diagnose::{Issue, Level};

pub struct Performance;

impl Dimension for Performance {
    fn name(&self) -> &'static str {
        "performance"
    }

    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>> {
        let project_path = get_project_path(conn, snapshot_id)?;
        let files = get_source_files(conn, snapshot_id)?;
        if files.is_empty() {
            return Ok(Some(100));
        }

        let analysis = analyze(&project_path, &files);
        let mut score: i32 = 100;
        debug!("performance: {} files, {} lines", analysis.file_count, analysis.total_lines);

        // Deep nested loops
        if analysis.deep_nests > 10 {
            score -= 30;
        } else if analysis.deep_nests > 3 {
            score -= 15;
        }

        // Clone/copy density
        if analysis.total_lines > 0 {
            let density = analysis.clone_count as f64 / (analysis.total_lines as f64 / 1000.0);
            if density > 25.0 {
                score -= 20;
            } else if density > 10.0 {
                score -= 10;
            }
        }

        // Sync blocking calls
        if analysis.blocking_calls > 15 {
            score -= 20;
        } else if analysis.blocking_calls > 5 {
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

        // Deep nested loops
        for (path, line_no, depth) in &analysis.nest_details {
            issues.push(Issue {
                level: if *depth >= 4 { Level::Critical } else { Level::Warning },
                category: cat.clone(),
                message: format!("{path}:{line_no} has {depth}-level nested loop"),
                prescription: Some("extract inner loops into separate functions or use iterators".to_string()),
            });
        }

        // High clone density files
        for (path, count) in &analysis.clone_details {
            if *count > 10 {
                issues.push(Issue {
                    level: Level::Warning,
                    category: cat.clone(),
                    message: format!("{path} has {count} clone/copy calls"),
                    prescription: Some(format!("reduce cloning in {path}, prefer references or Cow")),
                });
            }
        }

        // Blocking calls
        for (path, call) in &analysis.blocking_details {
            issues.push(Issue {
                level: Level::Info,
                category: cat.clone(),
                message: format!("{path}: blocking call {call}"),
                prescription: Some("consider async alternatives for I/O-bound operations".to_string()),
            });
        }

        Ok(issues)
    }
}

struct Analysis {
    file_count: usize,
    total_lines: usize,
    deep_nests: usize,
    clone_count: usize,
    blocking_calls: usize,
    nest_details: Vec<(String, usize, usize)>, // (path, line, depth)
    clone_details: Vec<(String, usize)>,
    blocking_details: Vec<(String, String)>,
}

fn analyze(project_path: &str, rel_paths: &[String]) -> Analysis {
    let base = Path::new(project_path);
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut deep_nests = 0;
    let mut clone_count = 0;
    let mut blocking_calls = 0;
    let mut nest_details = Vec::new();
    let mut clone_details = Vec::new();
    let mut blocking_details: Vec<(String, String)> = Vec::new();

    let clone_patterns = [".clone()", ".copy()", ".deepcopy(", ".to_owned()", "Clone::clone"];
    let blocking_patterns = [
        ("thread::sleep", "thread::sleep"),
        ("time.sleep", "time.sleep"),
        ("Sleep(", "Sleep"),
        ("std::thread::sleep", "std::thread::sleep"),
        (".block_on(", "block_on"),
    ];
    let loop_keywords = ["for ", "while ", "loop {", "loop{"];

    for rel_path in rel_paths {
        let abs_path = base.join(rel_path);
        let Ok(content) = std::fs::read_to_string(&abs_path) else {
            continue;
        };

        file_count += 1;
        let lines: Vec<&str> = content.lines().collect();
        total_lines += lines.len();

        let mut file_clones = 0;
        let mut loop_depth: usize = 0;
        let mut brace_stack: Vec<bool> = Vec::new(); // true = loop brace

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            // Track loop nesting via braces
            let is_loop_start = loop_keywords.iter().any(|kw| trimmed.starts_with(kw) || trimmed.contains(&format!(" {kw}")));
            let opens = trimmed.matches('{').count();
            let closes = trimmed.matches('}').count();

            if is_loop_start {
                loop_depth += 1;
                if loop_depth >= 3 {
                    deep_nests += 1;
                    nest_details.push((rel_path.clone(), i + 1, loop_depth));
                }
            }

            for _ in 0..opens {
                brace_stack.push(is_loop_start);
            }
            for _ in 0..closes {
                if let Some(was_loop) = brace_stack.pop() {
                    if was_loop && loop_depth > 0 {
                        loop_depth -= 1;
                    }
                }
            }

            // Clone/copy
            for pat in &clone_patterns {
                if trimmed.contains(pat) {
                    clone_count += 1;
                    file_clones += 1;
                }
            }

            // Blocking calls
            for (pat, label) in &blocking_patterns {
                if trimmed.contains(pat) {
                    blocking_calls += 1;
                    blocking_details.push((rel_path.clone(), label.to_string()));
                }
            }
        }

        if file_clones > 0 {
            clone_details.push((rel_path.clone(), file_clones));
        }
    }

    Analysis {
        file_count,
        total_lines,
        deep_nests,
        clone_count,
        blocking_calls,
        nest_details,
        clone_details,
        blocking_details,
    }
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
    fn test_clean_performance() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db(&dir);
        add_file(&conn, sid, &dir, "src/main.rs", "fn main() {\n    let x = 42;\n}\n");
        let dim = Performance;
        let score = dim.score(&conn, sid)?.unwrap();
        assert!(score > 80, "clean project should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_many_clones() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db(&dir);
        let content = (0..30).map(|i| format!("let x{i} = data.clone();")).collect::<Vec<_>>().join("\n");
        add_file(&conn, sid, &dir, "src/main.rs", &content);
        let dim = Performance;
        let issues = dim.diagnose(&conn, sid)?;
        assert!(issues.iter().any(|i| i.message.contains("clone/copy")));
        Ok(())
    }
}
