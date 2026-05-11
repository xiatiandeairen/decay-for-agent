use decay::evidence_store::{EvidenceItem, EvidenceStore};

fn open_temp_store() -> (tempfile::TempDir, EvidenceStore) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("evidence.db");
    let store = EvidenceStore::open(&db_path).expect("open evidence db");
    (dir, store)
}

fn table_count(conn: &rusqlite::Connection, names: &[&str]) -> i64 {
    let placeholders = names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    stmt.query_row(rusqlite::params_from_iter(names.iter()), |row| row.get(0))
        .unwrap()
}

#[test]
fn open_empty_db_initializes_schema_version_and_tables() {
    let (_dir, store) = open_temp_store();

    let version: i64 = store
        .conn()
        .query_row(
            "SELECT MAX(version) FROM schema_migrations WHERE namespace = 'evidence'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 1);

    assert_eq!(
        table_count(
            store.conn(),
            &[
                "schema_migrations",
                "changesets",
                "file_changes",
                "evidence_items"
            ],
        ),
        4
    );
}

#[test]
fn open_existing_db_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("evidence.db");

    let first = EvidenceStore::open(&db_path).expect("first open");
    drop(first);
    let second = EvidenceStore::open(&db_path).expect("second open");

    let migration_rows: i64 = second
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE namespace = 'evidence' AND version = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(migration_rows, 1);
}

#[test]
fn saving_same_base_head_changeset_is_idempotent() {
    let (_dir, store) = open_temp_store();

    let first = store
        .save_changeset("proj", "main", "abc123", "def456")
        .expect("first save");
    let second = store
        .save_changeset("proj", "main", "abc123", "def456")
        .expect("second save");

    assert_eq!(second.id, first.id);

    let row_count: i64 = store
        .conn()
        .query_row("SELECT COUNT(*) FROM changesets", [], |row| row.get(0))
        .unwrap();
    assert_eq!(row_count, 1);
}

#[test]
fn evidence_without_source_or_subject_is_rejected() {
    let (_dir, store) = open_temp_store();
    let changeset = store
        .save_changeset("proj", "main", "abc123", "def456")
        .expect("save changeset");

    let missing_source = store.save_evidence_item(EvidenceItem {
        changeset_id: changeset.id,
        source: "",
        subject: "src/lib.rs::f",
        kind: "regression",
        payload_json: "{}",
    });
    assert!(missing_source.is_err());

    let missing_subject = store.save_evidence_item(EvidenceItem {
        changeset_id: changeset.id,
        source: "diff",
        subject: "  ",
        kind: "regression",
        payload_json: "{}",
    });
    assert!(missing_subject.is_err());

    let row_count: i64 = store
        .conn()
        .query_row("SELECT COUNT(*) FROM evidence_items", [], |row| row.get(0))
        .unwrap();
    assert_eq!(row_count, 0);
}
