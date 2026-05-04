use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::config::{APP_NAME, DB_FILENAME};
use crate::error::{DecayError, Result};
use crate::types::{Function, Metrics, Snapshot};

/// Resolve the SQLite database path.
///
/// Priority:
/// 1. `DECAY_DB_PATH` env var (testing override; not part of public CLI surface)
/// 2. `dirs::data_dir()/<APP_NAME>/<DB_FILENAME>`
fn resolve_db_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("DECAY_DB_PATH") {
        return Ok(PathBuf::from(p));
    }
    let base = dirs::data_dir()
        .ok_or_else(|| DecayError::InvalidProject("could not resolve user data dir".to_string()))?;
    Ok(base.join(APP_NAME).join(DB_FILENAME))
}

fn map_db_err(message: impl Into<String>) -> impl FnOnce(rusqlite::Error) -> DecayError {
    let message = message.into();
    move |source| DecayError::Db { message, source }
}

/// Open (or create) the snapshots database, ensuring schema exists.
///
/// Side effects: creates parent directory and database file on first call.
pub fn open_db() -> Result<Connection> {
    let path = resolve_db_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DecayError::Io {
            path: parent.display().to_string(),
            source: e,
        })?;
    }
    let conn = Connection::open(&path).map_err(map_db_err(format!(
        "failed to open db at {}",
        path.display()
    )))?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    // v0.1 alpha: when a pre-impl_context / pre-cfg_context schema is
    // detected, drop both tables so the new schema can be created cleanly.
    // Snapshot history is reset; this is acceptable while metric algorithms
    // and thresholds are still settling.
    if has_outdated_functions_schema(conn)? {
        log::warn!("decay db schema is from an earlier v0.1 build; resetting (snapshots cleared)");
        conn.execute_batch("DROP TABLE IF EXISTS functions; DROP TABLE IF EXISTS snapshots;")
            .map_err(map_db_err("failed to drop outdated tables"))?;
    }
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
            impl_context     TEXT NOT NULL,
            cfg_context      TEXT NOT NULL,
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
    .map_err(map_db_err("failed to init schema"))?;
    Ok(())
}

/// Detect a pre-context schema: `functions` table exists but lacks one of the
/// context columns used by the fingerprint.
fn has_outdated_functions_schema(conn: &Connection) -> Result<bool> {
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='functions'",
            [],
            |row| row.get(0),
        )
        .map_err(map_db_err("failed to inspect schema"))?;
    if table_exists == 0 {
        return Ok(false);
    }
    let mut stmt = conn
        .prepare("SELECT name FROM pragma_table_info('functions')")
        .map_err(map_db_err("failed to read functions columns"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(map_db_err("failed to read functions columns"))?;
    let mut has_impl_context = false;
    let mut has_cfg_context = false;
    for row in rows {
        let name = row.map_err(map_db_err("failed to read column row"))?;
        if name == "impl_context" {
            has_impl_context = true;
        }
        if name == "cfg_context" {
            has_cfg_context = true;
        }
    }
    Ok(!(has_impl_context && has_cfg_context))
}

/// Persist a snapshot of `funcs` for `project_id`. Returns the new snapshot id.
///
/// Wrapped in a single transaction: snapshots row + all functions rows succeed
/// or roll back together.
pub fn save_snapshot(conn: &Connection, project_id: &str, funcs: Vec<Function>) -> Result<i64> {
    let created_at: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let tx = conn
        .unchecked_transaction()
        .map_err(map_db_err("failed to begin tx"))?;

    tx.execute(
        "INSERT INTO snapshots (project_id, created_at) VALUES (?1, ?2)",
        params![project_id, created_at],
    )
    .map_err(map_db_err("failed to insert snapshot"))?;
    let snapshot_id = tx.last_insert_rowid();

    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO functions (
                    snapshot_id, signature_hash, file, impl_context, cfg_context, name,
                    start_line, end_line, nesting, cyclomatic, cognitive, params
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )
            .map_err(map_db_err("failed to prepare functions insert"))?;
        for f in &funcs {
            // u64 -> i64 bit-cast preserves all 64 bits; read path mirrors this.
            let hash_i64 = f.signature_hash as i64;
            stmt.execute(params![
                snapshot_id,
                hash_i64,
                f.file,
                f.impl_context,
                f.cfg_context,
                f.name,
                f.start_line,
                f.end_line,
                f.metrics.nesting,
                f.metrics.cyclomatic,
                f.metrics.cognitive,
                f.metrics.params,
            ])
            .map_err(map_db_err("failed to insert function"))?;
        }
    }

    tx.commit().map_err(map_db_err("failed to commit tx"))?;
    Ok(snapshot_id)
}

