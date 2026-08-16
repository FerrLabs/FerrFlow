use anyhow::Result;
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::changelog::{
    ChangelogRender, GitLog, build_section_with, compute_changelog_update, update_changelog_with,
};
use crate::config::{Config, OnFailure, PackageConfig};
use crate::conventional_commits::BumpType;
use crate::diff::unified_diff;
use crate::formats::{get_handler, render_new_version, write_version};
use crate::git::{collect_all_tags, fetch_tags, get_changed_files, open_repo, tag_exists};
use crate::hooks::{
    HookCommit, HookContext, HookFile, HookPackage, HookPoint, resolve_hook, resolve_on_failure,
    run_hook,
};
use crate::prerelease::PrereleaseContext;
use crate::timing::Timing;
use crate::versioning::truncate_version;

use super::types::{CheckCommit, CheckPackage, CheckResult, RunOutput};
use super::util::{auto_stage_new_files, collect_dirty_files};

mod cascade;
pub(super) mod checkpoint;
mod commit_body;
mod drafts;
mod execute;
#[cfg(test)]
mod execute_tests;
mod forced;
mod graph;
mod groups;
mod lock;
mod plan;
mod release_json;
mod summary;
mod why;
use checkpoint::Checkpoint;
use drafts::publish_pending_drafts;
use execute::{ReleasePlan, execute_release, print_dry_run_hooks};
use forced::{Forced, parse_forced_version};
use plan::{PackagePlan, PlanInputs, SkipReason, compute_plan};
use rayon::prelude::*;
use release_json::{GitInfo, ReleaseJson, ReleasedPackage, SkippedPackage};
use summary::{PlannedTag, collect_outputs};
pub use why::why;

