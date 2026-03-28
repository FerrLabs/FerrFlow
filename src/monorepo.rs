use crate::changelog::{build_section, update_changelog};
use crate::config::{Config, PackageConfig, VersioningStrategy};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::{get_handler, read_version, write_version};
use crate::git::{
    create_commit, create_tag, fetch_tags, get_changed_files, get_commits_since_last_tag,
    get_repo_root, get_repo_slug, open_repo, push,
};
use crate::release::create_github_release;
use crate::telemetry;
use crate::versioning::compute_next_version;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn check(config_path: Option<&Path>, verbose: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    println!("{}", "FerrFlow — Check (dry run)".bold().blue());
    println!();

    let result = run_release_logic(&root, &config, true, verbose);

    if config.workspace.telemetry {
        telemetry::send_event("check", None, None, None);
    }

    result
}

pub fn release(config_path: Option<&Path>, dry_run: bool, verbose: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if dry_run {
        println!("{}", "FerrFlow — Release (dry run)".bold().blue());
    } else {
        println!("{}", "FerrFlow — Release".bold().green());
    }
    println!();

    run_release_logic(&root, &config, dry_run, verbose)
}

fn run_release_logic(root: &Path, config: &Config, dry_run: bool, verbose: bool) -> Result<()> {
    if config.packages.is_empty() {
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

    if verbose && !changed_files.is_empty() {
        println!("Changed files in last commit:");
        for f in &changed_files {
            println!("  {}", f.dimmed());
        }
        println!();
    }

    let mut any_bumped = false;
    let mut files_to_commit: Vec<String> = Vec::new();
    // (tag_name, tag_msg, body, pkg_name, version)
    let mut tags_to_create: Vec<(String, String, String, String, String)> = Vec::new();

    for pkg in &config.packages {
        let touched = is_package_touched(pkg, &changed_files, config.is_monorepo());
        if !touched {
            if verbose {
                println!(
                    "{} {} — not touched, skipping",
                    "○".dimmed(),
                    pkg.name.dimmed()
                );
            }
            continue;
        }

        let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let commits = get_commits_since_last_tag(&repo, &tag_search_prefix)?;

        if commits.is_empty() {
            if verbose {
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
            println!(
                "{} {} — no releasable commits",
                "○".dimmed(),
                pkg.name.dimmed()
            );
            continue;
        }

        let Some(vf) = pkg.versioned_files.first() else {
            println!(
                "{} {} — no versioned_files configured",
                "!".yellow(),
                pkg.name.yellow()
            );
            continue;
        };

        let current_version = read_version(vf, root)?;
        let new_version = compute_next_version(&current_version, bump, strategy)?;

        if current_version == new_version {
            if verbose {
                println!("{} {} — version unchanged", "○".dimmed(), pkg.name.dimmed());
            }
            continue;
        }

        let strategy_label = if is_date_or_seq {
            format!("{strategy:?}").to_lowercase()
        } else {
            bump.to_string()
        };

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

        if !dry_run {
            let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &new_version);
            if repo.refname_to_id(&format!("refs/tags/{tag}")).is_ok() {
                println!(
                    "  {} {} — tag {} already exists, skipping",
                    "○".dimmed(),
                    pkg.name.dimmed(),
                    tag.cyan()
                );
                continue;
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

            let body = build_section(&new_version, &commits);
            tags_to_create.push((
                tag.clone(),
                format!("Release {tag}"),
                body,
                pkg.name.clone(),
                new_version.clone(),
            ));
        }

        any_bumped = true;
    }

    if !dry_run && any_bumped {
        let file_refs: Vec<&str> = files_to_commit.iter().map(String::as_str).collect();
        create_commit(&repo, &file_refs, "chore: release [skip ci]")?;
        println!("  ✓ Committed release changes");

        for (tag_name, tag_msg, _, _, _) in &tags_to_create {
            create_tag(&repo, tag_name, tag_msg)?;
            println!("  ✓ Created tag {}", tag_name.cyan());
        }

        let tag_refs: Vec<&str> = tags_to_create
            .iter()
            .map(|(t, _, _, _, _)| t.as_str())
            .collect();
        push(
            &repo,
            &config.workspace.remote,
            &config.workspace.branch,
            &tag_refs,
        )?;
        println!(
            "  ✓ Pushed to {}/{}",
            config.workspace.remote, config.workspace.branch
        );

        if config.workspace.telemetry {
            for (_, _, _, pkg_name, version) in &tags_to_create {
                telemetry::send_event("release", Some(pkg_name), Some(version), None);
            }
        }

        if let Ok(token) = std::env::var("GITHUB_TOKEN")
            && let Some(slug) = get_repo_slug(&repo, &config.workspace.remote)
        {
            for (tag_name, _, body, _, _) in &tags_to_create {
                match create_github_release(&token, &slug, tag_name, body) {
                    Ok(()) => println!("  ✓ GitHub Release {}", tag_name.cyan()),
                    Err(err) => eprintln!(
                        "{}",
                        format!("  Warning: failed to create GitHub Release for {tag_name}: {err}")
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
                for (tag_name, _, body, _, _) in &tags_to_create {
                    let _ = writeln!(file, "### {tag_name}\n");
                    let _ = writeln!(file, "{body}");
                }
            }
        }
    }

    if !any_bumped && !verbose {
        println!("{}", "Nothing to release.".dimmed());
    }

    Ok(())
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
