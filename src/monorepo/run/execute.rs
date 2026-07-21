use anyhow::Result;
use colored::Colorize;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;

use crate::config::{Config, ReleaseCommitMode, ReleaseCommitScope};
use crate::error_code::{self, ErrorCodeExt};
use crate::git::{
    Repository, create_branch_and_commit, create_branch_and_commits, create_commit,
    create_or_move_tag, create_tag, force_push_tags, get_tag_message, push, push_branch, push_tags,
    tag_exists,
};
use crate::hooks::{HookContext, HookPoint, resolve_hook, resolve_on_failure, run_hook};
use crate::versioning::truncate_version;

use super::checkpoint::{Checkpoint, Phase};
use super::summary::{TagToCreate, write_github_step_summary};
use crate::forge::{Forge, ReleaseResult};
use crate::monorepo::preview::build_forge_instance;
use crate::monorepo::util::{auto_stage_new_files, collect_dirty_files};

/// Inputs threaded through the execution phase. Bundling these as one
/// `&mut ReleasePlan` keeps the per-step function signatures readable —
/// the planning phase used to declare 14 mutable locals and pass them
/// piecewise across each git/forge operation.
///
/// Lifetime: borrows from the caller's locals; lives only for the
/// duration of a single `run_release_logic` invocation.
pub(super) struct ReleasePlan<'a> {
    pub repo: &'a Repository,
    pub config: &'a Config,
    pub root: &'a Path,
    pub target_branch: &'a str,
    pub dry_run: bool,
    pub verbose: bool,
    pub force: bool,
    pub draft: bool,
    pub tags_to_create: &'a [TagToCreate],
    pub hook_contexts: &'a [(HookContext, usize)],
    pub files_to_commit: &'a mut Vec<String>,
    pub files_per_package: &'a mut HashMap<String, Vec<String>>,
    pub pkg_outputs: &'a mut Vec<(String, Vec<String>)>,
    pub shared_outputs: &'a mut Vec<String>,
    /// Per-tag forge release metadata captured from `create_release`,
    /// surfaced in `release --json`. Empty on dry-run.
    pub forge_results: &'a mut Vec<(String, ReleaseResult)>,
    /// Crash-resume marker, persisted at `.git/ferrflow.checkpoint.json`
    /// as each phase succeeds. `None` on dry-run (we never write
    /// anything in that mode) and on resumed runs that already finished
    /// every phase. See #549.
    pub checkpoint: Option<&'a mut Checkpoint>,
}

