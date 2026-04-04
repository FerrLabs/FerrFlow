use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

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
    Check {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Pre-release channel override (e.g. beta, rc, dev)
        #[arg(long)]
        channel: Option<String>,
    },
    /// Bump versions, update changelogs, create tags and push
    Release {
        /// Allow floating tags to move backward to a lower version
        #[arg(long)]
        force: bool,
        /// Pre-release channel override (e.g. beta, rc, dev)
        #[arg(long)]
        channel: Option<String>,
        /// Create releases as drafts (GitHub only). A subsequent `ferrflow release`
        /// without --draft will detect and publish existing drafts automatically.
        #[arg(long)]
        draft: bool,
    },
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
    /// Print the current version of a package
    Version {
        /// Package name (required in monorepos, optional in single repos)
        package: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Print the last release tag of a package
    Tag {
        /// Package name (required in monorepos, optional in single repos)
        package: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate config and versioned files
    Validate {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Remote repository (e.g. owner/repo for GitHub, or gitlab:group/project)
        #[arg(long)]
        repo: Option<String>,
        /// Git ref for remote validation (branch, tag, commit)
        #[arg(long, name = "ref")]
        git_ref: Option<String>,
    },
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Commands::Check { json, channel } => crate::monorepo::check(
                self.config.as_deref(),
                self.verbose,
                json,
                channel.as_deref(),
            ),
            Commands::Release {
                force,
                channel,
                draft,
            } => crate::monorepo::release(
                self.config.as_deref(),
                self.dry_run,
                self.verbose,
                force,
                channel.as_deref(),
                draft,
            ),
            Commands::Changelog => {
                crate::changelog::generate_only(self.config.as_deref(), self.dry_run)
            }
            Commands::Init { format } => crate::config::init(format),
            Commands::Status { output } => crate::status::run(self.config.as_deref(), &output),
            Commands::Version { package, json } => {
                crate::query::version(self.config.as_deref(), package.as_deref(), json)
            }
            Commands::Tag { package, json } => {
                crate::query::tag(self.config.as_deref(), package.as_deref(), json)
            }
            Commands::Validate {
                json,
                repo,
                git_ref,
            } => crate::validate::run(
                self.config.as_deref(),
                json,
                repo.as_deref(),
                git_ref.as_deref(),
            ),
            Commands::Completions { shell } => {
                clap_complete::generate(
                    shell,
                    &mut Cli::command(),
                    "ferrflow",
                    &mut std::io::stdout(),
                );
                Ok(())
            }
        }
    }
}
