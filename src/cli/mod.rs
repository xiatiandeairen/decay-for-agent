//! CLI entry point: clap definition + dispatch.
//!
//! Subcommands map to module-level handlers; the no-subcommand case dispatches
//! to `scan` (the default `decay` behaviour per §2.8).

pub mod diff_cmd;
pub mod scan;

use clap::{Parser, Subcommand};

use crate::error::Result;

#[derive(Parser)]
#[command(version, about = "Function-level complexity degradation early-warning")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compare the two most recent snapshots without creating a new one.
    Diff,
}

/// Parse argv, init logger (RUST_LOG controls verbosity), dispatch.
///
/// Returns the process exit code; `main` calls `process::exit(run())`.
pub fn run() -> Result<i32> {
    // Idempotent enough for our use; tests that invoke run() multiple times
    // would need init guard, but v0.1 only calls it once from main.
    env_logger::init();

    let cli = Cli::parse();
    match cli.command {
        None => scan::run(),
        Some(Commands::Diff) => diff_cmd::run(),
    }
}
