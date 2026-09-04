use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::config::package::{FileFormat, PackageConfig, VersionedFile};
use crate::config::workspace::WorkspaceConfig;
use crate::error_code::{self, ErrorCodeExt};

use super::{MigrationReport, Source, write_and_report};

pub(super) const CONFIG_FILE: &str = ".changeset/config.json";

pub(super) fn detect() -> Option<PathBuf> {
    let p = PathBuf::from(CONFIG_FILE);
    p.exists().then_some(p)
}

pub(super) fn run() -> Result<()> {
    let path = detect().ok_or_else(|| anyhow::anyhow!("no {CONFIG_FILE} found"))?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("could not read {}: {e}", path.display()))?;
    let (config, report) = build(&raw, Path::new("."))?;
    write_and_report(Source::Changesets, &path, &config, &report)
}

#[derive(Debug, Deserialize, Default)]
struct ChangesetsConfig {
    #[serde(default, rename = "baseBranch")]
    base_branch: Option<String>,
    #[serde(default)]
    linked: Vec<Vec<String>>,
    #[serde(default)]
    fixed: Vec<Vec<String>>,
    #[serde(default, rename = "updateInternalDependencies")]
    update_internal_dependencies: Option<String>,
    #[serde(default)]
    access: Option<String>,
    #[serde(default)]
    ignore: Vec<String>,
}

pub(super) fn build(raw: &str, root: &Path) -> Result<(Config, MigrationReport)> {
    let cs: ChangesetsConfig = serde_json::from_str(raw)
        .map_err(|e| anyhow::anyhow!("could not parse {CONFIG_FILE} as JSON: {e}"))
        .error_code(error_code::CONFIG_INVALID_JSON)?;

    let mut report = MigrationReport::default();
    let mut workspace = WorkspaceConfig::default();

    report.warnings.push(
        "changesets versions from `.changeset/*.md` files — ferrflow versions from conventional \
         commits instead. Adopt Conventional Commits; your existing `.changeset/*.md` files are \
         not read."
            .to_string(),
    );

    if let Some(base) = &cs.base_branch {
        workspace.branch = base.clone();
        report.mapped.push(format!("baseBranch → branch ({base})"));
    }

    if !cs.linked.is_empty() {
        workspace.linked = cs.linked.clone();
        report
            .mapped
            .push(format!("linked → {} linked group(s)", cs.linked.len()));
    }
    if !cs.fixed.is_empty() {
        workspace.fixed = cs.fixed.clone();
        report
            .mapped
            .push(format!("fixed → {} fixed group(s)", cs.fixed.len()));
    }

    if let Some(access) = &cs.access {
        report.ignored.push(format!(
            "access ({access}) — npm publish access isn't a ferrflow config concern; \
             configure `publishers` if you publish."
        ));
    }
    if let Some(uid) = &cs.update_internal_dependencies {
        report.ignored.push(format!(
            "updateInternalDependencies ({uid}) — ferrflow bumps dependents via `dependsOn` \
             cascades; wire those up per package."
        ));
    }
    if !cs.ignore.is_empty() {
        report.warnings.push(format!(
            "ignore lists {} package(s) that changesets never versions — omit them from the \
             `package` list to get the same effect.",
            cs.ignore.len()
        ));
    }

    let discovered = super::workspace_packages::discover(root);
    let packages = if discovered.is_empty() {
        report.warnings.push(
            "no JS workspace found (`workspaces` in package.json or pnpm-workspace.yaml), so a \
             single root package was scaffolded. If this is a monorepo, list each publishable \
             package under `package`."
                .to_string(),
        );
        vec![scaffold_package(
            crate::config::migrate::default_package_name(),
            ".".to_string(),
        )]
    } else {
        report.mapped.push(format!(
            "workspace globs → {} package(s) discovered",
            discovered.len()
        ));
        discovered
            .into_iter()
            .map(|p| scaffold_package(p.name, p.path))
            .collect()
    };

    warn_about_unmatched_groups(&cs, &packages, &mut report);

    Ok((
        Config {
            include: Vec::new(),
            workspace,
            packages,
        },
        report,
    ))
}

fn scaffold_package(name: String, path: String) -> PackageConfig {
    let manifest = if path == "." {
        "package.json".to_string()
    } else {
        format!("{path}/package.json")
    };
    let changelog = if path == "." {
        "CHANGELOG.md".to_string()
    } else {
        format!("{path}/CHANGELOG.md")
    };
    PackageConfig {
        name,
        path,
        versioned_files: vec![VersionedFile {
            path: manifest,
            format: FileFormat::Json,
            selector: None,
        }],
        changelog: Some(changelog),
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        version_template: None,
        hooks: None,
        floating_tags: None,
        latest_tag: None,
        build_metadata: None,
        publishers: vec![],
        update_lockfiles: None,
        version_source: None,
    }
}

