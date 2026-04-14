use crate::config::Config;
use crate::error_code::{self, ErrorCodeExt};
use crate::formats::read_version;
use crate::git::{find_last_tag_name, get_repo_root, open_repo};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct VersionEntry {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct TagEntry {
    name: String,
    tag: Option<String>,
}

pub fn version(
    config_path: Option<&std::path::Path>,
    package: Option<&str>,
    json: bool,
) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        Err(anyhow::anyhow!(
            "No packages configured. Run `ferrflow init` to create a config."
        ))
        .error_code(error_code::QUERY_NO_PACKAGES)?;
    }

    if let Some(name) = package {
        let pkg = config
            .packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("package '{}' not found", name))
            .error_code(error_code::QUERY_PACKAGE_NOT_FOUND)?;

        let version = pkg
            .versioned_files
            .first()
            .map(|vf| read_version(vf, &root))
            .transpose()?
            .unwrap_or_else(|| "unknown".to_string());

        if json {
            println!(
                "{}",
                serde_json::to_string(&VersionEntry {
                    name: pkg.name.clone(),
                    version,
                })?
            );
        } else {
            println!("{version}");
        }
        return Ok(());
    }

    // No package specified
    if config.packages.len() == 1 {
        let pkg = &config.packages[0];
        let version = pkg
            .versioned_files
            .first()
            .map(|vf| read_version(vf, &root))
            .transpose()?
            .unwrap_or_else(|| "unknown".to_string());

        if json {
            println!(
                "{}",
                serde_json::to_string(&VersionEntry {
                    name: pkg.name.clone(),
                    version,
                })?
            );
        } else {
            println!("{version}");
        }
    } else {
        let entries: Vec<VersionEntry> = config
            .packages
            .iter()
            .map(|pkg| {
                let version = pkg
                    .versioned_files
                    .first()
                    .and_then(|vf| read_version(vf, &root).ok())
                    .unwrap_or_else(|| "unknown".to_string());
                VersionEntry {
                    name: pkg.name.clone(),
                    version,
                }
            })
            .collect();

        if json {
            println!("{}", serde_json::to_string(&entries)?);
        } else {
            for e in &entries {
                println!("{}\t{}", e.name, e.version);
            }
        }
    }

    Ok(())
}

