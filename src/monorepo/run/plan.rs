use anyhow::Result;

use crate::changelog::GitLog;
use crate::config::{Config, OrphanedTagStrategy, PackageConfig, VersioningStrategy};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::read_version;
use crate::git::{
    Repository, TagIndex, find_highest_semver_tag_with_cache, get_changed_files_since_oid,
    get_changed_files_since_tag, get_commits_since_last_stable_tag, get_commits_since_last_tag,
};
use crate::prerelease::PrereleaseContext;
use crate::versioning::compute_next_version;
use gix::ObjectId;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::super::util::{is_package_touched, pick_higher_semver, tags_for_package};
use super::forced::{Forced, forced_version_for};

pub(super) enum SkipReason {
    NotTouched,
    NoNewCommits,
    NoReleasableCommits,
    VersionUnchanged,
}

impl SkipReason {
    pub(super) fn json_label(&self) -> &'static str {
        match self {
            SkipReason::NotTouched => "not touched",
            SkipReason::NoNewCommits => "no new commits",
            SkipReason::NoReleasableCommits => "no releasable commits",
            SkipReason::VersionUnchanged => "version unchanged",
        }
    }
}

pub(super) struct PackageBump {
    pub recovered: bool,
    pub current_version: String,
    pub new_version: String,
    pub is_prerelease: bool,
    pub last_tag: Option<String>,
    pub commits: Vec<GitLog>,
    pub bump: BumpType,
    pub strategy_label: String,
    pub tag: String,
}

pub(super) enum PackagePlan {
    Skipped { reason: SkipReason, recovered: bool },
    Bump(Box<PackageBump>),
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum PlanSummary {
    Skipped {
        reason: &'static str,
        recovered: bool,
    },
    Bump {
        current_version: String,
        new_version: String,
        tag: String,
        bump: BumpType,
        commit_count: usize,
        is_prerelease: bool,
        strategy_label: String,
    },
}

#[cfg(test)]
impl PackagePlan {
    pub(super) fn summary(&self) -> PlanSummary {
        match self {
            PackagePlan::Skipped { reason, recovered } => PlanSummary::Skipped {
                reason: reason.json_label(),
                recovered: *recovered,
            },
            PackagePlan::Bump(b) => PlanSummary::Bump {
                current_version: b.current_version.clone(),
                new_version: b.new_version.clone(),
                tag: b.tag.clone(),
                bump: b.bump,
                commit_count: b.commits.len(),
                is_prerelease: b.is_prerelease,
                strategy_label: b.strategy_label.clone(),
            },
        }
    }
}

pub(super) type ChangedFilesCache = Mutex<HashMap<Option<ObjectId>, Arc<Vec<String>>>>;

pub(super) struct PlanInputs<'a> {
    pub config: &'a Config,
    pub root: &'a Path,
    pub tag_index: Option<&'a TagIndex>,
    pub head_ancestors: Option<&'a HashSet<ObjectId>>,
    pub all_tags: &'a [String],
    pub prerelease_ctx: &'a PrereleaseContext,
    pub forced: &'a Option<Forced<'a>>,
    pub changed_files: &'a [String],
    pub short_hash: &'a str,
    pub changed_files_cache: &'a ChangedFilesCache,
    pub commit_walk: &'a crate::git::CommitWalkCache,
}

fn changed_files_since_oid_cached(
    repo: &Repository,
    last_tag_oid: Option<ObjectId>,
    cache: &ChangedFilesCache,
) -> Result<Arc<Vec<String>>> {
    if let Some(hit) = cache
        .lock()
        .expect("changed-files cache poisoned")
        .get(&last_tag_oid)
    {
        return Ok(Arc::clone(hit));
    }
    let files = Arc::new(get_changed_files_since_oid(repo, last_tag_oid)?);
    cache
        .lock()
        .expect("changed-files cache poisoned")
        .insert(last_tag_oid, Arc::clone(&files));
    Ok(files)
}

/// Which file set decided whether a package counts as touched, and what that
/// decision was. `ferrflow why` reports it, so it lives here rather than being
/// re-derived — a second implementation would eventually disagree with the one
/// the release actually uses.
pub(super) struct TouchOutcome {
    pub touched: bool,
    pub recovered: bool,
    pub files: Arc<Vec<String>>,
}

