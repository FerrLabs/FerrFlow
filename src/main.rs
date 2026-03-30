mod changelog;
mod cli;
mod config;
mod conventional_commits;
mod formats;
mod git;
mod hooks;
mod monorepo;
mod query;
mod release;
mod status;
mod telemetry;
mod versioning;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}
