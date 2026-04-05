use crate::config::Config;
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::read_version;
use crate::git::{find_last_tag_name, get_commits_since_last_tag, get_repo_root, open_repo};
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

pub fn run(config_path: Option<&std::path::Path>, output: &OutputFormat) -> Result<()> {
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

    let mut statuses: Vec<PackageStatus> = Vec::new();

    for pkg in &config.packages {
        let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let last_tag = find_last_tag_name(
            &repo,
            &tag_search_prefix,
            config.workspace.orphaned_tag_strategy,
        )?;

        let version = if let Some(vf) = pkg.versioned_files.first() {
            read_version(vf, &root).unwrap_or_else(|_| "unknown".to_string())
        } else {
            "unknown".to_string()
        };

        let commits = get_commits_since_last_tag(
            &repo,
            &tag_search_prefix,
            config.workspace.orphaned_tag_strategy,
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
    use crate::test_utils::with_cwd;
    use git2::{Repository, Signature};
    use std::fs;

    static COMMIT_TIME: std::sync::atomic::AtomicI64 =
        std::sync::atomic::AtomicI64::new(1_900_000_000);

    fn init_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    fn create_commit(repo: &Repository, dir: &std::path::Path, filename: &str, message: &str) {
        let file_path = dir.join(filename);
        fs::write(&file_path, format!("content of {filename}")).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(filename)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let ts = COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let sig = Signature::new("Test", "test@test.com", &git2::Time::new(ts, 0)).unwrap();
        let parents: Vec<git2::Commit> = match repo.head() {
            Ok(head) => vec![head.peel_to_commit().unwrap()],
            Err(_) => vec![],
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap();
    }

    fn create_tag(repo: &Repository, tag_name: &str) {
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight(tag_name, head.as_object(), false)
            .unwrap();
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
        with_cwd(dir.path(), || run(Some(&config_path), &OutputFormat::Text)).unwrap();
    }

    #[test]
    fn status_json_output() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || run(Some(&config_path), &OutputFormat::Json)).unwrap();
    }

    #[test]
    fn status_no_packages_prints_warning() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join(".ferrflow"), r#"{"package": []}"#).unwrap();
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || run(Some(&config_path), &OutputFormat::Text)).unwrap();
    }

    #[test]
    fn status_with_tag_no_changes() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "chore: initial");
        create_tag(&repo, "v1.0.0");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || run(Some(&config_path), &OutputFormat::Text)).unwrap();
    }

    #[test]
    fn status_with_tag_and_new_commits() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "chore: initial");
        create_tag(&repo, "v1.0.0");
        create_commit(&repo, dir.path(), "new.txt", "feat: new feature");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || run(Some(&config_path), &OutputFormat::Text)).unwrap();
    }

    #[test]
    fn status_detects_changes_after_tag() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "chore: initial");
        create_tag(&repo, "v1.0.0");
        create_commit(&repo, dir.path(), "feature.txt", "feat: add feature");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || run(Some(&config_path), &OutputFormat::Json)).unwrap();
    }
}
