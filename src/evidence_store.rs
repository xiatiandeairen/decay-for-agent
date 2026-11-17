use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{DecayError, Result};

#[derive(Debug)]
pub struct EvidenceStore {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changeset {
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct EvidenceItem<'a> {
    pub changeset_id: i64,
    pub source: &'a str,
    pub subject: &'a str,
    pub kind: &'a str,
    pub payload_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceFileChange {
    pub status: String,
    pub old_file: Option<String>,
    pub new_file: Option<String>,
    pub added_lines: u32,
    pub removed_lines: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEvidenceRecord {
    pub category: String,
    pub kind: String,
    pub file: String,
    pub subject: String,
    pub source: String,
    pub confidence: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedDiffEvidence {
    pub changeset_id: i64,
    pub evidence_saved: usize,
}

impl EvidenceStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| DecayError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let conn = Connection::open(path).map_err(map_db_err("failed to open evidence db"))?;
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn from_connection(conn: Connection) -> Result<Self> {
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    pub fn save_changeset(
        &self,
        project_key: &str,
        base_ref: &str,
        base_commit: &str,
        head_commit: &str,
    ) -> Result<Changeset> {
        if let Some(id) = self
            .conn
            .query_row(
                "SELECT id FROM changesets
                 WHERE project_key = ?1 AND base_ref = ?2 AND base_commit = ?3 AND head_commit = ?4",
                params![project_key, base_ref, base_commit, head_commit],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(map_db_err("failed to load changeset"))?
        {
            return Ok(Changeset { id });
        }

        self.conn
            .execute(
                "INSERT INTO changesets (project_key, base_ref, base_commit, head_commit, created_at)
                 VALUES (?1, ?2, ?3, ?4, strftime('%s','now'))",
                params![project_key, base_ref, base_commit, head_commit],
            )
            .map_err(map_db_err("failed to insert changeset"))?;
        Ok(Changeset {
            id: self.conn.last_insert_rowid(),
        })
    }

    pub fn save_evidence_item(&self, item: EvidenceItem<'_>) -> Result<i64> {
        if item.source.trim().is_empty() {
            return Err(DecayError::InvalidProject(
                "evidence source is required".to_string(),
            ));
        }
        if item.subject.trim().is_empty() {
            return Err(DecayError::InvalidProject(
                "evidence subject is required".to_string(),
            ));
        }

        self.conn
            .execute(
                "INSERT INTO evidence_items (
                    changeset_id, source, subject, kind, payload_json, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s','now'))",
                params![
                    item.changeset_id,
                    item.source,
                    item.subject,
                    item.kind,
                    item.payload_json
                ],
            )
            .map_err(map_db_err("failed to insert evidence item"))?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn save_diff_evidence(
        &self,
        project_key: &str,
        base_ref: &str,
        base_commit: &str,
        head_commit: &str,
        files: &[EvidenceFileChange],
        evidence: &[DiffEvidenceRecord],
    ) -> Result<SavedDiffEvidence> {
        let changeset = self.save_changeset(project_key, base_ref, base_commit, head_commit)?;

        self.conn
            .execute(
                "DELETE FROM file_changes WHERE changeset_id = ?1",
                [changeset.id],
            )
            .map_err(map_db_err("failed to clear file changes"))?;
        self.conn
            .execute(
                "DELETE FROM evidence_items WHERE changeset_id = ?1",
                [changeset.id],
            )
            .map_err(map_db_err("failed to clear evidence items"))?;

        for file in files {
            self.conn
                .execute(
                    "INSERT INTO file_changes (
                        changeset_id, status, old_file, new_file, added_lines, removed_lines
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        changeset.id,
                        file.status,
                        file.old_file,
                        file.new_file,
                        file.added_lines,
                        file.removed_lines
                    ],
                )
                .map_err(map_db_err("failed to insert file change"))?;
        }

        for record in evidence {
            if record.category.trim().is_empty() {
                return Err(DecayError::InvalidProject(
                    "evidence category is required".to_string(),
                ));
            }
            if record.source.trim().is_empty() {
                return Err(DecayError::InvalidProject(
                    "evidence source is required".to_string(),
                ));
            }
            if record.subject.trim().is_empty() {
                return Err(DecayError::InvalidProject(
                    "evidence subject is required".to_string(),
                ));
            }
            if record.confidence.trim().is_empty() {
                return Err(DecayError::InvalidProject(
                    "evidence confidence is required".to_string(),
                ));
            }
            if record.value.trim().is_empty() {
                return Err(DecayError::InvalidProject(
                    "evidence value is required".to_string(),
                ));
            }

            self.conn
                .execute(
                    "INSERT INTO evidence_items (
                        changeset_id, category, source, subject, kind, file, confidence, value, payload_json, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '{}', strftime('%s','now'))",
                    params![
                        changeset.id,
                        record.category,
                        record.source,
                        record.subject,
                        record.kind,
                        record.file,
                        record.confidence,
                        record.value
                    ],
                )
                .map_err(map_db_err("failed to insert diff evidence"))?;
        }

        Ok(SavedDiffEvidence {
            changeset_id: changeset.id,
            evidence_saved: evidence.len(),
        })
    }
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            namespace TEXT NOT NULL,
            version INTEGER NOT NULL,
            applied_at INTEGER NOT NULL,
            PRIMARY KEY (namespace, version)
        );

        CREATE TABLE IF NOT EXISTS changesets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_key TEXT NOT NULL,
            base_ref TEXT NOT NULL,
            base_commit TEXT NOT NULL,
            head_commit TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            UNIQUE(project_key, base_ref, base_commit, head_commit)
        );

        CREATE TABLE IF NOT EXISTS file_changes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            changeset_id INTEGER NOT NULL REFERENCES changesets(id) ON DELETE CASCADE,
            status TEXT NOT NULL,
            old_file TEXT,
            new_file TEXT,
            added_lines INTEGER NOT NULL DEFAULT 0,
            removed_lines INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS evidence_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            changeset_id INTEGER NOT NULL REFERENCES changesets(id) ON DELETE CASCADE,
            category TEXT NOT NULL DEFAULT 'unknown',
            source TEXT NOT NULL CHECK(length(trim(source)) > 0),
            subject TEXT NOT NULL CHECK(length(trim(subject)) > 0),
            kind TEXT NOT NULL,
            file TEXT NOT NULL DEFAULT '',
            confidence TEXT NOT NULL DEFAULT 'low',
            value TEXT NOT NULL DEFAULT 'context',
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );",
    )
    .map_err(map_db_err("failed to init evidence schema"))?;

    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (namespace, version, applied_at)
         VALUES ('evidence', 1, strftime('%s','now'))",
        [],
    )
    .map_err(map_db_err("failed to record evidence schema migration"))?;
    migrate_schema(conn)?;
    Ok(())
}

