use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

use crate::config::{APP_NAME, DB_FILENAME};
use crate::error::{DecayError, Result};
use crate::types::{Baseline, Function, Metrics};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveBaselineOutcome {
    Created { id: i64 },
    Unchanged { id: i64 },
    Replaced { id: i64 },
    ExistsDifferent { id: i64 },
}

/// Resolve the SQLite database path.
///
/// Priority:
/// 1. `DECAY_DB_PATH` env var (testing override; not part of public CLI surface)
/// 2. `$XDG_DATA_HOME/<APP_NAME>/<DB_FILENAME>`
/// 3. `$HOME/.local/share/<APP_NAME>/<DB_FILENAME>`
fn resolve_db_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("DECAY_DB_PATH") {
        return Ok(PathBuf::from(p));
    }
    let base = xdg_data_home()?;
    Ok(base.join(APP_NAME).join(DB_FILENAME))
}

fn xdg_data_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("XDG_DATA_HOME") {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    let home = std::env::var("HOME").map_err(|_| {
        DecayError::InvalidProject("could not resolve HOME for XDG data dir".to_string())
    })?;
    Ok(PathBuf::from(home).join(".local").join("share"))
}

fn map_db_err(message: impl Into<String>) -> impl FnOnce(rusqlite::Error) -> DecayError {
    let message = message.into();
    move |source| DecayError::Db { message, source }
}

/// Open (or create) the baselines database, ensuring schema exists.
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
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS baselines (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id   TEXT NOT NULL,
            scope        TEXT NOT NULL,
            version      TEXT NOT NULL,
            created_at   INTEGER NOT NULL,
            updated_at   INTEGER NOT NULL,
            is_partial   INTEGER NOT NULL,
            diagnostics  INTEGER NOT NULL,
            UNIQUE(project_id, scope, version)
        );
        CREATE INDEX IF NOT EXISTS idx_baseline_project
            ON baselines(project_id, scope, version);

        CREATE TABLE IF NOT EXISTS functions (
            baseline_id       INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
            signature_hash    INTEGER NOT NULL,
            file              TEXT NOT NULL,
            impl_context      TEXT NOT NULL,
            cfg_context       TEXT NOT NULL,
            name              TEXT NOT NULL,
            start_line        INTEGER NOT NULL,
            end_line          INTEGER NOT NULL,
            nesting           INTEGER NOT NULL,
            cyclomatic        INTEGER NOT NULL,
            cognitive         INTEGER NOT NULL,
            params            INTEGER NOT NULL,
            statement_count   INTEGER NOT NULL,
            max_condition_ops INTEGER NOT NULL,
            PRIMARY KEY (baseline_id, signature_hash)
        );
        CREATE INDEX IF NOT EXISTS idx_func_baseline ON functions(baseline_id);",
    )
    .map_err(map_db_err("failed to init schema"))?;
    Ok(())
}

pub fn save_baseline(
    conn: &Connection,
    project_id: &str,
    scope: &str,
    version: &str,
    funcs: Vec<Function>,
    diagnostic_count: usize,
    replace: bool,
) -> Result<SaveBaselineOutcome> {
    if let Some(existing) = load_baseline(conn, project_id, scope, version)? {
        if functions_equivalent(&existing.functions, &funcs)
            && existing.diagnostic_count == diagnostic_count as u32
        {
            return Ok(SaveBaselineOutcome::Unchanged { id: existing.id });
        }
        if !replace {
            return Ok(SaveBaselineOutcome::ExistsDifferent { id: existing.id });
        }
        replace_baseline(conn, existing.id, funcs, diagnostic_count)?;
        return Ok(SaveBaselineOutcome::Replaced { id: existing.id });
    }

    let id = insert_baseline(conn, project_id, scope, version, funcs, diagnostic_count)?;
    Ok(SaveBaselineOutcome::Created { id })
}

pub fn load_baseline(
    conn: &Connection,
    project_id: &str,
    scope: &str,
    version: &str,
) -> Result<Option<Baseline>> {
    let mut baseline = conn
        .query_row(
            "SELECT id, project_id, scope, version, created_at, updated_at, is_partial, diagnostics
             FROM baselines
             WHERE project_id = ?1 AND scope = ?2 AND version = ?3",
            params![project_id, scope, version],
            row_to_baseline,
        )
        .optional()
        .map_err(map_db_err("failed to load baseline"))?;

    if let Some(b) = &mut baseline {
        b.functions = load_functions(conn, b.id)?;
    }
    Ok(baseline)
}

fn insert_baseline(
    conn: &Connection,
    project_id: &str,
    scope: &str,
    version: &str,
    funcs: Vec<Function>,
    diagnostic_count: usize,
) -> Result<i64> {
    let now = unix_now();
    let tx = conn
        .unchecked_transaction()
        .map_err(map_db_err("failed to begin tx"))?;
    tx.execute(
        "INSERT INTO baselines (project_id, scope, version, created_at, updated_at, is_partial, diagnostics)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            project_id,
            scope,
            version,
            now,
            now,
            diagnostic_count > 0,
            diagnostic_count as i64
        ],
    )
    .map_err(map_db_err("failed to insert baseline"))?;
    let id = tx.last_insert_rowid();
    insert_functions(&tx, id, &funcs)?;
    tx.commit().map_err(map_db_err("failed to commit tx"))?;
    Ok(id)
}

