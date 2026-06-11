use anyhow::Result;
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::changelog::{build_section, update_changelog};
use crate::config::{Config, OrphanedTagStrategy, VersioningStrategy};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::{get_handler, read_version, write_version};
use crate::git::{
    collect_all_tags, fetch_tags, get_changed_files, get_changed_files_since_oid,
    get_changed_files_since_tag, get_commits_since_last_stable_tag, get_commits_since_last_tag,
    get_commits_since_oid, open_repo, tag_exists,
};
use crate::hooks::{HookContext, HookPoint, resolve_hook, resolve_on_failure, run_hook};
use crate::prerelease::PrereleaseContext;
use crate::telemetry;
use crate::versioning::{compute_next_version, truncate_version};

use super::types::{CheckCommit, CheckPackage, CheckResult};
use super::util::{
    auto_stage_new_files, collect_dirty_files, is_package_touched, pick_higher_semver,
    tags_for_package,
};

mod cascade;
mod checkpoint;
mod drafts;
mod execute;
mod forced;
mod lock;
mod summary;
use checkpoint::Checkpoint;
use drafts::publish_pending_drafts;
use execute::{ReleasePlan, execute_release, print_dry_run_hooks};
use forced::{Forced, forced_version_for, parse_forced_version};
use summary::{TagToCreate, print_outputs};

