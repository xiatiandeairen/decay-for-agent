use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use super::Dimension;
use crate::diagnose::{Issue, Level};

// --- Thresholds ---
const LONG_FILE_LINES: usize = 300;
const LONG_FILE_RATIO_WARN: f64 = 0.20;
const LONG_FILE_RATIO_CRIT: f64 = 0.40;
const LONG_FUNC_LINES: usize = 50;
const LONG_FUNC_RATIO_WARN: f64 = 0.10;
const LONG_FUNC_RATIO_CRIT: f64 = 0.25;
const DUP_FILE_RATIO_WARN: f64 = 0.05;
const DUP_FILE_RATIO_CRIT: f64 = 0.15;
const TODO_DENSITY_WARN: f64 = 20.0; // per 10K lines
const MIN_DUP_BLOCK: usize = 6;

pub struct Maintainability;

impl Dimension for Maintainability {
    fn name(&self) -> &'static str {
        "maintainability"
    }

    fn score(&self, conn: &Connection, snapshot_id: i64) -> Result<Option<i32>> {
        let project_path = get_project_path(conn, snapshot_id)?;
        let files = get_file_paths(conn, snapshot_id)?;
        if files.is_empty() {
            return Ok(Some(100));
        }

        let mut score: i32 = 100;
        let analysis = analyze_files(&project_path, &files);
        debug!("maintainability: {} files analyzed", analysis.file_count);

        // Duplicate code ratio
        if analysis.file_count > 0 {
            let dup_ratio = analysis.files_with_dups as f64 / analysis.file_count as f64;
            if dup_ratio > DUP_FILE_RATIO_CRIT {
                score -= 35;
            } else if dup_ratio > DUP_FILE_RATIO_WARN {
                score -= 15;
            }
        }

        // Long file ratio
        if analysis.file_count > 0 {
            let long_ratio = analysis.long_files as f64 / analysis.file_count as f64;
            if long_ratio > LONG_FILE_RATIO_CRIT {
                score -= 30;
            } else if long_ratio > LONG_FILE_RATIO_WARN {
                score -= 15;
            }
        }

        // Long function ratio
        if analysis.total_functions > 0 {
            let func_ratio = analysis.long_functions as f64 / analysis.total_functions as f64;
            if func_ratio > LONG_FUNC_RATIO_CRIT {
                score -= 20;
            } else if func_ratio > LONG_FUNC_RATIO_WARN {
                score -= 10;
            }
        }

        // TODO/FIXME density
        if analysis.total_lines > 0 {
            let density = analysis.todo_count as f64 / (analysis.total_lines as f64 / 10000.0);
            if density > TODO_DENSITY_WARN {
                score -= 5;
            }
        }

        Ok(Some(score.max(0)))
    }

    fn diagnose(&self, conn: &Connection, snapshot_id: i64) -> Result<Vec<Issue>> {
        let project_path = get_project_path(conn, snapshot_id)?;
        let files = get_file_paths(conn, snapshot_id)?;
        if files.is_empty() {
            return Ok(vec![]);
        }

        let mut issues = Vec::new();
        let name = self.name().to_string();
        let analysis = analyze_files(&project_path, &files);

        // Report duplicate blocks
        for (path, dup_count) in &analysis.dup_details {
            if *dup_count > 0 {
                issues.push(Issue {
                    level: Level::Warning,
                    category: name.clone(),
                    message: format!("{path} has {dup_count} duplicate block(s) shared with other files"),
                    prescription: Some(format!("extract shared logic from {path} into a common module")),
                });
            }
        }

        // Report long files
        for (path, lines) in &analysis.long_file_details {
            let level = if *lines > 600 {
                Level::Critical
            } else {
                Level::Warning
            };
            issues.push(Issue {
                level,
                category: name.clone(),
                message: format!("{path} has {lines} lines"),
                prescription: Some(format!("split {path} into smaller modules")),
            });
        }

        // Report long functions
        for (path, func_name, lines) in &analysis.long_func_details {
            issues.push(Issue {
                level: Level::Warning,
                category: name.clone(),
                message: format!("{func_name} in {path} is {lines} lines long"),
                prescription: Some(format!("break {func_name} into smaller functions")),
            });
        }

        // Report TODO/FIXME count
        if analysis.todo_count > 0 {
            issues.push(Issue {
                level: Level::Info,
                category: name,
                message: format!("{} TODO/FIXME comments across project", analysis.todo_count),
                prescription: None,
            });
        }

        Ok(issues)
    }
}

struct Analysis {
    file_count: usize,
    total_lines: usize,
    files_with_dups: usize,
    long_files: usize,
    total_functions: usize,
    long_functions: usize,
    todo_count: usize,
    dup_details: Vec<(String, usize)>,
    long_file_details: Vec<(String, usize)>,
    long_func_details: Vec<(String, String, usize)>,
}