/// Run the commit / branch+PR / tag / floating-tag / forge-release /
/// push phase of a release, given the per-package decisions already
/// captured into `plan.tags_to_create`.
///
/// Behaviour-preserving extraction from `run_release_logic` — see #529.
pub(super) fn execute_release(plan: &mut ReleasePlan<'_>) -> Result<()> {
    run_pre_commit_hooks(plan)?;

    // Snapshot files_to_commit AFTER pre-commit hooks may have appended
    // auto-staged paths. Clone to own the strings — `plan` itself stays
    // mutably borrowable by the commit/push helpers below.
    let files_snapshot: Vec<String> = plan.files_to_commit.clone();
    let mode = plan.config.workspace.release_commit_mode;
    let scope = plan.config.workspace.release_commit_scope;

    let release_parts: Vec<String> = plan
        .tags_to_create
        .iter()
        .map(|(_, _, _, name, ver, _, _)| format!("{name} v{ver}"))
        .collect();
    let skip_ci = if plan.config.workspace.effective_skip_ci() {
        " [skip ci]"
    } else {
        ""
    };
    let commit_msg = format!("chore(release): {}{skip_ci}", release_parts.join(", "));
    let mut floating_tag_names: Vec<String> = Vec::new();

    if !plan.dry_run {
        let file_refs: Vec<&str> = files_snapshot.iter().map(String::as_str).collect();
        if !checkpoint_is_done(plan, Phase::CommitDone) {
            run_commit_or_pr(
                plan,
                mode,
                scope,
                &file_refs,
                &commit_msg,
                &release_parts,
                skip_ci,
            )?;
            // Record HEAD post-commit so the checkpoint reflects the
            // actual release commit (not the pre-bump HEAD we started
            // from). Useful for debug + future resume sanity checks.
            if let (Some(cp), Some(id)) = (plan.checkpoint.as_mut(), plan.repo.head_id().ok()) {
                cp.commit_sha = Some(id.to_string());
            }
            checkpoint_advance(plan, Phase::CommitDone)?;
        } else if plan.verbose {
            tracing::info!("  ↻ Resumed: skipping commit (already done)");
        }
        run_package_hooks(plan, HookPoint::PostCommit)?;
        run_package_hooks(plan, HookPoint::PreTag)?;
        if !checkpoint_is_done(plan, Phase::TagsCreated) {
            create_release_tags(plan)?;
            create_and_move_floating_tags(plan, &mut floating_tag_names)?;
            checkpoint_advance(plan, Phase::TagsCreated)?;
        } else if plan.verbose {
            tracing::info!("  ↻ Resumed: skipping tag creation (already done)");
        }
        run_package_hooks(plan, HookPoint::PostTag)?;
    }

    run_package_hooks(plan, HookPoint::PrePublish)?;

    if !plan.dry_run {
        if !checkpoint_is_done(plan, Phase::ReleasesCreated) {
            push_and_publish(plan, mode, &floating_tag_names)?;
            checkpoint_advance(plan, Phase::ReleasesCreated)?;
        } else if plan.verbose {
            tracing::info!("  ↻ Resumed: skipping push + publish (already done)");
        }
    }

    if !checkpoint_is_done(plan, Phase::PostPublishDone) {
        run_post_publish_hooks(plan)?;
        checkpoint_advance(plan, Phase::PostPublishDone)?;
    } else if plan.verbose {
        tracing::info!("  ↻ Resumed: skipping post-publish hooks (already done)");
    }

    Ok(())
}

fn checkpoint_is_done(plan: &ReleasePlan<'_>, phase: Phase) -> bool {
    plan.checkpoint
        .as_ref()
        .map(|cp| cp.is_done(phase))
        .unwrap_or(false)
}

fn checkpoint_advance(plan: &mut ReleasePlan<'_>, phase: Phase) -> Result<()> {
    if let Some(cp) = plan.checkpoint.as_mut() {
        cp.advance(phase);
        cp.save(plan.root)?;
    }
    Ok(())
}

fn run_pre_commit_hooks(plan: &mut ReleasePlan<'_>) -> Result<()> {
    for (ctx, pkg_idx) in plan.hook_contexts {
        let pkg = &plan.config.packages[*pkg_idx];
        let ws_hooks = plan.config.workspace.hooks.as_ref();
        let pkg_hooks = pkg.hooks.as_ref();
        let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
        if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PreCommit) {
            let before = collect_dirty_files(plan.repo);
            run_hook(
                HookPoint::PreCommit,
                &cmd,
                ctx,
                on_failure,
                plan.dry_run,
                plan.verbose,
                plan.root,
            )?;
            if !plan.dry_run {
                let len_before = plan.files_to_commit.len();
                auto_stage_new_files(plan.repo, &before, plan.files_to_commit);
                let pkg_files = plan.files_per_package.entry(pkg.name.clone()).or_default();
                for f in &plan.files_to_commit[len_before..] {
                    pkg_files.push(f.clone());
                }
            }
        }
    }
    Ok(())
}

