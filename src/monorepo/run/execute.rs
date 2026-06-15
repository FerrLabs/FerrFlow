use anyhow::Result;
use colored::Colorize;
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
use crate::telemetry;
use crate::versioning::truncate_version;

use super::checkpoint::{Checkpoint, Phase};
use super::summary::{TagToCreate, write_github_step_summary};
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
            eprintln!("  ↻ Resumed: skipping commit (already done)");
        }
        if !checkpoint_is_done(plan, Phase::TagsCreated) {
            create_release_tags(plan)?;
            create_and_move_floating_tags(plan, &mut floating_tag_names)?;
            checkpoint_advance(plan, Phase::TagsCreated)?;
        } else if plan.verbose {
            eprintln!("  ↻ Resumed: skipping tag creation (already done)");
        }
    }

    run_pre_publish_hooks(plan)?;

    if !plan.dry_run {
        if !checkpoint_is_done(plan, Phase::ReleasesCreated) {
            push_and_publish(plan, mode, &floating_tag_names)?;
            checkpoint_advance(plan, Phase::ReleasesCreated)?;
        } else if plan.verbose {
            eprintln!("  ↻ Resumed: skipping push + publish (already done)");
        }
    }

    emit_release_telemetry(plan);
    if !checkpoint_is_done(plan, Phase::PostPublishDone) {
        run_post_publish_hooks(plan)?;
        checkpoint_advance(plan, Phase::PostPublishDone)?;
    } else if plan.verbose {
        eprintln!("  ↻ Resumed: skipping post-publish hooks (already done)");
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
                        if plan.config.workspace.auto_merge_releases {
                            match forge_instance.enable_auto_merge(&mr) {
                                Ok(()) => {
                                    plan.shared_outputs.push("✓ Auto-merge enabled".to_string())
                                }
                                Err(err) => eprintln!(
                                    "{}",
                                    format!("  Warning: failed to enable auto-merge: {err}")
                                        .yellow()
                                ),
                            }
                        }
                    }
                    Err(err) => eprintln!(
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
                    eprintln!(
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

fn run_pre_publish_hooks(plan: &mut ReleasePlan<'_>) -> Result<()> {
    for (ctx, pkg_idx) in plan.hook_contexts {
        let pkg = &plan.config.packages[*pkg_idx];
        let ws_hooks = plan.config.workspace.hooks.as_ref();
        let pkg_hooks = pkg.hooks.as_ref();
        let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
        if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PrePublish) {
            run_hook(
                HookPoint::PrePublish,
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
        for (tag_name, _, body, pkg_name, _, _, is_pre) in plan.tags_to_create {
            if !plan.draft {
                match forge_instance.find_draft_release(tag_name) {
                    Ok(Some(release_id)) => match forge_instance.publish_release(release_id) {
                        Ok(()) => {
                            if let Some((_, lines)) = plan
                                .pkg_outputs
                                .iter_mut()
                                .rev()
                                .find(|(n, _)| n == pkg_name)
                            {
                                lines.push(format!(
                                    "  ✓ Published draft {} {}",
                                    forge_instance.release_noun(),
                                    tag_name.cyan()
                                ));
                            }
                            continue;
                        }
                        Err(err) => eprintln!(
                            "{}",
                            format!("  Warning: failed to publish draft for {tag_name}: {err}")
                                .yellow()
                        ),
                    },
                    Ok(None) => {}
                    Err(err) => {
                        if plan.verbose {
                            eprintln!(
                                "{}",
                                format!(
                                    "  Warning: failed to check for draft release {tag_name}: {err}"
                                )
                                .yellow()
                            );
                        }
                    }
                }
            }

            match forge_instance.create_release(
                tag_name,
                body,
                *is_pre,
                plan.draft,
                target_sha.as_deref(),
            ) {
                Ok(()) => {
                    if let Some((_, lines)) = plan
                        .pkg_outputs
                        .iter_mut()
                        .rev()
                        .find(|(n, _)| n == pkg_name)
                    {
                        let noun = forge_instance.release_noun();
                        if plan.draft {
                            lines.push(format!("  ✓ Draft {} {}", noun, tag_name.cyan()));
                        } else {
                            lines.push(format!("  ✓ {} {}", noun, tag_name.cyan()));
                        }
                    }
                }
                Err(err) => eprintln!(
                    "{}",
                    format!(
                        "  Warning: failed to create {} for {tag_name}: {err}",
                        forge_instance.release_noun()
                    )
                    .yellow()
                ),
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

fn emit_release_telemetry(plan: &ReleasePlan<'_>) {
    if !plan.config.workspace.anonymous_telemetry {
        return;
    }
    for (_, _, _, pkg_name, version, commit_count, _) in plan.tags_to_create {
        telemetry::send_event(
            telemetry::EventType::Release,
            None,
            Some(*commit_count),
            Some(pkg_name.clone()),
            Some(version.clone()),
        );
    }
}

fn run_post_publish_hooks(plan: &mut ReleasePlan<'_>) -> Result<()> {
    for (ctx, pkg_idx) in plan.hook_contexts {
        let pkg = &plan.config.packages[*pkg_idx];
        let ws_hooks = plan.config.workspace.hooks.as_ref();
        let pkg_hooks = pkg.hooks.as_ref();
        let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
        if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PostPublish) {
            run_hook(
                HookPoint::PostPublish,
                &cmd,
                ctx,
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
    if pkg.publishers.is_empty() {
        return Ok(());
    }
    println!(
        "  {} {} publishers:",
        "→".cyan(),
        pkg.publishers.len().to_string().cyan()
    );
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
    for p in &pkg.publishers {
        let kind = p.kind_name();
        let preview = p.describe(package_name, new_version);
        match crate::publishers::run(p, &pub_ctx) {
            Ok(crate::publishers::PublishOutcome::Published { url }) => {
                let suffix = url.as_deref().unwrap_or("");
                println!("    [{kind}] {preview} → {} {suffix}", "published".green());
            }
            Ok(crate::publishers::PublishOutcome::Skipped { reason }) => {
                println!("    [{kind}] {preview} → {} ({reason})", "skipped".yellow());
            }
            Ok(crate::publishers::PublishOutcome::DryRun) => {
                println!("    [{kind}] {preview} {}", "(dry-run)".dimmed());
            }
            Err(e) => {
                eprintln!("    [{kind}] {} {e:#}", "ERROR".red());
                return Err(e);
            }
        }
    }
    Ok(())
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
