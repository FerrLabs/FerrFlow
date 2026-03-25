use crate::changelog::{build_section, update_changelog};
use crate::config::{Config, PackageConfig};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::{get_handler, read_version, write_version};
use crate::git::{
    create_commit, create_tag, get_changed_files, get_commits_since_last_tag, get_repo_root,
    get_repo_slug, open_repo, push,
};
use crate::release::create_github_release;
use crate::versioning::bump_version;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn check(verbose: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root)?;

    println!("{}", "FerrFlow — Check (dry run)".bold().blue());
    println!();

    run_release_logic(&root, &config, true, verbose)
}

pub fn release(dry_run: bool, verbose: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root)?;

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
            "No packages configured. Run `ferrflow init` to create a ferrflow.toml.".yellow()
        );
        return Ok(());
    }

    let repo = open_repo(root)?;
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
    let mut tags_to_create: Vec<(String, String, String)> = Vec::new();

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

        let tag_prefix = format!("{}@v", pkg.name);
        let commits = get_commits_since_last_tag(&repo, &tag_prefix)?;

        if commits.is_empty() {
            if verbose {
                println!("{} {} — no new commits", "○".dimmed(), pkg.name.dimmed());
            }
            continue;
        }

        let bump = commits
            .iter()
            .map(|c| determine_bump(&c.message))
            .max()
            .unwrap_or(BumpType::None);

        if bump == BumpType::None {
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
        let new_version = bump_version(&current_version, bump)?;

        println!(
            "{} {}  {} → {}  ({})",
            "●".green().bold(),
            pkg.name.bold(),
            current_version.dimmed(),
            new_version.green().bold(),
            bump.to_string().cyan()
        );

        if verbose {
            for c in &commits {
                if let Some(line) = c.message.lines().next() {
                    println!("    {} {}", c.hash.dimmed(), line.dimmed());
                }
            }
        }

        if !dry_run {
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

            let tag = format!("{}@v{}", pkg.name, new_version);
            let body = build_section(&new_version, &commits);
            tags_to_create.push((tag.clone(), format!("Release {tag}"), body));
        }

        any_bumped = true;
    }

    if !dry_run && any_bumped {
        let file_refs: Vec<&str> = files_to_commit.iter().map(String::as_str).collect();
        create_commit(&repo, &file_refs, "chore: release [skip ci]")?;
        println!("  ✓ Committed release changes");

        for (tag_name, tag_msg, _) in &tags_to_create {
            create_tag(&repo, tag_name, tag_msg)?;
            println!("  ✓ Created tag {}", tag_name.cyan());
        }

        let tag_refs: Vec<&str> = tags_to_create.iter().map(|(t, _, _)| t.as_str()).collect();
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

        if let Ok(token) = std::env::var("GITHUB_TOKEN")
            && let Some(slug) = get_repo_slug(&repo, &config.workspace.remote)
        {
            for (tag_name, _, body) in &tags_to_create {
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
                for (tag_name, _, body) in &tags_to_create {
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
