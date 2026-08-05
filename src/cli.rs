use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::config::ConfigFileFormat;
use crate::doctor::DoctorFormat;
use crate::logging::LogFormat;
use crate::status::OutputFormat;
use crate::timing::Timing;

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

    /// Print a per-stage timing breakdown to stderr after the command finishes
    #[arg(long, global = true)]
    pub timing: bool,

    /// Max threads for CPU-parallel work (per-package planning). Default: all cores. `1` forces single-threaded.
    #[arg(long, global = true, env = "FERRFLOW_JOBS", value_name = "N")]
    pub jobs: Option<usize>,

    /// Log output format. `human` (default) keeps the colored terminal output; `json` emits one structured JSON event per line for CI ingestion.
    #[arg(long, value_enum, default_value_t = LogFormat::default(), global = true)]
    pub log_format: LogFormat,

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
        /// Post a preview comment on the current PR/MR
        #[arg(long)]
        comment: bool,
    },
    /// Bump versions, update changelogs, create tags and push
    Release {
        /// Output a single JSON object describing the release
        #[arg(long)]
        json: bool,
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
        /// without --draft detects and publishes the draft for the version being
        /// released — older drafts left by failed runs are not swept up.
        /// On GitLab this is a no-op — the release is published immediately
        /// (the GitLab releases API has no draft state); a warning is printed.
        #[arg(long)]
        draft: bool,
        /// Break an existing `.git/ferrflow.lock` before acquiring it.
        /// Use only when you're sure no other `ferrflow release` is running —
        /// for example, after a crash that left the lockfile behind.
        #[arg(long)]
        force_unlock: bool,
    },
    /// Run the configured publishers for the currently-released version
    /// of each package, without bumping or tagging. Use after `release`
    /// has cut the version — typically in a separate CI job that has the
    /// build toolchain and registry auth the publishers need (docker
    /// buildx, helm, npm, …).
    Publish {
        /// Packages to publish. Omit to auto-detect from the triggering tag
        /// (GITHUB_REF / CI_COMMIT_TAG), falling back to every package.
        packages: Vec<String>,
        /// Publish every package, ignoring any triggering-tag scope.
        #[arg(short = 'a', long)]
        all: bool,
    },
    /// Generate/update CHANGELOG.md only
    Changelog,
    /// Scaffold a ferrflow configuration file
    Init {
        /// Config file format (json, json5, toml)
        #[arg(long)]
        format: Option<ConfigFileFormat>,
        /// Also scaffold a .ferrflow.manifest.json snapshot and enable
        /// manifest mode in the generated config
        #[arg(long)]
        manifest: bool,
    },
    /// Print each package name, current version, and last release tag
    Status {
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        output: OutputFormat,
    },
    /// Explain why a package would or would not be released
    Why {
        /// Package name (required in monorepos, optional in single repos)
        package: Option<String>,
        /// Pre-release channel override (e.g. beta, rc, dev)
        #[arg(long)]
        channel: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Compare two versions of a package: commits, bumps, files, and changelog
    Diff {
        /// `[package] <from>..<to>` — the version range (contains `..`), optionally preceded by a package name
        #[arg(value_name = "SPEC", required = true, num_args = 1..=2)]
        spec: Vec<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    /// Manage the cross-run cache under .git/ferrflow-cache/
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    /// Regenerate the version manifest from current filesystem state.
    /// Requires workspace.manifest_file to be configured. Use this to
    /// repair a manifest that diverged from the package files (e.g. after
    /// a crashed release).
    SyncManifest,
    /// Generate a ferrflow config from another release tool's config
    Migrate {
        /// Source tool (auto-detected if omitted)
        #[arg(long, value_enum)]
        from: Option<MigrateSourceArg>,
    },
    /// Run read-only diagnostics on the repo, config, and forge setup
    Doctor {
        /// Output format
        #[arg(long, value_enum, default_value = "human")]
        format: DoctorFormat,
        /// Also probe the forge API (rate limit / auth). Requires a token.
        #[arg(long)]
        online: bool,
    },
    /// Print the bundled JSON schema for the ferrflow config file
    Schema {
        /// Pretty-print the schema instead of compact single-line JSON
        #[arg(long)]
        pretty: bool,
        /// Write to a file instead of stdout
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum MigrateSourceArg {
    #[value(name = "semantic-release")]
    SemanticRelease,
    #[value(name = "changesets")]
    Changesets,
    #[value(name = "release-please")]
    ReleasePlease,
    #[value(name = "standard-version")]
    StandardVersion,
}

impl From<MigrateSourceArg> for crate::config::MigrateSource {
    fn from(arg: MigrateSourceArg) -> Self {
        match arg {
            MigrateSourceArg::SemanticRelease => crate::config::MigrateSource::SemanticRelease,
            MigrateSourceArg::Changesets => crate::config::MigrateSource::Changesets,
            MigrateSourceArg::ReleasePlease => crate::config::MigrateSource::ReleasePlease,
            MigrateSourceArg::StandardVersion => crate::config::MigrateSource::StandardVersion,
        }
    }
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Delete the cross-run cache directory
    Clear,
}

impl Commands {
    pub fn needs_bot_token(&self) -> bool {
        matches!(self, Commands::Release { .. } | Commands::Check { .. })
    }

    #[cfg(test)]
    pub fn name(&self) -> &'static str {
        match self {
            Commands::Check { .. } => "check",
            Commands::Release { .. } => "release",
            Commands::Publish { .. } => "publish",
            Commands::Changelog => "changelog",
            Commands::Init { .. } => "init",
            Commands::Status { .. } => "status",
            Commands::Why { .. } => "why",
            Commands::Diff { .. } => "diff",
            Commands::Version { .. } => "version",
            Commands::Tag { .. } => "tag",
            Commands::Validate { .. } => "validate",
            Commands::Completions { .. } => "completions",
            Commands::Cache { .. } => "cache",
            Commands::SyncManifest => "sync-manifest",
            Commands::Migrate { .. } => "migrate",
            Commands::Doctor { .. } => "doctor",
            Commands::Schema { .. } => "schema",
        }
    }
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let mut timing = Timing::new(self.timing);
        let result = self.dispatch(&mut timing);
        timing.report();
        result
    }

    fn dispatch(self, timing: &mut Timing) -> Result<()> {
        match self.command {
            Commands::Check {
                json,
                channel,
                comment,
            } => crate::monorepo::check(
                self.config.as_deref(),
                self.verbose,
                json,
                channel.as_deref(),
                comment,
                timing,
            ),
            Commands::Release {
                json,
                force,
                force_version,
                channel,
                draft,
                force_unlock,
            } => crate::monorepo::release(
                self.config.as_deref(),
                self.dry_run,
                self.verbose,
                json,
                force,
                force_version.as_deref(),
                channel.as_deref(),
                draft,
                force_unlock,
                timing,
            ),
            Commands::Publish { packages, all } => crate::publish::run(
                self.config.as_deref(),
                &packages,
                all,
                self.dry_run,
                self.verbose,
            ),
            Commands::Changelog => {
                crate::changelog::generate_only(self.config.as_deref(), self.dry_run)
            }
            Commands::Init { format, manifest } => crate::config::init(format, manifest),
            Commands::Status { output } => {
                crate::status::run(self.config.as_deref(), &output, timing)
            }
            Commands::Diff { spec, json } => {
                crate::version_diff::run(&spec, json, self.config.as_deref())
            }
            Commands::Why {
                package,
                channel,
                json,
            } => crate::monorepo::why(
                self.config.as_deref(),
                package.as_deref(),
                channel.as_deref(),
                json,
            ),
            Commands::Version { package, json } => {
                crate::query::version(self.config.as_deref(), package.as_deref(), json)
            }
            Commands::Tag { package, json } => {
                crate::query::tag(self.config.as_deref(), package.as_deref(), json, timing)
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
            Commands::Cache { command } => match command {
                CacheCommand::Clear => crate::cache::clear_cwd(),
            },
            Commands::SyncManifest => crate::manifest::sync_cwd(self.config.as_deref()),
            Commands::Migrate { from } => crate::config::migrate(from.map(Into::into)),
            Commands::Doctor { format, online } => {
                crate::doctor::run(self.config.as_deref(), format, online)
            }
            Commands::Schema { pretty, output } => crate::schema::run(pretty, output.as_deref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn only_push_capable_commands_need_the_bot_token() {
        let needs = |args: &[&str]| Cli::parse_from(args).command.needs_bot_token();
        assert!(needs(&["ferrflow", "release"]));
        assert!(needs(&["ferrflow", "check"]));
        assert!(!needs(&["ferrflow", "status"]));
        assert!(!needs(&["ferrflow", "version"]));
        assert!(!needs(&["ferrflow", "tag"]));
        assert!(!needs(&["ferrflow", "validate"]));
        assert!(!needs(&["ferrflow", "changelog"]));
        assert!(!needs(&["ferrflow", "init"]));
    }

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).unwrap()
    }

    #[test]
    fn parse_schema() {
        let cli = parse(&["ferrflow", "schema"]);
        assert!(matches!(
            cli.command,
            Commands::Schema {
                pretty: false,
                output: None
            }
        ));
        let cli = parse(&["ferrflow", "schema", "--pretty", "--output", "s.json"]);
        assert!(matches!(
            cli.command,
            Commands::Schema {
                pretty: true,
                output: Some(_)
            }
        ));
    }

    #[test]
    fn parse_check() {
        let cli = parse(&["ferrflow", "check"]);
        assert!(matches!(
            cli.command,
            Commands::Check {
                json: false,
                channel: None,
                comment: false,
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
                json: false,
                force: false,
                force_version: None,
                channel: None,
                draft: false,
                force_unlock: false,
            }
        ));
    }

    #[test]
    fn parse_release_json() {
        let cli = parse(&["ferrflow", "release", "--json"]);
        match cli.command {
            Commands::Release { json, .. } => assert!(json),
            _ => panic!("expected Release"),
        }
    }

    #[test]
    fn parse_release_force_unlock() {
        let cli = parse(&["ferrflow", "release", "--force-unlock"]);
        match cli.command {
            Commands::Release { force_unlock, .. } => assert!(force_unlock),
            _ => panic!("expected Release"),
        }
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
        assert!(matches!(
            cli.command,
            Commands::Init {
                format: None,
                manifest: false
            }
        ));
    }

    #[test]
    fn parse_why_with_package_and_flags() {
        let cli = parse(&["ferrflow", "why", "api", "--channel", "beta", "--json"]);
        match cli.command {
            Commands::Why {
                package,
                channel,
                json,
            } => {
                assert_eq!(package.as_deref(), Some("api"));
                assert_eq!(channel.as_deref(), Some("beta"));
                assert!(json);
            }
            _ => panic!("expected Why"),
        }
    }

    // The package is optional so single-package repos need no argument.
    #[test]
    fn parse_why_without_a_package() {
        let cli = parse(&["ferrflow", "why"]);
        match cli.command {
            Commands::Why { package, json, .. } => {
                assert!(package.is_none());
                assert!(!json);
            }
            _ => panic!("expected Why"),
        }
    }

    #[test]
    fn parse_init_with_format() {
        let cli = parse(&["ferrflow", "init", "--format", "toml"]);
        match cli.command {
            Commands::Init { format, .. } => assert!(format.is_some()),
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn parse_init_with_manifest() {
        let cli = parse(&["ferrflow", "init", "--manifest"]);
        match cli.command {
            Commands::Init { manifest, .. } => assert!(manifest),
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn parse_sync_manifest() {
        let cli = parse(&["ferrflow", "sync-manifest"]);
        assert!(matches!(cli.command, Commands::SyncManifest));
        assert_eq!(cli.command.name(), "sync-manifest");
    }

    #[test]
    fn parse_status_default() {
        let cli = parse(&["ferrflow", "status"]);
        assert!(matches!(cli.command, Commands::Status { .. }));
    }

    #[test]
    fn parse_publish_no_package() {
        let cli = parse(&["ferrflow", "publish"]);
        assert!(
            matches!(cli.command, Commands::Publish { packages, all } if packages.is_empty() && !all)
        );
    }

    #[test]
    fn parse_publish_with_packages_and_dry_run() {
        let cli = parse(&["ferrflow", "--dry-run", "publish", "api", "web"]);
        assert!(cli.dry_run);
        match cli.command {
            Commands::Publish { packages, all } => {
                assert_eq!(packages, vec!["api", "web"]);
                assert!(!all);
            }
            _ => panic!("expected Publish"),
        }
    }

    #[test]
    fn parse_publish_all_flag() {
        let cli = parse(&["ferrflow", "publish", "--all"]);
        match cli.command {
            Commands::Publish { packages, all } => {
                assert!(packages.is_empty());
                assert!(all);
            }
            _ => panic!("expected Publish"),
        }
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
    fn parse_cache_clear() {
        let cli = parse(&["ferrflow", "cache", "clear"]);
        assert!(matches!(
            cli.command,
            Commands::Cache {
                command: CacheCommand::Clear
            }
        ));
        assert_eq!(cli.command.name(), "cache");
    }

    #[test]
    fn cache_requires_subcommand() {
        assert!(Cli::try_parse_from(["ferrflow", "cache"]).is_err());
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
    fn global_timing_default_off() {
        let cli = parse(&["ferrflow", "check"]);
        assert!(!cli.timing);
    }

    #[test]
    fn global_timing() {
        let cli = parse(&["ferrflow", "--timing", "check"]);
        assert!(cli.timing);
    }

    #[test]
    fn global_jobs_default_none() {
        let cli = parse(&["ferrflow", "check"]);
        assert_eq!(cli.jobs, None);
    }

    #[test]
    fn global_jobs_flag() {
        let cli = parse(&["ferrflow", "release", "--jobs", "1", "--dry-run"]);
        assert_eq!(cli.jobs, Some(1));
    }

    #[test]
    fn global_timing_after_subcommand() {
        let cli = parse(&["ferrflow", "release", "--timing", "--dry-run"]);
        assert!(cli.timing);
        assert!(cli.dry_run);
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
