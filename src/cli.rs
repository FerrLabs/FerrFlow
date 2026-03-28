use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::ConfigFileFormat;
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

    /// Path to config file (overrides auto-detection, env: FERRFLOW_CONFIG)
    #[arg(long, global = true, env = "FERRFLOW_CONFIG")]
    pub config: Option<PathBuf>,

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
    /// Scaffold a ferrflow configuration file
    Init {
        /// Config file format (json, json5, toml)
        #[arg(long)]
        format: Option<ConfigFileFormat>,
    },
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
            Commands::Check => crate::monorepo::check(self.config.as_deref(), self.verbose),
            Commands::Release => {
                crate::monorepo::release(self.config.as_deref(), self.dry_run, self.verbose)
            }
            Commands::Changelog => {
                crate::changelog::generate_only(self.config.as_deref(), self.dry_run)
            }
            Commands::Init { format } => crate::config::init(format),
            Commands::Status { output } => crate::status::run(self.config.as_deref(), &output),
        }
    }
}
