use crate::changelog::{build_section, update_changelog};
use crate::config::ReleaseCommitMode;
use crate::config::{Config, PackageConfig, VersioningStrategy};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::{get_handler, read_version, write_version};
use crate::git::{
    create_branch_and_commit, create_commit, create_or_move_tag, create_tag, fetch_tags,
    force_push_tags, get_changed_files, get_changed_files_since_tag, get_commits_since_last_tag,
    get_repo_root, get_repo_slug, get_tag_message, open_repo, push, push_branch, push_tags,
    tag_exists,
};
use crate::hooks::{HookContext, HookPoint, resolve_hook, resolve_on_failure, run_hook};
use crate::release::{create_github_pr, create_github_release, enable_auto_merge};
use crate::telemetry;
use crate::versioning::{compute_next_version, truncate_version};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::Path;

#[derive(serde::Serialize)]
struct CheckCommit {
    hash: String,
    message: String,
}

#[derive(serde::Serialize)]
struct CheckPackage {
    name: String,
    current_version: String,
    next_version: String,
    bump_type: String,
    tag: String,
    commits: Vec<CheckCommit>,
}

#[derive(serde::Serialize)]
struct CheckResult {
    packages: Vec<CheckPackage>,
}

pub fn check(config_path: Option<&Path>, verbose: bool, json: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if !json {
        println!("{}", "FerrFlow — Check (dry run)".bold().blue());
        println!();
    }

    let result = run_release_logic(&root, &config, true, verbose, json, false);

    if config.workspace.anonymous_telemetry {
        telemetry::send_event(telemetry::EventType::Check, None, None);
    }

    result
}

pub fn release(
    config_path: Option<&Path>,
    dry_run: bool,
    verbose: bool,
    force: bool,
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

    run_release_logic(&root, &config, dry_run, verbose, false, force)
}