fn migrate_schema(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "evidence_items",
        "category",
        "TEXT NOT NULL DEFAULT 'unknown'",
    )?;
    add_column_if_missing(conn, "evidence_items", "file", "TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(
        conn,
        "evidence_items",
        "confidence",
        "TEXT NOT NULL DEFAULT 'low'",
    )?;
    add_column_if_missing(
        conn,
        "evidence_items",
        "value",
        "TEXT NOT NULL DEFAULT 'context'",
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (namespace, version, applied_at)
         VALUES ('evidence', 2, strftime('%s','now'))",
        [],
    )
    .map_err(map_db_err("failed to record evidence schema migration"))?;
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (namespace, version, applied_at)
         VALUES ('evidence', 3, strftime('%s','now'))",
        [],
    )
    .map_err(map_db_err("failed to record evidence schema migration"))?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    column_type: &str,
) -> Result<()> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(map_db_err("failed to inspect evidence schema"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(map_db_err("failed to read evidence schema"))?;
    for row in rows {
        if row.map_err(map_db_err("failed to read evidence schema row"))? == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}"),
        [],
    )
    .map_err(map_db_err("failed to migrate evidence schema column"))?;
    Ok(())
}

fn map_db_err(message: impl Into<String>) -> impl FnOnce(rusqlite::Error) -> DecayError {
    let message = message.into();
    move |source| DecayError::Db { message, source }
}