#[allow(clippy::too_many_arguments)]
pub(super) fn run_release_logic(
    root: &Path,
    config: &Config,
    dry_run: bool,
    verbose: bool,
    json: bool,
    release_json: bool,
    force: bool,
    force_version: Option<&str>,
    channel: Option<&str>,
    draft: bool,
    force_unlock: bool,
    timing: &mut Timing,
) -> Result<Option<RunOutput>> {
    if config.packages.is_empty() {
        if release_json {
            let out = RunOutput {
                json: Some(
                    ReleaseJson {
                        dry_run,
                        ..Default::default()
                    }
                    .to_json()?,
                ),
                text_lines: Vec::new(),
            };
            return finish(dry_run, out);
        }
        if json {
            let out = RunOutput {
                json: Some(serde_json::to_string(&CheckResult { packages: vec![] })?),
                text_lines: Vec::new(),
            };
            return finish(dry_run, out);
        }
        let out = RunOutput {
            json: None,
            text_lines: vec![
                "No packages configured. Run `ferrflow init` to create a ferrflow config."
                    .yellow()
                    .to_string(),
            ],
        };
        return finish(dry_run, out);
    }

    if let Err(errors) = config.validate_groups() {
        return Err(anyhow::anyhow!(
            "invalid linked/fixed groups:\n  - {}",
            errors.join("\n  - ")
        ));
    }

    let release_order = graph::release_order(&config.packages).map_err(graph::Cycle::into_error)?;

    let repo = open_repo(root)?;

    if dry_run {
        timing.skip("fetch_tags", "dry-run");
    }

    let _release_lock = if dry_run {
        None
    } else if force_unlock {
        Some(lock::ReleaseLock::acquire_force(root)?)
    } else {
        Some(lock::ReleaseLock::acquire(root)?)
    };

    if !dry_run {
        let start = std::time::Instant::now();
        let fetch = fetch_tags(&repo, &config.workspace.remote);
        timing.record("fetch_tags", start.elapsed());
        if let Err(e) = fetch
            && verbose
        {
            tracing::warn!("Warning: could not fetch remote tags: {e}");
        }
        crate::git::write_commit_graph_if_absent(&repo);
    }

    let current_branch = crate::git::resolve_current_branch(&repo, &config.workspace.branch);

    let forge_base = config.workspace.changelog.as_ref().and_then(|cl| {
        if cl.include_commit_links || cl.include_compare_link {
            crate::git::get_remote_url(&repo, &config.workspace.remote)
                .as_deref()
                .and_then(crate::forge::web_base_url)
        } else {
            None
        }
    });

    let prerelease_ctx = PrereleaseContext::resolve(
        channel,
        &current_branch,
        config.workspace.branches.as_deref(),
    )?;

    let short_hash = repo
        .head_id()
        .ok()
        .map(|id| id.to_string()[..7].to_string())
        .unwrap_or_default();

    let all_tags = collect_all_tags(&repo);
    let tag_index = timing.stage("build TagIndex", || crate::git::TagIndex::build(&repo).ok());
    let fallback_ancestors = match &tag_index {
        Some(_) => None,
        None => crate::git::build_head_ancestors(&repo).ok(),
    };

    let target_branch = if prerelease_ctx.is_prerelease() {
        current_branch.clone()
    } else {
        config.workspace.branch.clone()
    };

    let changed_files = get_changed_files(&repo)?;

    let quiet = json || release_json;

    if !quiet && !changed_files.is_empty() {
        tracing::debug!("Changed files in last commit:");
        for f in &changed_files {
            tracing::debug!("  {}", f.dimmed());
        }
        tracing::debug!("");
    }

    let mut any_bumped = false;
    let mut json_packages: Vec<CheckPackage> = Vec::new();
    let mut released: Vec<ReleasedPackage> = Vec::new();
    let mut skipped: Vec<SkippedPackage> = Vec::new();
    let mut files_to_commit: Vec<String> = Vec::new();
    let mut files_per_package: HashMap<String, Vec<String>> = HashMap::new();
    let mut tags_to_create: Vec<PlannedTag> = Vec::new();
    let mut hook_contexts: Vec<(HookContext, usize)> = Vec::new(); // (ctx, pkg_index)
    let mut bumped: HashMap<String, BumpType> = HashMap::new();
    let mut bumped_versions: HashMap<String, String> = HashMap::new();

    let mut pkg_outputs: Vec<(String, Vec<String>)> = Vec::new();
    let mut shared_outputs: Vec<String> = Vec::new();

    let forced: Option<Forced<'_>> = parse_forced_version(force_version, config.is_monorepo())?;

    let compute_start = std::time::Instant::now();

    let thread_safe_repo = repo.clone().into_sync();
    let changed_files_cache = plan::ChangedFilesCache::default();
    let commit_walk =
        crate::git::CommitWalkCache::new(config.workspace.effective_commit_skip_markers());
    let plan_inputs = PlanInputs {
        config,
        root,
        tag_index: tag_index.as_ref(),
        head_ancestors: tag_index
            .as_ref()
            .map(|idx| &idx.ancestors)
            .or(fallback_ancestors.as_ref()),
        all_tags: &all_tags,
        prerelease_ctx: &prerelease_ctx,
        forced: &forced,
        changed_files: &changed_files,
        short_hash: &short_hash,
        changed_files_cache: &changed_files_cache,
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
    groups::apply_groups(config, root, &mut plans);

    let all_packages = batch_package_snapshot(&release_order, &plans, &config.packages);

    for &pkg_idx in &release_order {
        let pkg = &config.packages[pkg_idx];
        let plan = plans[pkg_idx]
            .take()
            .expect("release_order visits each package exactly once");
        let recovered = match &plan {
            PackagePlan::Skipped { recovered, .. } => *recovered,
            PackagePlan::Bump(bump_plan) => bump_plan.recovered,
        };
        if recovered && !quiet {
            tracing::debug!(
                "{} {} — recovering missed release",
                "↻".cyan(),
                pkg.name.cyan()
            );
        }

        let bump_plan = match plan {
            PackagePlan::Skipped { reason, .. } => {
                match reason {
                    SkipReason::NotTouched => {
                        if !quiet {
                            tracing::debug!(
                                "{} {} — not touched, skipping",
                                "○".dimmed(),
                                pkg.name.dimmed()
                            );
                        }
                    }
                    SkipReason::NoNewCommits => {
                        if !quiet {
                            tracing::debug!(
                                "{} {} — no new commits",
                                "○".dimmed(),
                                pkg.name.dimmed()
                            );
                        }
                    }
                    SkipReason::NoReleasableCommits => {
                        if !quiet {
                            shared_outputs.push(format!(
                                "{} {} — no releasable commits",
                                "○".dimmed(),
                                pkg.name.dimmed()
                            ));
                        }
                    }
                    SkipReason::VersionUnchanged => {
                        if !quiet {
                            tracing::debug!(
                                "{} {} — version unchanged",
                                "○".dimmed(),
                                pkg.name.dimmed()
                            );
                        }
                    }
                }
                if release_json {
                    skipped.push(SkippedPackage {
                        package: pkg.name.clone(),
                        reason: reason.json_label().to_string(),
                    });
                }
                continue;
            }
            PackagePlan::Bump(bump_plan) => *bump_plan,
        };

        let current_version = bump_plan.current_version;
        let new_version = bump_plan.new_version;
        let is_prerelease = bump_plan.is_prerelease;
        let last_tag = bump_plan.last_tag;
        let commits = bump_plan.commits;
        let bump = bump_plan.bump;
        let strategy_label = bump_plan.strategy_label;
        let tag = bump_plan.tag;

        if release_json {
            released.push(ReleasedPackage {
                package: pkg.name.clone(),
                previous_version: current_version.clone(),
                new_version: new_version.clone(),
                bump_type: strategy_label.clone(),
                tag: tag.clone(),
                commit_count: commits.len(),
                prerelease: is_prerelease,
                forge_release_url: None,
                forge_release_id: None,
            });
        }

        if dry_run && verbose && !quiet {
            let changelog_render = ChangelogRender {
                formats: Some(&config.workspace.commit_formats),
                config: config.workspace.changelog.as_ref(),
                forge_base: forge_base.clone(),
                last_tag: last_tag.clone(),
                new_tag: Some(tag.clone()),
            };
            emit_dry_run_diffs(pkg, root, &new_version, &commits, bump, &changelog_render);
        }

        if json {
            let check_commits: Vec<CheckCommit> = commits
                .iter()
                .filter_map(|c| {
                    c.message.lines().next().map(|first_line| CheckCommit {
                        hash: c.hash.clone(),
                        message: first_line.to_string(),
                    })
                })
                .collect();
            json_packages.push(CheckPackage {
                name: pkg.name.clone(),
                current_version: current_version.clone(),
                next_version: new_version.clone(),
                bump_type: strategy_label.clone(),
                tag: tag.clone(),
                channel: prerelease_ctx.channel.clone(),
                prerelease: is_prerelease,
                commits: check_commits,
            });
        } else {
            let channel_label = if is_prerelease {
                format!(" [{}]", prerelease_ctx.channel.as_deref().unwrap_or("pre"))
            } else {
                String::new()
            };
            let mut lines = vec![format!(
                "{} {}  {} → {}  ({}{})",
                "●".green().bold(),
                pkg.name.bold(),
                current_version.dimmed(),
                new_version.green().bold(),
                strategy_label.cyan(),
                channel_label.yellow()
            )];

            if verbose {
                for c in &commits {
                    if let Some(line) = c.message.lines().next() {
                        lines.push(format!("    {} {}", c.hash.dimmed(), line.dimmed()));
                    }
                }
            }

            if !is_prerelease {
                let levels = pkg.effective_floating_tags(&config.workspace);
                for level in levels {
                    if let Some(truncated) = truncate_version(&new_version, *level) {
                        let float_tag = pkg.tag_for_version(
                            &config.workspace,
                            config.is_monorepo(),
                            &truncated,
                        );
                        let verb = if tag_exists(&repo, &float_tag) {
                            "move"
                        } else {
                            "create"
                        };
                        lines.push(format!(
                            "    {} floating tag {}",
                            format!("→ {verb}").dimmed(),
                            float_tag.cyan()
                        ));
                    }
                }
            }

            pkg_outputs.push((pkg.name.clone(), lines));
        }

        let hook_commits: Vec<HookCommit> = commits
            .iter()
            .map(|c| HookCommit::from_commit(&c.hash, &c.message, &config.workspace.commit_formats))
            .collect();
        let hook_bumped_files: Vec<HookFile> = pkg
            .versioned_files
            .iter()
            .filter(|vf| get_handler(&vf.format).modifies_file())
            .map(|vf| HookFile {
                path: vf.path.clone(),
                format: serde_json::to_value(&vf.format)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default(),
            })
            .collect();

        let mut hook_ctx = HookContext {
            package: pkg.name.clone(),
            old_version: current_version.clone(),
            new_version: new_version.clone(),
            bump_type: bump.to_string(),
            tag: tag.clone(),
            dry_run,
            package_path: root
                .join(pkg.path.trim_start_matches("./"))
                .to_string_lossy()
                .into_owned(),
            channel: prerelease_ctx.channel.clone(),
            error_code: None,
            monorepo: config.is_monorepo(),
            is_prerelease,
            changelog: String::new(),
            commits: hook_commits,
            bumped_files: hook_bumped_files,
            all_packages: all_packages.clone(),
            release_url: None,
        };

        let ws_hooks = config.workspace.hooks.as_ref();
        let pkg_hooks = pkg.hooks.as_ref();
        let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);

        if dry_run {
            if !json {
                for point in [HookPoint::PreBump, HookPoint::PostBump] {
                    if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, point) {
                        run_hook(point, &cmd, &hook_ctx, on_failure, true, verbose, root)?;
                    }
                }
            }
        } else {
            if crate::git::tag_exists(&repo, &tag) {
                if let Some((_, lines)) = pkg_outputs.iter_mut().rev().find(|(n, _)| n == &pkg.name)
                {
                    lines.push(format!(
                        "  {} {} — tag {} already exists, skipping",
                        "○".dimmed(),
                        pkg.name.dimmed(),
                        tag.cyan()
                    ));
                }
                continue;
            }

            if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PreBump) {
                run_hook(
                    HookPoint::PreBump,
                    &cmd,
                    &hook_ctx,
                    on_failure,
                    false,
                    verbose,
                    root,
                )?;
            }

            for vf in &pkg.versioned_files {
                write_version(vf, root, &new_version)?;
                if get_handler(&vf.format).modifies_file() {
                    if let Some((_, lines)) =
                        pkg_outputs.iter_mut().rev().find(|(n, _)| n == &pkg.name)
                    {
                        lines.push(format!("  ✓ Updated {}", vf.path));
                    }
                    files_to_commit.push(vf.path.clone());
                    files_per_package
                        .entry(pkg.name.clone())
                        .or_default()
                        .push(vf.path.clone());
                }
            }

            if pkg.effective_update_lockfiles(&config.workspace) {
                refresh_lockfiles(
                    pkg,
                    root,
                    &mut files_to_commit,
                    files_per_package.entry(pkg.name.clone()).or_default(),
                );
            }

            let changelog_render = ChangelogRender {
                formats: Some(&config.workspace.commit_formats),
                config: config.workspace.changelog.as_ref(),
                forge_base: forge_base.clone(),
                last_tag: last_tag.clone(),
                new_tag: Some(tag.clone()),
            };

            let body = build_section_with(&new_version, &commits, &changelog_render);
            hook_ctx.changelog = body.clone();

            if let Some(changelog_rel) = &pkg.changelog {
                let changelog_path = root.join(changelog_rel);
                update_changelog_with(
                    &changelog_path,
                    &pkg.name,
                    &new_version,
                    &commits,
                    bump,
                    false,
                    &changelog_render,
                )?;
                files_to_commit.push(changelog_rel.clone());
                files_per_package
                    .entry(pkg.name.clone())
                    .or_default()
                    .push(changelog_rel.clone());
            }

            if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PostBump) {
                let before = collect_dirty_files(&repo);
                run_hook(
                    HookPoint::PostBump,
                    &cmd,
                    &hook_ctx,
                    on_failure,
                    false,
                    verbose,
                    root,
                )?;
                let len_before = files_to_commit.len();
                auto_stage_new_files(&repo, &before, &mut files_to_commit);
                let pkg_files = files_per_package.entry(pkg.name.clone()).or_default();
                for f in &files_to_commit[len_before..] {
                    pkg_files.push(f.clone());
                }
            }

            tags_to_create.push(PlannedTag {
                tag: tag.clone(),
                message: format!("Release {tag}"),
                body,
                package: pkg.name.clone(),
                version: new_version.clone(),
                commit_count: commits.len() as i32,
                is_prerelease,
            });
        }

        hook_contexts.push((hook_ctx, pkg_idx));
        bumped.insert(pkg.name.clone(), bump);
        bumped_versions.insert(pkg.name.clone(), new_version.clone());
        any_bumped = true;
    }

    if config.is_monorepo() {
        let mut sink = cascade::CascadeSink {
            any_bumped: &mut any_bumped,
            json_packages: &mut json_packages,
            released: &mut released,
            files_to_commit: &mut files_to_commit,
            files_per_package: &mut files_per_package,
            tags_to_create: &mut tags_to_create,
            pkg_outputs: &mut pkg_outputs,
            bumped: &mut bumped,
            bumped_versions: &mut bumped_versions,
        };
        cascade::run_dependency_cascade(
            config,
            root,
            &all_tags,
            prerelease_ctx.channel.as_deref(),
            json,
            release_json,
            dry_run,
            &mut sink,
        )?;
    }

    if config.workspace.update_dependents {
        let rewritten = cascade::update_dependent_manifests(
            config,
            root,
            &bumped_versions,
            dry_run,
            &mut files_to_commit,
            &mut files_per_package,
        )?;
        for line in rewritten {
            shared_outputs.push(line);
        }
    }

    timing.record("per-package compute", compute_start.elapsed());

    if json {
        let out = RunOutput {
            json: Some(serde_json::to_string(&CheckResult {
                packages: json_packages,
            })?),
            text_lines: Vec::new(),
        };
        return finish(dry_run, out);
    }

    let mut forge_results: Vec<(String, crate::forge::ReleaseResult)> = Vec::new();

    if any_bumped && !tags_to_create.is_empty() {
        if !dry_run && let Some(manifest_rel) = config.workspace.manifest_file.as_deref() {
            let overrides: std::collections::BTreeMap<String, String> = tags_to_create
                .iter()
                .map(|t| (t.package.clone(), t.version.clone()))
                .collect();
            let packages = crate::manifest::snapshot_with_overrides(config, root, &overrides);
            let commit = repo
                .head_id()
                .ok()
                .map(|id| id.to_string()[..7.min(id.to_string().len())].to_string())
                .unwrap_or_default();
            let manifest = crate::manifest::Manifest::new(
                packages,
                crate::manifest::now_utc_iso8601(),
                commit,
            );
            let manifest_path = root.join(manifest_rel);
            crate::manifest::write_atomic(&manifest_path, &manifest)?;
            files_to_commit.push(manifest_rel.to_string());
        }

        let head_sha = repo
            .head_id()
            .ok()
            .map(|id| id.to_string())
            .unwrap_or_default();
        let tag_names: Vec<String> = tags_to_create.iter().map(|t| t.tag.clone()).collect();
        let mut checkpoint = if dry_run {
            None
        } else {
            match Checkpoint::load(root)? {
                Some(existing) if existing.head_sha == head_sha => {
                    if verbose {
                        tracing::info!(
                            "  ↻ Found in-progress release checkpoint at phase {:?}; resuming",
                            existing.phase
                        );
                    }
                    Some(existing)
                }
                Some(existing) => {
                    return Err(anyhow::anyhow!(
                        "found a stale release checkpoint at .git/ferrflow.checkpoint.json \
                         pinned to commit {} but HEAD is now {}.\n  \
                         Either reset HEAD back to {} and rerun to resume the previous release, \
                         or delete .git/ferrflow.checkpoint.json to start fresh.",
                        &existing.head_sha[..8.min(existing.head_sha.len())],
                        &head_sha[..8.min(head_sha.len())],
                        &existing.head_sha[..8.min(existing.head_sha.len())]
                    ));
                }
                None => Some(Checkpoint::new(head_sha, tag_names)),
            }
        };

        let mut plan = ReleasePlan {
            repo: &repo,
            config,
            root,
            target_branch: &target_branch,
            dry_run,
            verbose,
            force,
            draft,
            tags_to_create: &tags_to_create,
            hook_contexts: &hook_contexts,
            files_to_commit: &mut files_to_commit,
            files_per_package: &mut files_per_package,
            pkg_outputs: &mut pkg_outputs,
            shared_outputs: &mut shared_outputs,
            forge_results: &mut forge_results,
            checkpoint: checkpoint.as_mut(),
            forge: None,
        };
        let release_start = std::time::Instant::now();
        let release_result = execute_release(&mut plan);
        timing.record("release commit phase", release_start.elapsed());

        let released_tags: Vec<String> = tags_to_create.iter().map(|t| t.tag.clone()).collect();
        let ws_hooks = config.workspace.hooks.as_ref();
        match release_result {
            Ok(()) => {
                if !dry_run {
                    Checkpoint::delete(root)?;
                }
                if let Some(cmd) = resolve_hook(None, ws_hooks, HookPoint::OnSuccess) {
                    let mut ctx = HookContext::release_summary(
                        root,
                        &released_tags,
                        dry_run,
                        config.is_monorepo(),
                    );
                    ctx.all_packages = all_packages.clone();
                    let on_failure = resolve_on_failure(None, ws_hooks);
                    run_hook(
                        HookPoint::OnSuccess,
                        &cmd,
                        &ctx,
                        on_failure,
                        dry_run,
                        verbose,
                        root,
                    )?;
                }
            }
            Err(err) => {
                if let Some(cmd) = resolve_hook(None, ws_hooks, HookPoint::OnError) {
                    let mut ctx = HookContext::release_summary(
                        root,
                        &released_tags,
                        dry_run,
                        config.is_monorepo(),
                    );
                    ctx.all_packages = all_packages.clone();
                    ctx.error_code = crate::error_code::code_from_error(&err);
                    let _ = run_hook(
                        HookPoint::OnError,
                        &cmd,
                        &ctx,
                        OnFailure::Continue,
                        dry_run,
                        verbose,
                        root,
                    );
                }
                return Err(err);
            }
        }
    } else if dry_run && any_bumped {
        let plan = ReleasePlan {
            repo: &repo,
            config,
            root,
            target_branch: &target_branch,
            dry_run,
            verbose,
            force,
            draft,
            tags_to_create: &tags_to_create,
            hook_contexts: &hook_contexts,
            files_to_commit: &mut files_to_commit,
            files_per_package: &mut files_per_package,
            pkg_outputs: &mut pkg_outputs,
            shared_outputs: &mut shared_outputs,
            forge_results: &mut forge_results,
            checkpoint: None,
            forge: None,
        };
        print_dry_run_hooks(&plan)?;
        timing.skip("release commit phase", "dry-run");
    }

    if !any_bumped && !draft && !dry_run {
        publish_pending_drafts(&repo, config, root, verbose, &mut shared_outputs)?;
    }

    if release_json {
        for (tag, result) in &forge_results {
            if let Some(rp) = released.iter_mut().find(|rp| &rp.tag == tag) {
                rp.forge_release_url = result.url.clone();
                rp.forge_release_id = result.id;
            }
        }
        let commit = repo
            .head_id()
            .ok()
            .map(|id| id.to_string()[..7.min(id.to_string().len())].to_string())
            .unwrap_or_default();
        let tags_pushed = if dry_run {
            Vec::new()
        } else {
            tags_to_create.iter().map(|t| t.tag.clone()).collect()
        };
        let payload = ReleaseJson {
            released,
            skipped,
            git: GitInfo {
                commit,
                tags_pushed,
                branch: target_branch.clone(),
            },
            dry_run,
        };
        return finish(
            dry_run,
            RunOutput {
                json: Some(payload.to_json()?),
                text_lines: Vec::new(),
            },
        );
    }

    let mut text_lines = collect_outputs(&pkg_outputs, &shared_outputs);
    if !any_bumped && !verbose {
        text_lines.push("Nothing to release.".dimmed().to_string());
    }

    finish(
        dry_run,
        RunOutput {
            json: None,
            text_lines,
        },
    )
}