fn run_commit_or_pr(
    plan: &mut ReleasePlan<'_>,
    mode: ReleaseCommitMode,
    scope: ReleaseCommitScope,
    file_refs: &[&str],
    commit_msg: &str,
    release_parts: &[String],
    skip_ci: &str,
) -> Result<()> {
    match mode {
        ReleaseCommitMode::Commit => {
            if scope == ReleaseCommitScope::PerPackage && plan.tags_to_create.len() > 1 {
                for (_, _, _, pkg_name, ver, _, _) in plan.tags_to_create {
                    if let Some(pkg_files) = plan.files_per_package.get(pkg_name) {
                        let refs: Vec<&str> = pkg_files.iter().map(String::as_str).collect();
                        let msg = format!("chore(release): {pkg_name} v{ver}{skip_ci}");
                        create_commit(plan.repo, &refs, &msg)?;
                    }
                }
                plan.shared_outputs
                    .push("✓ Committed release changes (per-package)".to_string());
            } else {
                create_commit(plan.repo, file_refs, commit_msg)?;
                plan.shared_outputs
                    .push("✓ Committed release changes".to_string());
            }
        }
        ReleaseCommitMode::Pr => {
            let branch_name = format!(
                "release/{}",
                release_parts
                    .first()
                    .map(|s| s.replace(' ', "-"))
                    .unwrap_or_else(|| "bump".to_string())
            );
            if scope == ReleaseCommitScope::PerPackage && plan.tags_to_create.len() > 1 {
                let commit_list: Vec<(Vec<&str>, String)> = plan
                    .tags_to_create
                    .iter()
                    .filter_map(|(_, _, _, pkg_name, ver, _, _)| {
                        plan.files_per_package.get(pkg_name).map(|pf| {
                            let refs: Vec<&str> = pf.iter().map(String::as_str).collect();
                            let msg = format!("chore(release): {pkg_name} v{ver}{skip_ci}");
                            (refs, msg)
                        })
                    })
                    .collect();
                let commit_refs: Vec<(&[&str], &str)> = commit_list
                    .iter()
                    .map(|(f, m)| (f.as_slice(), m.as_str()))
                    .collect();
                create_branch_and_commits(plan.repo, &branch_name, &commit_refs)?;
            } else {
                create_branch_and_commit(plan.repo, &branch_name, file_refs, commit_msg)?;
            }
            push_branch(plan.repo, &plan.config.workspace.remote, &branch_name)?;
            plan.shared_outputs
                .push(format!("✓ Pushed branch {}", branch_name.cyan()));

            if let Some(forge_instance) = build_forge_instance(plan.repo, plan.config) {
                let pr_title = format!("chore(release): {}", release_parts.join(", "));
                let pr_body = format!(
                    "Automated release commit.\n\n{}",
                    plan.tags_to_create
                        .iter()
                        .map(|(tag, _, _, _, _, _, _)| format!("- `{tag}`"))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                match forge_instance.create_merge_request(
                    &branch_name,
                    plan.target_branch,
                    &pr_title,
                    &pr_body,
                ) {
                    Ok(mr) => {
                        plan.shared_outputs.push(format!(
                            "✓ Created {} #{}",
                            forge_instance.mr_noun(),
                            mr.id.to_string().cyan()
                        ));
                        run_release_summary_hook(plan, HookPoint::PreRelease)?;
                        if plan.config.workspace.auto_merge_releases {
                            match forge_instance.enable_auto_merge(&mr) {
                                Ok(()) => {
                                    plan.shared_outputs.push("✓ Auto-merge enabled".to_string())
                                }
                                Err(err) => tracing::warn!(
                                    "{}",
                                    format!("  Warning: failed to enable auto-merge: {err}")
                                        .yellow()
                                ),
                            }
                        }
                    }
                    Err(err) => tracing::warn!(
                        "{}",
                        format!(
                            "  Warning: failed to create {}: {err}",
                            forge_instance.mr_noun()
                        )
                        .yellow()
                    ),
                }
            }
        }
        ReleaseCommitMode::None => {}
    }
    Ok(())
}

fn create_release_tags(plan: &mut ReleasePlan<'_>) -> Result<()> {
    for (tag_name, tag_msg, _, pkg_name, _, _, _) in plan.tags_to_create {
        create_tag(plan.repo, tag_name, tag_msg)?;
        if let Some((_, lines)) = plan
            .pkg_outputs
            .iter_mut()
            .rev()
            .find(|(n, _)| n == pkg_name)
        {
            lines.push(format!("  ✓ Created tag {}", tag_name.cyan()));
        }
    }
    Ok(())
}

fn create_and_move_floating_tags(
    plan: &mut ReleasePlan<'_>,
    floating_tag_names: &mut Vec<String>,
) -> Result<()> {
    for (_, _, _, pkg_name, new_version, _, is_pre) in plan.tags_to_create {
        if *is_pre {
            continue;
        }
        let pkg = plan
            .config
            .packages
            .iter()
            .find(|p| &p.name == pkg_name)
            .ok_or_else(|| anyhow::anyhow!("package '{pkg_name}' not found in config"))
            .error_code(error_code::MONOREPO_PACKAGE_NOT_FOUND)?;
        let levels = pkg.effective_floating_tags(&plan.config.workspace);
        for level in levels {
            if let Some(truncated) = truncate_version(new_version, *level) {
                let float_tag = pkg.tag_for_version(
                    &plan.config.workspace,
                    plan.config.is_monorepo(),
                    &truncated,
                );
                if tag_exists(plan.repo, &float_tag)
                    && let Some(old_msg) = get_tag_message(plan.repo, &float_tag)
                    && let Some(old_ver) = old_msg.strip_prefix("Release ")
                    && semver::Version::parse(old_ver.trim_start_matches('v'))
                        .ok()
                        .zip(semver::Version::parse(new_version.trim_start_matches('v')).ok())
                        .is_some_and(|(old, new)| new < old)
                {
                    if !plan.force {
                        Err(anyhow::anyhow!(
                            "Floating tag {} would move backward ({} → {}). Use --force to override.",
                            float_tag,
                            old_ver,
                            new_version,
                        ))
                        .error_code(error_code::MONOREPO_PUSH_FAILED)?;
                    }
                    tracing::warn!(
                        "{}",
                        format!(
                            "  ⚠ Floating tag {} moves backward ({} → {})",
                            float_tag, old_ver, new_version,
                        )
                        .yellow()
                    );
                }
                let msg = format!("Release {new_version}");
                let moved = create_or_move_tag(plan.repo, &float_tag, &msg)?;
                let verb = if moved { "Moved" } else { "Created" };
                if let Some((_, lines)) = plan
                    .pkg_outputs
                    .iter_mut()
                    .rev()
                    .find(|(n, _)| n == pkg_name)
                {
                    lines.push(format!("  ✓ {} floating tag {}", verb, float_tag.cyan()));
                }
                floating_tag_names.push(float_tag);
            }
        }
    }
    Ok(())
}

fn run_package_hooks(plan: &ReleasePlan<'_>, point: HookPoint) -> Result<()> {
    for (ctx, pkg_idx) in plan.hook_contexts {
        let pkg = &plan.config.packages[*pkg_idx];
        let ws_hooks = plan.config.workspace.hooks.as_ref();
        let pkg_hooks = pkg.hooks.as_ref();
        let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
        if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, point) {
            run_hook(
                point,
                &cmd,
                ctx,
                on_failure,
                plan.dry_run,
                plan.verbose,
                plan.root,
            )?;
        }
    }
    Ok(())
}

