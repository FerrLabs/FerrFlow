use anyhow::{Result, anyhow};

use crate::changelog::GitLog;
use crate::config::{Config, OrphanedTagStrategy, PackageConfig, VersioningStrategy};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::error_code::{self, ErrorCodeExt};
use crate::formats::read_version;
use crate::git::{
    Repository, TagIndex, find_highest_semver_tag_with_cache, get_changed_files_for_commit,
    get_changed_files_since_oid, get_changed_files_since_tag, get_commits_since_last_stable_tag,
    get_commits_since_last_tag,
};
use crate::prerelease::PrereleaseContext;
use crate::versioning::compute_next_version;
use gix::ObjectId;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::super::util::{is_package_touched, tags_for_package};
use super::super::version_source::VersionSource;
use super::forced::{Forced, forced_version_for};

pub(super) enum SkipReason {
    NotTouched,
    NoNewCommits,
    NoReleasableCommits,
    VersionUnchanged { version: String },
    Excluded,
}

impl SkipReason {
    pub(super) fn json_label(&self) -> &'static str {
        match self {
            SkipReason::NotTouched => "not touched",
            SkipReason::NoNewCommits => "no new commits",
            SkipReason::NoReleasableCommits => "no releasable commits",
            SkipReason::VersionUnchanged { .. } => "version unchanged",
            SkipReason::Excluded => "excluded",
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
    pub version_source: Option<VersionSource>,
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
pub(super) type CommitFilesCache = Mutex<HashMap<ObjectId, Arc<Vec<String>>>>;

pub(super) struct PlanInputs<'a> {
    pub config: &'a Config,
    pub root: &'a Path,
    pub tag_index: Option<&'a TagIndex>,
    pub head_ancestors: Option<&'a HashSet<ObjectId>>,
    pub all_tags: &'a [String],
    pub prerelease_ctx: &'a PrereleaseContext,
    pub forced: &'a [Forced<'a>],
    pub excluded: &'a [String],
    pub changed_files: &'a [String],
    pub short_hash: &'a str,
    pub changed_files_cache: &'a ChangedFilesCache,
    pub commit_files_cache: &'a CommitFilesCache,
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

fn files_for_commit_cached(
    repo: &Repository,
    commit: ObjectId,
    cache: &CommitFilesCache,
) -> Arc<Vec<String>> {
    if let Some(hit) = cache
        .lock()
        .expect("commit-files cache poisoned")
        .get(&commit)
    {
        return Arc::clone(hit);
    }
    let files = Arc::new(get_changed_files_for_commit(repo, commit).unwrap_or_default());
    cache
        .lock()
        .expect("commit-files cache poisoned")
        .insert(commit, Arc::clone(&files));
    files
}

const PARALLEL_PREFETCH_THRESHOLD: usize = 256;

fn prefetch_commit_files(repo: &Repository, commits: &[GitLog], cache: &CommitFilesCache) {
    if rayon::current_num_threads() < 2 {
        return;
    }
    let missing: Vec<ObjectId> = {
        let known = cache.lock().expect("commit-files cache poisoned");
        commits
            .iter()
            .filter_map(|c| ObjectId::from_hex(c.id.as_bytes()).ok())
            .filter(|id| !known.contains_key(id))
            .collect()
    };
    if missing.len() < PARALLEL_PREFETCH_THRESHOLD {
        return;
    }

    let shared = repo.clone().into_sync();
    let fetched: Vec<(ObjectId, Arc<Vec<String>>)> = missing
        .par_iter()
        .map_init(
            || shared.to_thread_local(),
            |worker, id| {
                let files = get_changed_files_for_commit(worker, *id).unwrap_or_default();
                (*id, Arc::new(files))
            },
        )
        .collect();

    cache
        .lock()
        .expect("commit-files cache poisoned")
        .extend(fetched);
}

fn scope_commits_to_package(
    repo: &Repository,
    pkg: &PackageConfig,
    inputs: &PlanInputs<'_>,
    commits: Vec<GitLog>,
) -> Vec<GitLog> {
    if !inputs.config.is_monorepo() {
        return commits;
    }
    prefetch_commit_files(repo, &commits, inputs.commit_files_cache);
    commits
        .into_iter()
        .filter(|c| {
            let Ok(id) = ObjectId::from_hex(c.id.as_bytes()) else {
                return true;
            };
            let files = files_for_commit_cached(repo, id, inputs.commit_files_cache);
            files.is_empty() || pkg.is_touched_by(&files, true)
        })
        .collect()
}

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

pub(super) fn commits_for_package(
    repo: &Repository,
    pkg: &PackageConfig,
    inputs: &PlanInputs<'_>,
) -> Result<Vec<GitLog>> {
    let config = inputs.config;
    let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
    let strategy = config.workspace.orphaned_tag_strategy;
    let skip_markers = config.workspace.effective_commit_skip_markers();

    let commits = if inputs.prerelease_ctx.is_prerelease() {
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
    }?;

    Ok(scope_commits_to_package(repo, pkg, inputs, commits))
}

fn ensure_versioned_files_exist(pkg: &PackageConfig, root: &Path) -> Result<()> {
    let missing = pkg.versioned_files.iter().find(|vf| {
        crate::formats::get_handler(&vf.format).modifies_file() && !root.join(&vf.path).exists()
    });
    let Some(vf) = missing else {
        return Ok(());
    };
    let hint = suggested_versioned_path(pkg, &vf.path)
        .map(|suggestion| {
            format!(
                "\n  Paths in versionedFiles are relative to the repository root, not to the \
                 package's own path. Did you mean \"{suggestion}\"?"
            )
        })
        .unwrap_or_default();
    Err(anyhow!(
        "package \"{name}\": versioned file \"{path}\" does not exist, so the release \
         would fail when it tries to write it.{hint}",
        name = pkg.name,
        path = vf.path,
    ))
    .error_code(error_code::CONFIG_MISSING_VERSIONED_FILE)
}

fn suggested_versioned_path(pkg: &PackageConfig, path: &str) -> Option<String> {
    let prefix = pkg.path.trim_end_matches('/');
    if prefix.is_empty() || prefix == "." || Path::new(path).starts_with(prefix) {
        return None;
    }
    Some(format!("{prefix}/{path}"))
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

    if inputs.excluded.iter().any(|name| name == &pkg.name) {
        return Ok(PackagePlan::Skipped {
            reason: SkipReason::Excluded,
            recovered: false,
        });
    }

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

    let file_source = pkg.versioned_files.first().and_then(|vf| {
        read_version(vf, inputs.root)
            .ok()
            .map(|version| (vf.path.clone(), version))
    });
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
    let (resolved, source) = VersionSource::resolve(
        highest_tag,
        file_source,
        pkg.effective_version_source(&config.workspace),
    );
    let current_version =
        resolved.unwrap_or_else(|| crate::versioning::bootstrap_version(pkg_strategy));
    let version_source = Some(source);

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

        let version_template = pkg.effective_version_template(&config.workspace);

        if bump == BumpType::None && version_template.is_none() && !is_date_or_seq(pkg_strategy) {
            return Ok(PackagePlan::Skipped {
                reason: SkipReason::NoReleasableCommits,
                recovered,
            });
        }

        let base_version =
            compute_next_version(&current_version, bump, pkg_strategy, version_template)?;

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
            reason: SkipReason::VersionUnchanged {
                version: new_version,
            },
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

    ensure_versioned_files_exist(pkg, inputs.root)?;

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
        version_source,
    })))
}