pub fn tag(config_path: Option<&std::path::Path>, package: Option<&str>, json: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        Err(anyhow::anyhow!(
            "No packages configured. Run `ferrflow init` to create a config."
        ))
        .error_code(error_code::QUERY_NO_PACKAGES)?;
    }

    if let Some(name) = package {
        let pkg = config
            .packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("package '{}' not found", name))
            .error_code(error_code::QUERY_PACKAGE_NOT_FOUND)?;

        let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let last_tag = find_last_tag_name(&repo, &prefix, config.workspace.orphaned_tag_strategy)?;

        if json {
            println!(
                "{}",
                serde_json::to_string(&TagEntry {
                    name: pkg.name.clone(),
                    tag: last_tag,
                })?
            );
        } else {
            println!("{}", last_tag.unwrap_or_else(|| "none".to_string()));
        }
        return Ok(());
    }

    // No package specified
    if config.packages.len() == 1 {
        let pkg = &config.packages[0];
        let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let last_tag = find_last_tag_name(&repo, &prefix, config.workspace.orphaned_tag_strategy)?;

        if json {
            println!(
                "{}",
                serde_json::to_string(&TagEntry {
                    name: pkg.name.clone(),
                    tag: last_tag,
                })?
            );
        } else {
            println!("{}", last_tag.unwrap_or_else(|| "none".to_string()));
        }
    } else {
        let entries: Vec<TagEntry> = config
            .packages
            .iter()
            .map(|pkg| {
                let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
                let tag =
                    find_last_tag_name(&repo, &prefix, config.workspace.orphaned_tag_strategy)
                        .unwrap_or(None);
                TagEntry {
                    name: pkg.name.clone(),
                    tag,
                }
            })
            .collect();

        if json {
            println!("{}", serde_json::to_string(&entries)?);
        } else {
            for e in &entries {
                println!("{}\t{}", e.name, e.tag.as_deref().unwrap_or("none"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::with_cwd;
    use git2::{Repository, Signature};
    use std::fs;

    static COMMIT_TIME: std::sync::atomic::AtomicI64 =
        std::sync::atomic::AtomicI64::new(1_800_000_000);

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
            "[package]\nname = \"my-app\"\nversion = \"1.2.3\"\n",
        )
        .unwrap();
        fs::write(
            dir.join(".ferrflow"),
            r#"{"package": [{"name": "my-app", "path": ".", "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}]}]}"#,
        )
        .unwrap();
    }

    fn setup_monorepo(dir: &std::path::Path) {
        fs::create_dir_all(dir.join("packages/core")).unwrap();
        fs::create_dir_all(dir.join("packages/cli")).unwrap();
        fs::write(
            dir.join("packages/core/package.json"),
            r#"{"name": "core", "version": "2.0.0"}"#,
        )
        .unwrap();
        fs::write(
            dir.join("packages/cli/package.json"),
            r#"{"name": "cli", "version": "3.1.0"}"#,
        )
        .unwrap();
        fs::write(
            dir.join(".ferrflow"),
            r#"{
                "package": [
                    {"name": "core", "path": "packages/core", "versionedFiles": [{"path": "packages/core/package.json", "format": "json"}]},
                    {"name": "cli", "path": "packages/cli", "versionedFiles": [{"path": "packages/cli/package.json", "format": "json"}]}
                ]
            }"#,
        )
        .unwrap();
    }

    // -----------------------------------------------------------------------
    // version()
    // -----------------------------------------------------------------------

    #[test]
    fn version_single_package() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || version(Some(&config_path), None, false)).unwrap();
    }

    #[test]
    fn version_single_package_json() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || version(Some(&config_path), None, true)).unwrap();
    }

    #[test]
    fn version_specific_package_in_monorepo() {
        let (dir, repo) = init_repo();
        setup_monorepo(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || {
            version(Some(&config_path), Some("core"), false)
        })
        .unwrap();
    }

    #[test]
    fn version_unknown_package_errors() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        let result = with_cwd(dir.path(), || {
            version(Some(&config_path), Some("nonexistent"), false)
        });
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("not found"));
    }

    #[test]
    fn version_monorepo_all_text() {
        let (dir, repo) = init_repo();
        setup_monorepo(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || version(Some(&config_path), None, false)).unwrap();
    }

    #[test]
    fn version_monorepo_all_json() {
        let (dir, repo) = init_repo();
        setup_monorepo(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || version(Some(&config_path), None, true)).unwrap();
    }

    #[test]
    fn version_no_packages_errors() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join(".ferrflow"), r#"{"package": []}"#).unwrap();
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        let result = with_cwd(dir.path(), || version(Some(&config_path), None, false));
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("No packages"));
    }

    // -----------------------------------------------------------------------
    // tag()
    // -----------------------------------------------------------------------

    #[test]
    fn tag_single_package_no_tags() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || tag(Some(&config_path), None, false)).unwrap();
    }

    #[test]
    fn tag_single_package_with_existing_tag() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        create_tag(&repo, "v1.2.3");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || tag(Some(&config_path), None, false)).unwrap();
    }

    #[test]
    fn tag_json_output() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        create_tag(&repo, "v1.2.3");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || tag(Some(&config_path), None, true)).unwrap();
    }

    #[test]
    fn tag_unknown_package_errors() {
        let (dir, repo) = init_repo();
        setup_single_package(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        let result = with_cwd(dir.path(), || {
            tag(Some(&config_path), Some("nonexistent"), false)
        });
        assert!(result.is_err());
    }

    #[test]
    fn tag_monorepo_all_packages() {
        let (dir, repo) = init_repo();
        setup_monorepo(dir.path());
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        with_cwd(dir.path(), || tag(Some(&config_path), None, false)).unwrap();
    }

    #[test]
    fn tag_no_packages_errors() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join(".ferrflow"), r#"{"package": []}"#).unwrap();
        create_commit(&repo, dir.path(), "init.txt", "initial");
        let config_path = dir.path().join(".ferrflow");
        let result = with_cwd(dir.path(), || tag(Some(&config_path), None, false));
        assert!(result.is_err());
    }
}