fn run_release_summary_hook(plan: &ReleasePlan<'_>, point: HookPoint) -> Result<()> {
    let ws_hooks = plan.config.workspace.hooks.as_ref();
    if let Some(cmd) = resolve_hook(None, ws_hooks, point) {
        let on_failure = resolve_on_failure(None, ws_hooks);
        let tags: Vec<String> = plan
            .tags_to_create
            .iter()
            .map(|(t, _, _, _, _, _, _)| t.clone())
            .collect();
        let mut ctx =
            HookContext::release_summary(plan.root, &tags, plan.dry_run, plan.config.is_monorepo());
        if let Some((first, _)) = plan.hook_contexts.first() {
            ctx.all_packages = first.all_packages.clone();
        }
        run_hook(
            point,
            &cmd,
            &ctx,
            on_failure,
            plan.dry_run,
            plan.verbose,
            plan.root,
        )?;
    }
    Ok(())
}

fn push_and_publish(
    plan: &mut ReleasePlan<'_>,
    mode: ReleaseCommitMode,
    floating_tag_names: &[String],
) -> Result<()> {
    let tag_refs: Vec<&str> = plan
        .tags_to_create
        .iter()
        .map(|(t, _, _, _, _, _, _)| t.as_str())
        .collect();

    if let ReleaseCommitMode::Commit = mode {
        push(
            plan.repo,
            &plan.config.workspace.remote,
            plan.target_branch,
            &[],
        )?;
        plan.shared_outputs.push(format!(
            "✓ Pushed and verified on {}/{}",
            plan.config.workspace.remote, plan.target_branch
        ));
    }

    let target_sha = plan.repo.head_id().ok().map(|id| id.to_string());

    if let Some(forge_instance) = build_forge_instance(plan.repo, plan.config) {
        let forge_ref: &dyn Forge = forge_instance.as_ref();
        let draft = plan.draft;
        let target_sha_ref = target_sha.as_deref();

        let threads = forge_pool_threads(plan.tags_to_create.len(), crate::concurrency::max_jobs());
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build release thread pool: {e}"))
            .error_code(error_code::MONOREPO_PUSH_FAILED)?;

        let outcomes: Vec<(String, String, TagReleaseOutcome)> = pool.install(|| {
            plan.tags_to_create
                .par_iter()
                .map(|(tag_name, _, body, pkg_name, _, _, is_pre)| {
                    let outcome = process_release_tag(
                        forge_ref,
                        tag_name,
                        body,
                        *is_pre,
                        draft,
                        target_sha_ref,
                    );
                    (tag_name.clone(), pkg_name.clone(), outcome)
                })
                .collect()
        });

        for (tag_name, pkg_name, outcome) in outcomes {
            for warning in outcome.warnings {
                if !warning.verbose_only || plan.verbose {
                    tracing::warn!("{}", warning.message.yellow());
                }
            }
            if let Some(result) = outcome.result {
                plan.forge_results.push((tag_name, result));
            }
            if let Some(line) = outcome.success_line
                && let Some((_, lines)) = plan
                    .pkg_outputs
                    .iter_mut()
                    .rev()
                    .find(|(n, _)| *n == pkg_name)
            {
                lines.push(line);
            }
        }
    }

    if !tag_refs.is_empty() {
        push_tags(plan.repo, &plan.config.workspace.remote, &tag_refs)?;
        plan.shared_outputs.push("✓ Pushed tags".to_string());
    }

    if !floating_tag_names.is_empty() {
        let float_refs: Vec<&str> = floating_tag_names.iter().map(String::as_str).collect();
        force_push_tags(plan.repo, &plan.config.workspace.remote, &float_refs)?;
        plan.shared_outputs
            .push("✓ Pushed floating tags".to_string());
    }

    write_github_step_summary(plan.tags_to_create);
    Ok(())
}

