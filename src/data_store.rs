use std::cell::OnceCell;
use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

use crate::util;

/// A source file with its content loaded into memory.
pub struct SourceFile {
    pub path: String,
    pub content: String,
    pub lines: Vec<String>,
    pub line_count: usize,
}

/// Parsed dependency information.
pub struct DependencyInfo {
    pub direct_count: usize,
    pub names: Vec<String>,
}

/// Lazy-loading data store shared across all dimensions.
///
/// Each data source is loaded at most once (on first access) and cached.
/// Dimensions call getters to pull what they need — no upfront cost for unused data.
pub struct DataStore {
    conn: Connection,
    snapshot_id: i64,
    project_path: String,
    // Lazy-loaded caches
    source_files: OnceCell<Vec<SourceFile>>,
    dependency_info: OnceCell<DependencyInfo>,
}

impl DataStore {
    pub fn new(conn: Connection, snapshot_id: i64, project_path: String) -> Self {
        Self {
            conn,
            snapshot_id,
            project_path,
            source_files: OnceCell::new(),
            dependency_info: OnceCell::new(),
        }
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn snapshot_id(&self) -> i64 {
        self.snapshot_id
    }

    pub fn project_path(&self) -> &str {
        &self.project_path
    }

    /// Lazily load all source files with content.
    /// First call reads from disk; subsequent calls return cached data.
    pub fn source_files(&self) -> &[SourceFile] {
        self.source_files.get_or_init(|| {
            match load_source_files(&self.conn, self.snapshot_id, &self.project_path) {
                Ok(files) => {
                    debug!("data_store: loaded {} source files into cache", files.len());
                    files
                }
                Err(e) => {
                    debug!("data_store: failed to load source files: {e}");
                    Vec::new()
                }
            }
        })
    }

    /// Lazily parse dependency information.
    pub fn dependencies(&self) -> &DependencyInfo {
        self.dependency_info.get_or_init(|| {
            let info = parse_dependencies(&self.project_path);
            debug!("data_store: parsed {} dependencies", info.direct_count);
            info
        })
    }
}

fn load_source_files(
    conn: &Connection,
    snapshot_id: i64,
    project_path: &str,
) -> Result<Vec<SourceFile>> {
    let mut stmt = conn
        .prepare("SELECT path FROM files WHERE snapshot_id = ?1")
        .context("failed to prepare file paths query")?;

    let paths: Vec<String> = stmt
        .query_map([snapshot_id], |row| row.get(0))
        .context("failed to query file paths")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect file paths")?;

    let base = Path::new(project_path);
    let mut files = Vec::new();

    for path in paths {
        if !util::is_source_file(&path) {
            continue;
        }
        let abs_path = base.join(&path);
        let Ok(content) = std::fs::read_to_string(&abs_path) else {
            continue;
        };
        let lines: Vec<String> = content.lines().map(String::from).collect();
        let line_count = lines.len();
        files.push(SourceFile {
            path,
            content,
            lines,
            line_count,
        });
    }

    Ok(files)
}

fn parse_dependencies(project_path: &str) -> DependencyInfo {
    let base = Path::new(project_path);

    // Rust: parse Cargo.toml
    if let Ok(content) = std::fs::read_to_string(base.join("Cargo.toml")) {
        let mut in_deps = false;
        let mut names = Vec::new();
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
                if let Some(name) = trimmed.split('=').next() {
                    names.push(name.trim().to_string());
                }
            }
        }
        return DependencyInfo {
            direct_count: names.len(),
            names,
        };
    }

    // Node: parse package.json with serde_json
    if let Ok(content) = std::fs::read_to_string(base.join("package.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            let deps = v.get("dependencies").and_then(|d| d.as_object());
            let dev_deps = v.get("devDependencies").and_then(|d| d.as_object());
            let mut names = Vec::new();
            if let Some(d) = deps {
                names.extend(d.keys().cloned());
            }
            if let Some(d) = dev_deps {
                names.extend(d.keys().cloned());
            }
            return DependencyInfo {
                direct_count: names.len(),
                names,
            };
        }
    }

    DependencyInfo {
        direct_count: 0,
        names: vec![],
    }
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
        conn.execute(
            "INSERT INTO snapshots (project_path, version) VALUES (?1, '0.1.0')",
            [dir.path().to_string_lossy().to_string()],
        ).unwrap();
        let sid = conn.last_insert_rowid();
        (conn, sid)
    }

    #[test]
    fn test_source_files_lazy_load() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("data.json"), "{}").unwrap();

        let (conn, sid) = setup_db(&dir);
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, 'main.rs', 12, 1)",
            [sid],
        ).unwrap();
        conn.execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, 'data.json', 2, 1)",
            [sid],
        ).unwrap();

        let store = DataStore::new(conn, sid, dir.path().to_string_lossy().to_string());

        // First call loads
        let files = store.source_files();
        assert_eq!(files.len(), 1); // only main.rs, not data.json
        assert_eq!(files[0].path, "main.rs");

        // Second call returns cached (same pointer)
        let files2 = store.source_files();
        assert_eq!(files2.len(), 1);
    }

    #[test]
    fn test_dependencies_rust() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[dependencies]\nclap = \"4\"\nanyhow = \"1\"\n[dev-dependencies]\ntempfile = \"3\"\n",
        ).unwrap();

        let info = parse_dependencies(&dir.path().to_string_lossy());
        assert_eq!(info.direct_count, 3);
        assert!(info.names.contains(&"clap".to_string()));
    }
}
