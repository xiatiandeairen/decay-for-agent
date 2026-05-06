//! Tests for named baseline storage.

use std::sync::Mutex;

use decay::store::{load_baseline, open_db, save_baseline, SaveBaselineOutcome};
use decay::types::{Function, Metrics};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn make_func(file: &str, name: &str, hash: u64, nesting: u32) -> Function {
    Function {
        file: file.to_string(),
        impl_context: String::new(),
        cfg_context: String::new(),
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
            statement_count: 4,
            max_condition_ops: 1,
        },
    }
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

fn with_isolated_db<T>(f: impl FnOnce(&rusqlite::Connection) -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("nested").join("decay.db");
    std::env::set_var("DECAY_DB_PATH", &db_path);
    let conn = open_db().expect("open_db");
    let out = f(&conn);
    std::env::remove_var("DECAY_DB_PATH");
    out
}

#[test]
fn open_db_uses_xdg_data_home_by_default() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let xdg = dir.path().join("xdg-data");
    let old_db = std::env::var_os("DECAY_DB_PATH");
    let old_xdg = std::env::var_os("XDG_DATA_HOME");
    std::env::remove_var("DECAY_DB_PATH");
    std::env::set_var("XDG_DATA_HOME", &xdg);

    let conn = open_db().expect("open_db");
    drop(conn);

    assert!(xdg.join("decay").join("snapshots.db").exists());
    restore_env("DECAY_DB_PATH", old_db);
    restore_env("XDG_DATA_HOME", old_xdg);
}

#[test]
fn open_db_falls_back_to_home_local_share() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let old_db = std::env::var_os("DECAY_DB_PATH");
    let old_xdg = std::env::var_os("XDG_DATA_HOME");
    let old_home = std::env::var_os("HOME");
    std::env::remove_var("DECAY_DB_PATH");
    std::env::remove_var("XDG_DATA_HOME");
    std::env::set_var("HOME", dir.path());

    let conn = open_db().expect("open_db");
    drop(conn);

    assert!(dir
        .path()
        .join(".local")
        .join("share")
        .join("decay")
        .join("snapshots.db")
        .exists());
    restore_env("DECAY_DB_PATH", old_db);
    restore_env("XDG_DATA_HOME", old_xdg);
    restore_env("HOME", old_home);
}

#[test]
fn open_db_creates_named_baseline_schema() {
    with_isolated_db(|conn| {
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='table' AND name IN ('baselines', 'functions')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 2);

        let idx_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index' AND name IN ('idx_baseline_project', 'idx_func_baseline')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 2);
    });
}

#[test]
fn save_and_load_named_baseline_round_trip() {
    with_isolated_db(|conn| {
        let mut f = make_func("src/lib.rs", "deep", u64::MAX, 7);
        f.impl_context = "Display for Foo".to_string();
        f.cfg_context = "#[cfg(unix)]".to_string();

        let outcome =
            save_baseline(conn, "proj-A", "prod", "v1", vec![f.clone()], 0, false).unwrap();
        let id = match outcome {
            SaveBaselineOutcome::Created { id } => id,
            _ => panic!("expected created"),
        };

        let loaded = load_baseline(conn, "proj-A", "prod", "v1")
            .unwrap()
            .expect("baseline");
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.project_id, "proj-A");
        assert_eq!(loaded.scope, "prod");
        assert_eq!(loaded.version, "v1");
        assert!(loaded.created_at > 0);
        assert!(loaded.updated_at > 0);
        assert!(!loaded.is_partial);
        assert_eq!(loaded.diagnostic_count, 0);
        assert_eq!(loaded.functions.len(), 1);
        assert_eq!(loaded.functions[0].signature_hash, u64::MAX);
        assert_eq!(loaded.functions[0].metrics, f.metrics);
    });
}

#[test]
fn repeated_same_baseline_is_unchanged() {
    with_isolated_db(|conn| {
        let f = make_func("src/lib.rs", "deep", 1, 7);
        let first = save_baseline(conn, "proj-A", "prod", "v1", vec![f.clone()], 0, false).unwrap();
        let second = save_baseline(conn, "proj-A", "prod", "v1", vec![f], 0, false).unwrap();

        let first_id = match first {
            SaveBaselineOutcome::Created { id } => id,
            _ => panic!("expected created"),
        };
        assert_eq!(second, SaveBaselineOutcome::Unchanged { id: first_id });
    });
}

#[test]
fn repeated_different_baseline_requires_replace() {
    with_isolated_db(|conn| {
        let f1 = make_func("src/lib.rs", "deep", 1, 3);
        let f2 = make_func("src/lib.rs", "deep", 1, 8);
        let first = save_baseline(conn, "proj-A", "prod", "v1", vec![f1], 0, false).unwrap();
        let id = match first {
            SaveBaselineOutcome::Created { id } => id,
            _ => panic!("expected created"),
        };

        let blocked =
            save_baseline(conn, "proj-A", "prod", "v1", vec![f2.clone()], 0, false).unwrap();
        assert_eq!(blocked, SaveBaselineOutcome::ExistsDifferent { id });

        let replaced = save_baseline(conn, "proj-A", "prod", "v1", vec![f2], 1, true).unwrap();
        assert_eq!(replaced, SaveBaselineOutcome::Replaced { id });
        let loaded = load_baseline(conn, "proj-A", "prod", "v1")
            .unwrap()
            .expect("baseline");
        assert_eq!(loaded.functions[0].metrics.nesting, 8);
        assert!(loaded.is_partial);
        assert_eq!(loaded.diagnostic_count, 1);
    });
}

#[test]
fn project_scope_version_are_isolated() {
    with_isolated_db(|conn| {
        save_baseline(
            conn,
            "proj-A",
            "prod",
            "v1",
            vec![make_func("a.rs", "fa", 10, 1)],
            0,
            false,
        )
        .unwrap();
        save_baseline(
            conn,
            "proj-A",
            "all",
            "v1",
            vec![make_func("b.rs", "fb", 20, 2)],
            0,
            false,
        )
        .unwrap();
        save_baseline(
            conn,
            "proj-A",
            "prod",
            "v2",
            vec![make_func("c.rs", "fc", 30, 3)],
            0,
            false,
        )
        .unwrap();

        let prod_v1 = load_baseline(conn, "proj-A", "prod", "v1")
            .unwrap()
            .expect("prod v1");
        assert_eq!(prod_v1.functions[0].name, "fa");

        let all_v1 = load_baseline(conn, "proj-A", "all", "v1")
            .unwrap()
            .expect("all v1");
        assert_eq!(all_v1.functions[0].name, "fb");

        let prod_v2 = load_baseline(conn, "proj-A", "prod", "v2")
            .unwrap()
            .expect("prod v2");
        assert_eq!(prod_v2.functions[0].name, "fc");
    });
}