pub(super) fn evaluate_touch(
    repo: &Repository,
    pkg: &PackageConfig,
    inputs: &PlanInputs<'_>,
) -> Result<TouchOutcome> {
    let config = inputs.config;
    let is_monorepo = config.is_monorepo();

    if is_package_touched(pkg, inputs.changed_files, is_monorepo) {
        return Ok(TouchOutcome {
            touched: true,
            recovered: false,
            files: Arc::new(inputs.changed_files.to_vec()),
        });
    }

    if !config.workspace.recover_missed_releases || !is_monorepo {
        return Ok(TouchOutcome {
            touched: false,
            recovered: false,
            files: Arc::new(inputs.changed_files.to_vec()),
        });
    }

    let tag_search_prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
    let strategy = config.workspace.orphaned_tag_strategy;
    let files_since_tag =
        if let (Some(idx), OrphanedTagStrategy::Warn) = (inputs.tag_index, strategy) {
            let last_oid = idx.find_last_tag_commit(&tag_search_prefix, strategy);
            changed_files_since_oid_cached(repo, last_oid, inputs.changed_files_cache)?
        } else {
            Arc::new(get_changed_files_since_tag(
                repo,
                &tag_search_prefix,
                strategy,
                inputs.head_ancestors,
            )?)
        };

    let touched = is_package_touched(pkg, &files_since_tag, true);
    Ok(TouchOutcome {
        touched,
        recovered: touched,
        files: files_since_tag,
    })
}

/// The commits a release would classify for `pkg`: everything back to its last
/// tag, or to its last *stable* tag when the run is not a prerelease.
pub(super) fn commits_for_package(
    repo: &Repository,
    pkg: &PackageConfig,
    inputs: &PlanInputs<'_>,
) -> Result<Vec<GitLog>> {
    let config = inputs.config;
    let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
    let strategy = config.workspace.orphaned_tag_strategy;
    let skip_markers = config.workspace.effective_commit_skip_markers();

    if inputs.prerelease_ctx.is_prerelease() {
        if let (Some(idx), OrphanedTagStrategy::Warn) = (inputs.tag_index, strategy) {
            let stop = idx.find_last_tag_commit(&tag_search_prefix, strategy);
            inputs.commit_walk.commits_since(repo, stop)
        } else {
            get_commits_since_last_tag(
                repo,
                &tag_search_prefix,
                strategy,
                &skip_markers,
                inputs.head_ancestors,
            )
        }
    } else if let (Some(idx), OrphanedTagStrategy::Warn) = (inputs.tag_index, strategy) {
        let stop = idx.find_last_stable_tag_commit(&tag_search_prefix, strategy);
        inputs.commit_walk.commits_since(repo, stop)
    } else {
        get_commits_since_last_stable_tag(
            repo,
            &tag_search_prefix,
            strategy,
            &skip_markers,
            inputs.head_ancestors,
        )
    }
}

