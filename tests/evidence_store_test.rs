use decay::evidence_store::{DiffEvidenceRecord, EvidenceFileChange, EvidenceItem, EvidenceStore};

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
    assert_eq!(version, 2);

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
fn saving_diff_evidence_is_idempotent_for_same_changeset() {
    let (_dir, store) = open_temp_store();
    let files = vec![EvidenceFileChange {
        status: "modified".to_string(),
        old_file: Some("src/lib.rs".to_string()),
        new_file: Some("src/lib.rs".to_string()),
        added_lines: 3,
        removed_lines: 1,
    }];
    let evidence = vec![DiffEvidenceRecord {
        category: "history".to_string(),
        kind: "churn_hotspot".to_string(),
        file: "src/lib.rs".to_string(),
        subject: "3 commits in 90d".to_string(),
        source: "git_history".to_string(),
    }];

    let first = store
        .save_diff_evidence("proj", "HEAD", "abc123", "def456", &files, &evidence)
        .expect("first save");
    let second = store
        .save_diff_evidence("proj", "HEAD", "abc123", "def456", &files, &evidence)
        .expect("second save");

    assert_eq!(second.changeset_id, first.changeset_id);
    assert_eq!(second.evidence_saved, 1);

    let changesets: i64 = store
        .conn()
        .query_row("SELECT COUNT(*) FROM changesets", [], |row| row.get(0))
        .unwrap();
    let file_changes: i64 = store
        .conn()
        .query_row("SELECT COUNT(*) FROM file_changes", [], |row| row.get(0))
        .unwrap();
    let evidence_items: i64 = store
        .conn()
        .query_row("SELECT COUNT(*) FROM evidence_items", [], |row| row.get(0))
        .unwrap();

    assert_eq!(changesets, 1);
    assert_eq!(file_changes, 1);
    assert_eq!(evidence_items, 1);
}

#[test]
fn docs_only_changeset_can_be_recorded_without_code_evidence() {
    let (_dir, store) = open_temp_store();
    let files = vec![EvidenceFileChange {
        status: "modified".to_string(),
        old_file: Some("docs/roadmap.md".to_string()),
        new_file: Some("docs/roadmap.md".to_string()),
        added_lines: 9,
        removed_lines: 0,
    }];

    let saved = store
        .save_diff_evidence("proj", "HEAD~1", "abc123", "def456", &files, &[])
        .expect("save docs only");

    assert_eq!(saved.evidence_saved, 0);
    let file_changes: i64 = store
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM file_changes WHERE changeset_id = ?1",
            [saved.changeset_id],
            |row| row.get(0),
        )
        .unwrap();
    let evidence_items: i64 = store
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM evidence_items WHERE changeset_id = ?1",
            [saved.changeset_id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(file_changes, 1);
    assert_eq!(evidence_items, 0);
}

#[test]
fn saves_all_diff_evidence_categories_with_file_and_source() {
    let (_dir, store) = open_temp_store();
    let evidence = vec![
        DiffEvidenceRecord {
            category: "complexity".to_string(),
            kind: "worsened".to_string(),
            file: "src/lib.rs".to_string(),
            subject: "target".to_string(),
            source: "diff_hunk+ast_snapshot".to_string(),
        },
        DiffEvidenceRecord {
            category: "stability".to_string(),
            kind: "unsafe_added".to_string(),
            file: "src/lib.rs".to_string(),
            subject: "line 7".to_string(),
            source: "stability".to_string(),
        },
        DiffEvidenceRecord {
            category: "quality".to_string(),
            kind: "test_gap".to_string(),
            file: "src/lib.rs".to_string(),
            subject: "production change without tests".to_string(),
            source: "quality".to_string(),
        },
        DiffEvidenceRecord {
            category: "history".to_string(),
            kind: "recently_bugfixed_area".to_string(),
            file: "src/lib.rs".to_string(),
            subject: "fix: panic".to_string(),
            source: "git_history".to_string(),
        },
    ];

    let saved = store
        .save_diff_evidence("proj", "main", "abc123", "def456", &[], &evidence)
        .expect("save evidence");

    assert_eq!(saved.evidence_saved, 4);
    let categories = store
        .conn()
        .prepare(
            "SELECT category || ':' || kind || ':' || file || ':' || source
             FROM evidence_items
             WHERE changeset_id = ?1
             ORDER BY category",
        )
        .unwrap()
        .query_map([saved.changeset_id], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(
        categories,
        vec![
            "complexity:worsened:src/lib.rs:diff_hunk+ast_snapshot",
            "history:recently_bugfixed_area:src/lib.rs:git_history",
            "quality:test_gap:src/lib.rs:quality",
            "stability:unsafe_added:src/lib.rs:stability",
        ]
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
