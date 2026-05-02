//! Tests for `decay::store`.
//!
//! Tests that open `open_db()` rely on the `DECAY_DB_PATH` env var; mutating
//! process env is racy across threads, so those tests acquire `ENV_LOCK` first.
//! Tests that build a `Connection` directly (no env touch) skip the lock.

use std::sync::Mutex;

use rusqlite::Connection;

use decay::store::{load_latest_snapshots, open_db, save_snapshot};
use decay::types::{Function, Metrics};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn make_func(file: &str, name: &str, hash: u64, nesting: u32) -> Function {
    Function {
        file: file.to_string(),
        name: name.to_string(),
        start_line: 1,
        end_line: 10,
        param_types: vec!["i32".to_string(), "&str".to_string()],
        signature_hash: hash,
        metrics: Metrics {
            nesting,
            cyclomatic: 2,
            cognitive: 3,
            params: 2,
        },
    }
}

/// Open a Connection directly against a tempdir DB, bypassing env-var path
/// resolution so the test does not need to lock global env state.
fn open_isolated() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("snap.db");
    let conn = Connection::open(&path).unwrap();
    init_for_test(&conn);
    (dir, conn)
}

/// Mirror the schema applied by `open_db`. Used by isolated tests so that
/// `save_snapshot` / `load_latest_snapshots` can run without going through
/// env-var path resolution.
fn init_for_test(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id   TEXT NOT NULL,
            created_at   INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_snap_project ON snapshots(project_id, id DESC);

        CREATE TABLE IF NOT EXISTS functions (
            snapshot_id      INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
            signature_hash   INTEGER NOT NULL,
            file             TEXT NOT NULL,
            name             TEXT NOT NULL,
            start_line       INTEGER NOT NULL,
            end_line         INTEGER NOT NULL,
            nesting          INTEGER NOT NULL,
            cyclomatic       INTEGER NOT NULL,
            cognitive        INTEGER NOT NULL,
            params           INTEGER NOT NULL,
            PRIMARY KEY (snapshot_id, signature_hash)
        );
        CREATE INDEX IF NOT EXISTS idx_func_snap ON functions(snapshot_id);",
    )
    .unwrap();
}

#[test]
fn open_db_creates_schema() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("nested").join("snap.db");
    std::env::set_var("DECAY_DB_PATH", &db_path);

    let conn = open_db().expect("open_db");

    // Both expected tables exist.
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='table' AND name IN ('snapshots', 'functions')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 2, "snapshots and functions tables expected");

    // Indexes also created (idempotent).
    let idx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name IN ('idx_snap_project', 'idx_func_snap')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(idx_count, 2);

    std::env::remove_var("DECAY_DB_PATH");
}

#[test]
fn save_and_load_round_trip_preserves_u64_hash() {
    let (_dir, conn) = open_isolated();

    // u64::MAX is the boundary case for the i64 bit-cast round trip.
    let f = make_func("src/lib.rs", "deep", u64::MAX, 7);
    let snap_id = save_snapshot(&conn, "proj-A", vec![f.clone()]).unwrap();
    assert!(snap_id > 0);

    let loaded = load_latest_snapshots(&conn, "proj-A", 1).unwrap();
    assert_eq!(loaded.len(), 1);
    let snap = &loaded[0];
    assert_eq!(snap.id, snap_id);
    assert_eq!(snap.project_id, "proj-A");
    assert!(snap.created_at > 0);
    assert_eq!(snap.functions.len(), 1);

    let got = &snap.functions[0];
    assert_eq!(got.file, f.file);
    assert_eq!(got.name, f.name);
    assert_eq!(got.start_line, f.start_line);
    assert_eq!(got.end_line, f.end_line);
    // u64::MAX must survive the i64 bit-cast round trip without truncation.
    assert_eq!(got.signature_hash, u64::MAX);
    assert_eq!(got.metrics, f.metrics);
}

#[test]
fn load_latest_snapshots_n_boundaries() {
    let (_dir, conn) = open_isolated();

    // Empty: no snapshots stored yet.
    let empty = load_latest_snapshots(&conn, "proj-X", 10).unwrap();
    assert!(empty.is_empty());

    // n=0 always returns empty regardless of stored data.
    save_snapshot(&conn, "proj-X", vec![make_func("a.rs", "f", 1, 1)]).unwrap();
    let n0 = load_latest_snapshots(&conn, "proj-X", 0).unwrap();
    assert!(n0.is_empty());

    // 1 snapshot stored, n=1 -> returns 1.
    let one = load_latest_snapshots(&conn, "proj-X", 1).unwrap();
    assert_eq!(one.len(), 1);

    // Insert 2 more so total = 3, then ask for n=2 -> newest 2 (id DESC).
    let id2 = save_snapshot(&conn, "proj-X", vec![make_func("b.rs", "g", 2, 2)]).unwrap();
    let id3 = save_snapshot(&conn, "proj-X", vec![make_func("c.rs", "h", 3, 3)]).unwrap();

    let two = load_latest_snapshots(&conn, "proj-X", 2).unwrap();
    assert_eq!(two.len(), 2);
    assert_eq!(two[0].id, id3, "newest first");
    assert_eq!(two[1].id, id2);
}

#[test]
fn project_id_isolation() {
    let (_dir, conn) = open_isolated();

    save_snapshot(&conn, "proj-A", vec![make_func("a.rs", "fa", 10, 1)]).unwrap();
    save_snapshot(&conn, "proj-B", vec![make_func("b.rs", "fb", 20, 1)]).unwrap();

    let a = load_latest_snapshots(&conn, "proj-A", 10).unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].project_id, "proj-A");
    assert_eq!(a[0].functions[0].name, "fa");

    let b = load_latest_snapshots(&conn, "proj-B", 10).unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].project_id, "proj-B");
    assert_eq!(b[0].functions[0].name, "fb");
}

#[test]
fn decay_db_path_env_overrides_default() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let custom = dir.path().join("override").join("custom.db");
    assert!(!custom.exists());

    std::env::set_var("DECAY_DB_PATH", &custom);
    let _conn = open_db().expect("open_db with env override");

    assert!(custom.exists(), "db file should exist at overridden path");
    assert!(
        custom.parent().unwrap().exists(),
        "parent dir created automatically"
    );

    std::env::remove_var("DECAY_DB_PATH");
}