pub(super) fn compute_plan(
    repo: &Repository,
    pkg: &PackageConfig,
    inputs: &PlanInputs<'_>,
) -> Result<PackagePlan> {
    let config = inputs.config;
    let is_monorepo = config.is_monorepo();
    let tag_search_prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
    let forced_ver_for_pkg = forced_version_for(inputs.forced, &pkg.name);

    let touch = evaluate_touch(repo, pkg, inputs)?;
    let recovered = touch.recovered;

    if !touch.touched && forced_ver_for_pkg.is_none() {
        return Ok(PackagePlan::Skipped {
            reason: SkipReason::NotTouched,
            recovered,
        });
    }

    let pkg_strategy = pkg.effective_versioning(&config.workspace, || {
        tags_for_package(inputs.all_tags, &tag_search_prefix)
    });

    let file_version = pkg
        .versioned_files
        .first()
        .and_then(|vf| read_version(vf, inputs.root).ok());
    let strategy = config.workspace.orphaned_tag_strategy;
    let highest_tag = if let (Some(idx), OrphanedTagStrategy::Warn) = (inputs.tag_index, strategy) {
        idx.find_highest_semver_tag(&tag_search_prefix, strategy)
    } else {
        find_highest_semver_tag_with_cache(
            repo,
            &tag_search_prefix,
            strategy,
            inputs.head_ancestors,
        )?
    };
    let last_tag = highest_tag.as_ref().map(|(tag, _version)| tag.clone());
    let tag_version = highest_tag.map(|(_tag, version)| version);
    let current_version = match (tag_version, file_version) {
        (Some(tag), Some(file)) => pick_higher_semver(&file, &tag),
        (Some(tag), None) => tag,
        (None, Some(file)) => file,
        (None, None) => crate::versioning::bootstrap_version(pkg_strategy),
    };

    let prerelease = inputs.prerelease_ctx.is_prerelease();

    let (new_version, is_prerelease, commits, bump) = if let Some(fv) = forced_ver_for_pkg {
        let clean = fv.strip_prefix('v').unwrap_or(fv);
        let commits = commits_for_package(repo, pkg, inputs).unwrap_or_default();
        (clean.to_string(), false, commits, BumpType::None)
    } else {
        let commits = commits_for_package(repo, pkg, inputs)?;

        if commits.is_empty() {
            return Ok(PackagePlan::Skipped {
                reason: SkipReason::NoNewCommits,
                recovered,
            });
        }

        let bump = commits
            .iter()
            .map(|c| determine_bump(&c.message, &config.workspace.commit_formats))
            .max()
            .unwrap_or(BumpType::None);

        if bump == BumpType::None && !is_date_or_seq(pkg_strategy) {
            return Ok(PackagePlan::Skipped {
                reason: SkipReason::NoReleasableCommits,
                recovered,
            });
        }

        let base_version = compute_next_version(&current_version, bump, pkg_strategy)?;

        let (new_version, is_prerelease) = if prerelease {
            let tag_prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
            if let Some(resolved) = inputs.prerelease_ctx.compute_identifier(
                &base_version,
                &tag_prefix,
                inputs.all_tags,
                inputs.short_hash,
            ) {
                (format!("{base_version}{}", resolved.full_suffix), true)
            } else {
                (base_version, false)
            }
        } else {
            (base_version, false)
        };

        (new_version, is_prerelease, commits, bump)
    };

    if current_version == new_version {
        return Ok(PackagePlan::Skipped {
            reason: SkipReason::VersionUnchanged,
            recovered,
        });
    }

    let strategy_label = if forced_ver_for_pkg.is_some() {
        "forced".to_string()
    } else {
        if is_date_or_seq(pkg_strategy) {
            format!("{pkg_strategy:?}").to_lowercase()
        } else {
            bump.to_string()
        }
    };

    let tag = pkg.tag_for_version(&config.workspace, is_monorepo, &new_version);

    Ok(PackagePlan::Bump(Box::new(PackageBump {
        recovered,
        current_version,
        new_version,
        is_prerelease,
        last_tag,
        commits,
        bump,
        strategy_label,
        tag,
    })))
}

