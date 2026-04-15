mod action;
mod classify;
mod collector;
mod config;
mod data_store;
mod db;
mod diagnose;
mod dimension;
mod filter;
mod filter_pipeline;
mod git;
mod git_pipeline;
mod profile;
mod render;
mod run;
mod scan;
mod trend;
mod util;

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

    let mut log_builder = env_logger::Builder::from_default_env();
    if cli.debug {
        log_builder.filter_level(log::LevelFilter::Debug);
    }
    log_builder.init();

    let has_critical = run::run(cli.json, cli.markdown, cli.quiet)?;

    if has_critical {
        std::process::exit(1);
    }

    Ok(())
}
