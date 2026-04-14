mod db;
mod diagnose;
mod dimension;
mod filter;
mod git;
mod run;
mod scan;
mod trend;

use clap::Parser;

/// Project health monitoring for AI agents
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Output results as JSON
    #[arg(long, conflicts_with_all = ["markdown", "quiet"])]
    json: bool,

    /// Output results as Markdown report
    #[arg(long, conflicts_with_all = ["json", "quiet"])]
    markdown: bool,

    /// Output one-line summary; exit 1 if critical issues exist
    #[arg(long, conflicts_with_all = ["json", "markdown"])]
    quiet: bool,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        unsafe {
            std::env::set_var("RUST_LOG", "debug");
        }
    }
    env_logger::init();

    let has_critical = run::run(cli.json, cli.markdown, cli.quiet)?;

    if has_critical {
        std::process::exit(1);
    }

    Ok(())
}