fn is_date_or_seq(strategy: VersioningStrategy) -> bool {
    matches!(
        strategy,
        VersioningStrategy::Calver
            | VersioningStrategy::CalverShort
            | VersioningStrategy::CalverSeq
            | VersioningStrategy::Sequential
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::git::{TagIndex, build_head_ancestors, collect_all_tags, get_changed_files};
    use crate::test_utils::{commit_file, git, init_repo};
    use rayon::prelude::*;
    use std::path::Path;

    fn write_pkg(dir: &Path, name: &str, version: &str) {
        std::fs::create_dir_all(dir.join(name)).unwrap();
        std::fs::write(
            dir.join(name).join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .unwrap();
    }

    fn write_config(dir: &Path, names: &[&str]) {
        let packages: Vec<String> = names
            .iter()
            .map(|n| {
                format!(
                    r#"{{"name":"{n}","path":"{n}","versionedFiles":[{{"path":"{n}/Cargo.toml","format":"toml"}}]}}"#
                )
            })
            .collect();
        std::fs::write(
            dir.join(".ferrflow"),
            format!(r#"{{"package":[{}]}}"#, packages.join(",")),
        )
        .unwrap();
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        repo: crate::git::Repository,
        config: Config,
        root: std::path::PathBuf,
        cache: ChangedFilesCache,
        commit_walk: crate::git::CommitWalkCache,
    }

    fn build_inputs<'a>(
        fx: &'a Fixture,
        tag_index: &'a Option<TagIndex>,
        head_ancestors: &'a Option<std::collections::HashSet<ObjectId>>,
        all_tags: &'a [String],
        prerelease_ctx: &'a PrereleaseContext,
        forced: &'a Option<Forced<'a>>,
        changed_files: &'a [String],
    ) -> PlanInputs<'a> {
        PlanInputs {
            config: &fx.config,
            root: &fx.root,
            tag_index: tag_index.as_ref(),
            head_ancestors: head_ancestors.as_ref(),
            all_tags,
            prerelease_ctx,
            forced,
            changed_files,
            short_hash: "deadbee",
            commit_walk: &fx.commit_walk,
            changed_files_cache: &fx.cache,
        }
    }

    fn plan_all(fx: &Fixture, inputs: &PlanInputs<'_>, parallel: bool) -> Vec<PlanSummary> {
        if parallel {
            let thread_safe = fx.repo.clone().into_sync();
            fx.config
                .packages
                .par_iter()
                .map(|pkg| {
                    let repo = thread_safe.to_thread_local();
                    compute_plan(&repo, pkg, inputs).map(|p| p.summary())
                })
                .collect::<Result<Vec<_>>>()
                .unwrap()
        } else {
            fx.config
                .packages
                .iter()
                .map(|pkg| compute_plan(&fx.repo, pkg, inputs).map(|p| p.summary()))
                .collect::<Result<Vec<_>>>()
                .unwrap()
        }
    }

    #[test]
    fn parallel_planning_matches_sequential_baseline() {
        let (dir, repo) = init_repo();
        let root = dir.path().to_path_buf();
        let names = ["alpha", "beta", "gamma", "delta"];
        for n in names {
            write_pkg(&root, n, "1.0.0");
        }
        write_config(&root, &names);
        git(&root, &["add", "-A"]);
        commit_file(&root, "seed.txt", "x", "chore: seed", 1_950_000_000);

        // One tag per package so each package's commit walk starts from a
        // distinct baseline — this makes the bump decisions order-sensitive
        // enough that a racy parallel scan would diverge from sequential.
        for n in names {
            git(&root, &["tag", &format!("{n}-v1.0.0")]);
        }
        commit_file(
            &root,
            "alpha/feat.rs",
            "x",
            "feat: alpha feature",
            1_950_000_100,
        );
        commit_file(&root, "beta/fix.rs", "x", "fix: beta fix", 1_950_000_200);
        commit_file(
            &root,
            "gamma/docs.rs",
            "x",
            "docs: gamma note",
            1_950_000_300,
        );

        let config = Config::load(&root, Some(&root.join(".ferrflow"))).unwrap();
        let commit_walk =
            crate::git::CommitWalkCache::new(config.workspace.effective_commit_skip_markers());
        let fx = Fixture {
            _dir: dir,
            repo,
            config,
            root: root.clone(),
            cache: ChangedFilesCache::default(),
            commit_walk,
        };

        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Option<Forced<'_>> = None;
        let changed_files = get_changed_files(&fx.repo).unwrap();
        let inputs = build_inputs(
            &fx,
            &tag_index,
            &head_ancestors,
            &all_tags,
            &prerelease_ctx,
            &forced,
            &changed_files,
        );

        let sequential = plan_all(&fx, &inputs, false);
        let parallel = plan_all(&fx, &inputs, true);
        let parallel_again = plan_all(&fx, &inputs, true);

        assert_eq!(parallel, sequential);
        assert_eq!(parallel, parallel_again);

        assert!(
            parallel
                .iter()
                .any(|p| matches!(p, PlanSummary::Bump { .. })),
            "fixture should produce at least one bump"
        );
    }

    #[test]
    fn one_package_compute_error_aborts_collection() {
        let (dir, repo) = init_repo();
        let root = dir.path().to_path_buf();
        let names = ["good", "broken"];
        write_pkg(&root, "good", "1.0.0");
        write_pkg(&root, "broken", "not-a-semver");
        write_config(&root, &names);
        git(&root, &["add", "-A"]);
        commit_file(&root, "seed.txt", "x", "chore: seed", 1_950_000_000);
        commit_file(&root, "good/feat.rs", "x", "feat: good", 1_950_000_100);
        commit_file(&root, "broken/feat.rs", "x", "feat: broken", 1_950_000_200);

        let config = Config::load(&root, Some(&root.join(".ferrflow"))).unwrap();
        let commit_walk =
            crate::git::CommitWalkCache::new(config.workspace.effective_commit_skip_markers());
        let fx = Fixture {
            _dir: dir,
            repo,
            config,
            root: root.clone(),
            cache: ChangedFilesCache::default(),
            commit_walk,
        };

        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Option<Forced<'_>> = None;
        let changed_files = get_changed_files(&fx.repo).unwrap();
        let inputs = build_inputs(
            &fx,
            &tag_index,
            &head_ancestors,
            &all_tags,
            &prerelease_ctx,
            &forced,
            &changed_files,
        );

        let thread_safe = fx.repo.clone().into_sync();
        let result: Result<Vec<_>> = fx
            .config
            .packages
            .par_iter()
            .map(|pkg| {
                let repo = thread_safe.to_thread_local();
                compute_plan(&repo, pkg, &inputs)
            })
            .collect();

        assert!(
            result.is_err(),
            "a broken package must abort the collection"
        );
    }
}