fn analyze_files(project_path: &str, rel_paths: &[String]) -> Analysis {
    let base = Path::new(project_path);
    let mut file_count = 0;
    let mut total_lines = 0;
    let mut long_files = 0;
    let mut total_functions = 0;
    let mut long_functions = 0;
    let mut todo_count = 0;
    let mut long_file_details = Vec::new();
    let mut long_func_details = Vec::new();

    // For duplicate detection: map block fingerprint → list of (file, line_no)
    let mut block_index: HashMap<u64, Vec<(String, usize)>> = HashMap::new();

    for rel_path in rel_paths {
        // Skip auto-generated and non-source files
        if is_generated_file(rel_path) {
            continue;
        }
        let abs_path = base.join(rel_path);
        let Ok(content) = std::fs::read_to_string(&abs_path) else {
            continue;
        };

        file_count += 1;
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        total_lines += line_count;

        // Long file check
        if line_count > LONG_FILE_LINES {
            long_files += 1;
            long_file_details.push((rel_path.clone(), line_count));
        }

        // TODO/FIXME count
        for line in &lines {
            let upper = line.to_uppercase();
            if upper.contains("TODO") || upper.contains("FIXME") {
                todo_count += 1;
            }
        }

        // Function length detection
        let func_positions = detect_functions(&lines);
        total_functions += func_positions.len();
        for i in 0..func_positions.len() {
            let (func_name, start) = &func_positions[i];
            let end = if i + 1 < func_positions.len() {
                func_positions[i + 1].1
            } else {
                line_count
            };
            let func_len = end - start;
            if func_len > LONG_FUNC_LINES {
                long_functions += 1;
                long_func_details.push((rel_path.clone(), func_name.clone(), func_len));
            }
        }

        // Build block fingerprints for duplicate detection
        let normalized: Vec<u64> = lines
            .iter()
            .map(|l| {
                let trimmed = l.trim();
                if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                    0
                } else {
                    let mut hasher = DefaultHasher::new();
                    trimmed.hash(&mut hasher);
                    hasher.finish()
                }
            })
            .collect();

        for start in 0..normalized.len().saturating_sub(MIN_DUP_BLOCK) {
            // Skip blocks starting with blank/comment lines
            if normalized[start] == 0 {
                continue;
            }
            let block: Vec<u64> = normalized[start..start + MIN_DUP_BLOCK].to_vec();
            if block.iter().any(|h| *h == 0) {
                continue;
            }
            let mut hasher = DefaultHasher::new();
            block.hash(&mut hasher);
            let fingerprint = hasher.finish();
            block_index
                .entry(fingerprint)
                .or_default()
                .push((rel_path.clone(), start));
        }
    }

    // Count files with duplicates (blocks appearing in >1 file)
    let mut files_with_dups_set: HashMap<String, usize> = HashMap::new();
    for (_fp, locations) in &block_index {
        let unique_files: std::collections::HashSet<&str> =
            locations.iter().map(|(f, _)| f.as_str()).collect();
        if unique_files.len() > 1 {
            for f in &unique_files {
                *files_with_dups_set.entry(f.to_string()).or_default() += 1;
            }
        }
    }

    let files_with_dups = files_with_dups_set.len();
    let dup_details: Vec<(String, usize)> = files_with_dups_set.into_iter().collect();

    Analysis {
        file_count,
        total_lines,
        files_with_dups,
        long_files,
        total_functions,
        long_functions,
        todo_count,
        dup_details,
        long_file_details,
        long_func_details,
    }
}

/// Check if a file is auto-generated or non-source.
fn is_generated_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    // Lock files
    if lower.ends_with(".lock") || lower.ends_with("lock.json") {
        return true;
    }
    // Common generated files
    if lower.ends_with(".min.js") || lower.ends_with(".min.css") {
        return true;
    }
    // Markdown/docs — skip for maintainability analysis
    if lower.ends_with(".md") || lower.ends_with(".txt") || lower.ends_with(".rst") {
        return true;
    }
    // JSON/YAML config — not source code
    if lower.ends_with(".json") || lower.ends_with(".yaml") || lower.ends_with(".yml") || lower.ends_with(".toml") {
        return true;
    }
    false
}

/// Detect function definitions and return (name, line_number) pairs.
fn detect_functions(lines: &[&str]) -> Vec<(String, usize)> {
    let mut results = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = extract_func_name(trimmed) {
            results.push((name, i));
        }
    }
    results
}

fn extract_func_name(line: &str) -> Option<String> {
    // Rust: fn name(
    // Also matches pub fn, pub(crate) fn, async fn, etc.
    if let Some(pos) = line.find("fn ") {
        let after = &line[pos + 3..];
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() && (after.contains('(') || after.contains('<')) {
            return Some(name);
        }
    }

    // Python: def name(
    if line.starts_with("def ") || line.starts_with("async def ") {
        let after = if line.starts_with("async def ") {
            &line[10..]
        } else {
            &line[4..]
        };
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            return Some(name);
        }
    }

    // Go: func name(
    if line.starts_with("func ") {
        let after = &line[5..];
        // Skip receiver: func (r *Receiver) name(
        let after = if after.starts_with('(') {
            if let Some(close) = after.find(')') {
                after[close + 1..].trim_start()
            } else {
                after
            }
        } else {
            after
        };
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            return Some(name);
        }
    }

    // JavaScript/TypeScript: function name(
    if line.starts_with("function ") || line.starts_with("async function ") {
        let after = if line.starts_with("async function ") {
            &line[15..]
        } else {
            &line[9..]
        };
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            return Some(name);
        }
    }

    None
}

