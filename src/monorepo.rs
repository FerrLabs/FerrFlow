use crate::changelog::{build_section, update_changelog};
use crate::config::ReleaseCommitMode;
use crate::config::ReleaseCommitScope;
use crate::config::{Config, PackageConfig, VersioningStrategy};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::error_code::{self, ErrorCodeExt};
use crate::forge::{self, ForgeKind};
use crate::formats::{get_handler, read_version, write_version};
use crate::git::{
    collect_all_tags, create_branch_and_commit, create_branch_and_commits, create_commit,
    create_or_move_tag, create_tag, fetch_tags, force_push_tags, get_changed_files,
    get_changed_files_since_tag, get_commits_since_last_stable_tag, get_commits_since_last_tag,
    get_remote_url, get_repo_root, get_tag_message, open_repo, push, push_branch, push_tags,
    tag_exists,
};
use crate::hooks::{HookContext, HookPoint, resolve_hook, resolve_on_failure, run_hook};
use crate::prerelease::PrereleaseContext;
use crate::telemetry;
use crate::versioning::{compute_next_version, truncate_version};
use anyhow::Result;
use colored::Colorize;
use git2::Repository;
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn build_forge_instance(repo: &Repository, config: &Config) -> Option<Box<dyn forge::Forge>> {
    let remote_url = get_remote_url(repo, &config.workspace.remote)?;
    let slug = forge::extract_repo_slug(&remote_url)?;
    let host = forge::extract_host(&remote_url)?;

    let kind = match config.workspace.forge {
        ForgeKind::Auto => forge::detect_forge_from_url(&remote_url)?,
        explicit => explicit,
    };

    let token = forge::resolve_token(kind)?;
    Some(forge::build_forge(kind, token, slug, host))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CheckCommit {
    hash: String,
    message: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CheckPackage {
    name: String,
    current_version: String,
    next_version: String,
    bump_type: String,
    tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    prerelease: bool,
    commits: Vec<CheckCommit>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CheckResult {
    packages: Vec<CheckPackage>,
}

pub fn check(
    config_path: Option<&Path>,
    verbose: bool,
    json: bool,
    channel: Option<&str>,
    comment: bool,
) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if !json {
        println!("{}", "FerrFlow — Check (dry run)".bold().blue());
        println!();
    }

    let result = run_release_logic(
        &root, &config, true, verbose, json, false, None, channel, false,
    );

    // Post a preview comment on the PR/MR if requested
    if comment {
        post_preview_comment(&repo, &config, &root);
    }

    if config.workspace.anonymous_telemetry {
        telemetry::send_event(telemetry::EventType::Check, None, None, None, None);
    }

    result
}

/// Run `check` in JSON mode silently, parse the result, and post a preview comment.
fn post_preview_comment(repo: &git2::Repository, config: &Config, root: &Path) {
    let pr_id = match forge::detect_pr_number() {
        Some(id) => id,
        None => return, // Not in a PR context, skip silently
    };

    let forge_instance = match build_forge_instance(repo, config) {
        Some(f) => f,
        None => return, // No forge detected or no token, skip silently
    };

    // Re-run the check logic in JSON mode to capture structured output
    let json_result = capture_check_json(root);
    let body = format_preview_comment(&json_result);
    let marker = "<!-- ferrflow-preview -->";

    let result = (|| -> anyhow::Result<()> {
        match forge_instance.find_comment(pr_id, marker)? {
            Some(comment_id) => forge_instance.update_comment(pr_id, comment_id, &body)?,
            None => forge_instance.create_comment(pr_id, &body)?,
        }
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("Warning: failed to post preview comment: {e}");
    }
}

fn capture_check_json(root: &Path) -> Vec<CheckPackage> {
    let exe = std::env::current_exe().unwrap_or_else(|_| "ferrflow".into());
    let output = std::process::Command::new(exe)
        .args(["check", "--json"])
        .current_dir(root)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            serde_json::from_str::<CheckResult>(&stdout)
                .map(|r| r.packages)
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn format_preview_comment(packages: &[CheckPackage]) -> String {
    let mut body = String::from("<!-- ferrflow-preview -->\n**FerrFlow Release Preview**\n\n");
    if packages.is_empty() {
        body.push_str("No releasable changes detected.");
        return body;
    }
    body.push_str("| Package | Current | Next | Bump |\n");
    body.push_str("|---------|---------|------|------|\n");
    for pkg in packages {
        body.push_str(&format!(
            "| {} | `{}` | `{}` | {} |\n",
            pkg.name, pkg.current_version, pkg.next_version, pkg.bump_type
        ));
    }
    let commit_count: usize = packages.iter().map(|p| p.commits.len()).sum();
    body.push_str(&format!("\nBased on {} commit(s).", commit_count));
    body
}

pub fn release(
    config_path: Option<&Path>,
    dry_run: bool,
    verbose: bool,
    force: bool,
    force_version: Option<&str>,
    channel: Option<&str>,
    draft: bool,
) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if dry_run {
        println!("{}", "FerrFlow — Release (dry run)".bold().blue());
    } else {
        println!("{}", "FerrFlow — Release".bold().green());
    }
    println!();

    run_release_logic(
        &root,
        &config,
        dry_run,
        verbose,
        false,
        force,
        force_version,
        channel,
        draft,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_release_logic(
    root: &Path,
    config: &Config,
    dry_run: bool,
    verbose: bool,
    json: bool,
    force: bool,
    force_version: Option<&str>,
    channel: Option<&str>,
    draft: bool,
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
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| c.id().to_string()[..7].to_string())
        .unwrap_or_default();

    let all_tags = collect_all_tags(&repo);

    // For pre-releases, commit/push/PR target the current branch (e.g. develop),
    // not the configured stable branch (e.g. main).
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
    // (tag_name, tag_msg, body, pkg_name, version, commits_count, is_prerelease)
    let mut tags_to_create: Vec<(String, String, String, String, String, i32, bool)> = Vec::new();
    let mut hook_contexts: Vec<(HookContext, usize)> = Vec::new(); // (ctx, pkg_index)
    let mut bumped_names: HashSet<String> = HashSet::new();

    // Buffered output: per-package lines and shared (commit/push) lines.
    // Each entry is (pkg_name, lines) in insertion order.
    let mut pkg_outputs: Vec<(String, Vec<String>)> = Vec::new();
    let mut shared_outputs: Vec<String> = Vec::new();

    // Parse --force-version: "VERSION" (single repo) or "NAME@VERSION" (monorepo)
    let forced: Option<(Option<&str>, &str)> = if let Some(fv) = force_version {
        if let Some(at_pos) = fv.find('@') {
            let name = &fv[..at_pos];
            let version = &fv[at_pos + 1..];
            if name.is_empty() || version.is_empty() {
                anyhow::bail!("Invalid --force-version format: expected NAME@VERSION, got {fv:?}");
            }
            Some((Some(name), version))
        } else {
            if config.is_monorepo() {
                anyhow::bail!(
                    "In a monorepo, --force-version requires NAME@VERSION format (e.g. api@1.2.3)"
                );
            }
            Some((None, fv))
        }
    } else {
        None
    };

    // Validate forced version is valid semver (strip leading 'v' if present)
    if let Some((_, ver)) = &forced {
        let clean = ver.strip_prefix('v').unwrap_or(ver);
        if semver::Version::parse(clean).is_err() {
            anyhow::bail!("Invalid version in --force-version: {ver:?} is not valid semver");
        }
    }

    for (pkg_idx, pkg) in config.packages.iter().enumerate() {
        let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());

        // Check if this package is the target of --force-version
        let forced_ver_for_pkg = forced.and_then(|(name, ver)| {
            if let Some(target_name) = name {
                if pkg.name == target_name {
                    Some(ver)
                } else {
                    None
                }
            } else {
                // Single repo: applies to the first (only) package
                Some(ver)
            }
        });

        let mut touched = is_package_touched(pkg, &changed_files, config.is_monorepo());

        if !touched && config.workspace.recover_missed_releases && config.is_monorepo() {
            let files_since_tag = get_changed_files_since_tag(
                &repo,
                &tag_search_prefix,
                config.workspace.orphaned_tag_strategy,
            )?;
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

        let Some(vf) = pkg.versioned_files.first() else {
            if !json {
                println!(
                    "{} {} — no versioned files configured",
                    "!".yellow(),
                    pkg.name.yellow()
                );
            }
            continue;
        };

        let current_version = read_version(vf, root)?;

        // Determine new version: forced or computed from commits
        let (new_version, is_prerelease, commits, bump) = if let Some(fv) = forced_ver_for_pkg {
            let clean = fv.strip_prefix('v').unwrap_or(fv);
            let commits = if !prerelease_ctx.is_prerelease() {
                get_commits_since_last_stable_tag(
                    &repo,
                    &tag_search_prefix,
                    config.workspace.orphaned_tag_strategy,
                )
                .unwrap_or_default()
            } else {
                get_commits_since_last_tag(
                    &repo,
                    &tag_search_prefix,
                    config.workspace.orphaned_tag_strategy,
                )
                .unwrap_or_default()
            };
            (clean.to_string(), false, commits, BumpType::None)
        } else {
            let commits = if !prerelease_ctx.is_prerelease() {
                get_commits_since_last_stable_tag(
                    &repo,
                    &tag_search_prefix,
                    config.workspace.orphaned_tag_strategy,
                )?
            } else {
                get_commits_since_last_tag(
                    &repo,
                    &tag_search_prefix,
                    config.workspace.orphaned_tag_strategy,
                )?
            };

            if commits.is_empty() {
                if verbose && !json {
                    println!("{} {} — no new commits", "○".dimmed(), pkg.name.dimmed());
                }
                continue;
            }

            let strategy = pkg.effective_versioning(&config.workspace);

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
            let strategy = pkg.effective_versioning(&config.workspace);
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

            // Floating tags (e.g. v1, v1.2) — skip for pre-releases.
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
            if repo.refname_to_id(&format!("refs/tags/{tag}")).is_ok() {
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

            // --- pre_bump hook ---
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

            // --- post_bump hook ---
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

    // --- Dependency cascade: auto-bump packages that depend on bumped packages ---
    if config.is_monorepo() {
        let mut cascade_round = 0;
        loop {
            cascade_round += 1;
            if cascade_round > config.packages.len() {
                break; // safety: avoid infinite loops from circular deps
            }
            let mut new_bumps = Vec::new();
            for (pkg_idx, pkg) in config.packages.iter().enumerate() {
                if bumped_names.contains(&pkg.name) {
                    continue;
                }
                if pkg.depends_on.iter().any(|dep| bumped_names.contains(dep)) {
                    new_bumps.push(pkg_idx);
                }
            }
            if new_bumps.is_empty() {
                break;
            }
            for pkg_idx in new_bumps {
                let pkg = &config.packages[pkg_idx];
                let Some(vf) = pkg.versioned_files.first() else {
                    continue;
                };
                let Ok(current_version) = read_version(vf, root) else {
                    continue;
                };
                let strategy = pkg.effective_versioning(&config.workspace);
                let Ok(new_version) =
                    compute_next_version(&current_version, BumpType::Patch, strategy)
                else {
                    continue;
                };
                if current_version == new_version {
                    continue;
                }
                let tag =
                    pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version);
                let dep_trigger: Vec<&str> = pkg
                    .depends_on
                    .iter()
                    .filter(|d| bumped_names.contains(*d))
                    .map(|s| s.as_str())
                    .collect();

                if json {
                    json_packages.push(CheckPackage {
                        name: pkg.name.clone(),
                        current_version: current_version.clone(),
                        next_version: new_version.clone(),
                        bump_type: "patch".to_string(),
                        tag: tag.clone(),
                        channel: prerelease_ctx.channel.clone(),
                        prerelease: false,
                        commits: vec![],
                    });
                } else {
                    let mut lines = vec![format!(
                        "{} {}  {} → {}  ({}, dependency: {})",
                        "●".green().bold(),
                        pkg.name.bold(),
                        current_version.dimmed(),
                        new_version.green().bold(),
                        "patch".cyan(),
                        dep_trigger.join(", ").cyan()
                    )];
                    if !dry_run {
                        for vf in &pkg.versioned_files {
                            write_version(vf, root, &new_version)?;
                            if get_handler(&vf.format).modifies_file() {
                                lines.push(format!("  ✓ Updated {}", vf.path));
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
                                &[],
                                BumpType::Patch,
                                false,
                            )?;
                            files_to_commit.push(changelog_rel.clone());
                            files_per_package
                                .entry(pkg.name.clone())
                                .or_default()
                                .push(changelog_rel.clone());
                        }
                    }
                    pkg_outputs.push((pkg.name.clone(), lines));
                }
                let body = format!("Dependency update: {}", dep_trigger.join(", "));
                tags_to_create.push((
                    tag,
                    format!(
                        "Release {}",
                        pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version)
                    ),
                    body,
                    pkg.name.clone(),
                    new_version,
                    0,
                    false,
                ));
                bumped_names.insert(pkg.name.clone());
                any_bumped = true;
            }
        }
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
        // --- pre_commit hooks (per released package) ---
        for (ctx, pkg_idx) in &hook_contexts {
            let pkg = &config.packages[*pkg_idx];
            let ws_hooks = config.workspace.hooks.as_ref();
            let pkg_hooks = pkg.hooks.as_ref();
            let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
            if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PreCommit) {
                let before = collect_dirty_files(&repo);
                run_hook(
                    HookPoint::PreCommit,
                    &cmd,
                    ctx,
                    on_failure,
                    dry_run,
                    verbose,
                    root,
                )?;
                if !dry_run {
                    let len_before = files_to_commit.len();
                    auto_stage_new_files(&repo, &before, &mut files_to_commit);
                    let pkg_files = files_per_package.entry(pkg.name.clone()).or_default();
                    for f in &files_to_commit[len_before..] {
                        pkg_files.push(f.clone());
                    }
                }
            }
        }

        let file_refs: Vec<&str> = files_to_commit.iter().map(String::as_str).collect();
        let mode = config.workspace.release_commit_mode;
        let scope = config.workspace.release_commit_scope;

        // Build the release commit message.
        let release_parts: Vec<String> = tags_to_create
            .iter()
            .map(|(_, _, _, name, ver, _, _)| format!("{name} v{ver}"))
            .collect();
        let skip_ci = if config.workspace.effective_skip_ci() {
            " [skip ci]"
        } else {
            ""
        };
        let commit_msg = format!("chore(release): {}{skip_ci}", release_parts.join(", "));
        let mut floating_tag_names: Vec<String> = Vec::new();

        if !dry_run {
            match mode {
                ReleaseCommitMode::Commit => {
                    if scope == ReleaseCommitScope::PerPackage && tags_to_create.len() > 1 {
                        for (_, _, _, pkg_name, ver, _, _) in &tags_to_create {
                            if let Some(pkg_files) = files_per_package.get(pkg_name) {
                                let refs: Vec<&str> =
                                    pkg_files.iter().map(String::as_str).collect();
                                let msg = format!("chore(release): {pkg_name} v{ver}{skip_ci}");
                                create_commit(&repo, &refs, &msg)?;
                            }
                        }
                        shared_outputs
                            .push("✓ Committed release changes (per-package)".to_string());
                    } else {
                        create_commit(&repo, &file_refs, &commit_msg)?;
                        shared_outputs.push("✓ Committed release changes".to_string());
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
                    if scope == ReleaseCommitScope::PerPackage && tags_to_create.len() > 1 {
                        let commit_list: Vec<(Vec<&str>, String)> = tags_to_create
                            .iter()
                            .filter_map(|(_, _, _, pkg_name, ver, _, _)| {
                                files_per_package.get(pkg_name).map(|pf| {
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
                        create_branch_and_commits(&repo, &branch_name, &commit_refs)?;
                    } else {
                        create_branch_and_commit(&repo, &branch_name, &file_refs, &commit_msg)?;
                    }
                    push_branch(&repo, &config.workspace.remote, &branch_name)?;
                    shared_outputs.push(format!("✓ Pushed branch {}", branch_name.cyan()));

                    if let Some(forge_instance) = build_forge_instance(&repo, config) {
                        let pr_title = format!("chore(release): {}", release_parts.join(", "));
                        let pr_body = format!(
                            "Automated release commit.\n\n{}",
                            tags_to_create
                                .iter()
                                .map(|(tag, _, _, _, _, _, _)| format!("- `{tag}`"))
                                .collect::<Vec<_>>()
                                .join("\n")
                        );
                        match forge_instance.create_merge_request(
                            &branch_name,
                            &target_branch,
                            &pr_title,
                            &pr_body,
                        ) {
                            Ok(mr) => {
                                shared_outputs.push(format!(
                                    "✓ Created {} #{}",
                                    forge_instance.mr_noun(),
                                    mr.id.to_string().cyan()
                                ));
                                if config.workspace.auto_merge_releases {
                                    match forge_instance.enable_auto_merge(&mr) {
                                        Ok(()) => {
                                            shared_outputs.push("✓ Auto-merge enabled".to_string())
                                        }
                                        Err(err) => eprintln!(
                                            "{}",
                                            format!(
                                                "  Warning: failed to enable auto-merge: {err}"
                                            )
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

            // Tags are always created on the current HEAD.
            for (tag_name, tag_msg, _, pkg_name, _, _, _) in &tags_to_create {
                create_tag(&repo, tag_name, tag_msg)?;
                if let Some((_, lines)) = pkg_outputs.iter_mut().rev().find(|(n, _)| n == pkg_name)
                {
                    lines.push(format!("  ✓ Created tag {}", tag_name.cyan()));
                }
            }

            // Floating tags (e.g. v1, v1.2) point to the latest release.
            // Pre-releases never move floating tags.
            for (_, _, _, pkg_name, new_version, _, is_pre) in &tags_to_create {
                if *is_pre {
                    continue;
                }
                let pkg = config
                    .packages
                    .iter()
                    .find(|p| &p.name == pkg_name)
                    .ok_or_else(|| anyhow::anyhow!("package '{pkg_name}' not found in config"))
                    .error_code(error_code::MONOREPO_PACKAGE_NOT_FOUND)?;
                let levels = pkg.effective_floating_tags(&config.workspace);
                for level in levels {
                    if let Some(truncated) = truncate_version(new_version, *level) {
                        let float_tag = pkg.tag_for_version(
                            &config.workspace,
                            config.is_monorepo(),
                            &truncated,
                        );
                        // Backward detection: if the floating tag already exists,
                        // check whether the new version is actually newer.
                        if tag_exists(&repo, &float_tag)
                            && let Some(old_msg) = get_tag_message(&repo, &float_tag)
                            && let Some(old_ver) = old_msg.strip_prefix("Release ")
                            && semver::Version::parse(old_ver.trim_start_matches('v'))
                                .ok()
                                .zip(
                                    semver::Version::parse(new_version.trim_start_matches('v'))
                                        .ok(),
                                )
                                .is_some_and(|(old, new)| new < old)
                        {
                            if !force {
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
                        let moved = create_or_move_tag(&repo, &float_tag, &msg)?;
                        let verb = if moved { "Moved" } else { "Created" };
                        if let Some((_, lines)) =
                            pkg_outputs.iter_mut().rev().find(|(n, _)| n == pkg_name)
                        {
                            lines.push(format!("  ✓ {} floating tag {}", verb, float_tag.cyan()));
                        }
                        floating_tag_names.push(float_tag);
                    }
                }
            }
        }

        // --- pre_publish hooks (per released package) ---
        for (ctx, pkg_idx) in &hook_contexts {
            let pkg = &config.packages[*pkg_idx];
            let ws_hooks = config.workspace.hooks.as_ref();
            let pkg_hooks = pkg.hooks.as_ref();
            let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
            if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PrePublish) {
                run_hook(
                    HookPoint::PrePublish,
                    &cmd,
                    ctx,
                    on_failure,
                    dry_run,
                    verbose,
                    root,
                )?;
            }
        }

        if !dry_run {
            // Push tags (and branch for commit mode).
            let tag_refs: Vec<&str> = tags_to_create
                .iter()
                .map(|(t, _, _, _, _, _, _)| t.as_str())
                .collect();
            match mode {
                ReleaseCommitMode::Commit => {
                    push(&repo, &config.workspace.remote, &target_branch, &tag_refs)?;
                    shared_outputs.push(format!(
                        "✓ Pushed and verified on {}/{}",
                        config.workspace.remote, target_branch
                    ));
                }
                ReleaseCommitMode::Pr | ReleaseCommitMode::None => {
                    if !tag_refs.is_empty() {
                        push_tags(&repo, &config.workspace.remote, &tag_refs)?;
                        shared_outputs.push("✓ Pushed tags".to_string());
                    }
                }
            }

            // Force-push floating tags (they may already exist on the remote).
            if !floating_tag_names.is_empty() {
                let float_refs: Vec<&str> = floating_tag_names.iter().map(String::as_str).collect();
                force_push_tags(&repo, &config.workspace.remote, &float_refs)?;
                shared_outputs.push("✓ Pushed floating tags".to_string());
            }

            if let Some(forge_instance) = build_forge_instance(&repo, config) {
                for (tag_name, _, body, pkg_name, _, _, is_pre) in &tags_to_create {
                    if !draft {
                        // Check for existing draft release and publish it
                        match forge_instance.find_draft_release(tag_name) {
                            Ok(Some(release_id)) => {
                                match forge_instance.publish_release(release_id) {
                                    Ok(()) => {
                                        if let Some((_, lines)) = pkg_outputs
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
                                        format!(
                                            "  Warning: failed to publish draft for {tag_name}: {err}"
                                        )
                                        .yellow()
                                    ),
                                }
                            }
                            Ok(None) => {}
                            Err(err) => {
                                if verbose {
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

                    match forge_instance.create_release(tag_name, body, *is_pre, draft) {
                        Ok(()) => {
                            if let Some((_, lines)) =
                                pkg_outputs.iter_mut().rev().find(|(n, _)| n == pkg_name)
                            {
                                let noun = forge_instance.release_noun();
                                if draft {
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

            if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&summary_path)
                {
                    let _ = writeln!(file, "## Released\n");
                    for (tag_name, _, body, _, _, _, _) in &tags_to_create {
                        let _ = writeln!(file, "### {tag_name}\n");
                        let _ = writeln!(file, "{body}");
                    }
                }
            }
        }

        if config.workspace.anonymous_telemetry {
            for (_, _, _, pkg_name, version, commit_count, _) in &tags_to_create {
                telemetry::send_event(
                    telemetry::EventType::Release,
                    None,
                    Some(*commit_count),
                    Some(pkg_name.clone()),
                    Some(version.clone()),
                );
            }
        }

        // --- post_publish hooks (per released package) ---
        for (ctx, pkg_idx) in &hook_contexts {
            let pkg = &config.packages[*pkg_idx];
            let ws_hooks = config.workspace.hooks.as_ref();
            let pkg_hooks = pkg.hooks.as_ref();
            let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
            if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, HookPoint::PostPublish) {
                run_hook(
                    HookPoint::PostPublish,
                    &cmd,
                    ctx,
                    on_failure,
                    dry_run,
                    verbose,
                    root,
                )?;
            }
        }
    } else if dry_run && any_bumped {
        // Dry-run: print pre_commit/pre_publish/post_publish hooks.
        for (ctx, pkg_idx) in &hook_contexts {
            let pkg = &config.packages[*pkg_idx];
            let ws_hooks = config.workspace.hooks.as_ref();
            let pkg_hooks = pkg.hooks.as_ref();
            let on_failure = resolve_on_failure(pkg_hooks, ws_hooks);
            for point in [
                HookPoint::PreCommit,
                HookPoint::PrePublish,
                HookPoint::PostPublish,
            ] {
                if let Some(cmd) = resolve_hook(pkg_hooks, ws_hooks, point) {
                    run_hook(point, &cmd, ctx, on_failure, true, verbose, root)?;
                }
            }
        }
    }

    // Publish orphaned draft releases when nothing was bumped.
    // This handles the case where `ferrflow release` runs after the
    // tag and release commit already exist (e.g. in a publish workflow).
    if !any_bumped
        && !draft
        && !dry_run
        && let Some(forge_instance) = build_forge_instance(&repo, config)
    {
        for pkg in &config.packages {
            let Some(vf) = pkg.versioned_files.first() else {
                continue;
            };
            let Ok(version) = read_version(vf, root) else {
                continue;
            };
            let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &version);
            match forge_instance.find_draft_release(&tag) {
                Ok(Some(release_id)) => match forge_instance.publish_release(release_id) {
                    Ok(()) => {
                        shared_outputs.push(format!(
                            "✓ Published draft {} {}",
                            forge_instance.release_noun(),
                            tag.cyan()
                        ));
                    }
                    Err(err) => eprintln!(
                        "{}",
                        format!("  Warning: failed to publish draft for {tag}: {err}").yellow()
                    ),
                },
                Ok(None) => {}
                Err(err) => {
                    if verbose {
                        eprintln!(
                            "{}",
                            format!("  Warning: failed to check draft release {tag}: {err}")
                                .yellow()
                        );
                    }
                }
            }
        }
    }

    // Print grouped output: per-package sections, then shared operations.
    for (i, (_, lines)) in pkg_outputs.iter().enumerate() {
        if i > 0 {
            println!();
        }
        for line in lines {
            println!("{line}");
        }
    }
    if !shared_outputs.is_empty() {
        println!();
        for line in &shared_outputs {
            println!("{line}");
        }
    }

    if !any_bumped && !verbose {
        println!("{}", "Nothing to release.".dimmed());
    }

    Ok(())
}

/// Collect the set of dirty (modified/new) file paths in the working tree.
fn collect_dirty_files(repo: &git2::Repository) -> HashSet<String> {
    let mut files = HashSet::new();
    if let Ok(statuses) = repo.statuses(None) {
        for entry in statuses.iter() {
            let status = entry.status();
            if status.intersects(
                git2::Status::WT_MODIFIED
                    | git2::Status::WT_NEW
                    | git2::Status::WT_TYPECHANGE
                    | git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED,
            ) && let Some(path) = entry.path()
            {
                files.insert(path.to_string());
            }
        }
    }
    files
}

/// Auto-stage files that became dirty after a hook ran.
fn auto_stage_new_files(
    repo: &git2::Repository,
    before: &HashSet<String>,
    files_to_commit: &mut Vec<String>,
) {
    let after = collect_dirty_files(repo);
    for path in after.difference(before) {
        if !files_to_commit.contains(path) {
            files_to_commit.push(path.clone());
        }
    }
}

fn is_package_touched(pkg: &PackageConfig, changed_files: &[String], is_monorepo: bool) -> bool {
    // In single-package mode, always consider it touched
    if !is_monorepo {
        return true;
    }

    let pkg_path = pkg.path.trim_start_matches("./").trim_end_matches('/');

    // Root package
    if pkg_path == "." || pkg_path.is_empty() {
        return true;
    }

    let prefix = format!("{pkg_path}/");
    if changed_files.iter().any(|f| f.starts_with(&prefix)) {
        return true;
    }

    // Check shared paths
    for shared in &pkg.shared_paths {
        let shared = shared.trim_end_matches('/');
        if changed_files
            .iter()
            .any(|f| f.starts_with(shared) || f == shared)
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PackageConfig;

    fn make_pkg(name: &str, path: &str, shared: &[&str]) -> PackageConfig {
        PackageConfig {
            name: name.into(),
            path: path.into(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: shared.iter().map(|s| s.to_string()).collect(),
            depends_on: vec![],
            versioning: None,
            tag_template: None,
            hooks: None,
            floating_tags: None,
        }
    }

    #[test]
    fn single_package_always_touched() {
        let pkg = make_pkg("app", ".", &[]);
        let files = vec!["README.md".to_string()];
        assert!(is_package_touched(&pkg, &files, false));
    }

    #[test]
    fn monorepo_root_package_always_touched() {
        let pkg = make_pkg("root", ".", &[]);
        let files = vec!["something.rs".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_package_touched_by_own_files() {
        let pkg = make_pkg("api", "packages/api", &[]);
        let files = vec!["packages/api/src/main.rs".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_package_not_touched_by_other_files() {
        let pkg = make_pkg("api", "packages/api", &[]);
        let files = vec!["packages/site/index.ts".to_string()];
        assert!(!is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_package_touched_by_shared_path() {
        let pkg = make_pkg("api", "packages/api", &["packages/shared/"]);
        let files = vec!["packages/shared/types.ts".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_shared_path_trailing_slash_trimmed() {
        let pkg = make_pkg("api", "packages/api", &["lib/"]);
        let files = vec!["lib/utils.rs".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_no_changed_files() {
        let pkg = make_pkg("api", "packages/api", &[]);
        let files: Vec<String> = vec![];
        assert!(!is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_path_with_dot_slash_prefix() {
        let pkg = make_pkg("api", "./packages/api", &[]);
        let files = vec!["packages/api/src/main.rs".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn single_package_mode_always_touched() {
        let pkg = make_pkg("api", "packages/api", &[]);
        let files = vec!["unrelated/file.rs".to_string()];
        assert!(is_package_touched(&pkg, &files, false));
    }

    #[test]
    fn monorepo_empty_path_is_root() {
        let pkg = make_pkg("root", "", &[]);
        let files = vec!["anything.rs".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_exact_shared_path_file() {
        let pkg = make_pkg("api", "packages/api", &["shared-config.json"]);
        let files = vec!["shared-config.json".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_multiple_shared_paths() {
        let pkg = make_pkg("api", "packages/api", &["lib/", "proto/"]);
        let files = vec!["proto/schema.proto".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_similar_prefix_no_false_positive() {
        let pkg = make_pkg("api", "packages/api", &[]);
        // "packages/api-docs" should NOT match "packages/api/"
        let files = vec!["packages/api-docs/README.md".to_string()];
        assert!(!is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_shared_path_with_trailing_slash() {
        let pkg = make_pkg("api", "packages/api", &["packages/shared/"]);
        let files = vec!["packages/shared/types.ts".to_string()];
        assert!(is_package_touched(&pkg, &files, true));
    }

    #[test]
    fn monorepo_empty_changed_files_single_package() {
        let pkg = make_pkg("app", "packages/app", &[]);
        let files: Vec<String> = vec![];
        // Even single-package mode returns true regardless of changed files
        assert!(is_package_touched(&pkg, &files, false));
    }

    #[test]
    fn parse_force_version_single_repo() {
        let fv = "1.2.3";
        let result: Option<(Option<&str>, &str)> = if let Some(at_pos) = fv.find('@') {
            let name = &fv[..at_pos];
            let version = &fv[at_pos + 1..];
            Some((Some(name), version))
        } else {
            Some((None, fv))
        };
        assert_eq!(result, Some((None, "1.2.3")));
    }

    #[test]
    fn parse_force_version_monorepo() {
        let fv = "api@2.0.0";
        let result: Option<(Option<&str>, &str)> = if let Some(at_pos) = fv.find('@') {
            let name = &fv[..at_pos];
            let version = &fv[at_pos + 1..];
            Some((Some(name), version))
        } else {
            Some((None, fv))
        };
        assert_eq!(result, Some((Some("api"), "2.0.0")));
    }

    #[test]
    fn parse_force_version_with_v_prefix() {
        let fv = "v3.0.0";
        let clean = fv.strip_prefix('v').unwrap_or(fv);
        assert!(semver::Version::parse(clean).is_ok());
    }

    #[test]
    fn parse_force_version_invalid_semver() {
        let fv = "not-a-version";
        let clean = fv.strip_prefix('v').unwrap_or(fv);
        assert!(semver::Version::parse(clean).is_err());
    }
}
