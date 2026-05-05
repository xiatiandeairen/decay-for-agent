pub struct Thresholds {
    pub nesting: u32,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub params: u32,
    pub statement_count: u32,
    pub max_condition_ops: u32,
    pub mutable_bindings: u32,
}

pub const DEFAULT_THRESHOLDS: Thresholds = Thresholds {
    nesting: 4,
    cyclomatic: 10,
    cognitive: 15,
    params: 5,
    statement_count: 25,
    max_condition_ops: 4,
    mutable_bindings: 5,
};

pub const EXCLUDED_DIRS: &[&str] = &["target", ".git"];
pub const DB_FILENAME: &str = "snapshots.db";
pub const APP_NAME: &str = "decay";
