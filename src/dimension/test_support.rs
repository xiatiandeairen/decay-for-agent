/// Shared test fixtures for dimension tests.
///
/// Eliminates the duplicated `setup_store()` / `add_file()` across 8 dimension test modules.
use crate::data_store::DataStore;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

/// Create an in-memory DataStore with all required tables.
///
/// Uses the provided `TempDir` as the project path so file-based dimensions
/// (maintainability, observability, quality, reliability, performance)
/// can write real files to disk.
pub fn setup_store(dir: &TempDir) -> DataStore {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, project_path TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), version TEXT NOT NULL);
         CREATE TABLE files (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, size_bytes INTEGER NOT NULL, depth INTEGER NOT NULL);
         CREATE TABLE git_changes (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, change_count INTEGER NOT NULL, lines_added INTEGER NOT NULL, lines_deleted INTEGER NOT NULL, last_modified TEXT NOT NULL);",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO snapshots (project_path, version) VALUES (?1, '0.1.0')",
        [dir.path().to_string_lossy().to_string()],
    )
    .unwrap();
    let sid = conn.last_insert_rowid();
    DataStore::new(conn, sid, dir.path().to_string_lossy().to_string())
}

/// Create a simpler DataStore without TempDir (for DB-only dimensions: structural, complexity, fragility).
pub fn setup_db_store() -> DataStore {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, project_path TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), version TEXT NOT NULL);
         CREATE TABLE files (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, size_bytes INTEGER NOT NULL, depth INTEGER NOT NULL);
         CREATE TABLE git_changes (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, path TEXT NOT NULL, change_count INTEGER NOT NULL, lines_added INTEGER NOT NULL, lines_deleted INTEGER NOT NULL, last_modified TEXT NOT NULL);",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO snapshots (project_path, version) VALUES ('/tmp', '0.1.0')",
        [],
    )
    .unwrap();
    DataStore::new(conn, 1, "/tmp".to_string())
}

/// Insert a file record into the database.
pub fn insert_file(store: &DataStore, path: &str, size: i64, depth: i64) {
    store
        .conn()
        .execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![store.snapshot_id(), path, size, depth],
        )
        .unwrap();
}

/// Insert a git_changes record into the database.
pub fn insert_git_change(
    store: &DataStore,
    path: &str,
    count: i64,
    added: i64,
    deleted: i64,
) {
    store
        .conn()
        .execute(
            "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, '2025-01-01')",
            rusqlite::params![store.snapshot_id(), path, count, added, deleted],
        )
        .unwrap();
}

/// Write a real file to disk and insert its record into the database.
pub fn add_file(store: &DataStore, dir: &TempDir, path: &str, content: &str) {
    fs::create_dir_all(dir.path().join(path).parent().unwrap()).unwrap();
    fs::write(dir.path().join(path), content).unwrap();
    store
        .conn()
        .execute(
            "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![store.snapshot_id(), path, content.len()],
        )
        .unwrap();
}
