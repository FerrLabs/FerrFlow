#[cfg(feature = "cli")]
use crate::conventional_commits::determine_bump;
use crate::conventional_commits::{BumpType, parse_subject};
use anyhow::Result;
use chrono::Local;
use std::path::Path;

pub struct GitLog {
    pub hash: String,
    pub message: String,
}

#[cfg(feature = "cli")]
pub fn generate_only(config_path: Option<&Path>, dry_run: bool) -> Result<()> {
    use crate::config::Config;
    use crate::formats::read_version;
    use crate::git::{get_commits_since_last_tag, get_repo_root, open_repo};
    use crate::versioning::bump_version;
    use colored::Colorize;
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
                    "  Skipping {}: no versioned files configured, cannot determine version.",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_commits(messages: &[&str]) -> Vec<GitLog> {
        messages
            .iter()
            .map(|m| GitLog {
                hash: "abc1234".to_string(),
                message: m.to_string(),
            })
            .collect()
    }

    #[test]
    fn build_section_features_only() {
        let commits = make_commits(&["feat: add login", "feat(ui): new dashboard"]);
        let section = build_section("1.1.0", &commits);
        assert!(section.contains("## [1.1.0]"));
        assert!(section.contains("### Features"));
        assert!(section.contains("- feat: add login"));
        assert!(section.contains("- feat(ui): new dashboard"));
        assert!(!section.contains("### Bug Fixes"));
        assert!(!section.contains("### Breaking Changes"));
    }

    #[test]
    fn build_section_fixes_only() {
        let commits = make_commits(&["fix: null pointer", "perf: faster query"]);
        let section = build_section("1.0.1", &commits);
        assert!(section.contains("### Bug Fixes"));
        assert!(section.contains("- fix: null pointer"));
        assert!(section.contains("- perf: faster query"));
        assert!(!section.contains("### Features"));
    }

    #[test]
    fn build_section_breaking_changes() {
        let commits = make_commits(&["feat!: remove old API"]);
        let section = build_section("2.0.0", &commits);
        assert!(section.contains("### Breaking Changes"));
        assert!(section.contains("- feat!: remove old API"));
    }

    #[test]
    fn build_section_mixed_commits() {
        let commits = make_commits(&[
            "feat: new feature",
            "fix: bug fix",
            "feat!: breaking",
            "chore: update deps",
        ]);
        let section = build_section("2.0.0", &commits);
        assert!(section.contains("### Breaking Changes"));
        assert!(section.contains("### Features"));
        assert!(section.contains("### Bug Fixes"));
        // chore should not appear in any section
        assert!(!section.contains("chore: update deps"));
    }

    #[test]
    fn build_section_empty_commits() {
        let section = build_section("1.0.0", &[]);
        assert!(section.contains("## [1.0.0]"));
        assert!(!section.contains("### "));
    }

    #[test]
    fn update_changelog_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        let commits = make_commits(&["feat: initial"]);
        update_changelog(&path, "myapp", "0.1.0", &commits, BumpType::Minor, false).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Changelog"));
        assert!(content.contains("## [0.1.0]"));
        assert!(content.contains("- feat: initial"));
    }

    #[test]
    fn update_changelog_inserts_before_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(
            &path,
            "# Changelog\n\n## [1.0.0] - 2025-01-01\n\n- old stuff\n",
        )
        .unwrap();
        let commits = make_commits(&["feat: new stuff"]);
        update_changelog(&path, "myapp", "1.1.0", &commits, BumpType::Minor, false).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let pos_new = content.find("## [1.1.0]").unwrap();
        let pos_old = content.find("## [1.0.0]").unwrap();
        assert!(pos_new < pos_old, "new version should come before old");
    }

    #[test]
    fn update_changelog_skips_none_bump() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        update_changelog(&path, "myapp", "1.0.0", &[], BumpType::None, false).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn update_changelog_dry_run_no_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        let commits = make_commits(&["feat: something"]);
        update_changelog(&path, "myapp", "1.0.0", &commits, BumpType::Minor, true).unwrap();
        assert!(!path.exists());
    }
}