const RELEASE_FORGE_CONCURRENCY: usize = 8;

fn forge_pool_threads(tag_count: usize, max_jobs: usize) -> usize {
    tag_count.clamp(1, RELEASE_FORGE_CONCURRENCY.min(max_jobs))
}

struct TagReleaseWarning {
    message: String,
    verbose_only: bool,
}

struct TagReleaseOutcome {
    warnings: Vec<TagReleaseWarning>,
    success_line: Option<String>,
    result: Option<ReleaseResult>,
}

fn process_release_tag(
    forge: &dyn Forge,
    tag_name: &str,
    body: &str,
    is_pre: bool,
    draft: bool,
    target_sha: Option<&str>,
) -> TagReleaseOutcome {
    let noun = forge.release_noun();
    let mut warnings = Vec::new();

    if !draft {
        match forge.find_draft_release(tag_name) {
            Ok(Some(release_id)) => match forge.publish_release(release_id) {
                Ok(()) => {
                    return TagReleaseOutcome {
                        warnings,
                        success_line: Some(format!(
                            "  ✓ Published draft {} {}",
                            noun,
                            tag_name.cyan()
                        )),
                        result: None,
                    };
                }
                Err(err) => warnings.push(TagReleaseWarning {
                    message: format!("  Warning: failed to publish draft for {tag_name}: {err}"),
                    verbose_only: false,
                }),
            },
            Ok(None) => {}
            Err(err) => warnings.push(TagReleaseWarning {
                message: format!("  Warning: failed to check for draft release {tag_name}: {err}"),
                verbose_only: true,
            }),
        }
    }

    match forge.create_release(tag_name, body, is_pre, draft, target_sha) {
        Ok(result) => {
            let success_line = if draft {
                format!("  ✓ Draft {} {}", noun, tag_name.cyan())
            } else {
                format!("  ✓ {} {}", noun, tag_name.cyan())
            };
            TagReleaseOutcome {
                warnings,
                success_line: Some(success_line),
                result: Some(result),
            }
        }
        Err(err) => {
            warnings.push(TagReleaseWarning {
                message: format!("  Warning: failed to create {noun} for {tag_name}: {err}"),
                verbose_only: false,
            });
            TagReleaseOutcome {
                warnings,
                success_line: None,
                result: None,
            }
        }
    }
}