fn is_date_or_seq(strategy: VersioningStrategy) -> bool {
    matches!(
        strategy,
        VersioningStrategy::Calver
            | VersioningStrategy::CalverShort
            | VersioningStrategy::CalverSeq
            | VersioningStrategy::CalverShortSeq
            | VersioningStrategy::Sequential
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::git::{TagIndex, build_head_ancestors, collect_all_tags, get_changed_files};
    use crate::test_utils::{commit_file, git, init_repo};
    use std::path::Path;

    #[test]
    fn every_date_or_sequence_strategy_reports_itself_rather_than_a_bump_type() {
        for strategy in [
            VersioningStrategy::Calver,
            VersioningStrategy::CalverShort,
            VersioningStrategy::CalverSeq,
            VersioningStrategy::CalverShortSeq,
            VersioningStrategy::Sequential,
        ] {
            assert!(
                is_date_or_seq(strategy),
                "{strategy:?} computes its version from the date or a counter, so the                  output must name the strategy, not a bump type it never used"
            );
        }
        assert!(!is_date_or_seq(VersioningStrategy::Semver));
        assert!(!is_date_or_seq(VersioningStrategy::Zerover));
    }

    #[test]
    fn version_unchanged_carries_the_version_it_recomputed() {
        let reason = SkipReason::VersionUnchanged {
            version: "26.8.28".to_string(),
        };

        assert_eq!(reason.json_label(), "version unchanged");
        match reason {
            SkipReason::VersionUnchanged { version } => assert_eq!(version, "26.8.28"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn version_unchanged_is_distinct_from_having_nothing_to_release() {
        assert_ne!(
            SkipReason::VersionUnchanged {
                version: "26.8.28".to_string()
            }
            .json_label(),
            SkipReason::NoReleasableCommits.json_label(),
            "the two must stay tellable apart: one is idle, the other is work held back"
        );
    }

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
        commit_files: CommitFilesCache,
        commit_walk: crate::git::CommitWalkCache,
    }

    fn build_inputs<'a>(
        fx: &'a Fixture,
        tag_index: &'a Option<TagIndex>,
        head_ancestors: &'a Option<std::collections::HashSet<ObjectId>>,
        all_tags: &'a [String],
        prerelease_ctx: &'a PrereleaseContext,
        forced: &'a [Forced<'a>],
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
            excluded: &[],
            changed_files,
            short_hash: "deadbee",
            commit_walk: &fx.commit_walk,
            changed_files_cache: &fx.cache,
            commit_files_cache: &fx.commit_files,
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
            "gamma/feat.rs",
            "x",
            "feat: gamma feature",
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
            commit_files: CommitFilesCache::default(),
            commit_walk,
        };

        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Vec<Forced<'_>> = Vec::new();
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

    fn plan_for(fx: &Fixture, inputs: &PlanInputs<'_>, name: &str) -> PackagePlan {
        let pkg = fx
            .config
            .packages
            .iter()
            .find(|p| p.name == name)
            .expect("package in fixture");
        compute_plan(&fx.repo, pkg, inputs).unwrap()
    }

    fn subjects(plan: &PackagePlan) -> Vec<String> {
        match plan {
            PackagePlan::Bump(bump) => bump
                .commits
                .iter()
                .map(|c| c.message.lines().next().unwrap_or_default().to_string())
                .collect(),
            PackagePlan::Skipped { .. } => Vec::new(),
        }
    }

    fn two_package_fixture(extra: impl FnOnce(&Path)) -> (Fixture, Vec<String>) {
        let (dir, repo) = init_repo();
        let root = dir.path().to_path_buf();
        let names = ["site", "server"];
        for n in names {
            write_pkg(&root, n, "1.0.0");
        }
        write_config(&root, &names);
        git(&root, &["add", "-A"]);
        commit_file(&root, "seed.txt", "x", "chore: seed", 1_950_000_000);
        for n in names {
            git(&root, &["tag", &format!("{n}-v1.0.0")]);
        }

        commit_file(
            &root,
            "server/oid.rs",
            "x",
            "refactor(server): make the object id a type",
            1_950_000_100,
        );
        extra(&root);

        let config = Config::load(&root, Some(&root.join(".ferrflow"))).unwrap();
        let commit_walk =
            crate::git::CommitWalkCache::new(config.workspace.effective_commit_skip_markers());
        let fx = Fixture {
            _dir: dir,
            repo,
            config,
            root,
            cache: ChangedFilesCache::default(),
            commit_files: CommitFilesCache::default(),
            commit_walk,
        };
        let changed_files = get_changed_files(&fx.repo).unwrap();
        (fx, changed_files)
    }

    #[test]
    fn a_release_does_not_list_another_packages_commits() {
        let (fx, changed_files) = two_package_fixture(|root| {
            commit_file(
                root,
                "site/hero.rs",
                "x",
                "feat(site): dress the site",
                1_950_000_200,
            );
        });

        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Vec<Forced<'_>> = Vec::new();
        let inputs = build_inputs(
            &fx,
            &tag_index,
            &head_ancestors,
            &all_tags,
            &prerelease_ctx,
            &forced,
            &changed_files,
        );

        let site = subjects(&plan_for(&fx, &inputs, "site"));

        assert!(
            site.iter().any(|s| s.contains("dress the site")),
            "site release must carry its own commit, got {site:?}"
        );
        assert!(
            !site.iter().any(|s| s.contains("object id")),
            "a server-only commit must not appear in the site changelog, got {site:?}"
        );
    }

    fn missing_file_fixture(versioned_path: &str, api_commit: &str) -> (Fixture, Vec<String>) {
        let (dir, repo) = init_repo();
        let root = dir.path().to_path_buf();
        write_pkg(&root, "api", "2.4.0");
        write_pkg(&root, "sdk", "1.0.0");
        write_config_raw(
            &root,
            "",
            &format!(
                r#"{{"name":"api","path":"api","versionedFiles":[{{"path":"{versioned_path}","format":"toml"}}]}},
                   {{"name":"sdk","path":"sdk","versionedFiles":[{{"path":"sdk/Missing.toml","format":"toml"}}]}}"#
            ),
        );
        git(&root, &["add", "-A"]);
        commit_file(&root, "seed.txt", "x", "chore: seed", 1_950_000_000);
        commit_file(&root, "api/endpoint.rs", "x", api_commit, 1_950_000_100);
        let fx = build_fixture(root, dir, repo);
        let changed_files = get_changed_files(&fx.repo).unwrap();
        (fx, changed_files)
    }

    fn plan_result(fx: &Fixture, changed_files: &[String], name: &str) -> Result<PackagePlan> {
        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Vec<Forced<'_>> = Vec::new();
        let inputs = build_inputs(
            fx,
            &tag_index,
            &head_ancestors,
            &all_tags,
            &prerelease_ctx,
            &forced,
            changed_files,
        );
        let pkg = fx
            .config
            .packages
            .iter()
            .find(|p| p.name == name)
            .expect("package in fixture");
        compute_plan(&fx.repo, pkg, &inputs)
    }

    fn package(name: &str, path: &str) -> PackageConfig {
        serde_json::from_str(&format!(r#"{{"name":"{name}","path":"{path}"}}"#)).unwrap()
    }

    #[test]
    fn a_versioned_file_that_does_not_exist_fails_the_plan_rather_than_bumping_nothing() {
        let (fx, changed) = missing_file_fixture("Cargo.toml", "feat(api): add an endpoint");

        let err = match plan_result(&fx, &changed, "api") {
            Ok(plan) => panic!(
                "a missing versioned file must fail, got {:?}",
                plan.summary()
            ),
            Err(err) => err,
        };
        let msg = format!("{err:?}");
        assert!(msg.contains("does not exist"), "{msg}");
        assert!(
            msg.contains("Did you mean \"api/Cargo.toml\""),
            "the error should point at the repo-root path it probably meant: {msg}"
        );
    }

    #[test]
    fn a_versioned_file_that_exists_still_plans_normally() {
        let (fx, changed) = missing_file_fixture("api/Cargo.toml", "feat(api): add an endpoint");

        let plan = plan_result(&fx, &changed, "api")
            .unwrap_or_else(|e| panic!("a correct config must still plan: {e:?}"));
        assert!(
            matches!(plan, PackagePlan::Bump(_)),
            "expected a bump, got {:?}",
            plan.summary()
        );
    }

    #[test]
    fn a_touched_package_with_nothing_to_release_is_skipped_not_failed() {
        let (fx, changed) = missing_file_fixture("Cargo.toml", "chore(api): bump lint config");

        let plan = plan_result(&fx, &changed, "api").unwrap_or_else(|e| {
            panic!("a package this run will not write must not fail the release: {e:?}")
        });
        assert!(
            matches!(
                plan,
                PackagePlan::Skipped {
                    reason: SkipReason::NoReleasableCommits,
                    ..
                }
            ),
            "expected api to be skipped for having nothing to release, got {:?}",
            plan.summary()
        );
    }

    #[test]
    fn a_format_that_never_writes_the_file_does_not_need_it_to_exist() {
        let (dir, repo) = init_repo();
        let root = dir.path().to_path_buf();
        write_pkg(&root, "mymod", "1.0.0");
        write_config_raw(
            &root,
            "",
            r#"{"name":"mymod","path":".","versionedFiles":[{"path":"go.mod","format":"gomod"}]}"#,
        );
        git(&root, &["add", "-A"]);
        commit_file(&root, "seed.txt", "x", "chore: seed", 1_950_000_000);
        git(&root, &["tag", "v1.0.0"]);
        commit_file(
            &root,
            "handler.go",
            "x",
            "fix: handle a nil pointer",
            1_950_000_100,
        );
        let fx = build_fixture(root, dir, repo);
        let changed = get_changed_files(&fx.repo).unwrap();

        assert!(
            !fx.root.join("go.mod").exists(),
            "the fixture must not create go.mod, or this proves nothing"
        );
        let plan = plan_result(&fx, &changed, "mymod").unwrap_or_else(|e| {
            panic!("a gomod package must plan without a go.mod on disk: {e:?}")
        });
        assert!(
            matches!(plan, PackagePlan::Bump(_)),
            "the plan must reach the far side of the file check, got {:?}",
            plan.summary()
        );
    }

    #[test]
    fn an_untouched_package_is_skipped_before_its_files_are_checked() {
        let (fx, changed) = missing_file_fixture("api/Cargo.toml", "feat(api): add an endpoint");
        assert!(
            !fx.root.join("sdk/Missing.toml").exists(),
            "sdk's versioned file must be absent, or this proves nothing"
        );

        let plan = plan_result(&fx, &changed, "sdk")
            .unwrap_or_else(|e| panic!("an untouched package must not fail: {e:?}"));
        assert!(
            matches!(
                plan,
                PackagePlan::Skipped {
                    reason: SkipReason::NotTouched,
                    ..
                }
            ),
            "expected sdk to be skipped, got {:?}",
            plan.summary()
        );
    }

    #[test]
    fn the_path_hint_is_only_given_when_it_names_a_different_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut root_pkg = package("root", ".");
        root_pkg.versioned_files =
            serde_json::from_str(r#"[{"path":"Cargo.toml","format":"toml"}]"#).unwrap();

        let msg = format!(
            "{:?}",
            ensure_versioned_files_exist(&root_pkg, dir.path()).unwrap_err()
        );
        assert!(msg.contains("does not exist"), "{msg}");
        assert!(
            !msg.contains("Did you mean"),
            "suggesting the path the user already wrote says nothing: {msg}"
        );
    }

    #[test]
    fn a_suggestion_prefixes_the_package_path_only_when_it_is_missing() {
        let api = package("api", "api");
        assert_eq!(
            suggested_versioned_path(&api, "Cargo.toml").as_deref(),
            Some("api/Cargo.toml")
        );
        assert_eq!(suggested_versioned_path(&api, "api/Cargo.toml"), None);
        assert_eq!(
            suggested_versioned_path(&api, "apiv2/Cargo.toml").as_deref(),
            Some("api/apiv2/Cargo.toml"),
            "a sibling directory sharing the prefix is not inside the package"
        );
        assert_eq!(
            suggested_versioned_path(&package("root", "."), "Cargo.toml"),
            None
        );
    }

    fn fast_import_history(dir: &Path, commits: usize) {
        use std::io::Write;
        let mut stream = String::new();
        for i in 0..commits {
            let pkg = if i % 3 == 0 { "api" } else { "sdk" };
            let body = format!("fix({pkg}): change {i}\n");
            stream.push_str("commit refs/heads/main\n");
            stream.push_str(&format!(
                "committer Test <test@test.com> {} +0000\n",
                1_950_000_000 + i
            ));
            stream.push_str(&format!("data {}\n{body}", body.len()));
            let content = format!("{i}\n");
            stream.push_str(&format!(
                "M 100644 inline {pkg}/file.txt\ndata {}\n{content}",
                content.len()
            ));
        }
        let mut child = std::process::Command::new("git")
            .current_dir(dir)
            .args(["fast-import", "--quiet"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("git should be on PATH");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stream.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success(), "git fast-import failed");
    }

    fn history(repo: &Repository) -> Vec<GitLog> {
        let head = repo.head_id().unwrap().detach();
        repo.rev_walk([head])
            .all()
            .unwrap()
            .map(|info| {
                let id = info.unwrap().id.to_string();
                GitLog {
                    hash: id[..7].to_string(),
                    id,
                    message: String::new(),
                }
            })
            .collect()
    }

    fn in_pool<T: Send>(threads: usize, f: impl FnOnce() -> T + Send) -> T {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(f)
    }

    #[test]
    fn a_parallel_prefetch_caches_exactly_what_the_sequential_path_computes() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        fast_import_history(dir.path(), PARALLEL_PREFETCH_THRESHOLD + 40);
        let repo = crate::git::open_repo(dir.path()).unwrap();
        let commits = history(&repo);
        assert!(commits.len() > PARALLEL_PREFETCH_THRESHOLD);

        let prefetched = CommitFilesCache::default();
        in_pool(4, || {
            let worker = crate::git::open_repo(dir.path()).unwrap();
            prefetch_commit_files(&worker, &commits, &prefetched);
        });
        let prefetched = prefetched.into_inner().unwrap();
        assert_eq!(
            prefetched.len(),
            commits.len(),
            "every commit in the range should have been diffed up front"
        );

        for commit in &commits {
            let id = ObjectId::from_hex(commit.id.as_bytes()).unwrap();
            let sequential = get_changed_files_for_commit(&repo, id).unwrap();
            assert_eq!(
                *prefetched[&id], sequential,
                "commit {} differs between the two paths",
                commit.hash
            );
        }
        let touched_api = prefetched
            .values()
            .filter(|files| files.iter().any(|f| f.starts_with("api/")))
            .count();
        assert_eq!(touched_api, commits.len().div_ceil(3));
    }

    #[test]
    fn a_single_threaded_pool_leaves_the_work_to_the_sequential_path() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        fast_import_history(dir.path(), PARALLEL_PREFETCH_THRESHOLD + 40);
        let repo = crate::git::open_repo(dir.path()).unwrap();
        let commits = history(&repo);

        let cache = CommitFilesCache::default();
        in_pool(1, || {
            let worker = crate::git::open_repo(dir.path()).unwrap();
            prefetch_commit_files(&worker, &commits, &cache);
        });
        assert!(
            cache.lock().unwrap().is_empty(),
            "--jobs 1 must not pay for a thread-safe clone it cannot use"
        );
    }

    #[test]
    fn a_short_range_is_not_worth_a_prefetch() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        fast_import_history(dir.path(), 20);
        let repo = crate::git::open_repo(dir.path()).unwrap();
        let commits = history(&repo);

        let cache = CommitFilesCache::default();
        in_pool(4, || {
            let worker = crate::git::open_repo(dir.path()).unwrap();
            prefetch_commit_files(&worker, &commits, &cache);
        });
        assert!(cache.lock().unwrap().is_empty());
    }

    fn write_config_raw(dir: &Path, workspace: &str, packages: &str) {
        std::fs::write(
            dir.join(".ferrflow"),
            format!(r#"{{"workspace":{{{workspace}}},"package":[{packages}]}}"#),
        )
        .unwrap();
    }

    fn build_fixture(
        root: std::path::PathBuf,
        dir: tempfile::TempDir,
        repo: crate::git::Repository,
    ) -> Fixture {
        let config = Config::load(&root, Some(&root.join(".ferrflow"))).unwrap();
        let commit_walk =
            crate::git::CommitWalkCache::new(config.workspace.effective_commit_skip_markers());
        Fixture {
            _dir: dir,
            repo,
            config,
            root,
            cache: ChangedFilesCache::default(),
            commit_files: CommitFilesCache::default(),
            commit_walk,
        }
    }

    #[test]
    fn a_release_bump_ignores_another_packages_commit_type() {
        let (fx, changed_files) = two_package_fixture(|root| {
            commit_file(
                root,
                "server/api.rs",
                "x",
                "feat(server): add an endpoint",
                1_950_000_150,
            );
            commit_file(
                root,
                "site/typo.rs",
                "x",
                "fix(site): correct a label",
                1_950_000_200,
            );
        });

        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Vec<Forced<'_>> = Vec::new();
        let inputs = build_inputs(
            &fx,
            &tag_index,
            &head_ancestors,
            &all_tags,
            &prerelease_ctx,
            &forced,
            &changed_files,
        );

        let PackagePlan::Bump(site) = plan_for(&fx, &inputs, "site") else {
            panic!("site should release");
        };

        assert_eq!(
            site.new_version, "1.0.1",
            "a feat in another package must not turn the site's fix into a minor"
        );
    }

    #[test]
    fn a_shared_path_commit_stays_in_the_package_changelog() {
        let (dir, repo) = init_repo();
        let root = dir.path().to_path_buf();
        write_pkg(&root, "site", "1.0.0");
        write_pkg(&root, "server", "1.0.0");
        write_config_raw(
            &root,
            r#""recoverMissedReleases":false"#,
            r#"{"name":"site","path":"site","sharedPaths":["docs"],"versionedFiles":[{"path":"site/Cargo.toml","format":"toml"}]},{"name":"server","path":"server","versionedFiles":[{"path":"server/Cargo.toml","format":"toml"}]}"#,
        );
        git(&root, &["add", "-A"]);
        commit_file(&root, "seed.txt", "x", "chore: seed", 1_950_000_000);
        git(&root, &["tag", "site-v1.0.0"]);
        git(&root, &["tag", "server-v1.0.0"]);
        std::fs::create_dir_all(root.join("docs")).unwrap();
        commit_file(
            &root,
            "docs/guide.md",
            "x",
            "feat(docs): document the flow",
            1_950_000_100,
        );

        let fx = build_fixture(root, dir, repo);
        let changed_files = get_changed_files(&fx.repo).unwrap();
        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Vec<Forced<'_>> = Vec::new();
        let inputs = build_inputs(
            &fx,
            &tag_index,
            &head_ancestors,
            &all_tags,
            &prerelease_ctx,
            &forced,
            &changed_files,
        );

        let site = subjects(&plan_for(&fx, &inputs, "site"));

        assert!(
            site.iter().any(|s| s.contains("document the flow")),
            "a sharedPaths commit belongs to the package, got {site:?}"
        );
        let server = subjects(&plan_for(&fx, &inputs, "server"));
        assert!(
            !server.iter().any(|s| s.contains("document the flow")),
            "a sharedPaths commit belongs only to the packages that declare it, got {server:?}"
        );
    }

    #[test]
    fn a_recovered_release_does_not_list_another_packages_commits() {
        let (dir, repo) = init_repo();
        let root = dir.path().to_path_buf();
        write_pkg(&root, "site", "1.0.0");
        write_pkg(&root, "server", "1.0.0");
        write_config_raw(
            &root,
            r#""recoverMissedReleases":true"#,
            r#"{"name":"site","path":"site","versionedFiles":[{"path":"site/Cargo.toml","format":"toml"}]},{"name":"server","path":"server","versionedFiles":[{"path":"server/Cargo.toml","format":"toml"}]}"#,
        );
        git(&root, &["add", "-A"]);
        commit_file(&root, "seed.txt", "x", "chore: seed", 1_950_000_000);
        git(&root, &["tag", "site-v1.0.0"]);
        git(&root, &["tag", "server-v1.0.0"]);

        commit_file(
            &root,
            "site/hero.rs",
            "x",
            "feat(site): dress the site",
            1_950_000_100,
        );
        commit_file(
            &root,
            "server/oid.rs",
            "x",
            "refactor(server): make the object id a type",
            1_950_000_200,
        );

        let fx = build_fixture(root, dir, repo);
        let changed_files = get_changed_files(&fx.repo).unwrap();
        assert!(
            !changed_files.iter().any(|f| f.starts_with("site/")),
            "the head commit must not touch site, or there is nothing to recover"
        );

        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Vec<Forced<'_>> = Vec::new();
        let inputs = build_inputs(
            &fx,
            &tag_index,
            &head_ancestors,
            &all_tags,
            &prerelease_ctx,
            &forced,
            &changed_files,
        );

        let plan = plan_for(&fx, &inputs, "site");
        let PackagePlan::Bump(bump) = &plan else {
            panic!("site should be recovered, got a skip");
        };
        assert!(bump.recovered, "site should come back through recovery");

        let site = subjects(&plan);
        assert!(
            site.iter().any(|s| s.contains("dress the site")),
            "the recovered release must carry the commit it missed, got {site:?}"
        );
        assert!(
            !site.iter().any(|s| s.contains("object id")),
            "a server-only commit must not ride along in a recovered release, got {site:?}"
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
            commit_files: CommitFilesCache::default(),
            commit_walk,
        };

        let all_tags = collect_all_tags(&fx.repo);
        let head_ancestors = build_head_ancestors(&fx.repo).ok();
        let tag_index = TagIndex::build(&fx.repo).ok();
        let prerelease_ctx = PrereleaseContext::resolve(None, "main", None).unwrap();
        let forced: Vec<Forced<'_>> = Vec::new();
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
