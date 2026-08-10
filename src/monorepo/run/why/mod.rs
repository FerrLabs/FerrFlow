use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::config::{Config, PackageConfig};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::read_version;
use crate::git::{collect_all_tags, get_changed_files, get_repo_root, open_repo};
use crate::prerelease::PrereleaseContext;
use crate::versioning::compute_next_version;

use super::super::util::tags_for_package;
use super::plan::{
    ChangedFilesCache, PackagePlan, PlanInputs, commits_for_package, compute_plan, evaluate_touch,
};

mod render;
mod tag_info;

#[derive(Serialize)]
pub(super) struct Explanation {
    package: String,
    path: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    shared_paths: Vec<String>,
    strategy: String,
    current_version: String,
    monorepo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    touch: TouchReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_tag: Option<tag_info::TagReport>,
    commits: Vec<CommitReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dependencies: Vec<DependencyReport>,
    decision: Decision,
}

#[derive(Serialize)]
struct TouchReport {
    touched: bool,
    /// The package was pulled in by `recoverMissedReleases`, so the file set
    /// below spans everything since its last tag rather than just HEAD.
    recovered: bool,
    files: Vec<FileMatch>,
}

#[derive(Serialize)]
struct FileMatch {
    path: String,
    /// The `path` / `sharedPaths` prefix this file matched, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    matched: Option<String>,
}

#[derive(Serialize)]
struct CommitReport {
    hash: String,
    subject: String,
    bump: String,
}

#[derive(Serialize)]
struct DependencyReport {
    name: String,
    propagate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream_bump: Option<String>,
    resulting_bump: String,
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
enum Decision {
    Bump {
        bump: String,
        from: String,
        to: String,
        tag: String,
        prerelease: bool,
        triggered_by: Trigger,
    },
    Skipped {
        reason: String,
    },
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum Trigger {
    Commits,
    Dependency,
    Forced,
}

pub fn why(
    config_path: Option<&Path>,
    package: Option<&str>,
    channel: Option<&str>,
    json: bool,
) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    let pkg = resolve_package(&config, package)?;
    let explanation = explain(&repo, &root, &config, pkg, channel)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&explanation)?);
    } else {
        for line in render::lines(&explanation) {
            tracing::info!("{line}");
        }
    }
    Ok(())
}

