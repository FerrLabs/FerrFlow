mod floating_tags;
mod release_commit;

use anyhow::Result;
use colored::Colorize;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;

use crate::config::{Config, ReleaseCommitMode};
use crate::error_code::{self, ErrorCodeExt};
use crate::git::{Remote, Repository, create_tag, force_push_tags, push, push_tags};
use crate::hooks::{HookContext, HookPoint, resolve_hook, resolve_on_failure, run_hook};

use super::checkpoint::{Checkpoint, Phase};
use super::summary::{PlannedTag, write_github_step_summary};
use crate::forge::{Forge, ReleaseResult};
use crate::monorepo::preview::{ForgeUnavailable, build_forge_instance};
use crate::monorepo::util::{auto_stage_new_files, collect_dirty_files};
use floating_tags::create_and_move_floating_tags;
use release_commit::{ReleaseCommit, run_commit_or_pr};

pub(super) struct ReleasePlan<'a> {
    pub repo: &'a Repository,
    pub config: &'a Config,
    pub root: &'a Path,
    pub target_branch: &'a str,
    pub dry_run: bool,
    pub verbose: bool,
    pub force: bool,
    pub draft: bool,
    pub tags_to_create: &'a [PlannedTag],
    pub finalizing: bool,
    pub hook_contexts: &'a [(HookContext, usize)],
    pub files_to_commit: &'a mut Vec<String>,
    pub files_per_package: &'a mut HashMap<String, Vec<String>>,
    pub pkg_outputs: &'a mut Vec<(String, Vec<String>)>,
    pub shared_outputs: &'a mut Vec<String>,
    pub forge_results: &'a mut Vec<(String, ReleaseResult)>,
    pub checkpoint: Option<&'a mut Checkpoint>,
    pub forge: Option<&'a dyn Forge>,
}

pub(super) fn execute_release(plan: &mut ReleasePlan<'_>) -> Result<()> {
    run_pre_commit_hooks(plan)?;

    let files_snapshot: Vec<String> = plan.files_to_commit.clone();
    let release_parts: Vec<String> = plan
        .tags_to_create
        .iter()
        .map(|t| format!("{} v{}", t.package, t.version))
        .collect();
    let skip_ci = skip_ci_marker(plan.config);
    let commit_msg = super::commit_body::build_commit_message(
        &format!("chore(release): {}{skip_ci}", release_parts.join(", ")),
        plan.tags_to_create,
        plan.config.workspace.release_commit_body,
    );
    let file_refs: Vec<&str> = files_snapshot.iter().map(String::as_str).collect();
    let commit = ReleaseCommit {
        mode: release_commit_mode(plan),
        scope: plan.config.workspace.release_commit_scope,
        file_refs: &file_refs,
        message: &commit_msg,
        release_parts: &release_parts,
        skip_ci,
    };
    let mut floating_tag_names: Vec<String> = Vec::new();

    if !plan.dry_run {
        commit_and_tag(plan, &commit, &mut floating_tag_names)?;
    }

    run_package_hooks(plan, HookPoint::PrePublish)?;

    if !plan.dry_run {
        push_and_publish(plan, commit.mode, &floating_tag_names)?;
    }

    run_phase(
        plan,
        Phase::PostPublishDone,
        "  ↻ Resumed: skipping post-publish hooks (already done)",
        run_post_publish_hooks,
    )
}

fn release_commit_mode(plan: &ReleasePlan<'_>) -> ReleaseCommitMode {
    if plan.finalizing {
        ReleaseCommitMode::None
    } else {
        plan.config.workspace.release_commit_mode
    }
}

fn skip_ci_marker(config: &Config) -> &'static str {
    if config.workspace.effective_skip_ci() {
        " [skip ci]"
    } else {
        ""
    }
}

fn commit_and_tag(
    plan: &mut ReleasePlan<'_>,
    commit: &ReleaseCommit<'_>,
    floating_tag_names: &mut Vec<String>,
) -> Result<()> {
    run_phase(
        plan,
        Phase::CommitDone,
        "  ↻ Resumed: skipping commit (already done)",
        |plan| {
            run_commit_or_pr(plan, commit)?;
            if let (Some(cp), Some(id)) = (plan.checkpoint.as_mut(), plan.repo.head_id().ok()) {
                cp.commit_sha = Some(id.to_string());
            }
            Ok(())
        },
    )?;
    run_package_hooks(plan, HookPoint::PostCommit)?;
    run_package_hooks(plan, HookPoint::PreTag)?;
    // The release is only proposed at this point. Tagging here would
    // label the pre-bump commit and make later runs report "nothing to
    // release", which is what froze the PR before #934. Tags and
    // releases are produced by the finalising run, once the release
    // commit has landed on the target branch.
    if commit.mode != ReleaseCommitMode::Pr {
        run_phase(
            plan,
            Phase::TagsCreated,
            "  ↻ Resumed: skipping tag creation (already done)",
            |plan| {
                create_release_tags(plan)?;
                create_and_move_floating_tags(plan, floating_tag_names)
            },
        )?;
    }
    run_package_hooks(plan, HookPoint::PostTag)
}

