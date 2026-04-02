mod changelog;
mod cli;
mod config;
mod conventional_commits;
mod forge;
mod formats;
mod git;
mod hooks;
mod monorepo;
mod query;
mod status;
mod telemetry;
mod validate;
mod versioning;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let result = cli.run();
    telemetry::flush();
    result
}