fn resolve_package<'a>(config: &'a Config, package: Option<&str>) -> Result<&'a PackageConfig> {
    if let Some(name) = package {
        return config
            .packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| {
                let known: Vec<&str> = config.packages.iter().map(|p| p.name.as_str()).collect();
                anyhow::anyhow!(
                    "unknown package '{name}'. Configured packages: {}",
                    known.join(", ")
                )
            });
    }
    match config.packages.as_slice() {
        [] => Err(anyhow::anyhow!(
            "No packages configured. Run `ferrflow init` to create a config."
        )),
        [only] => Ok(only),
        many => Err(anyhow::anyhow!(
            "this repo has {} packages — name the one to explain: {}",
            many.len(),
            many.iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn explain(
    repo: &crate::git::Repository,
    root: &Path,
    config: &Config,
    pkg: &PackageConfig,
    channel: Option<&str>,
) -> Result<Explanation> {
    let is_monorepo = config.is_monorepo();
    let current_branch = crate::git::resolve_current_branch(repo, &config.workspace.branch);
    let prerelease_ctx = PrereleaseContext::resolve(
        channel,
        &current_branch,
        config.workspace.branches.as_deref(),
    )?;

    let all_tags = collect_all_tags(repo);
    let tag_index = crate::git::TagIndex::build(repo).ok();
    let fallback_ancestors = match &tag_index {
        Some(_) => None,
        None => crate::git::build_head_ancestors(repo).ok(),
    };
    let head_ancestors = tag_index
        .as_ref()
        .map(|idx| &idx.ancestors)
        .or(fallback_ancestors.as_ref());
    let short_hash = repo
        .head_id()
        .ok()
        .map(|id| id.to_string()[..7].to_string())
        .unwrap_or_default();
    let changed_files = get_changed_files(repo)?;
    let changed_files_cache = ChangedFilesCache::default();
    let commit_walk =
        crate::git::CommitWalkCache::new(config.workspace.effective_commit_skip_markers());

    let inputs = PlanInputs {
        config,
        root,
        tag_index: tag_index.as_ref(),
        head_ancestors,
        all_tags: &all_tags,
        prerelease_ctx: &prerelease_ctx,
        forced: &None,
        changed_files: &changed_files,
        short_hash: &short_hash,
        changed_files_cache: &changed_files_cache,
        commit_walk: &commit_walk,
    };

    let touch = evaluate_touch(repo, pkg, &inputs)?;
    let plan = compute_plan(repo, pkg, &inputs)?;

    // The commit list is evidence, not a decision: when the package is skipped
    // for not being touched there is nothing to classify, and walking the
    // history anyway would only invite the reader to argue with the verdict.
    let commits = if touch.touched {
        commits_for_package(repo, pkg, &inputs)?
            .into_iter()
            .map(|c| CommitReport {
                bump: determine_bump(&c.message, &config.workspace.commit_formats).to_string(),
                subject: c.message.lines().next().unwrap_or_default().to_string(),
                hash: c.hash,
            })
            .collect()
    } else {
        Vec::new()
    };

    let tag_prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
    let strategy = pkg.effective_versioning(&config.workspace, || {
        tags_for_package(&all_tags, &tag_prefix)
    });

    let cascade = cascade_bumps(repo, root, config, &inputs)?;
    let dependencies = dependency_reports(pkg, &cascade);

    let current_version = current_version(pkg, root, &plan);
    let decision = decide(
        config,
        pkg,
        root,
        strategy,
        &plan,
        &cascade,
        &current_version,
    )?;

    Ok(Explanation {
        package: pkg.name.clone(),
        path: pkg.path.clone(),
        shared_paths: pkg.shared_paths.clone(),
        strategy: format!("{strategy:?}").to_lowercase(),
        current_version,
        monorepo: is_monorepo,
        channel: prerelease_ctx.channel.clone(),
        touch: TouchReport {
            touched: touch.touched,
            recovered: touch.recovered,
            files: touch
                .files
                .iter()
                .map(|f| FileMatch {
                    matched: matching_rule(pkg, f, is_monorepo),
                    path: f.clone(),
                })
                .collect(),
        },
        last_tag: tag_info::for_package(repo, &tag_prefix, config, head_ancestors),
        commits,
        dependencies,
        decision,
    })
}

/// The `path` / `sharedPaths` prefix that makes `file` belong to `pkg`, mirroring
/// [`PackageConfig::is_touched_by`] one file at a time so the report can name the
/// rule that fired.
fn matching_rule(pkg: &PackageConfig, file: &str, is_monorepo: bool) -> Option<String> {
    if !is_monorepo {
        return Some("single-package repo".to_string());
    }
    let pkg_path = pkg.path.trim_start_matches("./").trim_end_matches('/');
    if pkg_path == "." || pkg_path.is_empty() {
        return Some("repo root".to_string());
    }
    let prefix = format!("{pkg_path}/");
    if file.starts_with(&prefix) {
        return Some(prefix);
    }
    pkg.shared_paths.iter().find_map(|shared| {
        let trimmed = shared.trim_end_matches('/');
        (file.starts_with(trimmed) || file == trimmed).then(|| shared.clone())
    })
}

/// Every package the release would bump, and with what. Seeded from each
/// package's own plan, then propagated through `dependsOn` to a fixed point the
/// same way the release cascade does.
fn cascade_bumps(
    repo: &crate::git::Repository,
    root: &Path,
    config: &Config,
    inputs: &PlanInputs<'_>,
) -> Result<std::collections::HashMap<String, BumpType>> {
    let mut bumped = std::collections::HashMap::new();
    for other in &config.packages {
        if let PackagePlan::Bump(plan) = compute_plan(repo, other, inputs)? {
            bumped.insert(other.name.clone(), plan.bump);
        }
    }

    for _ in 0..config.packages.len() {
        let mut added = false;
        for other in &config.packages {
            if bumped.contains_key(&other.name) {
                continue;
            }
            let bump = other
                .depends_on
                .iter()
                .filter_map(|dep| Some(dep.propagate().resolve(*bumped.get(dep.name())?)))
                .max()
                .unwrap_or(BumpType::None);
            if bump == BumpType::None {
                continue;
            }
            // The release skips a cascaded package whose version would not move;
            // mirror that so the explanation cannot promise a bump that never lands.
            let unchanged = other
                .versioned_files
                .first()
                .and_then(|vf| read_version(vf, root).ok())
                .is_some_and(|current| {
                    let strategy = other.effective_versioning(&config.workspace, || {
                        tags_for_package(
                            inputs.all_tags,
                            &other.tag_prefix(&config.workspace, true),
                        )
                    });
                    compute_next_version(&current, bump, strategy).is_ok_and(|next| next == current)
                });
            if unchanged {
                continue;
            }
            bumped.insert(other.name.clone(), bump);
            added = true;
        }
        if !added {
            break;
        }
    }

    Ok(bumped)
}

fn dependency_reports(
    pkg: &PackageConfig,
    cascade: &std::collections::HashMap<String, BumpType>,
) -> Vec<DependencyReport> {
    pkg.depends_on
        .iter()
        .map(|dep| {
            let upstream = cascade.get(dep.name()).copied();
            DependencyReport {
                name: dep.name().to_string(),
                propagate: serde_json::to_value(dep.propagate())
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default(),
                resulting_bump: upstream
                    .map(|u| dep.propagate().resolve(u))
                    .unwrap_or(BumpType::None)
                    .to_string(),
                upstream_bump: upstream.map(|u| u.to_string()),
            }
        })
        .collect()
}

fn current_version(pkg: &PackageConfig, root: &Path, plan: &PackagePlan) -> String {
    match plan {
        PackagePlan::Bump(bump) => bump.current_version.clone(),
        PackagePlan::Skipped { .. } => pkg
            .versioned_files
            .first()
            .and_then(|vf| read_version(vf, root).ok())
            .unwrap_or_else(|| "unknown".to_string()),
    }
}

fn decide(
    config: &Config,
    pkg: &PackageConfig,
    root: &Path,
    strategy: crate::config::VersioningStrategy,
    plan: &PackagePlan,
    cascade: &std::collections::HashMap<String, BumpType>,
    current_version: &str,
) -> Result<Decision> {
    if let PackagePlan::Bump(bump) = plan {
        return Ok(Decision::Bump {
            bump: bump.strategy_label.clone(),
            from: bump.current_version.clone(),
            to: bump.new_version.clone(),
            tag: bump.tag.clone(),
            prerelease: bump.is_prerelease,
            triggered_by: if bump.strategy_label == "forced" {
                Trigger::Forced
            } else {
                Trigger::Commits
            },
        });
    }

    let PackagePlan::Skipped { reason, .. } = plan else {
        unreachable!("plan is either a bump or a skip");
    };

    // A package can be skipped on its own commits and still be released because
    // something it depends on moved — the report has to say so, or it flatly
    // contradicts what the next `ferrflow release` does.
    if let Some(bump) = cascade.get(&pkg.name).copied()
        && bump != BumpType::None
        && let Some(next) = read_version_for_cascade(pkg, root)
            .and_then(|current| compute_next_version(&current, bump, strategy).ok())
        && next != current_version
    {
        return Ok(Decision::Bump {
            bump: bump.to_string(),
            from: current_version.to_string(),
            to: next.clone(),
            tag: pkg.tag_for_version(&config.workspace, config.is_monorepo(), &next),
            prerelease: false,
            triggered_by: Trigger::Dependency,
        });
    }

    Ok(Decision::Skipped {
        reason: reason.json_label().to_string(),
    })
}

fn read_version_for_cascade(pkg: &PackageConfig, root: &Path) -> Option<String> {
    pkg.versioned_files
        .first()
        .and_then(|vf| read_version(vf, root).ok())
}

#[cfg(test)]
mod tests;
