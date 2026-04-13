use anyhow::Result;
use clap::{CommandFactory, Parser};

/// Project health monitoring for AI agents
#[derive(Parser)]
#[command(version, about)]
struct Cli {}

fn main() -> Result<()> {
    let cli = Cli::try_parse();

    match cli {
        Ok(_) => {
            // No subcommands yet — show help when called with no args
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
        }
        Err(e) => {
            // Let clap handle --help, --version, and errors naturally
            e.exit();
        }
    }

    Ok(())
}
