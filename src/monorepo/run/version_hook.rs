use anyhow::{Result, bail};
use semver::Version;
use std::path::Path;

use crate::changelog::GitLog;
use crate::config::{Config, PackageConfig, VersioningStrategy};
use crate::conventional_commits::BumpType;
use crate::hooks::{HookCommit, HookContext, capture_version};
use crate::prerelease::PrereleaseContext;

use super::super::util::tags_for_package;
use super::plan::{PackagePlan, SkipReason};

enum Settled {
    Version { version: String, tag: String },
    Unchanged,
}

struct Request<'a> {
    pub config: &'a Config,
    pub root: &'a Path,
    pub all_tags: &'a [String],
    pub pkg: &'a PackageConfig,
    pub command: &'a str,
    pub current: &'a str,
    pub proposed: &'a str,
    pub bump: BumpType,
    pub is_prerelease: bool,
    pub channel: Option<&'a str>,
    pub dry_run: bool,
    pub commits: &'a [GitLog],
}

fn settle(req: &Request<'_>) -> Result<Settled> {
    let config = req.config;
    let is_monorepo = config.is_monorepo();
    let proposed_tag = req
        .pkg
        .tag_for_version(&config.workspace, is_monorepo, req.proposed);
    let context = HookContext {
        package: req.pkg.name.clone(),
        old_version: req.current.to_string(),
        new_version: req.proposed.to_string(),
        bump_type: req.bump.to_string(),
        tag: proposed_tag,
        dry_run: req.dry_run,
        package_path: req
            .root
            .join(req.pkg.path.trim_start_matches("./"))
            .to_string_lossy()
            .into_owned(),
        channel: req.channel.map(str::to_string),
        error_code: None,
        monorepo: is_monorepo,
        is_prerelease: req.is_prerelease,
        changelog: String::new(),
        commits: req
            .commits
            .iter()
            .map(|c| HookCommit::from_commit(&c.hash, &c.message, &config.workspace.commit_formats))
            .collect(),
        bumped_files: Vec::new(),
        all_packages: Vec::new(),
        release_url: None,
    };

    let output = capture_version(req.command, &context, req.root)?;
    let prefix = req.pkg.tag_prefix(&config.workspace, is_monorepo);
    let strategy = req.pkg.effective_versioning(&config.workspace, || {
        tags_for_package(req.all_tags, &prefix)
    });

    let Some(version) = validate(strategy, req.current, &output, req.command)? else {
        return Ok(Settled::Unchanged);
    };
    let tag = req
        .pkg
        .tag_for_version(&config.workspace, is_monorepo, &version);
    if req.all_tags.contains(&tag) {
        bail!(
            "versionCommand returned {version} for {}, but tag {tag} already exists",
            req.pkg.name
        );
    }
    Ok(Settled::Version { version, tag })
}

fn validate(
    strategy: VersioningStrategy,
    current: &str,
    output: &str,
    command: &str,
) -> Result<Option<String>> {
    let version = output.strip_prefix('v').unwrap_or(output);
    if version == current {
        return Ok(None);
    }
    match strategy {
        VersioningStrategy::Semver | VersioningStrategy::Zerover => {
            let next = Version::parse(version).map_err(|e| {
                anyhow::anyhow!("versionCommand must print a semver version, got {output:?} from: {command} ({e})")
            })?;
            if let Ok(previous) = Version::parse(current)
                && next <= previous
            {
                bail!(
                    "versionCommand returned {version}, which does not go past {current}: {command}"
                );
            }
        }
        _ => {
            let valid = version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'));
            if !valid {
                bail!("versionCommand must print a bare version, got {output:?} from: {command}");
            }
        }
    }
    Ok(Some(version.to_string()))
}

pub(super) enum Resolved {
    Plain,
    Pending,
    Version { version: String, tag: String },
    Unchanged,
}

pub(super) struct Candidate<'a> {
    pub pkg: &'a PackageConfig,
    pub current: &'a str,
    pub proposed: &'a str,
    pub bump: BumpType,
    pub is_prerelease: bool,
    pub commits: &'a [GitLog],
}

