use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::status::OutputFormat;

#[derive(Parser)]
#[command(name = "ferrflow")]
#[command(about = "Universal semantic versioning for monorepos and classic repos")]
#[command(version)]
pub struct Cli {
    /// Dry run — show what would happen without making changes
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show what versions would be bumped (dry run)
    Check,
    /// Bump versions, update changelogs, create tags and push
    Release,
    /// Generate/update CHANGELOG.md only
    Changelog,
    /// Scaffold a ferrflow.toml configuration file
    Init,
    /// Print each package name, current version, and last release tag
    Status {
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        output: OutputFormat,
    },
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Commands::Check => crate::monorepo::check(self.verbose),
            Commands::Release => crate::monorepo::release(self.dry_run, self.verbose),
            Commands::Changelog => crate::changelog::generate_only(self.dry_run),
            Commands::Init => crate::config::init(),
            Commands::Status { output } => crate::status::run(&output),
        }
    }
}
