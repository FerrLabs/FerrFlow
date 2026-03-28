use crate::config::Config;
use crate::conventional_commits::{BumpType, determine_bump, parse_subject};
use crate::formats::read_version;
use crate::git::{GitLog, get_commits_since_last_tag, get_repo_root, open_repo};
use crate::versioning::bump_version;
use anyhow::Result;
use chrono::Local;
use colored::Colorize;
use std::path::Path;

pub fn generate_only(config_path: Option<&Path>, dry_run: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        println!(
            "{}",
            "No packages configured. Run `ferrflow init` to create a ferrflow config.".yellow()
        );
        return Ok(());
    }

    for pkg in &config.packages {
        let tag_prefix = format!("{}@v", pkg.name);
        let commits = get_commits_since_last_tag(&repo, &tag_prefix)?;

        if commits.is_empty() {
            continue;
        }

        let bump = commits
            .iter()
            .map(|c| determine_bump(&c.message))
            .max()
            .unwrap_or(BumpType::None);

        if bump == BumpType::None {
            continue;
        }

        let Some(vf) = pkg.versioned_files.first() else {
            println!(
                "{}",
                format!(
                    "  Skipping {}: no versioned_files configured, cannot determine version.",
                    pkg.name
                )
                .yellow()
            );
            continue;
        };

        let current_version = read_version(vf, &root)?;
        let new_version = bump_version(&current_version, bump)?;

        let changelog_path = match &pkg.changelog {
            Some(rel) => root.join(rel),
            None => {
                println!(
                    "{}",
                    format!(
                        "  No changelog configured for '{}', defaulting to CHANGELOG.md.",
                        pkg.name
                    )
                    .yellow()
                );
                root.join("CHANGELOG.md")
            }
        };

        update_changelog(
            &changelog_path,
            &pkg.name,
            &new_version,
            &commits,
            bump,
            dry_run,
        )?;
    }

    Ok(())
}

pub fn build_section(new_version: &str, commits: &[GitLog]) -> String {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let mut features = Vec::new();
    let mut fixes = Vec::new();
    let mut breaking = Vec::new();

    for commit in commits {
        let subject = parse_subject(&commit.message);
        let first_line = commit.message.lines().next().unwrap_or("").to_lowercase();

        if commit.message.contains("BREAKING CHANGE") || first_line.contains("!:") {
            breaking.push(format!("- {subject}"));
        } else if first_line.starts_with("feat") {
            features.push(format!("- {subject}"));
        } else if first_line.starts_with("fix") || first_line.starts_with("perf") {
            fixes.push(format!("- {subject}"));
        }
    }

    let mut section = format!("\n## [{new_version}] - {date}\n");

    if !breaking.is_empty() {
        section.push_str("\n### Breaking Changes\n\n");
        section.push_str(&breaking.join("\n"));
        section.push('\n');
    }
    if !features.is_empty() {
        section.push_str("\n### Features\n\n");
        section.push_str(&features.join("\n"));
        section.push('\n');
    }
    if !fixes.is_empty() {
        section.push_str("\n### Bug Fixes\n\n");
        section.push_str(&fixes.join("\n"));
        section.push('\n');
    }

    section
}

pub fn update_changelog(
    changelog_path: &Path,
    package_name: &str,
    new_version: &str,
    commits: &[GitLog],
    bump: BumpType,
    dry_run: bool,
) -> Result<()> {
    if bump == BumpType::None {
        return Ok(());
    }

    let section = build_section(new_version, commits);

    if dry_run {
        println!(
            "  [dry-run] Would update {}: {}",
            changelog_path.display(),
            section.trim()
        );
        return Ok(());
    }

    let existing = if changelog_path.exists() {
        std::fs::read_to_string(changelog_path)?
    } else {
        format!(
            "# Changelog\n\nAll notable changes to `{package_name}` will be documented here.\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/).\n"
        )
    };

    let new_content = if let Some(pos) = existing.find("\n## ") {
        format!("{}{}{}", &existing[..pos], section, &existing[pos..])
    } else {
        format!("{}\n{}", existing.trim_end(), section)
    };

    std::fs::write(changelog_path, new_content)?;
    println!("  ✓ Updated {}", changelog_path.display());
    Ok(())
}
