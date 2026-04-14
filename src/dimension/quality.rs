use std::path::Path;

use anyhow::Result;
use log::debug;

use super::Dimension;
use crate::data_store::{DataStore, SourceFile};
use crate::diagnose::{Issue, Level};

pub struct QualityAssurance;

impl Dimension for QualityAssurance {
    fn name(&self) -> &'static str {
        "quality_assurance"
    }

    fn score(&self, store: &DataStore) -> Result<Option<i32>> {
        let source_files = store.source_files();
        if source_files.is_empty() {
            return Ok(Some(100));
        }

        let analysis = analyze(source_files);
        let mut score: i32 = 100;
        debug!("quality_assurance: {} source, {} test files", analysis.source_files, analysis.test_files);

        // Test file ratio
        let total = analysis.source_files + analysis.test_files;
        if total > 0 {
            let test_ratio = analysis.test_files as f64 / total as f64;
            if test_ratio == 0.0 {
                score -= 40;
            } else if test_ratio < 0.10 {
                score -= 25;
            } else if test_ratio < 0.20 {
                score -= 10;
            }
        }

        // Test/source line ratio
        if analysis.source_lines > 0 {
            let line_ratio = analysis.test_lines as f64 / analysis.source_lines as f64;
            if line_ratio < 0.1 {
                score -= 20;
            } else if line_ratio < 0.3 {
                score -= 10;
            }
        }

        // Assertion density in test files
        if analysis.test_lines > 0 {
            let assert_per_20_lines = analysis.assert_count as f64 / (analysis.test_lines as f64 / 20.0);
            if assert_per_20_lines < 1.0 {
                score -= 10;
            }
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

        let total = analysis.source_files + analysis.test_files;
        if total > 0 {
            let test_ratio = analysis.test_files as f64 / total as f64;
            if test_ratio == 0.0 {
                issues.push(Issue {
                    level: Level::Critical,
                    category: cat.clone(),
                    message: "no test files found in project".to_string(),
                    prescription: Some("add tests for critical paths and public APIs".to_string()),
                });
            } else if test_ratio < 0.10 {
                let pct = (test_ratio * 100.0) as i32;
                issues.push(Issue {
                    level: Level::Warning,
                    category: cat.clone(),
                    message: format!("only {pct}% of files are tests"),
                    prescription: Some("increase test coverage, focus on complex and critical modules".to_string()),
                });
            }
        }

        if analysis.source_lines > 0 && analysis.test_lines > 0 {
            let line_ratio = analysis.test_lines as f64 / analysis.source_lines as f64;
            if line_ratio < 0.1 {
                issues.push(Issue {
                    level: Level::Warning,
                    category: cat.clone(),
                    message: format!("test/source line ratio is {:.1}% (very low)", line_ratio * 100.0),
                    prescription: Some("add more tests to improve confidence in changes".to_string()),
                });
            }
        }

        // Report source files without corresponding test files
        for path in &analysis.untested_source_files {
            issues.push(Issue {
                level: Level::Info,
                category: cat.clone(),
                message: format!("{path} has no corresponding test file"),
                prescription: None,
            });
        }

        Ok(issues)
    }
}

struct Analysis {
    source_files: usize,
    test_files: usize,
    source_lines: usize,
    test_lines: usize,
    assert_count: usize,
    untested_source_files: Vec<String>,
}

fn analyze(files: &[SourceFile]) -> Analysis {
    let mut source_files = 0;
    let mut test_files = 0;
    let mut source_lines = 0;
    let mut test_lines = 0;
    let mut assert_count = 0;
    let mut source_names: Vec<String> = Vec::new();
    let mut test_names: Vec<String> = Vec::new();

    let assert_patterns = ["assert", "expect(", "should.", ".toBe(", ".toEqual(",
        ".to_equal(", "assert_eq!", "assert_ne!", "assert!(", "#[test]",
        "@test", "@Test", "def test_"];

    for sf in files {
        if is_test_file(&sf.path) || is_inline_test_module(&sf.content) {
            test_files += 1;
            test_lines += sf.line_count;
            test_names.push(sf.path.clone());

            // Count assertions
            for line in &sf.lines {
                let trimmed = line.trim();
                for pat in &assert_patterns {
                    if trimmed.contains(pat) {
                        assert_count += 1;
                        break;
                    }
                }
            }
        } else {
            source_files += 1;
            source_lines += sf.line_count;
            source_names.push(sf.path.clone());
        }
    }

    // Find untested source files (no matching test file)
    let untested: Vec<String> = source_names
        .iter()
        .filter(|src| {
            let stem = Path::new(src)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            !test_names.iter().any(|t| {
                t.contains(&format!("test_{stem}"))
                    || t.contains(&format!("{stem}_test"))
                    || t.contains(&format!("{stem}_spec"))
                    || t.contains(&format!("{stem}.test"))
            })
        })
        .take(10) // Limit to top 10
        .cloned()
        .collect();

    Analysis {
        source_files,
        test_files,
        source_lines,
        test_lines,
        assert_count,
        untested_source_files: untested,
    }
}

fn is_test_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("/test/") || lower.contains("/tests/")
        || lower.contains("_test.") || lower.contains("_spec.")
        || lower.contains(".test.") || lower.contains(".spec.")
        || lower.starts_with("test_") || lower.contains("/test_")
}

fn is_inline_test_module(content: &str) -> bool {
    // Rust: #[cfg(test)] mod tests
    content.contains("#[cfg(test)]")
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
    fn test_no_tests() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        add_file(&store, &dir, "src/main.rs", "fn main() {}\n");
        let dim = QualityAssurance;
        let score = dim.score(&store)?.unwrap();
        assert!(score < 50, "no-test project should score <50, got {score}");
        let issues = dim.diagnose(&store)?;
        assert!(issues.iter().any(|i| i.level == Level::Critical && i.message.contains("no test")));
        Ok(())
    }

    #[test]
    fn test_with_tests() -> Result<()> {
        let dir = TempDir::new()?;
        let store = setup_store(&dir);
        add_file(&store, &dir, "src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n");
        add_file(&store, &dir, "tests/test_lib.rs", "#[test]\nfn test_add() {\n    assert_eq!(add(1, 2), 3);\n}\n");
        let dim = QualityAssurance;
        let score = dim.score(&store)?.unwrap();
        assert!(score > 50, "project with tests should score >50, got {score}");
        Ok(())
    }
}