pub(super) struct Pass<'a> {
    pub config: &'a Config,
    pub root: &'a Path,
    pub all_tags: &'a [String],
    pub channel: Option<&'a str>,
    pub prerelease_ctx: &'a PrereleaseContext,
    pub short_hash: &'a str,
    pub dry_run: bool,
    pub run_commands: bool,
}

impl Pass<'_> {
    pub(super) fn resolve(&self, candidate: &Candidate<'_>) -> Result<Resolved> {
        let Some(command) = candidate
            .pkg
            .effective_version_command(&self.config.workspace)
        else {
            return Ok(Resolved::Plain);
        };
        if !self.run_commands {
            return Ok(Resolved::Pending);
        }
        let settled = settle(&Request {
            config: self.config,
            root: self.root,
            all_tags: self.all_tags,
            pkg: candidate.pkg,
            command,
            current: candidate.current,
            proposed: candidate.proposed,
            bump: candidate.bump,
            is_prerelease: candidate.is_prerelease,
            channel: self.channel,
            dry_run: self.dry_run,
            commits: candidate.commits,
        })?;
        Ok(match settled {
            Settled::Version { version, tag } => Resolved::Version { version, tag },
            Settled::Unchanged => Resolved::Unchanged,
        })
    }

    pub(super) fn apply(&self, order: &[usize], plans: &mut [Option<PackagePlan>]) -> Result<()> {
        for &idx in order {
            let Some(PackagePlan::Bump(bump)) = plans[idx].as_mut() else {
                continue;
            };
            if bump.strategy_label == "forced" {
                continue;
            }
            let resolved = self.resolve(&Candidate {
                pkg: &self.config.packages[idx],
                current: &bump.current_version,
                proposed: &bump.new_version,
                bump: bump.bump,
                is_prerelease: bump.is_prerelease,
                commits: &bump.commits,
            })?;
            match resolved {
                Resolved::Plain => {}
                Resolved::Pending => bump.version_pending = true,
                Resolved::Version { version, tag } => {
                    bump.new_version = version;
                    bump.tag = tag;
                }
                Resolved::Unchanged => {
                    let version = bump.current_version.clone();
                    let recovered = bump.recovered;
                    plans[idx] = Some(PackagePlan::Skipped {
                        reason: SkipReason::VersionUnchanged { version },
                        recovered,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(strategy: VersioningStrategy, current: &str, output: &str) -> Result<Option<String>> {
        validate(strategy, current, output, "cmd")
    }

    #[test]
    fn semver_output_must_go_past_the_current_version() {
        let semver = VersioningStrategy::Semver;
        assert_eq!(
            check(semver, "1.2.0", "1.3.0").unwrap().as_deref(),
            Some("1.3.0")
        );
        assert!(check(semver, "1.2.0", "1.1.0").is_err());
    }

    #[test]
    fn the_current_version_means_nothing_to_release() {
        assert_eq!(
            check(VersioningStrategy::Semver, "1.2.0", "1.2.0").unwrap(),
            None
        );
        assert_eq!(
            check(VersioningStrategy::Semver, "1.2.0", "v1.2.0").unwrap(),
            None
        );
    }

    #[test]
    fn a_leading_v_is_dropped_because_the_tag_template_adds_it() {
        assert_eq!(
            check(VersioningStrategy::Semver, "1.0.0", "v2.0.0")
                .unwrap()
                .as_deref(),
            Some("2.0.0")
        );
    }

    #[test]
    fn semver_strategies_reject_a_non_version() {
        assert!(check(VersioningStrategy::Semver, "1.0.0", "next").is_err());
        assert!(check(VersioningStrategy::Zerover, "0.1.0", "1.0").is_err());
    }

    #[test]
    fn other_strategies_reject_whitespace_and_shell_noise() {
        assert!(check(VersioningStrategy::Calver, "2026.1.1", "2026.2.1 beta").is_err());
        assert_eq!(
            check(VersioningStrategy::Calver, "2026.1.1", "2026.2.1")
                .unwrap()
                .as_deref(),
            Some("2026.2.1")
        );
    }
}
