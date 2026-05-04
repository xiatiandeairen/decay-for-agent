//! CLI entry point: clap definition + dispatch.
//!
//! Subcommands map to module-level handlers; the no-subcommand case dispatches
//! to `check`, which is the primary day-to-day workflow.

pub mod check_cmd;
pub mod common;
pub mod diff_cmd;
pub mod hotspots_cmd;
pub mod init_cmd;

use clap::{Parser, Subcommand};

use crate::error::Result;

#[derive(Parser)]
#[command(version, about = "Function-level complexity degradation early-warning")]
pub struct Cli {
    #[command(flatten)]
    pub scan: common::ScanArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create or refresh the baseline snapshot for the current project.
    Init,
    /// Compare the current tree against the latest saved baseline snapshot.
    Check,
    /// Compare the two most recent snapshots without creating a new one.
    Diff,
    /// Show current functions whose metrics exceed thresholds.
    Hotspots,
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
        None | Some(Commands::Check) => check_cmd::run(&cli.scan),
        Some(Commands::Init) => init_cmd::run(&cli.scan),
        Some(Commands::Diff) => diff_cmd::run(),
        Some(Commands::Hotspots) => hotspots_cmd::run(&cli.scan),
    }
}
