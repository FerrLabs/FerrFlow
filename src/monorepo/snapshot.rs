use anyhow::{Context, Result, bail};
use semver::{BuildMetadata, Prerelease, Version};
use serde::Serialize;
use std::path::Path;

use crate::config::{Config, PackageConfig};
use crate::formats::write_version;
use crate::git::{get_repo_root, open_repo};
use crate::hooks::{HookContext, HookPackage, HookPoint, resolve_hook, resolve_policy, run_hook};

use super::check::plan_json_at;
use super::run::refresh_lockfiles;
use super::types::{CheckPackage, CheckResult};
use super::util::collect_dirty_files;

pub struct SnapshotOptions<'a> {
    pub tag: &'a str,
    pub from_zero: bool,
    pub no_git: bool,
    pub run_hooks: bool,
    pub dry_run: bool,
    pub verbose: bool,
}

#[derive(Debug, Serialize)]
struct SnapshotReport {
    tag: String,
    commit: String,
    dry_run: bool,
    packages: Vec<SnapshotPackage>,
}

#[derive(Debug, Serialize)]
struct SnapshotPackage {
    name: String,
    version: String,
    previous_version: String,
    #[serde(skip)]
    bump_type: String,
}

pub fn snapshot(config_path: Option<&Path>, opts: &SnapshotOptions<'_>) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;
    let report = run(&root, config, opts)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

fn run(root: &Path, mut config: Config, opts: &SnapshotOptions<'_>) -> Result<SnapshotReport> {
    config.workspace.recover_missed_releases = true;
    let suffix = parse_suffix(opts.tag)?;
    let repo = open_repo(root)?;

    if !opts.no_git {
        let mut dirty: Vec<String> = collect_dirty_files(&repo).into_iter().collect();
        if !dirty.is_empty() {
            dirty.sort();
            bail!(
                "working tree has uncommitted changes ({}); commit them or pass --no-git",
                dirty.join(", ")
            );
        }
    }

    let commit = repo
        .head_id()
        .context("snapshot needs a commit to stamp the version with")?
        .to_string()[..7]
        .to_string();

    let plan: CheckResult = serde_json::from_str(&plan_json_at(root, &config, None)?)?;
    let packages = plan
        .packages
        .into_iter()
        .filter_map(|pkg| snapshot_package(pkg, &suffix, &commit, opts.from_zero).transpose())
        .collect::<Result<Vec<_>>>()?;

    if !opts.dry_run {
        for pkg in &packages {
            let pkg_config = package_config(&config, &pkg.name)?;
            for vf in &pkg_config.versioned_files {
                write_version(vf, root, &pkg.version)?;
            }
            if pkg_config.effective_update_lockfiles(&config.workspace) {
                refresh_lockfiles(pkg_config, root, &mut Vec::new(), &mut Vec::new());
            }
        }
    }

    if opts.run_hooks {
        for point in [HookPoint::PrePublish, HookPoint::PostPublish] {
            run_publish_hooks(root, &config, &packages, opts, point)?;
        }
    }

    Ok(SnapshotReport {
        tag: opts.tag.to_string(),
        commit,
        dry_run: opts.dry_run,
        packages,
    })
}

fn parse_suffix(tag: &str) -> Result<Prerelease> {
    if tag.is_empty() {
        bail!("--tag must not be empty");
    }
    Prerelease::new(tag).with_context(|| {
        format!("--tag `{tag}` is not a valid semver pre-release (use [0-9A-Za-z-], dot-separated)")
    })
}

fn snapshot_package(
    pkg: CheckPackage,
    suffix: &Prerelease,
    commit: &str,
    from_zero: bool,
) -> Result<Option<SnapshotPackage>> {
    let Some(next) = pkg.next_version else {
        return Ok(None);
    };
    let version = snapshot_version(&next, suffix, commit, from_zero)
        .with_context(|| format!("package '{}': cannot snapshot version `{next}`", pkg.name))?;
    Ok(Some(SnapshotPackage {
        name: pkg.name,
        version,
        previous_version: pkg.current_version,
        bump_type: pkg.bump_type,
    }))
}

fn snapshot_version(
    next: &str,
    suffix: &Prerelease,
    commit: &str,
    from_zero: bool,
) -> Result<String> {
    let mut version = if from_zero {
        Version::new(0, 0, 0)
    } else {
        Version::parse(next)?
    };
    let commit_id = if commit.starts_with('0') && commit.bytes().all(|b| b.is_ascii_digit()) {
        format!("g{commit}")
    } else {
        commit.to_string()
    };
    version.pre = Prerelease::new(&format!("{suffix}.{commit_id}"))?;
    version.build = BuildMetadata::EMPTY;
    Ok(version.to_string())
}

fn package_config<'a>(config: &'a Config, name: &str) -> Result<&'a PackageConfig> {
    config
        .packages
        .iter()
        .find(|p| p.name == name)
        .with_context(|| format!("package '{name}' not found in config"))
}

fn run_publish_hooks(
    root: &Path,
    config: &Config,
    packages: &[SnapshotPackage],
    opts: &SnapshotOptions<'_>,
    point: HookPoint,
) -> Result<()> {
    let all_packages: Vec<HookPackage> = packages
        .iter()
        .map(|p| HookPackage {
            name: p.name.clone(),
            version: p.version.clone(),
            bump: p.bump_type.clone(),
        })
        .collect();
    let ws_hooks = config.workspace.hooks.as_ref();

    for pkg in packages {
        let pkg_config = package_config(config, &pkg.name)?;
        let pkg_hooks = pkg_config.hooks.as_ref();
        let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, point) else {
            continue;
        };
        let ctx = HookContext {
            package: pkg.name.clone(),
            old_version: pkg.previous_version.clone(),
            new_version: pkg.version.clone(),
            bump_type: pkg.bump_type.clone(),
            tag: String::new(),
            dry_run: opts.dry_run,
            package_path: root
                .join(pkg_config.path.trim_start_matches("./"))
                .to_string_lossy()
                .into_owned(),
            channel: Some(opts.tag.to_string()),
            error_code: None,
            monorepo: config.is_monorepo(),
            is_prerelease: true,
            changelog: String::new(),
            commits: Vec::new(),
            bumped_files: Vec::new(),
            all_packages: all_packages.clone(),
            release_url: None,
        };
        run_hook(
            point,
            &cmd,
            &ctx,
            resolve_policy(pkg_hooks, ws_hooks),
            opts.dry_run,
            opts.verbose,
            root,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