fn get_project_path(conn: &Connection, snapshot_id: i64) -> Result<String> {
    conn.query_row(
        "SELECT s.project_path FROM snapshots s
         JOIN (SELECT snapshot_id FROM files WHERE snapshot_id = ?1 LIMIT 1) f
         ON s.id = f.snapshot_id",
        [snapshot_id],
        |row| row.get(0),
    )
    .or_else(|_| {
        // Fallback: get from snapshots directly
        conn.query_row(
            "SELECT project_path FROM snapshots WHERE id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
    })
    .context("failed to get project path")
}

fn get_file_paths(conn: &Connection, snapshot_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT path FROM files WHERE snapshot_id = ?1")
        .context("failed to prepare file paths query")?;
    let paths = stmt
        .query_map([snapshot_id], |row| row.get(0))
        .context("failed to query file paths")?
        .collect::<std::result::Result<Vec<String>, _>>()
        .context("failed to collect file paths")?;
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_db_with_files(dir: &TempDir) -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                version TEXT NOT NULL
            );
            CREATE TABLE files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                depth INTEGER NOT NULL
            );",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO snapshots (project_path, version) VALUES (?1, '0.1.0')",
            [dir.path().to_string_lossy().to_string()],
        )
        .unwrap();
        let snapshot_id = conn.last_insert_rowid();
        (conn, snapshot_id)
    }

    fn add_file(conn: &Connection, snapshot_id: i64, dir: &TempDir, rel_path: &str, content: &str) {
        fs::create_dir_all(dir.path().join(rel_path).parent().unwrap()).unwrap();
        fs::write(dir.path().join(rel_path), content).unwrap();
        let size = content.len() as i64;
        let depth = rel_path.matches('/').count() + 1;
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![snapshot_id, rel_path, size, depth],
        )
        .unwrap();
    }

    #[test]
    fn test_healthy_project() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db_with_files(&dir);
        add_file(&conn, sid, &dir, "src/main.rs", "fn main() {\n    println!(\"hello\");\n}\n");
        add_file(&conn, sid, &dir, "src/lib.rs", "pub fn greet() {\n    println!(\"hi\");\n}\n");

        let dim = Maintainability;
        let score = dim.score(&conn, sid)?.unwrap();
        assert!(score > 80, "healthy project should score >80, got {score}");
        let issues = dim.diagnose(&conn, sid)?;
        assert!(issues.is_empty() || issues.iter().all(|i| i.level == Level::Info));
        Ok(())
    }

    #[test]
    fn test_long_file_detected() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db_with_files(&dir);
        let long_content = (0..400).map(|i| format!("let x{i} = {i};")).collect::<Vec<_>>().join("\n");
        add_file(&conn, sid, &dir, "src/big.rs", &long_content);
        add_file(&conn, sid, &dir, "src/small.rs", "fn main() {}\n");

        let dim = Maintainability;
        let issues = dim.diagnose(&conn, sid)?;
        assert!(issues.iter().any(|i| i.message.contains("big.rs") && i.message.contains("400")));
        Ok(())
    }

    #[test]
    fn test_todo_detected() -> Result<()> {
        let dir = TempDir::new()?;
        let (conn, sid) = setup_db_with_files(&dir);
        let content = "fn main() {\n    // TODO: fix this\n    // FIXME: and this\n}\n";
        add_file(&conn, sid, &dir, "src/main.rs", content);

        let dim = Maintainability;
        let issues = dim.diagnose(&conn, sid)?;
        assert!(issues.iter().any(|i| i.message.contains("TODO/FIXME")));
        Ok(())
    }

    #[test]
    fn test_extract_func_name_rust() {
        assert_eq!(extract_func_name("fn main() {"), Some("main".to_string()));
        assert_eq!(extract_func_name("pub fn hello(x: i32) {"), Some("hello".to_string()));
        assert_eq!(extract_func_name("async fn run() {"), Some("run".to_string()));
        assert_eq!(extract_func_name("let x = 5;"), None);
    }

    #[test]
    fn test_extract_func_name_python() {
        assert_eq!(extract_func_name("def hello():"), Some("hello".to_string()));
        assert_eq!(extract_func_name("async def run():"), Some("run".to_string()));
    }

    #[test]
    fn test_extract_func_name_go() {
        assert_eq!(extract_func_name("func main() {"), Some("main".to_string()));
        assert_eq!(extract_func_name("func (s *Server) Run() {"), Some("Run".to_string()));
    }
}
