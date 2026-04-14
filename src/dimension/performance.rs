use anyhow::Result;
use log::debug;

use super::Dimension;
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

pub struct Performance;

impl Dimension for Performance {
    fn name(&self) -> &'static str {
        "performance"
    }

    fn score(&self, store: &DataStore) -> Result<Option<i32>> {
        let source_files = store.source_files();
        if source_files.is_empty() {
            return Ok(Some(100));
        }

        let analysis = analyze(source_files);
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

    fn diagnose(&self, store: &DataStore) -> Result<Vec<Issue>> {
        let source_files = store.source_files();
        if source_files.is_empty() {
            return Ok(vec![]);
        }

        let analysis = analyze(source_files);
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

fn analyze(source_files: &[SourceFile]) -> Analysis {
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

    for sf in source_files {
        file_count += 1;
        total_lines += sf.line_count;

        let mut file_clones = 0;
        let mut loop_depth: usize = 0;
        let mut brace_stack: Vec<bool> = Vec::new(); // true = loop brace

        for (i, line) in sf.lines.iter().enumerate() {
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
                    nest_details.push((sf.path.clone(), i + 1, loop_depth));
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
                    blocking_details.push((sf.path.clone(), label.to_string()));
                }
            }
        }

        if file_clones > 0 {
            clone_details.push((sf.path.clone(), file_clones));
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
    fn test_clean_performance() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        add_file(&store, &dir, "src/main.rs", "fn main() {\n    let x = 42;\n}\n");
        let dim = Performance;
        let score = dim.score(&store)?.unwrap();
        assert!(score > 80, "clean project should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_many_clones() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        let content = (0..30).map(|i| format!("let x{i} = data.clone();")).collect::<Vec<_>>().join("\n");
        add_file(&store, &dir, "src/main.rs", &content);
        let dim = Performance;
        let issues = dim.diagnose(&store)?;
        assert!(issues.iter().any(|i| i.message.contains("clone/copy")));
        Ok(())
    }
}
