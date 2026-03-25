mod changelog;
mod cli;
mod config;
mod conventional_commits;
mod formats;
mod git;
mod monorepo;
mod release;
mod status;
mod versioning;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}
