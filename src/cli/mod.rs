//! CLI entry point: clap definition + dispatch.
//!
//! Subcommands map to module-level handlers.

pub(crate) mod baseline_cmd;
pub(crate) mod common;
pub(crate) mod diff_cmd;
pub(crate) mod doctor_cmd;

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
    /// Diagnose current maintainability risks without reading baselines.
    Doctor,
    /// Save the current tree as a named baseline.
    Baseline {
        /// Required baseline version/name, e.g. v1.0.0.
        version: String,
        /// Replace an existing baseline with the same version when it differs.
        #[arg(long)]
        replace: bool,
    },
    /// Compare a baseline to the current tree, or compare two baselines.
    Diff {
        /// Older baseline version.
        from: String,
        /// Newer baseline version. Omit to compare current tree against `from`.
        to: Option<String>,
    },
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
        Some(Commands::Doctor) => doctor_cmd::run(&cli.scan),
        Some(Commands::Baseline { version, replace }) => {
            baseline_cmd::run(&cli.scan, &version, replace)
        }
        Some(Commands::Diff { from, to }) => diff_cmd::run(&cli.scan, &from, to.as_deref()),
        None => {
            println!("decay commands:");
            println!("  decay doctor              Diagnose current code risks");
            println!("  decay baseline <version>  Save current code as a named baseline");
            println!("  decay diff <from> [to]    Compare baseline(s) and report degradations");
            println!();
            println!("Run `decay --help` for details.");
            Ok(0)
        }
    }
}