fn run_release_logic(
    root: &Path,
    config: &Config,
    dry_run: bool,
    verbose: bool,
    json: bool,
    force: bool,
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
    // (tag_name, tag_msg, body, pkg_name, version, commits_count)
    let mut tags_to_create: Vec<(String, String, String, String, String, i32)> = Vec::new();
    let mut hook_contexts: Vec<(HookContext, usize)> = Vec::new(); // (ctx, pkg_index)

    for (pkg_idx, pkg) in config.packages.iter().enumerate() {
        let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
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

        if !touched {
            if verbose && !json {
                println!(
                    "{} {} — not touched, skipping",
                    "○".dimmed(),
                    pkg.name.dimmed()
                );
            }
            continue;
        }

        let commits = get_commits_since_last_tag(
            &repo,
            &tag_search_prefix,
            config.workspace.orphaned_tag_strategy,
        )?;

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
        let new_version = compute_next_version(&current_version, bump, strategy)?;

        if current_version == new_version {
            if verbose && !json {
                println!("{} {} — version unchanged", "○".dimmed(), pkg.name.dimmed());
            }
            continue;
        }

        let strategy_label = if is_date_or_seq {
            format!("{strategy:?}").to_lowercase()
        } else {
            bump.to_string()
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
                commits: check_commits,
            });
        } else {
            println!(
                "{} {}  {} → {}  ({})",
                "●".green().bold(),
                pkg.name.bold(),
                current_version.dimmed(),
                new_version.green().bold(),
                strategy_label.cyan()
            );

            if verbose {
                for c in &commits {
                    if let Some(line) = c.message.lines().next() {
                        println!("    {} {}", c.hash.dimmed(), line.dimmed());
                    }
                }
            }

            // Show floating tags that would be created/moved.
            let levels = pkg.effective_floating_tags(&config.workspace);
            for level in levels {
                if let Some(truncated) = truncate_version(&new_version, *level) {
                    let float_tag =
                        pkg.tag_for_version(&config.workspace, config.is_monorepo(), &truncated);
                    let verb = if tag_exists(&repo, &float_tag) {
                        "move"
                    } else {
                        "create"
                    };
                    println!(
                        "    {} floating tag {}",
                        format!("→ {verb}").dimmed(),
                        float_tag.cyan()
                    );
                }
            }
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
                println!(
                    "  {} {} — tag {} already exists, skipping",
                    "○".dimmed(),
                    pkg.name.dimmed(),
                    tag.cyan()
                );
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
                    println!("  ✓ Updated {}", vf.path);
                    files_to_commit.push(vf.path.clone());
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
                auto_stage_new_files(&repo, &before, &mut files_to_commit);
            }

            if config.workspace.anonymous_telemetry {
                telemetry::send_event(
                    telemetry::EventType::VersionBump,
                    None,
                    Some(commits.len() as i32),
                );
            }

            let body = build_section(&new_version, &commits);
            tags_to_create.push((
                tag.clone(),
                format!("Release {tag}"),
                body,
                pkg.name.clone(),
                new_version.clone(),
                commits.len() as i32,
            ));
        }

        hook_contexts.push((hook_ctx, pkg_idx));
        any_bumped = true;
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
                    auto_stage_new_files(&repo, &before, &mut files_to_commit);
                }
            }
        }

        let file_refs: Vec<&str> = files_to_commit.iter().map(String::as_str).collect();
        let mode = config.workspace.release_commit_mode;

        // Build the release commit message.
        let release_parts: Vec<String> = tags_to_create
            .iter()
            .map(|(_, _, _, name, ver, _)| format!("{name} v{ver}"))
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
                    create_commit(&repo, &file_refs, &commit_msg)?;
                    println!("  ✓ Committed release changes");
                }
                ReleaseCommitMode::Pr => {
                    let branch_name = format!(
                        "release/{}",
                        release_parts
                            .first()
                            .map(|s| s.replace(' ', "-"))
                            .unwrap_or_else(|| "bump".to_string())
                    );
                    create_branch_and_commit(&repo, &branch_name, &file_refs, &commit_msg)?;
                    push_branch(&repo, &config.workspace.remote, &branch_name)?;
                    println!("  ✓ Pushed branch {}", branch_name.cyan());

                    if let Ok(token) = std::env::var("GITHUB_TOKEN")
                        && let Some(slug) = get_repo_slug(&repo, &config.workspace.remote)
                    {
                        let pr_title = format!("chore(release): {}", release_parts.join(", "));
                        let pr_body = format!(
                            "Automated release commit.\n\n{}",
                            tags_to_create
                                .iter()
                                .map(|(tag, _, _, _, _, _)| format!("- `{tag}`"))
                                .collect::<Vec<_>>()
                                .join("\n")
                        );
                        match create_github_pr(
                            &token,
                            &slug,
                            &branch_name,
                            &config.workspace.branch,
                            &pr_title,
                            &pr_body,
                        ) {
                            Ok(pr_number) => {
                                println!("  ✓ Created PR #{}", pr_number.to_string().cyan());
                                if config.workspace.auto_merge_releases {
                                    match enable_auto_merge(&token, &slug, pr_number) {
                                        Ok(()) => println!("  ✓ Auto-merge enabled"),
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
                                format!("  Warning: failed to create PR: {err}").yellow()
                            ),
                        }
                    }
                }
                ReleaseCommitMode::None => {}
            }

            // Tags are always created on the current HEAD.
            for (tag_name, tag_msg, _, _, _, _) in &tags_to_create {
                create_tag(&repo, tag_name, tag_msg)?;
                println!("  ✓ Created tag {}", tag_name.cyan());
            }

            // Floating tags (e.g. v1, v1.2) point to the latest release.
            for (_, _, _, pkg_name, new_version, _) in &tags_to_create {
                let pkg = config
                    .packages
                    .iter()
                    .find(|p| &p.name == pkg_name)
                    .unwrap();
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
                                anyhow::bail!(
                                    "Floating tag {} would move backward ({} → {}). Use --force to override.",
                                    float_tag,
                                    old_ver,
                                    new_version,
                                );
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
                        println!("  ✓ {} floating tag {}", verb, float_tag.cyan());
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
                .map(|(t, _, _, _, _, _)| t.as_str())
                .collect();
            match mode {
                ReleaseCommitMode::Commit => {
                    push(
                        &repo,
                        &config.workspace.remote,
                        &config.workspace.branch,
                        &tag_refs,
                    )?;
                    println!(
                        "  ✓ Pushed and verified on {}/{}",
                        config.workspace.remote, config.workspace.branch
                    );
                }
                ReleaseCommitMode::Pr | ReleaseCommitMode::None => {
                    if !tag_refs.is_empty() {
                        push_tags(&repo, &config.workspace.remote, &tag_refs)?;
                        println!("  ✓ Pushed tags");
                    }
                }
            }

            // Force-push floating tags (they may already exist on the remote).
            if !floating_tag_names.is_empty() {
                let float_refs: Vec<&str> = floating_tag_names.iter().map(String::as_str).collect();
                force_push_tags(&repo, &config.workspace.remote, &float_refs)?;
                println!("  ✓ Pushed floating tags");
            }

            if config.workspace.anonymous_telemetry {
                for (_, _, _, _, _, commit_count) in &tags_to_create {
                    telemetry::send_event(telemetry::EventType::Release, None, Some(*commit_count));
                }
            }

            if let Ok(token) = std::env::var("GITHUB_TOKEN")
                && let Some(slug) = get_repo_slug(&repo, &config.workspace.remote)
            {
                for (tag_name, _, body, _, _, _) in &tags_to_create {
                    match create_github_release(&token, &slug, tag_name, body) {
                        Ok(()) => println!("  ✓ GitHub Release {}", tag_name.cyan()),
                        Err(err) => eprintln!(
                            "{}",
                            format!(
                                "  Warning: failed to create GitHub Release for {tag_name}: {err}"
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
                    for (tag_name, _, body, _, _, _) in &tags_to_create {
                        let _ = writeln!(file, "### {tag_name}\n");
                        let _ = writeln!(file, "{body}");
                    }
                }
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
}
