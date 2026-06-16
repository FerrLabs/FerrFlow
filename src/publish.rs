use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::{Config, PackageConfig};
use crate::error_code::{self, ErrorCodeExt};
use crate::formats::read_version;
use crate::git::{Repository, find_highest_semver_tag_with_cache, get_repo_root, open_repo};
use crate::publishers::{self, PublishContext};

pub fn run(
    config_path: Option<&Path>,
    package: Option<&str>,
    dry_run: bool,
    verbose: bool,
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

    let targets: Vec<&PackageConfig> = match package {
        Some(name) => vec![
            config
                .packages
                .iter()
                .find(|p| p.name == name)
                .ok_or_else(|| anyhow::anyhow!("package '{name}' not found"))
                .error_code(error_code::QUERY_PACKAGE_NOT_FOUND)?,
        ],
        None => config.packages.iter().collect(),
    };

    let mut ran = 0usize;
    for pkg in targets {
        if pkg.publishers.is_empty() {
            continue;
        }
        let version = current_version(&repo, pkg, &config, &root)?;
        let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &version);
        let package_path = root.join(&pkg.path);

        println!(
            "{} {} @ {}",
            "publish".bold(),
            pkg.name.cyan(),
            version.cyan()
        );
        let ctx = PublishContext {
            package_name: &pkg.name,
            package_path: &package_path,
            new_version: &version,
            tag: &tag,
            registries: &config.workspace.registries,
            dry_run,
            verbose,
        };
        publishers::run_all(&pkg.publishers, &ctx)?;
        ran += 1;
    }

    if ran == 0 {
        println!(
            "{}",
            "No publishers configured on any package — nothing to publish.".yellow()
        );
    }

    Ok(())
}

/// Resolve the version currently on disk for `pkg` — the last released
/// version, since `publish` runs after `release` has written and tagged
/// it. Falls back to the highest matching tag for tag-only packages
/// (#531) that carry no `versionedFiles`.
fn current_version(
    repo: &Repository,
    pkg: &PackageConfig,
    config: &Config,
    root: &Path,
) -> Result<String> {
    if let Some(vf) = pkg.versioned_files.first() {
        return read_version(vf, root);
    }
    let prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
    find_highest_semver_tag_with_cache(repo, &prefix, config.workspace.orphaned_tag_strategy, None)?
        .map(|(_, version)| version)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "package '{}' has no versionedFiles and no matching release tag; \
                 run `ferrflow release` before `ferrflow publish`",
                pkg.name
            )
        })
        .error_code(error_code::QUERY_PACKAGE_NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};
    use std::fs;

    fn write_config(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join(".ferrflow");
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn no_packages_errors() {
        let (dir, repo) = init_repo();
        let cfg = write_config(dir.path(), r#"{"package": []}"#);
        commit_file(dir.path(), "init.txt", "x", "chore: init", 1_800_000_001);
        let _ = &repo;
        let err = with_cwd(dir.path(), || run(Some(&cfg), None, true, false)).unwrap_err();
        assert!(format!("{err:?}").contains("No packages"));
    }

    #[test]
    fn unknown_package_errors() {
        let (dir, _repo) = init_repo();
        let cfg = write_config(
            dir.path(),
            r#"{"package": [{"name": "a", "path": ".", "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}]}]}"#,
        );
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        commit_file(dir.path(), "init.txt", "x", "chore: init", 1_800_000_002);
        let err = with_cwd(dir.path(), || run(Some(&cfg), Some("nope"), true, false)).unwrap_err();
        assert!(format!("{err:?}").contains("not found"));
    }

    #[test]
    fn package_without_publishers_is_a_noop() {
        let (dir, _repo) = init_repo();
        let cfg = write_config(
            dir.path(),
            r#"{"package": [{"name": "a", "path": ".", "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}]}]}"#,
        );
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        commit_file(dir.path(), "init.txt", "x", "chore: init", 1_800_000_003);
        // No publishers → nothing runs, no error.
        with_cwd(dir.path(), || run(Some(&cfg), None, true, false)).unwrap();
    }

    #[test]
    fn dry_run_dispatches_publisher_against_current_version() {
        let (dir, _repo) = init_repo();
        // cargo publisher with no registry targets crates.io, so it skips
        // token validation and short-circuits to DryRun without spawning
        // cargo — exercising the read-version → dispatch path end to end.
        let cfg = write_config(
            dir.path(),
            r#"{"package": [{"name": "a", "path": ".", "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}], "publishers": [{"kind": "cargo"}]}]}"#,
        );
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"2.3.4\"\n",
        )
        .unwrap();
        commit_file(dir.path(), "init.txt", "x", "chore: init", 1_800_000_004);
        with_cwd(dir.path(), || run(Some(&cfg), Some("a"), true, false)).unwrap();
    }

    #[test]
    fn current_version_reads_from_versioned_file() {
        let (dir, repo) = init_repo();
        write_config(
            dir.path(),
            r#"{"package": [{"name": "a", "path": ".", "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}]}]}"#,
        );
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"9.9.9\"\n",
        )
        .unwrap();
        commit_file(dir.path(), "init.txt", "x", "chore: init", 1_800_000_005);
        let config = Config::load(dir.path(), None).unwrap();
        let v = current_version(&repo, &config.packages[0], &config, dir.path()).unwrap();
        assert_eq!(v, "9.9.9");
    }

    #[test]
    fn current_version_falls_back_to_tag_for_tag_only_package() {
        let (dir, repo) = init_repo();
        write_config(
            dir.path(),
            r#"{"package": [{"name": "img", "path": ".", "changelog": "CHANGELOG.md"}]}"#,
        );
        commit_file(dir.path(), "init.txt", "x", "chore: init", 1_800_000_006);
        // Single-package repo → default tag template is `v{version}`.
        git(dir.path(), &["tag", "v1.4.0"]);
        let config = Config::load(dir.path(), None).unwrap();
        let v = current_version(&repo, &config.packages[0], &config, dir.path()).unwrap();
        assert_eq!(v, "1.4.0");
    }
}