fn release_url_for_tag(forge_results: &[(String, ReleaseResult)], tag: &str) -> Option<String> {
    forge_results
        .iter()
        .find(|(t, _)| t == tag)
        .and_then(|(_, result)| result.url.clone())
}

fn run_post_publish_hooks(plan: &mut ReleasePlan<'_>) -> Result<()> {
    for (ctx, pkg_idx) in plan.hook_contexts {
        let pkg = &plan.config.packages[*pkg_idx];
        let ws_hooks = plan.config.workspace.hooks.as_ref();
        let pkg_hooks = pkg.hooks.as_ref();
        let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
        if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PostPublish) {
            let mut ctx = ctx.clone();
            ctx.release_url = release_url_for_tag(plan.forge_results, &ctx.tag);
            run_hook(
                HookPoint::PostPublish,
                &cmd,
                &ctx,
                on_failure,
                plan.dry_run,
                plan.verbose,
                plan.root,
            )?;
        }
        // Declarative publishers preview. v1 ships the plan only —
        // Declarative publishers: cargo executes for real now (#572 +
        // this PR), the other kinds still preview-only until their PR
        // lands. The dispatcher hides the kind-by-kind degradation —
        // users see uniform "[kind] action … → status" log lines and
        // can declare their full publishing plan today without waiting
        // for every executor.
        run_publishers_for_package(plan, pkg, &ctx.package, &ctx.new_version, &ctx.tag)?;
    }
    Ok(())
}

fn run_publishers_for_package(
    plan: &ReleasePlan<'_>,
    pkg: &crate::config::PackageConfig,
    package_name: &str,
    new_version: &str,
    tag: &str,
) -> Result<()> {
    // `deferPublish` hands publishing off to a separate `ferrflow
    // publish` run (typically a CI job that carries the build toolchain
    // the release job lacks). Skip the inline run here.
    if plan.config.workspace.defer_publish {
        return Ok(());
    }
    let package_path = plan.root.join(&pkg.path);
    let pub_ctx = crate::publishers::PublishContext {
        package_name,
        package_path: &package_path,
        new_version,
        tag,
        registries: &plan.config.workspace.registries,
        dry_run: plan.dry_run,
        verbose: plan.verbose,
    };
    crate::publishers::run_all(&pkg.publishers, &pub_ctx)
}

/// Dry-run hook trace: when nothing will actually fire (no commit, no
/// tag push), still print what the user's PreCommit/PrePublish/PostPublish
/// hooks WOULD have run. Mirrors the old inline `else if dry_run &&
/// any_bumped` branch.
pub(super) fn print_dry_run_hooks(plan: &ReleasePlan<'_>) -> Result<()> {
    for (ctx, pkg_idx) in plan.hook_contexts {
        let pkg = &plan.config.packages[*pkg_idx];
        let ws_hooks = plan.config.workspace.hooks.as_ref();
        let pkg_hooks = pkg.hooks.as_ref();
        let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
        for point in [
            HookPoint::PreCommit,
            HookPoint::PostCommit,
            HookPoint::PreTag,
            HookPoint::PostTag,
            HookPoint::PrePublish,
            HookPoint::PostPublish,
        ] {
            if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, point) {
                run_hook(point, &cmd, ctx, on_failure, true, plan.verbose, plan.root)?;
            }
        }
        run_publishers_for_package(plan, pkg, &ctx.package, &ctx.new_version, &ctx.tag)?;
    }
    Ok(())
}