fn push_and_publish(
    plan: &mut ReleasePlan<'_>,
    mode: ReleaseCommitMode,
    floating_tag_names: &[String],
) -> Result<()> {
    run_phase(
        plan,
        Phase::Pushed,
        "  ↻ Resumed: skipping push (already done)",
        |plan| push_refs(plan, mode, floating_tag_names),
    )?;
    // Same reason as the tags above.
    if mode == ReleaseCommitMode::Pr {
        return Ok(());
    }
    run_phase(
        plan,
        Phase::ReleasesCreated,
        "  ↻ Resumed: skipping publish (already done)",
        publish_releases,
    )
}

fn run_phase<'a>(
    plan: &mut ReleasePlan<'a>,
    phase: Phase,
    resumed: &str,
    step: impl FnOnce(&mut ReleasePlan<'a>) -> Result<()>,
) -> Result<()> {
    if checkpoint_is_done(plan, phase) {
        if plan.verbose {
            tracing::info!("{resumed}");
        }
        return Ok(());
    }
    step(plan)?;
    checkpoint_advance(plan, phase)
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

fn release_branch_name(target_branch: &str) -> String {
    format!("ferrflow/release-{}", target_branch.replace('/', "-"))
}

pub(super) fn forge_unavailable(branch: &str, reason: ForgeUnavailable) -> anyhow::Error {
    anyhow::anyhow!("cannot open the release pull request for branch {branch}: {reason}")
}

pub(super) fn release_pr_title(parts: &[String], limit: usize) -> String {
    let full = format!("chore(release): {}", parts.join(", "));
    if full.chars().count() <= limit {
        return full;
    }

    for kept in (1..parts.len()).rev() {
        let candidate = format!(
            "chore(release): {} and {} more",
            parts[..kept].join(", "),
            parts.len() - kept
        );
        if candidate.chars().count() <= limit {
            return candidate;
        }
    }

    let mut cut: String = full.chars().take(limit.saturating_sub(1)).collect();
    cut.push('\u{2026}');
    cut
}

fn create_release_tags(plan: &mut ReleasePlan<'_>) -> Result<()> {
    for t in plan.tags_to_create {
        create_tag(plan.repo, &t.tag, &t.message)?;
        // Pin the tag to the commit it points at, so `ferrflow rollback` can
        // tell a tag this run created from one someone else has since moved.
        if let Some(sha) = crate::git::resolve_tag_name_to_commit(plan.repo, &t.tag)
            && let Some(cp) = plan.checkpoint.as_mut()
        {
            cp.created_tags
                .push(crate::monorepo::run::checkpoint::RecordedTag {
                    name: t.tag.clone(),
                    sha: sha.to_string(),
                });
        }
        if let Some((_, lines)) = plan
            .pkg_outputs
            .iter_mut()
            .rev()
            .find(|(n, _)| n == &t.package)
        {
            lines.push(format!("  ✓ Created tag {}", t.tag.cyan()));
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
        let tags: Vec<String> = plan.tags_to_create.iter().map(|t| t.tag.clone()).collect();
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

fn push_refs(
    plan: &mut ReleasePlan<'_>,
    mode: ReleaseCommitMode,
    floating_tag_names: &[String],
) -> Result<()> {
    let tag_refs: Vec<&str> = plan.tags_to_create.iter().map(|t| t.tag.as_str()).collect();

    if let ReleaseCommitMode::Commit = mode {
        push(
            plan.repo,
            Remote::of(&plan.config.workspace),
            plan.target_branch,
        )?;
        plan.shared_outputs.push(format!(
            "✓ Pushed and verified on {}/{}",
            plan.config.workspace.remote, plan.target_branch
        ));
    }

    if !tag_refs.is_empty() && mode != ReleaseCommitMode::Pr {
        push_tags(plan.repo, Remote::of(&plan.config.workspace), &tag_refs)?;
        plan.shared_outputs.push("✓ Pushed tags".to_string());
    }

    if !floating_tag_names.is_empty() {
        let float_refs: Vec<&str> = floating_tag_names.iter().map(String::as_str).collect();
        force_push_tags(plan.repo, Remote::of(&plan.config.workspace), &float_refs)?;
        plan.shared_outputs
            .push("✓ Pushed floating tags".to_string());
    }

    Ok(())
}

fn publish_releases(plan: &mut ReleasePlan<'_>) -> Result<()> {
    match plan.forge {
        Some(forge) => publish_releases_with(plan, forge)?,
        None => {
            if let Some(forge_instance) = build_forge_instance(plan.repo, plan.config) {
                publish_releases_with(plan, forge_instance.as_ref())?;
            }
        }
    }

    write_github_step_summary(plan.tags_to_create);
    Ok(())
}

fn publish_releases_with(plan: &mut ReleasePlan<'_>, forge: &dyn Forge) -> Result<()> {
    let draft = plan.draft;

    let threads = forge_pool_threads(plan.tags_to_create.len(), crate::concurrency::max_jobs());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build release thread pool: {e}"))
        .error_code(error_code::MONOREPO_PUSH_FAILED)?;

    let outcomes: Vec<(String, String, TagReleaseOutcome)> = pool.install(|| {
        plan.tags_to_create
            .par_iter()
            .map(|t| {
                let outcome = process_release_tag(forge, &t.tag, &t.body, t.is_prerelease, draft);
                (t.tag.clone(), t.package.clone(), outcome)
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
            if let (Some(id), Some(cp)) = (result.id, plan.checkpoint.as_mut()) {
                cp.forge_releases
                    .push(crate::monorepo::run::checkpoint::RecordedRelease {
                        tag: tag_name.clone(),
                        id,
                    });
            }
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

    match forge.create_release(tag_name, body, is_pre, draft) {
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
        run_publishers_for_package(plan, pkg, &ctx.package, &ctx.new_version, &ctx.tag)?;
    }
    Ok(())
}

fn run_publishers_for_package(
    plan: &mut ReleasePlan<'_>,
    pkg: &crate::config::PackageConfig,
    package_name: &str,
    new_version: &str,
    tag: &str,
) -> Result<()> {
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

    let mut published: Vec<(&'static str, bool)> = Vec::new();
    let outcome = crate::publishers::run_all(&pkg.publishers, &pub_ctx, &mut published);

    // Recorded whether or not the batch succeeded: a publisher that already
    // pushed to crates.io is a fact rollback has to respect even when a later
    // one in the same package failed.
    if !plan.dry_run
        && let Some(cp) = plan.checkpoint.as_mut()
    {
        for (kind, immutable) in published {
            cp.published
                .push(crate::monorepo::run::checkpoint::RecordedPublish {
                    package: package_name.to_string(),
                    kind: kind.to_string(),
                    immutable,
                });
        }
        cp.save(plan.root)?;
    }

    outcome
}

pub(super) fn print_dry_run_hooks(plan: &mut ReleasePlan<'_>) -> Result<()> {
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

        fn find_open_pr(&self, _head: &str, _base: &str) -> Result<Option<u64>> {
            unreachable!("not exercised by release-tag tests")
        }

        fn update_merge_request(
            &self,
            _id: u64,
            _title: &str,
            _body: &str,
        ) -> Result<MergeRequestResult> {
            unreachable!("not exercised by release-tag tests")
        }
    }

    #[test]
    fn release_branch_name_is_stable_and_target_scoped() {
        assert_eq!(release_branch_name("main"), "ferrflow/release-main");
        assert_eq!(release_branch_name("main"), release_branch_name("main"));
        assert_eq!(
            release_branch_name("release/beta"),
            "ferrflow/release-release-beta"
        );
    }

    #[test]
    fn publishes_existing_draft_and_skips_create() {
        let forge = MockForge {
            drafts: HashMap::from([("v1".to_string(), 42)]),
            ..Default::default()
        };
        let outcome = process_release_tag(&forge, "v1", "body", false, false);

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
        let outcome = process_release_tag(&forge, "v1", "body", false, false);

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
        let outcome = process_release_tag(&forge, "v1", "body", false, false);

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
        let outcome = process_release_tag(&forge, "v1", "body", false, false);

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
        let outcome = process_release_tag(&forge, "v1", "body", false, false);

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
        let outcome = process_release_tag(&forge, "v1", "body", false, true);

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
                    let outcome = process_release_tag(&forge, t, "body", false, false);
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
        assert_eq!(release_url_for_tag(&results, "web@v2.0.0"), None);
        assert_eq!(release_url_for_tag(&results, "cli@v3.0.0"), None);
    }
}
