use anyhow::Result;
use std::path::Path;

use crate::config::{Config, OnFailure};
use crate::git::Repository;
use crate::hooks::{
    HookContext, HookPackage, HookPoint, resolve_hook, resolve_on_failure, run_hook,
};
use crate::timing::Timing;

use super::checkpoint::Checkpoint;
use super::context::RunFlags;
use super::drafts::publish_pending_drafts;
use super::execute::{ReleasePlan, execute_release, print_dry_run_hooks};
use super::state::ReleaseState;
use super::summary::PlannedTag;

pub(super) struct Publisher<'a> {
    pub repo: &'a Repository,
    pub config: &'a Config,
    pub root: &'a Path,
    pub target_branch: &'a str,
    pub flags: RunFlags,
    pub finalizing: bool,
    pub all_packages: &'a [HookPackage],
}

impl Publisher<'_> {
    pub fn publish(&self, state: &mut ReleaseState, timing: &mut Timing) -> Result<()> {
        if state.any_bumped && !state.tags_to_create.is_empty() {
            self.release(state, timing)?;
        } else if self.flags.dry_run && state.any_bumped {
            print_dry_run_hooks(&mut self.release_plan(state, None))?;
            timing.skip("release commit phase", "dry-run");
        }

        if !state.any_bumped && !self.flags.draft && !self.flags.dry_run && !self.flags.shadow {
            publish_pending_drafts(
                self.repo,
                self.config,
                self.root,
                self.flags.verbose,
                &mut state.shared_outputs,
            )?;
        }
        Ok(())
    }

    fn release(&self, state: &mut ReleaseState, timing: &mut Timing) -> Result<()> {
        if !self.flags.dry_run {
            self.write_manifest(state)?;
        }

        let mut checkpoint = self.checkpoint(&state.tags_to_create)?;
        let release_start = std::time::Instant::now();
        let release_result = execute_release(&mut self.release_plan(state, checkpoint.as_mut()));
        timing.record("release commit phase", release_start.elapsed());

        let released_tags: Vec<String> =
            state.tags_to_create.iter().map(|t| t.tag.clone()).collect();
        match release_result {
            Ok(()) => {
                if !self.flags.dry_run {
                    Checkpoint::delete(self.root)?;
                }
                self.run_success_hook(&released_tags)
            }
            Err(err) => {
                self.run_error_hook(&released_tags, &err);
                Err(err)
            }
        }
    }

    fn release_plan<'b>(
        &'b self,
        state: &'b mut ReleaseState,
        checkpoint: Option<&'b mut Checkpoint>,
    ) -> ReleasePlan<'b> {
        ReleasePlan {
            repo: self.repo,
            config: self.config,
            root: self.root,
            target_branch: self.target_branch,
            dry_run: self.flags.dry_run,
            shadow: self.flags.shadow,
            verbose: self.flags.verbose,
            force: self.flags.force,
            draft: self.flags.draft,
            tags_to_create: &state.tags_to_create,
            finalizing: self.finalizing,
            hook_contexts: &state.hook_contexts,
            files_to_commit: &mut state.files_to_commit,
            files_per_package: &mut state.files_per_package,
            pkg_outputs: &mut state.pkg_outputs,
            shared_outputs: &mut state.shared_outputs,
            forge_results: &mut state.forge_results,
            checkpoint,
            forge: None,
        }
    }

    fn write_manifest(&self, state: &mut ReleaseState) -> Result<()> {
        let Some(manifest_rel) = self.config.workspace.manifest_file.as_deref() else {
            return Ok(());
        };
        let overrides: std::collections::BTreeMap<String, String> = state
            .tags_to_create
            .iter()
            .map(|t| (t.package.clone(), t.version.clone()))
            .collect();
        let packages = crate::manifest::snapshot_with_overrides(self.config, self.root, &overrides);
        let manifest = crate::manifest::Manifest::new(
            packages,
            crate::manifest::now_utc_iso8601(),
            short_head(self.repo),
        );
        let manifest_path = self.root.join(manifest_rel);
        crate::manifest::write_atomic(&manifest_path, &manifest)?;
        state.files_to_commit.push(manifest_rel.to_string());
        Ok(())
    }

    fn checkpoint(&self, tags_to_create: &[PlannedTag]) -> Result<Option<Checkpoint>> {
        if self.flags.dry_run {
            return Ok(None);
        }
        let head_sha = self
            .repo
            .head_id()
            .ok()
            .map(|id| id.to_string())
            .unwrap_or_default();
        let tag_names: Vec<String> = tags_to_create.iter().map(|t| t.tag.clone()).collect();
        resume_checkpoint(self.root, head_sha, tag_names, self.flags.verbose).map(Some)
    }

    fn summary_context(&self, released_tags: &[String]) -> HookContext {
        let mut ctx = HookContext::release_summary(
            self.root,
            released_tags,
            self.flags.dry_run,
            self.config.is_monorepo(),
        );
        ctx.all_packages = self.all_packages.to_vec();
        ctx
    }

    fn run_success_hook(&self, released_tags: &[String]) -> Result<()> {
        let ws_hooks = self.config.workspace.hooks.as_ref();
        let Some(cmd) = resolve_hook(None, ws_hooks, HookPoint::OnSuccess) else {
            return Ok(());
        };
        let ctx = self.summary_context(released_tags);
        let on_failure = resolve_on_failure(None, ws_hooks);
        run_hook(
            HookPoint::OnSuccess,
            &cmd,
            &ctx,
            on_failure,
            self.flags.dry_run,
            self.flags.verbose,
            self.root,
        )
    }

    fn run_error_hook(&self, released_tags: &[String], err: &anyhow::Error) {
        let ws_hooks = self.config.workspace.hooks.as_ref();
        let Some(cmd) = resolve_hook(None, ws_hooks, HookPoint::OnError) else {
            return;
        };
        let mut ctx = self.summary_context(released_tags);
        ctx.error_code = crate::error_code::code_from_error(err);
        let _ = run_hook(
            HookPoint::OnError,
            &cmd,
            &ctx,
            OnFailure::Continue,
            self.flags.dry_run,
            self.flags.verbose,
            self.root,
        );
    }
}

fn resume_checkpoint(
    root: &Path,
    head_sha: String,
    tag_names: Vec<String>,
    verbose: bool,
) -> Result<Checkpoint> {
    match Checkpoint::load(root)? {
        Some(existing) if existing.head_sha == head_sha => {
            if verbose {
                tracing::info!(
                    "  ↻ Found in-progress release checkpoint at phase {:?}; resuming",
                    existing.phase
                );
            }
            Ok(existing)
        }
        Some(existing) => Err(anyhow::anyhow!(
            "found a stale release checkpoint at .git/ferrflow.checkpoint.json \
             pinned to commit {} but HEAD is now {}.\n  \
             Either reset HEAD back to {} and rerun to resume the previous release, \
             or delete .git/ferrflow.checkpoint.json to start fresh.",
            &existing.head_sha[..8.min(existing.head_sha.len())],
            &head_sha[..8.min(head_sha.len())],
            &existing.head_sha[..8.min(existing.head_sha.len())]
        )),
        None => Ok(Checkpoint::new(head_sha, tag_names)),
    }
}

pub(super) fn short_head(repo: &Repository) -> String {
    repo.head_id()
        .ok()
        .map(|id| id.to_string()[..7.min(id.to_string().len())].to_string())
        .unwrap_or_default()
}
