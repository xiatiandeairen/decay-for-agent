use crate::error::Result;
use crate::types::{Function, Snapshot};

#[allow(clippy::missing_errors_doc)]
pub fn open_db() -> Result<rusqlite::Connection> {
    todo!()
}

#[allow(clippy::missing_errors_doc)]
pub fn save_snapshot(
    _conn: &rusqlite::Connection,
    _project_id: &str,
    _funcs: Vec<Function>,
) -> Result<i64> {
    todo!()
}

#[allow(clippy::missing_errors_doc)]
pub fn load_latest_snapshots(
    _conn: &rusqlite::Connection,
    _project_id: &str,
    _n: usize,
) -> Result<Vec<Snapshot>> {
    todo!()
}

pub fn _stub() {}