#[allow(clippy::too_many_arguments)]
pub(super) fn run_release_logic(
    root: &Path,
    config: &Config,
    dry_run: bool,
    verbose: bool,
    json: bool,
    force: bool,
    force_version: Option<&str>,
    channel: Option<&str>,
    draft: bool,
    force_unlock: bool,
) -> Result<()> {
    if config.packages.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::to_string(&CheckResult { packages: vec![] })?
            );
            return Ok(());
        }
        println!(
            "{}",
            "No packages configured. Run `ferrflow init` to create a ferrflow config.".yellow()
        );
        return Ok(());
    }

    let repo = open_repo(root)?;

    // Acquire the release lock before any mutating step. Dropped at the
    // end of run_release_logic via RAII. Skipped on dry-run (no writes).
    // The lockfile lives at .git/ferrflow.lock; concurrent invocations
    // get a clear error instead of racing on git refs. See #514.
    let _release_lock = if dry_run {
        None
    } else if force_unlock {
        Some(lock::ReleaseLock::acquire_force(root)?)
    } else {
        Some(lock::ReleaseLock::acquire(root)?)
    };

    if !dry_run
        && let Err(e) = fetch_tags(&repo, &config.workspace.remote)
        && verbose
    {
        eprintln!("Warning: could not fetch remote tags: {e}");
    }

    let current_branch = crate::git::resolve_current_branch(&repo, &config.workspace.branch);

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
    // Build the HEAD ancestor set once so the per-package find_*_tag calls
    // below can skip their per-tag graph_descendant_of walk. On dense
    // monorepos (mono-large: 200 pkg × 10k commits) this is the
    // difference between 1.8 s and a couple hundred ms for the tag-bound
    // commands.
    let head_ancestors = crate::git::build_head_ancestors(&repo).ok();
    // Pre-collect all tags + their commit OIDs in one tag_foreach scan
    // so the per-package find_*_tag / get_*_since_tag calls below don't
    // each repeat that scan. Coupled with the ancestor set, the per-pkg
    // lookups collapse from O(tags) callbacks + O(commits) walk to O(1)
    // hash hits.
    let tag_index = crate::git::TagIndex::build(&repo).ok();

    let target_branch = if prerelease_ctx.is_prerelease() {
        current_branch.clone()
    } else {
        config.workspace.branch.clone()
    };

    let changed_files = get_changed_files(&repo)?;

    if verbose && !json && !changed_files.is_empty() {
        println!("Changed files in last commit:");
        for f in &changed_files {
            println!("  {}", f.dimmed());
        }
        println!();
    }

    let mut any_bumped = false;
    let mut json_packages: Vec<CheckPackage> = Vec::new();
    let mut files_to_commit: Vec<String> = Vec::new();
    let mut files_per_package: HashMap<String, Vec<String>> = HashMap::new();
    let mut tags_to_create: Vec<TagToCreate> = Vec::new();
    let mut hook_contexts: Vec<(HookContext, usize)> = Vec::new(); // (ctx, pkg_index)
    let mut bumped_names: HashSet<String> = HashSet::new();

    let mut pkg_outputs: Vec<(String, Vec<String>)> = Vec::new();
    let mut shared_outputs: Vec<String> = Vec::new();

    let forced: Option<Forced<'_>> = parse_forced_version(force_version, config.is_monorepo())?;

    for (pkg_idx, pkg) in config.packages.iter().enumerate() {
        let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());

        let forced_ver_for_pkg = forced_version_for(&forced, &pkg.name);

        let mut touched = is_package_touched(pkg, &changed_files, config.is_monorepo());

        if !touched && config.workspace.recover_missed_releases && config.is_monorepo() {
            let strategy = config.workspace.orphaned_tag_strategy;
            let files_since_tag =
                if let (Some(idx), OrphanedTagStrategy::Warn) = (tag_index.as_ref(), strategy) {
                    let last_oid = idx.find_last_tag_commit(&tag_search_prefix, strategy);
                    get_changed_files_since_oid(&repo, last_oid)?
                } else {
                    get_changed_files_since_tag(&repo, &tag_search_prefix, strategy)?
                };
            if is_package_touched(pkg, &files_since_tag, true) {
                touched = true;
                if verbose && !json {
                    println!(
                        "{} {} — recovering missed release",
                        "↻".cyan(),
                        pkg.name.cyan()
                    );
                }
            }
        }

        if !touched && forced_ver_for_pkg.is_none() {
            if verbose && !json {
                println!(
                    "{} {} — not touched, skipping",
                    "○".dimmed(),
                    pkg.name.dimmed()
                );
            }
            continue;
        }

        // `versionedFiles` is optional (schema says so, and #531
        // requested tag-only releases for Docker-image / content repos
        // that have no version field to bump). Resolve the file-derived
        // version only when at least one is configured; otherwise let
        // the tag history alone drive the version computation. The
        // file-write loop below is already a `for vf in
        // &pkg.versioned_files` so it naturally no-ops when empty.
        let pkg_strategy = pkg.effective_versioning(
            &config.workspace,
            &tags_for_package(&all_tags, &tag_search_prefix),
        );

        let file_version = pkg
            .versioned_files
            .first()
            .and_then(|vf| read_version(vf, root).ok());
        let strategy = config.workspace.orphaned_tag_strategy;
        // Fast path via the pre-built TagIndex for Warn (default); otherwise
        // fall back to the per-call form that still uses the ancestor cache.
        let tag_version =
            if let (Some(idx), OrphanedTagStrategy::Warn) = (tag_index.as_ref(), strategy) {
                idx.find_highest_semver_tag(&tag_search_prefix, strategy)
                    .map(|(_tag, version)| version)
            } else {
                crate::git::find_highest_semver_tag_with_cache(
                    &repo,
                    &tag_search_prefix,
                    strategy,
                    head_ancestors.as_ref(),
                )?
                .map(|(_tag, version)| version)
            };
        let current_version = match (tag_version, file_version) {
            (Some(tag), Some(file)) => pick_higher_semver(&file, &tag),
            (Some(tag), None) => tag,
            (None, Some(file)) => file,
            (None, None) => crate::versioning::bootstrap_version(pkg_strategy),
        };

        // Helpers that route through TagIndex when the strategy allows it,
        // falling back to the per-call slow path. Returns commits walked
        // back to the last (stable or any) tag for the current package.
        let skip_markers = config.workspace.effective_commit_skip_markers();
        let commits_since_stable = || -> Result<Vec<crate::git::GitLog>> {
            if let (Some(idx), OrphanedTagStrategy::Warn) = (tag_index.as_ref(), strategy) {
                let stop = idx.find_last_stable_tag_commit(&tag_search_prefix, strategy);
                get_commits_since_oid(&repo, stop, &skip_markers)
            } else {
                get_commits_since_last_stable_tag(
                    &repo,
                    &tag_search_prefix,
                    strategy,
                    &skip_markers,
                )
            }
        };
        let commits_since_any = || -> Result<Vec<crate::git::GitLog>> {
            if let (Some(idx), OrphanedTagStrategy::Warn) = (tag_index.as_ref(), strategy) {
                let stop = idx.find_last_tag_commit(&tag_search_prefix, strategy);
                get_commits_since_oid(&repo, stop, &skip_markers)
            } else {
                get_commits_since_last_tag(&repo, &tag_search_prefix, strategy, &skip_markers)
            }
        };

        let (new_version, is_prerelease, commits, bump) = if let Some(fv) = forced_ver_for_pkg {
            let clean = fv.strip_prefix('v').unwrap_or(fv);
            let commits = if !prerelease_ctx.is_prerelease() {
                commits_since_stable().unwrap_or_default()
            } else {
                commits_since_any().unwrap_or_default()
            };
            (clean.to_string(), false, commits, BumpType::None)
        } else {
            let commits = if !prerelease_ctx.is_prerelease() {
                commits_since_stable()?
            } else {
                commits_since_any()?
            };

            if commits.is_empty() {
                if verbose && !json {
                    println!("{} {} — no new commits", "○".dimmed(), pkg.name.dimmed());
                }
                continue;
            }

            let strategy = pkg.effective_versioning(
                &config.workspace,
                &tags_for_package(&all_tags, &tag_search_prefix),
            );

            let bump = commits
                .iter()
                .map(|c| determine_bump(&c.message))
                .max()
                .unwrap_or(BumpType::None);

            let is_date_or_seq = matches!(
                strategy,
                VersioningStrategy::Calver
                    | VersioningStrategy::CalverShort
                    | VersioningStrategy::CalverSeq
                    | VersioningStrategy::Sequential
            );

            if bump == BumpType::None && !is_date_or_seq {
                if !json {
                    println!(
                        "{} {} — no releasable commits",
                        "○".dimmed(),
                        pkg.name.dimmed()
                    );
                }
                continue;
            }

            let base_version = compute_next_version(&current_version, bump, strategy)?;

            let (new_version, is_prerelease) = if prerelease_ctx.is_prerelease() {
                let tag_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
                if let Some(resolved) = prerelease_ctx.compute_identifier(
                    &base_version,
                    &tag_prefix,
                    &all_tags,
                    &short_hash,
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
            if verbose && !json {
                println!("{} {} — version unchanged", "○".dimmed(), pkg.name.dimmed());
            }
            continue;
        }

        let strategy_label = if forced_ver_for_pkg.is_some() {
            "forced".to_string()
        } else {
            let strategy = pkg.effective_versioning(
                &config.workspace,
                &tags_for_package(&all_tags, &tag_search_prefix),
            );
            let is_date_or_seq = matches!(
                strategy,
                VersioningStrategy::Calver
                    | VersioningStrategy::CalverShort
                    | VersioningStrategy::CalverSeq
                    | VersioningStrategy::Sequential
            );
            if is_date_or_seq {
                format!("{strategy:?}").to_lowercase()
            } else {
                bump.to_string()
            }
        };

        let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version);

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

        let hook_ctx = HookContext {
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

            if let Some(changelog_rel) = &pkg.changelog {
                let changelog_path = root.join(changelog_rel);
                update_changelog(
                    &changelog_path,
                    &pkg.name,
                    &new_version,
                    &commits,
                    bump,
                    false,
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

            let body = build_section(&new_version, &commits);
            tags_to_create.push((
                tag.clone(),
                format!("Release {tag}"),
                body,
                pkg.name.clone(),
                new_version.clone(),
                commits.len() as i32,
                is_prerelease,
            ));
        }

        if config.workspace.anonymous_telemetry {
            telemetry::send_event(
                telemetry::EventType::VersionBump,
                None,
                Some(commits.len() as i32),
                None,
                None,
            );
        }

        hook_contexts.push((hook_ctx, pkg_idx));
        bumped_names.insert(pkg.name.clone());
        any_bumped = true;
    }

    if config.is_monorepo() {
        let mut sink = cascade::CascadeSink {
            any_bumped: &mut any_bumped,
            json_packages: &mut json_packages,
            files_to_commit: &mut files_to_commit,
            files_per_package: &mut files_per_package,
            tags_to_create: &mut tags_to_create,
            pkg_outputs: &mut pkg_outputs,
            bumped_names: &mut bumped_names,
        };
        cascade::run_dependency_cascade(
            config,
            root,
            &all_tags,
            prerelease_ctx.channel.as_deref(),
            json,
            dry_run,
            &mut sink,
        )?;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string(&CheckResult {
                packages: json_packages
            })?
        );
        return Ok(());
    }

    if any_bumped && !tags_to_create.is_empty() {
        // Crash-resume checkpoint (#549). On dry-run we record nothing
        // — the run is read-only by design. Otherwise we either load an
        // in-progress checkpoint left behind by a previous crash, or
        // create a new one. HEAD-mismatch is fatal: an existing
        // checkpoint pinned to a different commit means the repo state
        // diverged from the in-flight release, and silently retrying
        // would replay tags onto the wrong graph.
        let head_sha = repo
            .head_id()
            .ok()
            .map(|id| id.to_string())
            .unwrap_or_default();
        let tag_names: Vec<String> = tags_to_create
            .iter()
            .map(|(t, _, _, _, _, _, _)| t.clone())
            .collect();
        let mut checkpoint = if dry_run {
            None
        } else {
            match Checkpoint::load(root)? {
                Some(existing) if existing.head_sha == head_sha => {
                    if verbose {
                        eprintln!(
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
            checkpoint: checkpoint.as_mut(),
        };
        execute_release(&mut plan)?;
        // Release finished cleanly — drop the checkpoint so the next
        // run starts from scratch instead of trying to resume a
        // finished release.
        if !dry_run {
            Checkpoint::delete(root)?;
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
            checkpoint: None,
        };
        print_dry_run_hooks(&plan)?;
    }

    if !any_bumped && !draft && !dry_run {
        publish_pending_drafts(&repo, config, root, verbose, &mut shared_outputs)?;
    }

    print_outputs(&pkg_outputs, &shared_outputs);

    if !any_bumped && !verbose {
        println!("{}", "Nothing to release.".dimmed());
    }

    Ok(())
}