fn replace_baseline(
    conn: &Connection,
    baseline_id: i64,
    funcs: Vec<Function>,
    diagnostic_count: usize,
) -> Result<()> {
    let now = unix_now();
    let tx = conn
        .unchecked_transaction()
        .map_err(map_db_err("failed to begin tx"))?;
    tx.execute(
        "DELETE FROM functions WHERE baseline_id = ?1",
        params![baseline_id],
    )
    .map_err(map_db_err("failed to delete baseline functions"))?;
    tx.execute(
        "UPDATE baselines SET updated_at = ?1, is_partial = ?2, diagnostics = ?3 WHERE id = ?4",
        params![
            now,
            diagnostic_count > 0,
            diagnostic_count as i64,
            baseline_id
        ],
    )
    .map_err(map_db_err("failed to update baseline"))?;
    insert_functions(&tx, baseline_id, &funcs)?;
    tx.commit().map_err(map_db_err("failed to commit tx"))?;
    Ok(())
}

fn insert_functions(conn: &Connection, baseline_id: i64, funcs: &[Function]) -> Result<()> {
    let mut stmt = conn
        .prepare(
            "INSERT INTO functions (
                baseline_id, signature_hash, file, impl_context, cfg_context, name,
                start_line, end_line, nesting, cyclomatic, cognitive, params,
        statement_count, max_condition_ops
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .map_err(map_db_err("failed to prepare functions insert"))?;

    for f in funcs {
        stmt.execute(params![
            baseline_id,
            f.signature_hash as i64,
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
            f.metrics.statement_count,
            f.metrics.max_condition_ops,
        ])
        .map_err(map_db_err("failed to insert function"))?;
    }
    Ok(())
}

fn load_functions(conn: &Connection, baseline_id: i64) -> Result<Vec<Function>> {
    let mut stmt = conn
        .prepare(
            "SELECT signature_hash, file, impl_context, cfg_context, name,
                    start_line, end_line, nesting, cyclomatic, cognitive, params,
                    statement_count, max_condition_ops
             FROM functions
             WHERE baseline_id = ?1
             ORDER BY file ASC, start_line ASC, name ASC",
        )
        .map_err(map_db_err("failed to prepare functions select"))?;

    let rows = stmt
        .query_map(params![baseline_id], row_to_function)
        .map_err(map_db_err("failed to load functions"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(map_db_err("failed to read function row"))?);
    }
    Ok(out)
}

fn row_to_baseline(row: &rusqlite::Row<'_>) -> rusqlite::Result<Baseline> {
    Ok(Baseline {
        id: row.get(0)?,
        project_id: row.get(1)?,
        scope: row.get(2)?,
        version: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        is_partial: row.get(6)?,
        diagnostic_count: row.get::<_, i64>(7)? as u32,
        functions: Vec::new(),
    })
}

fn row_to_function(row: &rusqlite::Row<'_>) -> rusqlite::Result<Function> {
    let hash_i64: i64 = row.get(0)?;
    Ok(Function {
        signature_hash: hash_i64 as u64,
        file: row.get(1)?,
        impl_context: row.get(2)?,
        cfg_context: row.get(3)?,
        name: row.get(4)?,
        start_line: row.get::<_, i64>(5)? as u32,
        end_line: row.get::<_, i64>(6)? as u32,
        param_types: Vec::new(),
        metrics: Metrics {
            nesting: row.get::<_, i64>(7)? as u32,
            cyclomatic: row.get::<_, i64>(8)? as u32,
            cognitive: row.get::<_, i64>(9)? as u32,
            params: row.get::<_, i64>(10)? as u32,
            statement_count: row.get::<_, i64>(11)? as u32,
            max_condition_ops: row.get::<_, i64>(12)? as u32,
        },
    })
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn functions_equivalent(a: &[Function], b: &[Function]) -> bool {
    canonical_functions(a) == canonical_functions(b)
}

fn canonical_functions(funcs: &[Function]) -> Vec<FunctionKey> {
    let mut keys: Vec<FunctionKey> = funcs.iter().map(FunctionKey::from).collect();
    keys.sort();
    keys
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FunctionKey {
    signature_hash: u64,
    file: String,
    impl_context: String,
    cfg_context: String,
    name: String,
    start_line: u32,
    end_line: u32,
    metrics: MetricKey,
}

impl From<&Function> for FunctionKey {
    fn from(f: &Function) -> Self {
        Self {
            signature_hash: f.signature_hash,
            file: f.file.clone(),
            impl_context: f.impl_context.clone(),
            cfg_context: f.cfg_context.clone(),
            name: f.name.clone(),
            start_line: f.start_line,
            end_line: f.end_line,
            metrics: MetricKey::from(f.metrics),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MetricKey {
    nesting: u32,
    cyclomatic: u32,
    cognitive: u32,
    params: u32,
    statement_count: u32,
    max_condition_ops: u32,
}

impl From<Metrics> for MetricKey {
    fn from(m: Metrics) -> Self {
        Self {
            nesting: m.nesting,
            cyclomatic: m.cyclomatic,
            cognitive: m.cognitive,
            params: m.params,
            statement_count: m.statement_count,
            max_condition_ops: m.max_condition_ops,
        }
    }
}
