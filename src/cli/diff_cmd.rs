//! `decay diff` — owned by T11. This file currently exposes a stub `run()` so
//! `cli::mod` compiles; T11 will replace the body with the real implementation.

use crate::error::Result;

pub fn run() -> Result<i32> {
    // T11 will implement: load_latest_snapshots(2) → diff::diff → §2.8 output.
    // Returning Ok(0) keeps `decay diff` invocations harmless until T11 lands.
    Ok(0)
}
