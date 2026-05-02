pub struct Thresholds {
    pub nesting: u32,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub params: u32,
}

pub const DEFAULT_THRESHOLDS: Thresholds = Thresholds {
    nesting: 4,
    cyclomatic: 10,
    cognitive: 15,
    params: 5,
};

pub const EXCLUDED_DIRS: &[&str] = &["target", ".git"];
pub const DB_FILENAME: &str = "snapshots.db";
pub const APP_NAME: &str = "decay";
