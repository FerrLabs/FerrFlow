use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::Config;
use crate::config::package::{FileFormat, PackageConfig, VersionedFile};
use crate::config::types::ReleaseCommitMode;
use crate::config::workspace::WorkspaceConfig;
use crate::error_code::{self, ErrorCodeExt};

use super::{MigrationReport, Source, write_and_report};

pub(super) const CONFIG_FILE: &str = "release-please-config.json";

pub(super) fn detect() -> Option<PathBuf> {
    let p = PathBuf::from(CONFIG_FILE);
    p.exists().then_some(p)
}

pub(super) fn run() -> Result<()> {
    let path = detect().ok_or_else(|| anyhow::anyhow!("no {CONFIG_FILE} found"))?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("could not read {}: {e}", path.display()))?;
    let (config, report) = build(&raw)?;
    write_and_report(Source::ReleasePlease, &path, &config, &report)
}

#[derive(Debug, Deserialize, Default)]
struct ReleasePleaseConfig {
    // BTreeMap → deterministic (path-sorted) output.
    #[serde(default)]
    packages: BTreeMap<String, PackageEntry>,
    #[serde(default, rename = "release-type")]
    release_type: Option<String>,
    #[serde(default, rename = "include-component-in-tag")]
    include_component_in_tag: Option<bool>,
    #[serde(default, rename = "tag-separator")]
    tag_separator: Option<String>,
    #[serde(default, rename = "separate-pull-requests")]
    separate_pull_requests: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct PackageEntry {
    #[serde(default, rename = "release-type")]
    release_type: Option<String>,
    #[serde(default, rename = "package-name")]
    package_name: Option<String>,
    #[serde(default)]
    component: Option<String>,
    #[serde(default, rename = "changelog-path")]
    changelog_path: Option<String>,
}

pub(super) fn build(raw: &str) -> Result<(Config, MigrationReport)> {
    let rp: ReleasePleaseConfig = serde_json::from_str(raw)
        .map_err(|e| anyhow::anyhow!("could not parse {CONFIG_FILE} as JSON: {e}"))
        .error_code(error_code::CONFIG_INVALID_JSON)?;

    let mut report = MigrationReport::default();
    // release-please always drives releases through a release PR.
    let mut workspace = WorkspaceConfig {
        release_commit_mode: ReleaseCommitMode::Pr,
        ..Default::default()
    };
    report
        .mapped
        .push("release-please PR flow → releaseCommitMode: pr".to_string());

    // Component-in-tag → a {name}-scoped tag template.
    if rp.include_component_in_tag.unwrap_or(false) {
        let sep = rp.tag_separator.as_deref().unwrap_or("-");
        let template = format!("{{name}}{sep}v{{version}}");
        workspace.tag_template = Some(template.clone());
        report
            .mapped
            .push(format!("include-component-in-tag → tagTemplate {template}"));
    }

    if rp.separate_pull_requests == Some(true) {
        report.warnings.push(
            "separate-pull-requests: true opens one PR per package — ferrflow uses a single \
             release PR. The per-package split isn't reproduced."
                .to_string(),
        );
    }

    let default_type = rp.release_type.as_deref();
    let mut packages = Vec::new();
    for (path, entry) in &rp.packages {
        packages.push(build_package(path, entry, default_type, &mut report));
    }

    // A release-please-config with no `packages` map is malformed for a
    // manifest release; scaffold a single root package so the output is usable.
    if packages.is_empty() {
        packages.push(build_package(
            ".",
            &PackageEntry::default(),
            default_type,
            &mut report,
        ));
        report.warnings.push(
            "no `packages` in the config — scaffolded a single root package. Add entries if this \
             is a monorepo."
                .to_string(),
        );
    }

    Ok((
        Config {
            workspace,
            packages,
        },
        report,
    ))
}

fn build_package(
    path: &str,
    entry: &PackageEntry,
    default_type: Option<&str>,
    report: &mut MigrationReport,
) -> PackageConfig {
    let name = entry
        .package_name
        .clone()
        .or_else(|| entry.component.clone())
        .unwrap_or_else(|| name_from_path(path));

    let rt = entry.release_type.as_deref().or(default_type);
    let versioned_files = match rt.and_then(|t| versioned_file_for(t, path)) {
        Some(vf) => {
            report.mapped.push(format!(
                "package {path} ({}) → {}",
                rt.unwrap_or("?"),
                vf.path
            ));
            vec![vf]
        }
        None => {
            report.warnings.push(format!(
                "package {path}: release-type {} has no direct ferrflow file mapping — set \
                 `versionedFiles` by hand.",
                rt.unwrap_or("(none)")
            ));
            Vec::new()
        }
    };

    PackageConfig {
        name,
        path: path.to_string(),
        versioned_files,
        changelog: Some(
            entry
                .changelog_path
                .clone()
                .unwrap_or_else(|| "CHANGELOG.md".to_string()),
        ),
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    }
}

fn name_from_path(path: &str) -> String {
    if path == "." || path.is_empty() {
        super::default_package_name()
    } else {
        path.rsplit('/').next().unwrap_or(path).to_string()
    }
}

/// Map a release-please `release-type` to the version file ferrflow bumps for
/// it. Returns None for types that carry no in-repo version file (e.g. `go`,
/// which release-please versions from tags alone).
fn versioned_file_for(release_type: &str, pkg_path: &str) -> Option<VersionedFile> {
    let join = |file: &str| -> String {
        if pkg_path == "." || pkg_path.is_empty() {
            file.to_string()
        } else {
            format!("{}/{file}", pkg_path.trim_end_matches('/'))
        }
    };
    let (file, format) = match release_type {
        "node" => ("package.json", FileFormat::Json),
        "rust" => ("Cargo.toml", FileFormat::Toml),
        "python" => ("pyproject.toml", FileFormat::Toml),
        "helm" => ("Chart.yaml", FileFormat::ChartYaml),
        "dart" => ("pubspec.yaml", FileFormat::PubspecYaml),
        "elixir" => ("mix.exs", FileFormat::MixExs),
        "simple" => ("version.txt", FileFormat::Txt),
        _ => return None,
    };
    Some(VersionedFile {
        path: join(file),
        format,
        selector: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_ok(raw: &str) -> (Config, MigrationReport) {
        build(raw).expect("valid release-please config")
    }

    #[test]
    fn packages_map_to_ferrflow_packages_with_paths_and_files() {
        let (cfg, _) = build_ok(
            r#"{"packages": {
                "packages/api": {"release-type": "node", "package-name": "@acme/api"},
                "crates/core": {"release-type": "rust"}
            }}"#,
        );
        assert_eq!(cfg.packages.len(), 2);
        let api = cfg
            .packages
            .iter()
            .find(|p| p.path == "packages/api")
            .unwrap();
        assert_eq!(api.name, "@acme/api");
        assert_eq!(api.versioned_files[0].path, "packages/api/package.json");
        assert!(matches!(api.versioned_files[0].format, FileFormat::Json));
        let core = cfg
            .packages
            .iter()
            .find(|p| p.path == "crates/core")
            .unwrap();
        assert_eq!(core.name, "core");
        assert_eq!(core.versioned_files[0].path, "crates/core/Cargo.toml");
        assert!(matches!(core.versioned_files[0].format, FileFormat::Toml));
    }

    #[test]
    fn root_package_uses_bare_file_paths() {
        let (cfg, _) = build_ok(r#"{"packages": {".": {"release-type": "python"}}}"#);
        assert_eq!(cfg.packages[0].versioned_files[0].path, "pyproject.toml");
        assert!(matches!(
            cfg.packages[0].versioned_files[0].format,
            FileFormat::Toml
        ));
    }

    #[test]
    fn default_release_type_applies_to_entries_without_one() {
        let (cfg, _) = build_ok(r#"{"release-type": "node", "packages": {"apps/web": {}}}"#);
        assert_eq!(
            cfg.packages[0].versioned_files[0].path,
            "apps/web/package.json"
        );
    }

    #[test]
    fn component_in_tag_sets_a_scoped_template() {
        let (cfg, report) = build_ok(
            r#"{"include-component-in-tag": true, "tag-separator": "-", "packages": {".": {"release-type": "node"}}}"#,
        );
        assert_eq!(
            cfg.workspace.tag_template.as_deref(),
            Some("{name}-v{version}")
        );
        assert!(report.mapped.iter().any(|m| m.contains("tagTemplate")));
    }

    #[test]
    fn release_please_uses_pr_mode() {
        let (cfg, _) = build_ok(r#"{"packages": {".": {"release-type": "node"}}}"#);
        assert!(matches!(
            cfg.workspace.release_commit_mode,
            ReleaseCommitMode::Pr
        ));
    }

    #[test]
    fn changelog_path_maps_to_the_package_changelog() {
        let (cfg, _) = build_ok(
            r#"{"packages": {".": {"release-type": "node", "changelog-path": "docs/CHANGES.md"}}}"#,
        );
        assert_eq!(
            cfg.packages[0].changelog.as_deref(),
            Some("docs/CHANGES.md")
        );
    }

    #[test]
    fn unmappable_release_type_warns_and_leaves_files_empty() {
        // `go` versions from tags — there's no in-repo version file to bump.
        let (cfg, report) = build_ok(r#"{"packages": {".": {"release-type": "go"}}}"#);
        assert!(cfg.packages[0].versioned_files.is_empty());
        assert!(report.warnings.iter().any(|w| w.contains("go")));
    }

    #[test]
    fn separate_pull_requests_warns() {
        let (_, report) = build_ok(
            r#"{"separate-pull-requests": true, "packages": {".": {"release-type": "node"}}}"#,
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("separate-pull-requests"))
        );
    }

    #[test]
    fn empty_packages_scaffolds_a_root_package_and_warns() {
        let (cfg, report) = build_ok(r#"{"release-type": "node"}"#);
        assert_eq!(cfg.packages.len(), 1);
        assert_eq!(cfg.packages[0].path, ".");
        assert!(report.warnings.iter().any(|w| w.contains("no `packages`")));
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(build("{ not json").is_err());
    }
}
