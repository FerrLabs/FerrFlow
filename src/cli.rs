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
        /// Force a specific version, skipping commit analysis.
        /// Format: VERSION (single repo) or NAME@VERSION (monorepo)
        #[arg(long, value_name = "VERSION")]
        force_version: Option<String>,
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

impl Commands {
    pub fn name(&self) -> &'static str {
        match self {
            Commands::Check { .. } => "check",
            Commands::Release { .. } => "release",
            Commands::Changelog => "changelog",
            Commands::Init { .. } => "init",
            Commands::Status { .. } => "status",
            Commands::Version { .. } => "version",
            Commands::Tag { .. } => "tag",
            Commands::Validate { .. } => "validate",
            Commands::Completions { .. } => "completions",
        }
    }
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
                force_version,
                channel,
                draft,
            } => crate::monorepo::release(
                self.config.as_deref(),
                self.dry_run,
                self.verbose,
                force,
                force_version.as_deref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).unwrap()
    }

    #[test]
    fn parse_check() {
        let cli = parse(&["ferrflow", "check"]);
        assert!(matches!(
            cli.command,
            Commands::Check {
                json: false,
                channel: None
            }
        ));
    }

    #[test]
    fn parse_check_json() {
        let cli = parse(&["ferrflow", "check", "--json"]);
        assert!(matches!(cli.command, Commands::Check { json: true, .. }));
    }

    #[test]
    fn parse_check_channel() {
        let cli = parse(&["ferrflow", "check", "--channel", "beta"]);
        match cli.command {
            Commands::Check { channel, .. } => assert_eq!(channel.as_deref(), Some("beta")),
            _ => panic!("expected Check"),
        }
    }

    #[test]
    fn parse_release() {
        let cli = parse(&["ferrflow", "release"]);
        assert!(matches!(
            cli.command,
            Commands::Release {
                force: false,
                force_version: None,
                channel: None,
                draft: false
            }
        ));
    }

    #[test]
    fn parse_release_force_draft_channel() {
        let cli = parse(&[
            "ferrflow",
            "release",
            "--force",
            "--draft",
            "--channel",
            "rc",
        ]);
        match cli.command {
            Commands::Release {
                force,
                channel,
                draft,
                ..
            } => {
                assert!(force);
                assert!(draft);
                assert_eq!(channel.as_deref(), Some("rc"));
            }
            _ => panic!("expected Release"),
        }
    }

    #[test]
    fn parse_release_force_version() {
        let cli = parse(&["ferrflow", "release", "--force-version", "api@2.0.0"]);
        match cli.command {
            Commands::Release { force_version, .. } => {
                assert_eq!(force_version.as_deref(), Some("api@2.0.0"));
            }
            _ => panic!("expected Release"),
        }
    }

    #[test]
    fn parse_init_no_format() {
        let cli = parse(&["ferrflow", "init"]);
        assert!(matches!(cli.command, Commands::Init { format: None }));
    }

    #[test]
    fn parse_init_with_format() {
        let cli = parse(&["ferrflow", "init", "--format", "toml"]);
        match cli.command {
            Commands::Init { format } => assert!(format.is_some()),
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn parse_status_default() {
        let cli = parse(&["ferrflow", "status"]);
        assert!(matches!(cli.command, Commands::Status { .. }));
    }

    #[test]
    fn parse_status_json() {
        let cli = parse(&["ferrflow", "status", "--output", "json"]);
        match cli.command {
            Commands::Status { output } => assert!(matches!(output, OutputFormat::Json)),
            _ => panic!("expected Status"),
        }
    }

    #[test]
    fn parse_version_no_package() {
        let cli = parse(&["ferrflow", "version"]);
        assert!(matches!(
            cli.command,
            Commands::Version {
                package: None,
                json: false
            }
        ));
    }

    #[test]
    fn parse_version_with_package_json() {
        let cli = parse(&["ferrflow", "version", "my-pkg", "--json"]);
        match cli.command {
            Commands::Version { package, json } => {
                assert_eq!(package.as_deref(), Some("my-pkg"));
                assert!(json);
            }
            _ => panic!("expected Version"),
        }
    }

    #[test]
    fn parse_tag_no_package() {
        let cli = parse(&["ferrflow", "tag"]);
        assert!(matches!(
            cli.command,
            Commands::Tag {
                package: None,
                json: false
            }
        ));
    }

    #[test]
    fn parse_tag_with_package() {
        let cli = parse(&["ferrflow", "tag", "core"]);
        match cli.command {
            Commands::Tag { package, .. } => assert_eq!(package.as_deref(), Some("core")),
            _ => panic!("expected Tag"),
        }
    }

    #[test]
    fn parse_validate() {
        let cli = parse(&["ferrflow", "validate"]);
        assert!(matches!(
            cli.command,
            Commands::Validate {
                json: false,
                repo: None,
                git_ref: None
            }
        ));
    }

    #[test]
    fn parse_validate_remote() {
        let cli = parse(&[
            "ferrflow",
            "validate",
            "--json",
            "--repo",
            "owner/repo",
            "--git-ref",
            "main",
        ]);
        match cli.command {
            Commands::Validate {
                json,
                repo,
                git_ref,
            } => {
                assert!(json);
                assert_eq!(repo.as_deref(), Some("owner/repo"));
                assert_eq!(git_ref.as_deref(), Some("main"));
            }
            _ => panic!("expected Validate"),
        }
    }

    #[test]
    fn parse_completions() {
        let cli = parse(&["ferrflow", "completions", "bash"]);
        assert!(matches!(cli.command, Commands::Completions { .. }));
    }

    #[test]
    fn parse_changelog() {
        let cli = parse(&["ferrflow", "changelog"]);
        assert!(matches!(cli.command, Commands::Changelog));
    }

    #[test]
    fn global_dry_run() {
        let cli = parse(&["ferrflow", "--dry-run", "check"]);
        assert!(cli.dry_run);
    }

    #[test]
    fn global_verbose() {
        let cli = parse(&["ferrflow", "-v", "check"]);
        assert!(cli.verbose);
    }

    #[test]
    fn global_config_path() {
        let cli = parse(&["ferrflow", "--config", "/tmp/ferrflow.json", "check"]);
        assert_eq!(cli.config, Some(PathBuf::from("/tmp/ferrflow.json")));
    }

    #[test]
    fn global_flags_after_subcommand() {
        let cli = parse(&["ferrflow", "release", "--dry-run", "--verbose"]);
        assert!(cli.dry_run);
        assert!(cli.verbose);
    }

    #[test]
    fn unknown_subcommand_fails() {
        assert!(Cli::try_parse_from(["ferrflow", "unknown"]).is_err());
    }

    #[test]
    fn missing_subcommand_fails() {
        assert!(Cli::try_parse_from(["ferrflow"]).is_err());
    }

    #[test]
    fn command_names() {
        assert_eq!(parse(&["ferrflow", "check"]).command.name(), "check");
        assert_eq!(parse(&["ferrflow", "release"]).command.name(), "release");
        assert_eq!(
            parse(&["ferrflow", "changelog"]).command.name(),
            "changelog"
        );
        assert_eq!(parse(&["ferrflow", "init"]).command.name(), "init");
        assert_eq!(parse(&["ferrflow", "status"]).command.name(), "status");
        assert_eq!(parse(&["ferrflow", "version"]).command.name(), "version");
        assert_eq!(parse(&["ferrflow", "tag"]).command.name(), "tag");
        assert_eq!(parse(&["ferrflow", "validate"]).command.name(), "validate");
    }
}
