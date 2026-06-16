use crate::config::Config;
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::read_version;
use crate::git::{find_last_tag_name, get_commits_since_last_tag, get_repo_root, open_repo};
use crate::timing::Timing;
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Serialize)]
struct PackageStatus {
    name: String,
    version: String,
    last_tag: Option<String>,
    has_changes: bool,
}

pub fn run(
    config_path: Option<&std::path::Path>,
    output: &OutputFormat,
    timing: &mut Timing,
) -> Result<()> {
    let repo = timing.stage("open_repo", || open_repo(&std::env::current_dir()?))?;
    let root = get_repo_root(&repo)?;
    let config = timing.stage("load config", || Config::load(&root, config_path))?;

    if config.packages.is_empty() {
        println!(
            "{}",
            "No packages configured. Run `ferrflow init` to create a ferrflow config.".yellow()
        );
        return Ok(());
    }

    let manifest = crate::manifest::manifest_path(&config, &root)
        .and_then(|path| crate::manifest::read_if_present(&path).ok().flatten());

    let mut statuses: Vec<PackageStatus> = Vec::new();

    let compute_start = std::time::Instant::now();
    for pkg in &config.packages {
        let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let last_tag = find_last_tag_name(
            &repo,
            &tag_search_prefix,
            config.workspace.orphaned_tag_strategy,
        )?;

        let version = manifest
            .as_ref()
            .and_then(|m| m.version_of(&pkg.name))
            .map(str::to_string)
            .or_else(|| {
                pkg.versioned_files
                    .first()
                    .and_then(|vf| read_version(vf, &root).ok())
            })
            .unwrap_or_else(|| "unknown".to_string());

        let skip_markers = config.workspace.effective_commit_skip_markers();
        let commits = get_commits_since_last_tag(
            &repo,
            &tag_search_prefix,
            config.workspace.orphaned_tag_strategy,
            &skip_markers,
        )?;
        let has_changes = commits
            .iter()
            .map(|c| determine_bump(&c.message))
            .any(|b| b != BumpType::None);

        statuses.push(PackageStatus {
            name: pkg.name.clone(),
            version,
            last_tag,
            has_changes,
        });
    }
    timing.record("per-package compute", compute_start.elapsed());

    match output {
        OutputFormat::Text => print_text(&statuses),
        OutputFormat::Json => print_json(&statuses)?,
    }

    Ok(())
}

fn print_text(statuses: &[PackageStatus]) {
    for s in statuses {
        let dot = if s.has_changes {
            "●".green().bold()
        } else {
            "○".dimmed()
        };
        let tag_info = match &s.last_tag {
            Some(tag) => format!("(tag: {})", tag),
            None => "(no tag)".to_string(),
        };
        println!(
            "{} {:<20} v{}   {}",
            dot,
            s.name,
            s.version,
            tag_info.dimmed()
        );
    }
}

fn print_json(statuses: &[PackageStatus]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&statuses)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};
    use std::fs;
    use std::path::Path;

    static COMMIT_TIME: std::sync::atomic::AtomicI64 =
        std::sync::atomic::AtomicI64::new(1_900_000_000);

    fn next_ts() -> i64 {
        COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    fn create_commit(_repo: &crate::git::Repository, dir: &Path, filename: &str, message: &str) {
        commit_file(
            dir,
            filename,
            &format!("content of {filename}"),
            message,
            next_ts(),
        );
    }

    fn create_tag(repo: &crate::git::Repository, tag_name: &str) {
        let workdir = repo.workdir().expect("workdir");
        git(workdir, &["tag", tag_name]);
    }

    fn setup_single_package(dir: &std::path::Path) {
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"my-app\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        fs::write(
            dir.join(".ferrflow"),
            r#"{"package": [{"name": "my-app", "path": ".", "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}]}]}"#,
        )
        .unwrap();
    }

    #[test]
    fn status_text_output() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || {
            run(
                Some(&config_path),
                &OutputFormat::Text,
                &mut Timing::new(false),
            )
        })
        .unwrap();
    }

    #[test]
    fn status_json_output() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || {
            run(
                Some(&config_path),
                &OutputFormat::Json,
                &mut Timing::new(false),
            )
        })
        .unwrap();
    }

    #[test]
    fn status_no_packages_prints_warning() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join(".ferrflow"), r#"{"package": []}"#).unwrap();
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || {
            run(
                Some(&config_path),
                &OutputFormat::Text,
                &mut Timing::new(false),
            )
        })
        .unwrap();
    }

    #[test]
    fn status_with_tag_no_changes() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "chore: initial");
        create_tag(&repo, "v1.0.0");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || {
            run(
                Some(&config_path),
                &OutputFormat::Text,
                &mut Timing::new(false),
            )
        })
        .unwrap();
    }

    #[test]
    fn status_with_tag_and_new_commits() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "chore: initial");
        create_tag(&repo, "v1.0.0");
        create_commit(&repo, dir.path(), "new.txt", "feat: new feature");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || {
            run(
                Some(&config_path),
                &OutputFormat::Text,
                &mut Timing::new(false),
            )
        })
        .unwrap();
    }

    #[test]
    fn status_detects_changes_after_tag() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "chore: initial");
        create_tag(&repo, "v1.0.0");
        create_commit(&repo, dir.path(), "feature.txt", "feat: add feature");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || {
            run(
                Some(&config_path),
                &OutputFormat::Json,
                &mut Timing::new(false),
            )
        })
        .unwrap();
    }
}