#[cfg(test)]
mod tag_release_tests {
    use super::*;
    use crate::forge::MergeRequestResult;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockForge {
        drafts: HashMap<String, u64>,
        publish_fails: bool,
        find_draft_errors_for: Vec<String>,
        create_fails_for: Vec<String>,
        create_calls: Mutex<Vec<String>>,
    }

    impl Forge for MockForge {
        fn create_release(
            &self,
            tag: &str,
            _body: &str,
            _prerelease: bool,
            _draft: bool,
            _target_commitish: Option<&str>,
        ) -> Result<ReleaseResult> {
            self.create_calls.lock().unwrap().push(tag.to_string());
            if self.create_fails_for.iter().any(|t| t == tag) {
                anyhow::bail!("create failed for {tag}");
            }
            Ok(ReleaseResult {
                id: Some(1),
                url: Some(format!("https://forge/{tag}")),
            })
        }

        fn find_draft_release(&self, tag: &str) -> Result<Option<u64>> {
            if self.find_draft_errors_for.iter().any(|t| t == tag) {
                anyhow::bail!("draft lookup failed for {tag}");
            }
            Ok(self.drafts.get(tag).copied())
        }

        fn publish_release(&self, _release_id: u64) -> Result<()> {
            if self.publish_fails {
                anyhow::bail!("publish failed");
            }
            Ok(())
        }

        fn create_merge_request(
            &self,
            _head: &str,
            _base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<MergeRequestResult> {
            unreachable!("not exercised by release-tag tests")
        }

        fn enable_auto_merge(&self, _mr: &MergeRequestResult) -> Result<()> {
            unreachable!("not exercised by release-tag tests")
        }

        fn mr_noun(&self) -> &'static str {
            "pull request"
        }

        fn release_noun(&self) -> &'static str {
            "release"
        }

        fn find_comment(&self, _pr_id: u64, _marker: &str) -> Result<Option<u64>> {
            unreachable!("not exercised by release-tag tests")
        }

        fn create_comment(&self, _pr_id: u64, _body: &str) -> Result<()> {
            unreachable!("not exercised by release-tag tests")
        }