/// Load up to `n` most recent snapshots for `project_id`, newest first (id DESC).
pub fn load_latest_snapshots(
    conn: &Connection,
    project_id: &str,
    n: usize,
) -> Result<Vec<Snapshot>> {
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut snapshots = query_snapshots(conn, project_id, n)?;
    for snap in &mut snapshots {
        snap.functions = load_functions(conn, snap.id)?;
    }
    Ok(snapshots)
}

/// Read snapshot metadata rows (id / project_id / created_at) only; the
/// `functions` field is left empty for `load_functions` to populate.
fn query_snapshots(conn: &Connection, project_id: &str, n: usize) -> Result<Vec<Snapshot>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, created_at
             FROM snapshots
             WHERE project_id = ?1
             ORDER BY id DESC
             LIMIT ?2",
        )
        .map_err(map_db_err("failed to prepare snapshot select"))?;

    let rows = stmt
        .query_map(params![project_id, n as i64], row_to_snapshot)
        .map_err(map_db_err("failed to query snapshots"))?;

    let mut snapshots: Vec<Snapshot> = Vec::new();
    for r in rows {
        snapshots.push(r.map_err(map_db_err("failed to read snapshot row"))?);
    }
    Ok(snapshots)
}

/// Load every Function row associated with `snapshot_id`. param_types is left
/// empty per §2.4 (not persisted; signature_hash carries identity for diff).
fn load_functions(conn: &Connection, snapshot_id: i64) -> Result<Vec<Function>> {
    let mut stmt = conn
        .prepare(
            "SELECT signature_hash, file, impl_context, cfg_context, name, start_line, end_line,
                    nesting, cyclomatic, cognitive, params
             FROM functions
             WHERE snapshot_id = ?1",
        )
        .map_err(map_db_err("failed to prepare functions select"))?;

    let rows = stmt
        .query_map(params![snapshot_id], row_to_function)
        .map_err(map_db_err("failed to query functions"))?;

    let mut funcs: Vec<Function> = Vec::new();
    for r in rows {
        funcs.push(r.map_err(map_db_err("failed to read function row"))?);
    }
    Ok(funcs)
}

/// Map a snapshots-table row to a `Snapshot` (without functions populated).
/// Extracted so the per-column `?` operators stay out of `query_snapshots`'s
/// cyclomatic budget.
fn row_to_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<Snapshot> {
    Ok(Snapshot {
        id: row.get(0)?,
        project_id: row.get(1)?,
        created_at: row.get(2)?,
        functions: Vec::new(),
    })
}

/// Map a functions-table row to a `Function`. Mirrors the i64 → u64 bit-cast
/// performed on the save path. Extracted so the per-column `?` operators stay
/// out of `load_functions`'s cyclomatic budget.
fn row_to_function(row: &rusqlite::Row<'_>) -> rusqlite::Result<Function> {
    let hash_i64: i64 = row.get(0)?;
    Ok(Function {
        signature_hash: hash_i64 as u64,
        file: row.get(1)?,
        impl_context: row.get(2)?,
        cfg_context: row.get(3)?,
        name: row.get(4)?,
        start_line: row.get(5)?,
        end_line: row.get(6)?,
        param_types: Vec::new(),
        metrics: Metrics {
            nesting: row.get(7)?,
            cyclomatic: row.get(8)?,
            cognitive: row.get(9)?,
            params: row.get(10)?,
        },
    })
}

pub fn _stub() {}
