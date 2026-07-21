use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

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
    let (config, report) = build(&raw)?;
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

pub(super) fn build(raw: &str) -> Result<(Config, MigrationReport)> {
    let cs: ChangesetsConfig = serde_json::from_str(raw)
        .map_err(|e| anyhow::anyhow!("could not parse {CONFIG_FILE} as JSON: {e}"))
        .error_code(error_code::CONFIG_INVALID_JSON)?;

    let mut report = MigrationReport::default();
    let mut workspace = WorkspaceConfig::default();

    // The defining difference: changesets versions from hand-written
    // `.changeset/*.md` intent files; ferrflow versions from conventional
    // commits. This is the one thing a user MUST know after migrating.
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

    // changesets discovers packages from the JS workspace; ferrflow needs them
    // listed explicitly. Scaffold a single root package and tell the user.
    let package = PackageConfig {
        name: crate::config::migrate::default_package_name(),
        path: ".".to_string(),
        versioned_files: vec![VersionedFile {
            path: "package.json".to_string(),
            format: FileFormat::Json,
            selector: None,
        }],
        changelog: Some("CHANGELOG.md".to_string()),
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    report.warnings.push(
        "changesets is a monorepo tool; ferrflow can't read your workspace globs, so it \
         scaffolded a single root package. List each workspace package under `package` (one entry \
         per publishable package)."
            .to_string(),
    );

    Ok((
        Config {
            workspace,
            packages: vec![package],
        },
        report,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_ok(raw: &str) -> (Config, MigrationReport) {
        build(raw).expect("valid changesets config")
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
    fn monorepo_scaffold_warning_is_present() {
        let (cfg, report) = build_ok("{}");
        assert_eq!(cfg.packages.len(), 1);
        assert!(report.warnings.iter().any(|w| w.contains("monorepo tool")));
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
        assert!(build("{ not json").is_err());
    }
}