fn batch_package_snapshot(
    release_order: &[usize],
    plans: &[Option<PackagePlan>],
    packages: &[PackageConfig],
) -> Vec<HookPackage> {
    release_order
        .iter()
        .filter_map(|&idx| match plans[idx].as_ref() {
            Some(PackagePlan::Bump(bump)) => Some(HookPackage {
                name: packages[idx].name.clone(),
                version: bump.new_version.clone(),
                bump: bump.bump.to_string(),
            }),
            _ => None,
        })
        .collect()
}

fn emit_dry_run_diffs(
    pkg: &PackageConfig,
    root: &Path,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    changelog_render: &ChangelogRender,
) {
    for (path, diff) in
        collect_dry_run_diffs(pkg, root, new_version, commits, bump, changelog_render)
    {
        println!("{}", path.bold());
        print!("{diff}");
        println!();
    }
}

pub(super) fn collect_dry_run_diffs(
    pkg: &PackageConfig,
    root: &Path,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    changelog_render: &ChangelogRender,
) -> Vec<(String, String)> {
    let mut diffs = Vec::new();
    for vf in &pkg.versioned_files {
        if !get_handler(&vf.format).modifies_file() {
            continue;
        }
        let (Ok(old), Ok(new)) = (
            std::fs::read_to_string(root.join(&vf.path)),
            render_new_version(vf, root, new_version),
        ) else {
            continue;
        };
        let diff = unified_diff(&old, &new);
        if !diff.is_empty() {
            diffs.push((vf.path.clone(), diff));
        }
    }

    if let Some(changelog_rel) = &pkg.changelog {
        let changelog_path = root.join(changelog_rel);
        if let Ok(Some((old, new))) = compute_changelog_update(
            &changelog_path,
            &pkg.name,
            new_version,
            commits,
            bump,
            changelog_render,
        ) {
            let diff = unified_diff(&old, &new);
            if !diff.is_empty() {
                diffs.push((changelog_rel.clone(), diff));
            }
        }
    }
    diffs
}

