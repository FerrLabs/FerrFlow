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
    packages: &[String],
    all: bool,
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

    let targets = resolve_targets(&config, packages, all, triggering_tag().as_deref())?;

    let mut ran = 0usize;
    for pkg in targets {
        if pkg.publishers.is_empty() {
            continue;
        }
        let version = current_version(&repo, pkg, &config, &root)?;
        let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &version);
        let package_path = root.join(&pkg.path);

        tracing::info!(
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
        tracing::warn!(
            "{}",
            "No publishers configured on any package — nothing to publish.".yellow()
        );
    }

    Ok(())
}

fn resolve_targets<'a>(
    config: &'a Config,
    packages: &[String],
    all: bool,
    trigger_tag: Option<&str>,
) -> Result<Vec<&'a PackageConfig>> {
    if !packages.is_empty() {
        return packages
            .iter()
            .map(|name| {
                config
                    .packages
                    .iter()
                    .find(|p| &p.name == name)
                    .ok_or_else(|| anyhow::anyhow!("package '{name}' not found"))
                    .error_code(error_code::QUERY_PACKAGE_NOT_FOUND)
            })
            .collect();
    }
    if !all
        && let Some(tag) = trigger_tag
        && let Some(pkg) = match_package_for_tag(config, tag)
    {
        return Ok(vec![pkg]);
    }
    Ok(config.packages.iter().collect())
}

fn triggering_tag() -> Option<String> {
    if let Ok(github_ref) = std::env::var("GITHUB_REF")
        && let Some(tag) = github_ref.strip_prefix("refs/tags/")
        && !tag.is_empty()
    {
        return Some(tag.to_string());
    }
    match std::env::var("CI_COMMIT_TAG") {
        Ok(tag) if !tag.is_empty() => Some(tag),
        _ => None,
    }
}

fn match_package_for_tag<'a>(config: &'a Config, tag: &str) -> Option<&'a PackageConfig> {
    let is_monorepo = config.is_monorepo();
    config
        .packages
        .iter()
        .filter_map(|pkg| {
            let prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
            tag.starts_with(prefix.as_str())
                .then_some((pkg, prefix.len()))
        })
        .max_by_key(|(_, len)| *len)
        .map(|(pkg, _)| pkg)
}

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
        let err = with_cwd(dir.path(), || run(Some(&cfg), &[], false, true, false)).unwrap_err();
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
        let err = with_cwd(dir.path(), || {
            run(Some(&cfg), &["nope".to_string()], false, true, false)
        })
        .unwrap_err();
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
        with_cwd(dir.path(), || run(Some(&cfg), &[], false, true, false)).unwrap();
    }

    #[test]
    fn dry_run_dispatches_publisher_against_current_version() {
        let (dir, _repo) = init_repo();
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
        with_cwd(dir.path(), || {
            run(Some(&cfg), &["a".to_string()], false, true, false)
        })
        .unwrap();
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
        git(dir.path(), &["tag", "v1.4.0"]);
        let config = Config::load(dir.path(), None).unwrap();
        let v = current_version(&repo, &config.packages[0], &config, dir.path()).unwrap();
        assert_eq!(v, "1.4.0");
    }

    fn two_pkg_config(dir: &Path) -> Config {
        let cfg = write_config(
            dir,
            r#"{"package": [
                {"name": "api", "path": "api", "versionedFiles": [{"path": "api/Cargo.toml", "format": "toml"}]},
                {"name": "web", "path": "web", "versionedFiles": [{"path": "web/Cargo.toml", "format": "toml"}]}
            ]}"#,
        );
        Config::load(dir, Some(&cfg)).unwrap()
    }

    fn names(targets: Vec<&PackageConfig>) -> Vec<String> {
        targets.iter().map(|p| p.name.clone()).collect()
    }

    #[test]
    fn resolve_targets_explicit_packages_win() {
        let dir = tempfile::tempdir().unwrap();
        let config = two_pkg_config(dir.path());
        let targets =
            resolve_targets(&config, &["web".to_string()], false, Some("api@v2.2.1")).unwrap();
        assert_eq!(names(targets), vec!["web"]);
    }

    #[test]
    fn resolve_targets_auto_detects_from_triggering_tag() {
        let dir = tempfile::tempdir().unwrap();
        let config = two_pkg_config(dir.path());
        let targets = resolve_targets(&config, &[], false, Some("api@v2.2.1")).unwrap();
        assert_eq!(names(targets), vec!["api"]);
    }

    #[test]
    fn resolve_targets_all_flag_overrides_tag_scope() {
        let dir = tempfile::tempdir().unwrap();
        let config = two_pkg_config(dir.path());
        let targets = resolve_targets(&config, &[], true, Some("api@v2.2.1")).unwrap();
        assert_eq!(names(targets), vec!["api", "web"]);
    }

    #[test]
    fn resolve_targets_unmatched_tag_falls_back_to_all() {
        let dir = tempfile::tempdir().unwrap();
        let config = two_pkg_config(dir.path());
        assert_eq!(
            names(resolve_targets(&config, &[], false, Some("ghost@v1.0.0")).unwrap()),
            vec!["api", "web"]
        );
        assert_eq!(
            names(resolve_targets(&config, &[], false, None).unwrap()),
            vec!["api", "web"]
        );
    }

    #[test]
    fn resolve_targets_unknown_explicit_package_errors() {
        let dir = tempfile::tempdir().unwrap();
        let config = two_pkg_config(dir.path());
        assert!(resolve_targets(&config, &["nope".to_string()], false, None).is_err());
    }

    #[test]
    fn match_package_maps_tag_to_owning_package() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_config(
            dir.path(),
            r#"{"package": [
                {"name": "api", "path": "api", "versionedFiles": [{"path": "api/Cargo.toml", "format": "toml"}]},
                {"name": "api-core", "path": "api-core", "versionedFiles": [{"path": "api-core/Cargo.toml", "format": "toml"}]}
            ]}"#,
        );
        let config = Config::load(dir.path(), Some(&cfg)).unwrap();
        assert_eq!(
            match_package_for_tag(&config, "api-core@v1.0.0")
                .unwrap()
                .name,
            "api-core"
        );
        assert_eq!(
            match_package_for_tag(&config, "api@v1.0.0").unwrap().name,
            "api"
        );
        assert!(match_package_for_tag(&config, "v1.0.0").is_none());
    }
}