        fn update_comment(&self, _pr_id: u64, _comment_id: u64, _body: &str) -> Result<()> {
            unreachable!("not exercised by release-tag tests")
        }
    }

    #[test]
    fn publishes_existing_draft_and_skips_create() {
        let forge = MockForge {
            drafts: HashMap::from([("v1".to_string(), 42)]),
            ..Default::default()
        };
        let outcome = process_release_tag(&forge, "v1", "body", false, false, None);

        assert!(outcome.warnings.is_empty());
        assert!(outcome.result.is_none());
        let line = outcome.success_line.expect("a success line");
        assert!(line.contains("Published draft release"));
        assert!(line.contains("v1"));
        assert!(
            forge.create_calls.lock().unwrap().is_empty(),
            "create_release must not run when a draft was published"
        );
    }

    #[test]
    fn falls_through_to_create_when_publish_fails() {
        let forge = MockForge {
            drafts: HashMap::from([("v1".to_string(), 42)]),
            publish_fails: true,
            ..Default::default()
        };
        let outcome = process_release_tag(&forge, "v1", "body", false, false, None);

        assert_eq!(outcome.warnings.len(), 1);
        assert!(!outcome.warnings[0].verbose_only);
        assert!(
            outcome.warnings[0]
                .message
                .contains("failed to publish draft")
        );
        assert!(outcome.result.is_some());
        assert_eq!(forge.create_calls.lock().unwrap().as_slice(), ["v1"]);
    }

    #[test]
    fn creates_release_when_no_draft() {
        let forge = MockForge::default();
        let outcome = process_release_tag(&forge, "v1", "body", false, false, None);

        assert!(outcome.warnings.is_empty());
        assert_eq!(
            outcome.result.and_then(|r| r.url).as_deref(),
            Some("https://forge/v1")
        );
        assert_eq!(forge.create_calls.lock().unwrap().as_slice(), ["v1"]);
    }

    #[test]
    fn create_failure_yields_warning_and_no_result() {
        let forge = MockForge {
            create_fails_for: vec!["v1".to_string()],
            ..Default::default()
        };
        let outcome = process_release_tag(&forge, "v1", "body", false, false, None);

        assert_eq!(outcome.warnings.len(), 1);
        assert!(!outcome.warnings[0].verbose_only);
        assert!(
            outcome.warnings[0]
                .message
                .contains("failed to create release")
        );
        assert!(outcome.result.is_none());
        assert!(outcome.success_line.is_none());
    }

    #[test]
    fn draft_check_error_is_verbose_only_then_creates() {
        let forge = MockForge {
            find_draft_errors_for: vec!["v1".to_string()],
            ..Default::default()
        };
        let outcome = process_release_tag(&forge, "v1", "body", false, false, None);

        assert_eq!(outcome.warnings.len(), 1);
        assert!(outcome.warnings[0].verbose_only);
        assert!(outcome.result.is_some());
    }

    #[test]
    fn draft_mode_skips_draft_lookup_and_labels_draft() {
        let forge = MockForge {
            drafts: HashMap::from([("v1".to_string(), 42)]),
            ..Default::default()
        };
        let outcome = process_release_tag(&forge, "v1", "body", false, true, None);

        assert!(outcome.warnings.is_empty());
        assert!(outcome.result.is_some());
        let line = outcome.success_line.expect("a success line");
        assert!(line.contains("✓ Draft release"));
        assert!(!line.contains("Published"));
        assert!(line.contains("v1"));
        assert_eq!(forge.create_calls.lock().unwrap().as_slice(), ["v1"]);
    }

    #[test]
    fn forge_pool_thread_sizing_respects_max_jobs() {
        assert_eq!(forge_pool_threads(50, usize::MAX), 8);
        assert_eq!(forge_pool_threads(3, usize::MAX), 3);
        assert_eq!(forge_pool_threads(50, 1), 1);
        assert_eq!(forge_pool_threads(50, 4), 4);
        assert_eq!(forge_pool_threads(0, usize::MAX), 1);
    }

    #[test]
    fn parallel_collection_preserves_tag_order() {
        let forge = MockForge::default();
        let tags = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(RELEASE_FORGE_CONCURRENCY)
            .build()
            .unwrap();

        let collected: Vec<(String, Option<String>)> = pool.install(|| {
            tags.par_iter()
                .map(|t| {
                    let outcome = process_release_tag(&forge, t, "body", false, false, None);
                    (t.to_string(), outcome.result.and_then(|r| r.url))
                })
                .collect()
        });

        let order: Vec<&str> = collected.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(order, tags);
        for (tag, url) in &collected {
            assert_eq!(
                url.as_deref(),
                Some(format!("https://forge/{tag}").as_str())
            );
        }
    }

    #[test]
    fn release_url_for_tag_matches_by_tag_only() {
        let results = vec![
            (
                "api@v1.0.0".to_string(),
                ReleaseResult {
                    id: Some(1),
                    url: Some("https://forge/api".to_string()),
                },
            ),
            (
                "web@v2.0.0".to_string(),
                ReleaseResult {
                    id: Some(2),
                    url: None,
                },
            ),
        ];

        assert_eq!(
            release_url_for_tag(&results, "api@v1.0.0").as_deref(),
            Some("https://forge/api")
        );
        // Tag released but the forge returned no URL.
        assert_eq!(release_url_for_tag(&results, "web@v2.0.0"), None);
        // Tag not in this batch's forge results.
        assert_eq!(release_url_for_tag(&results, "cli@v3.0.0"), None);
    }
}