fn warn_about_unmatched_groups(
    cs: &ChangesetsConfig,
    packages: &[PackageConfig],
    report: &mut MigrationReport,
) {
    let known: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let mut unmatched: Vec<&str> = cs
        .linked
        .iter()
        .chain(cs.fixed.iter())
        .flatten()
        .map(String::as_str)
        .filter(|name| !known.contains(name))
        .collect();
    unmatched.sort_unstable();
    unmatched.dedup();

    if !unmatched.is_empty() {
        report.warnings.push(format!(
            "linked/fixed reference {} package(s) that were not discovered ({}). Add them under \
             `package` or drop them from the group, otherwise `ferrflow validate` fails.",
            unmatched.len(),
            unmatched.join(", ")
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_ok(raw: &str) -> (Config, MigrationReport) {
        let dir = tempfile::tempdir().unwrap();
        build(raw, dir.path()).expect("valid changesets config")
    }

    fn write(root: &std::path::Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn workspace_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"name": "root", "private": true}"#);
        write(root, "pnpm-workspace.yaml", "packages:\n  - 'packages/*'\n");
        write(
            root,
            "packages/a/package.json",
            r#"{"name": "@acme/a", "version": "1.0.0"}"#,
        );
        write(
            root,
            "packages/b/package.json",
            r#"{"name": "@acme/b", "version": "2.0.0"}"#,
        );
        dir
    }

    #[test]
    fn workspace_packages_are_scaffolded_one_entry_each() {
        let dir = workspace_dir();
        let (cfg, report) = build(r#"{}"#, dir.path()).unwrap();

        let names: Vec<&str> = cfg.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["@acme/a", "@acme/b"]);

        let a = &cfg.packages[0];
        assert_eq!(a.path, "packages/a");
        assert_eq!(a.versioned_files[0].path, "packages/a/package.json");
        assert_eq!(a.changelog.as_deref(), Some("packages/a/CHANGELOG.md"));

        assert!(report.mapped.iter().any(|m| m.contains("2 package(s)")));
        assert!(
            !report.warnings.iter().any(|w| w.contains("single root")),
            "the hand-list-your-packages warning is obsolete once discovery works"
        );
    }

    #[test]
    fn discovered_packages_satisfy_the_linked_groups() {
        let dir = workspace_dir();
        let (cfg, report) = build(r#"{"linked": [["@acme/a", "@acme/b"]]}"#, dir.path()).unwrap();

        let names: Vec<&str> = cfg.packages.iter().map(|p| p.name.as_str()).collect();
        for member in cfg.workspace.linked.iter().flatten() {
            assert!(
                names.contains(&member.as_str()),
                "{member} is in a linked group but was not scaffolded"
            );
        }
        assert!(!report.warnings.iter().any(|w| w.contains("not discovered")));
    }

    #[test]
    fn a_group_member_outside_the_workspace_is_warned_about() {
        let dir = workspace_dir();
        let (_, report) = build(r#"{"fixed": [["@acme/a", "@acme/ghost"]]}"#, dir.path()).unwrap();
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("not discovered") && w.contains("@acme/ghost")),
            "warnings were: {:?}",
            report.warnings
        );
    }

    #[test]
    fn a_repo_without_a_workspace_still_gets_a_root_package() {
        let (cfg, report) = build_ok("{}");
        assert_eq!(cfg.packages.len(), 1);
        assert_eq!(cfg.packages[0].path, ".");
        assert_eq!(cfg.packages[0].versioned_files[0].path, "package.json");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("no JS workspace"))
        );
    }

    #[test]
    fn base_branch_maps_to_branch() {
        let (cfg, report) = build_ok(r#"{"baseBranch": "develop"}"#);
        assert_eq!(cfg.workspace.branch, "develop");
        assert!(report.mapped.iter().any(|m| m.contains("baseBranch")));
    }

    #[test]
    fn linked_and_fixed_groups_carry_over() {
        let (cfg, _) =
            build_ok(r#"{"linked": [["@acme/a", "@acme/b"]], "fixed": [["@acme/c", "@acme/d"]]}"#);
        assert_eq!(cfg.workspace.linked, vec![vec!["@acme/a", "@acme/b"]]);
        assert_eq!(cfg.workspace.fixed, vec![vec!["@acme/c", "@acme/d"]]);
    }

    #[test]
    fn the_commit_model_difference_is_always_warned() {
        let (_, report) = build_ok("{}");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("conventional commits")),
            "the changeset-vs-commit difference must always be surfaced"
        );
    }

    #[test]
    fn access_is_reported_as_ignored() {
        let (_, report) = build_ok(r#"{"access": "public"}"#);
        assert!(report.ignored.iter().any(|i| i.contains("access")));
    }

    #[test]
    fn ignore_list_warns() {
        let (_, report) = build_ok(r#"{"ignore": ["@acme/internal"]}"#);
        assert!(report.warnings.iter().any(|w| w.contains("ignore")));
    }

    #[test]
    fn malformed_json_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(build("{ not json", dir.path()).is_err());
    }
}
