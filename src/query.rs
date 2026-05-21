use crate::config::Config;
use crate::error_code::{self, ErrorCodeExt};
use crate::formats::read_version;
use crate::git::{
    TagIndex, find_last_tag_name, find_last_tag_name_with_cache, get_repo_root, open_repo,
};
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
        // Build the tag index ONCE across the multi-package loop. Without
        // it, each find_last_tag_name call ran tag_foreach independently
        // (200 pkg × 200 tag callbacks ≈ 40k invocations) plus per-tag
        // graph_descendant_of (now O(1) via the embedded ancestor set).
        // The index pre-resolves both, leaving per-package work as a
        // linear scan over already-resolved entries.
        let strategy = config.workspace.orphaned_tag_strategy;
        let index = TagIndex::build(&repo).ok();
        let entries: Vec<TagEntry> = config
            .packages
            .iter()
            .map(|pkg| {
                let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
                // Fast path via the index for the Warn strategy; tree-hash /
                // message recovery falls back to the slow per-call form
                // which still uses the embedded ancestor set.
                let tag = match index.as_ref().and_then(|idx| {
                    idx.find_last_tag_name(&prefix, strategy)
                        .map(Some)
                        .or_else(|| {
                            // Non-Warn strategy: index returns None, fall back
                            find_last_tag_name_with_cache(
                                &repo,
                                &prefix,
                                strategy,
                                Some(&idx.ancestors),
                            )
                            .ok()
                        })
                }) {
                    Some(t) => t,
                    None => find_last_tag_name(&repo, &prefix, strategy).unwrap_or(None),
                };
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
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};
    use std::fs;
    use std::path::Path;

    static COMMIT_TIME: std::sync::atomic::AtomicI64 =
        std::sync::atomic::AtomicI64::new(1_800_000_000);

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