fn finish(dry_run: bool, out: RunOutput) -> Result<Option<RunOutput>> {
    if dry_run {
        Ok(Some(out))
    } else {
        out.print();
        Ok(None)
    }
}

pub(super) fn refresh_lockfiles(
    pkg: &PackageConfig,
    root: &Path,
    files_to_commit: &mut Vec<String>,
    pkg_files: &mut Vec<String>,
) {
    use crate::formats::lockfiles::{self, UpdateOutcome};

    let mut handled: HashSet<String> = HashSet::new();
    for vf in &pkg.versioned_files {
        let outcome = match lockfiles::update_for_manifest(root, &vf.path) {
            Ok(outcome) => outcome,
            Err(err) => {
                tracing::warn!(package = %pkg.name, manifest = %vf.path, error = %err, "lockfile update skipped");
                continue;
            }
        };
        match outcome {
            UpdateOutcome::Updated { lockfile_rel } => {
                if handled.insert(lockfile_rel.clone()) {
                    tracing::info!(package = %pkg.name, lockfile = %lockfile_rel, "refreshed lockfile");
                    files_to_commit.push(lockfile_rel.clone());
                    pkg_files.push(lockfile_rel);
                }
            }
            UpdateOutcome::NotOnPath { program } => {
                tracing::warn!(package = %pkg.name, program = %program, "package manager not on PATH; lockfile left stale");
            }
            UpdateOutcome::Failed { program, detail } => {
                tracing::warn!(package = %pkg.name, program = %program, detail = %detail, "lockfile update failed; lockfile left stale");
            }
            UpdateOutcome::NoLockfile | UpdateOutcome::UnsupportedManifest => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conventional_commits::BumpType;
    use plan::PackageBump;

    fn pkg(name: &str) -> PackageConfig {
        serde_json::from_str(&format!(r#"{{"name":"{name}","path":"{name}"}}"#)).unwrap()
    }

    fn bump_plan(new_version: &str, bump: BumpType) -> PackagePlan {
        PackagePlan::Bump(Box::new(PackageBump {
            recovered: false,
            current_version: "1.0.0".to_string(),
            new_version: new_version.to_string(),
            is_prerelease: false,
            last_tag: None,
            commits: Vec::new(),
            bump,
            strategy_label: bump.to_string(),
            tag: format!("v{new_version}"),
        }))
    }

    #[test]
    fn batch_snapshot_lists_only_bumped_packages_in_release_order() {
        let packages = vec![pkg("api"), pkg("web"), pkg("cli")];
        let plans = vec![
            Some(bump_plan("2.0.0", BumpType::Major)),
            Some(PackagePlan::Skipped {
                reason: SkipReason::NotTouched,
                recovered: false,
            }),
            Some(bump_plan("1.4.0", BumpType::Minor)),
        ];

        let snapshot = batch_package_snapshot(&[2, 0, 1], &plans, &packages);

        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].name, "cli");
        assert_eq!(snapshot[0].version, "1.4.0");
        assert_eq!(snapshot[0].bump, "minor");
        assert_eq!(snapshot[1].name, "api");
        assert_eq!(snapshot[1].version, "2.0.0");
        assert_eq!(snapshot[1].bump, "major");
        assert!(!snapshot.iter().any(|p| p.name == "web"));
    }
}
