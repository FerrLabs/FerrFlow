use anyhow::Result;
use colored::Colorize;
use gix::ObjectId;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::Path;

use crate::config::{Config, ReleaseCommitMode};
use crate::git::{
    Repository, TagIndex, collect_all_tags, fetch_tags, get_changed_files, open_repo,
};
use crate::prerelease::PrereleaseContext;
use crate::timing::Timing;

use super::forced::Forced;
use super::lock::ReleaseLock;
use super::plan::{self, PackagePlan, PlanInputs, compute_plan};
use super::summary::PlannedTag;
use super::{finalize, groups, upgrade};

#[derive(Clone, Copy)]
pub(super) struct RunFlags {
    pub dry_run: bool,
    pub verbose: bool,
    pub json: bool,
    pub release_json: bool,
    pub force: bool,
    pub draft: bool,
    pub shadow: bool,
}

impl RunFlags {
    pub fn quiet(self) -> bool {
        self.json || self.release_json
    }
}

pub(super) struct RepoAccess<'a> {
    pub channel: Option<&'a str>,
    pub force_unlock: bool,
}

pub(super) struct ReleaseInputs {
    pub repo: Repository,
    _lock: Option<ReleaseLock>,
    pub forge_base: Option<String>,
    pub prerelease_ctx: PrereleaseContext,
    pub short_hash: String,
    pub all_tags: Vec<String>,
    pub tag_index: Option<TagIndex>,
    pub fallback_ancestors: Option<HashSet<ObjectId>>,
    pub target_branch: String,
    pub changed_files: Vec<String>,
}

impl ReleaseInputs {
    pub fn resolve(
        root: &Path,
        config: &Config,
        flags: RunFlags,
        access: RepoAccess<'_>,
        timing: &mut Timing,
    ) -> Result<Self> {
        let repo = open_repo(root)?;
        let lock = open_release(&repo, config, flags, access.force_unlock, timing)?;

        let current_branch = crate::git::resolve_current_branch(&repo, &config.workspace.branch);
        let forge_base = forge_base(&repo, config);
        let prerelease_ctx = PrereleaseContext::resolve(
            access.channel,
            &current_branch,
            config.workspace.branches.as_deref(),
        )?;

        let short_hash = repo
            .head_id()
            .ok()
            .map(|id| id.to_string()[..7].to_string())
            .unwrap_or_default();

        let all_tags = collect_all_tags(&repo);
        let tag_index = timing.stage("build TagIndex", || TagIndex::build(&repo).ok());
        let fallback_ancestors = match &tag_index {
            Some(_) => None,
            None => crate::git::build_head_ancestors(&repo).ok(),
        };

        let target_branch = if prerelease_ctx.is_prerelease() {
            current_branch
        } else {
            config.workspace.branch.clone()
        };

        let changed_files = get_changed_files(&repo)?;
        if !flags.quiet() {
            log_changed_files(&changed_files);
        }

        Ok(Self {
            repo,
            _lock: lock,
            forge_base,
            prerelease_ctx,
            short_hash,
            all_tags,
            tag_index,
            fallback_ancestors,
            target_branch,
            changed_files,
        })
    }

    pub fn finalize_tags(&self, root: &Path, config: &Config, dry_run: bool) -> Vec<PlannedTag> {
        if dry_run || config.workspace.release_commit_mode != ReleaseCommitMode::Pr {
            return Vec::new();
        }
        finalize::merged_release_tags(
            &self.repo,
            config,
            root,
            &self.all_tags,
            self.forge_base.clone(),
        )
    }

    pub fn compute_plans(
        &self,
        root: &Path,
        config: &Config,
        forced: &[Forced<'_>],
        excluded: &[String],
    ) -> Result<Vec<Option<PackagePlan>>> {
        let thread_safe_repo = self.repo.clone().into_sync();
        let changed_files_cache = plan::ChangedFilesCache::default();
        let commit_files_cache = plan::CommitFilesCache::default();
        let commit_walk =
            crate::git::CommitWalkCache::new(config.workspace.effective_commit_skip_markers());
        let plan_inputs = PlanInputs {
            config,
            root,
            tag_index: self.tag_index.as_ref(),
            head_ancestors: self
                .tag_index
                .as_ref()
                .map(|idx| &idx.ancestors)
                .or(self.fallback_ancestors.as_ref()),
            all_tags: &self.all_tags,
            prerelease_ctx: &self.prerelease_ctx,
            forced,
            excluded,
            changed_files: &self.changed_files,
            short_hash: &self.short_hash,
            changed_files_cache: &changed_files_cache,
            commit_files_cache: &commit_files_cache,
            commit_walk: &commit_walk,
        };
        let plans: Vec<PackagePlan> = config
            .packages
            .par_iter()
            .map(|pkg| {
                let repo = thread_safe_repo.to_thread_local();
                compute_plan(&repo, pkg, &plan_inputs)
            })
            .collect::<Result<Vec<_>>>()?;

        let mut plans: Vec<Option<PackagePlan>> = plans.into_iter().map(Some).collect();
        upgrade::apply_cascade_upgrades(config, &self.all_tags, &mut plans);
        groups::apply_groups(config, root, &mut plans);
        Ok(plans)
    }
}

fn open_release(
    repo: &Repository,
    config: &Config,
    flags: RunFlags,
    force_unlock: bool,
    timing: &mut Timing,
) -> Result<Option<ReleaseLock>> {
    if flags.dry_run {
        timing.skip("fetch_tags", "dry-run");
        return Ok(None);
    }

    let lock = if force_unlock {
        ReleaseLock::acquire_force(repo)?
    } else {
        ReleaseLock::acquire(repo)?
    };

    let start = std::time::Instant::now();
    let fetch = fetch_tags(repo, crate::git::Remote::of(&config.workspace));
    timing.record("fetch_tags", start.elapsed());
    if let Err(e) = fetch
        && flags.verbose
    {
        tracing::warn!("Warning: could not fetch remote tags: {e}");
    }
    crate::git::write_commit_graph_if_absent(repo);
    Ok(Some(lock))
}

fn forge_base(repo: &Repository, config: &Config) -> Option<String> {
    config.workspace.changelog.as_ref().and_then(|cl| {
        if cl.include_commit_links || cl.include_compare_link {
            crate::git::get_remote_url(repo, &config.workspace.remote)
                .as_deref()
                .and_then(crate::forge::web_base_url)
        } else {
            None
        }
    })
}

fn log_changed_files(changed_files: &[String]) {
    if changed_files.is_empty() {
        return;
    }
    tracing::debug!("Changed files in last commit:");
    for f in changed_files {
        tracing::debug!("  {}", f.dimmed());
    }
    tracing::debug!("");
}
