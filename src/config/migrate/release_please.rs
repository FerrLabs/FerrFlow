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
    #[serde(default)]
    plugins: Vec<Plugin>,
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
    // release-please's explicit version file (ruby, simple, …). When present it
    // wins over the release-type default.
    #[serde(default, rename = "version-file")]
    version_file: Option<String>,
}

/// A release-please plugin: a bare `"name"` or `{ "type": ..., ... }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Plugin {
    Name(String),
    Config {
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        components: Vec<String>,
    },
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

    for plugin in &rp.plugins {
        apply_plugin(plugin, &mut workspace, &mut report);
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
    let versioned_files = if let Some(vf_path) = &entry.version_file {
        // release-please's explicit `version-file` wins over the type default.
        let joined = join_path(path, vf_path);
        match format_from_ext(&joined) {
            Some(format) => {
                report
                    .mapped
                    .push(format!("package {path}: version-file → {joined}"));
                vec![VersionedFile {
                    path: joined,
                    format,
                    selector: None,
                }]
            }
            None => {
                report.warnings.push(format!(
                    "package {path}: version-file {vf_path} has an unrecognised extension — set \
                     its format in `versionedFiles` by hand."
                ));
                Vec::new()
            }
        }
    } else {
        match rt.and_then(|t| versioned_file_for(t, path)) {
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
/// it. Returns None for types that carry no single in-repo version file (e.g.
/// `go`, which release-please versions from tags alone) — the caller warns.
fn versioned_file_for(release_type: &str, pkg_path: &str) -> Option<VersionedFile> {
    let (file, format) = match release_type {
        "node" => ("package.json", FileFormat::Json),
        "rust" => ("Cargo.toml", FileFormat::Toml),
        "python" => ("pyproject.toml", FileFormat::Toml),
        "helm" => ("Chart.yaml", FileFormat::ChartYaml),
        "dart" => ("pubspec.yaml", FileFormat::PubspecYaml),
        "elixir" => ("mix.exs", FileFormat::MixExs),
        "expo" => ("app.json", FileFormat::Json),
        // pom.xml: the xml handler's default selector targets the first
        // <version> child of the root, sidestepping the <parent><version> pit.
        "maven" => ("pom.xml", FileFormat::Xml),
        "simple" => ("version.txt", FileFormat::Txt),
        _ => return None,
    };
    Some(VersionedFile {
        path: join_path(pkg_path, file),
        format,
        selector: None,
    })
}

fn join_path(pkg_path: &str, file: &str) -> String {
    if pkg_path == "." || pkg_path.is_empty() {
        file.to_string()
    } else {
        format!("{}/{file}", pkg_path.trim_end_matches('/'))
    }
}

fn format_from_ext(path: &str) -> Option<FileFormat> {
    let lower = path.to_ascii_lowercase();
    let basename = lower.rsplit('/').next().unwrap_or(&lower);
    if lower.ends_with(".json") {
        Some(FileFormat::Json)
    } else if lower.ends_with(".toml") {
        Some(FileFormat::Toml)
    } else if lower.ends_with(".gemspec") {
        Some(FileFormat::Gemspec)
    } else if lower.ends_with(".xml") {
        Some(FileFormat::Xml)
    } else if lower.ends_with(".cabal") {
        Some(FileFormat::Cabal)
    } else if basename == "cmakelists.txt" {
        Some(FileFormat::Cmake)
    } else if lower.ends_with(".txt") || basename == "version" {
        Some(FileFormat::Txt)
    } else {
        None
    }
}

/// Translate a release-please plugin. Only `linked-versions` has a direct
/// ferrflow equivalent (→ a `linked` version group); the rest are reported.
fn apply_plugin(plugin: &Plugin, workspace: &mut WorkspaceConfig, report: &mut MigrationReport) {
    if let Plugin::Config { kind, components } = plugin
        && kind == "linked-versions"
        && components.len() >= 2
    {
        workspace.linked.push(components.clone());
        report.mapped.push(format!(
            "linked-versions plugin → linked group ({} packages)",
            components.len()
        ));
        return;
    }
    let kind = match plugin {
        Plugin::Name(n) => n.as_str(),
        Plugin::Config { kind, .. } => kind.as_str(),
    };
    match kind {
        "linked-versions" => report.warnings.push(
            "linked-versions plugin lists no `components` — add the linked group to `linked` by \
             hand."
                .to_string(),
        ),
        "node-workspace" | "cargo-workspace" | "maven-workspace" => report.ignored.push(format!(
            "{kind} plugin — ferrflow bumps internal dependents via `dependsOn` cascades; wire \
             those up per package."
        )),
        "sentence-case" => report.ignored.push(
            "sentence-case plugin — changelog casing only, not a ferrflow concern.".to_string(),
        ),
        other => report.warnings.push(format!(
            "plugin {other} has no ferrflow equivalent — review manually."
        )),
    }
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

    #[test]
    fn maven_and_expo_release_types_map() {
        let (cfg, _) = build_ok(
            r#"{"packages": {"svc": {"release-type": "maven"}, "app": {"release-type": "expo"}}}"#,
        );
        let svc = cfg.packages.iter().find(|p| p.path == "svc").unwrap();
        assert_eq!(svc.versioned_files[0].path, "svc/pom.xml");
        assert!(matches!(svc.versioned_files[0].format, FileFormat::Xml));
        let app = cfg.packages.iter().find(|p| p.path == "app").unwrap();
        assert_eq!(app.versioned_files[0].path, "app/app.json");
        assert!(matches!(app.versioned_files[0].format, FileFormat::Json));
    }

    #[test]
    fn version_file_overrides_the_type_default() {
        let (cfg, _) = build_ok(
            r#"{"packages": {"gems/foo": {"release-type": "ruby", "version-file": "lib/foo.gemspec"}}}"#,
        );
        assert_eq!(
            cfg.packages[0].versioned_files[0].path,
            "gems/foo/lib/foo.gemspec"
        );
        assert!(matches!(
            cfg.packages[0].versioned_files[0].format,
            FileFormat::Gemspec
        ));
    }

    // `CMakeLists.txt` ends in `.txt`, so the cmake branch has to be checked
    // first or the file is inferred as a plain-text version file and the whole
    // CMakeLists gets overwritten with a bare version string.
    #[test]
    fn cmakelists_is_inferred_as_cmake_not_txt() {
        let (cfg, _) = build_ok(
            r#"{"packages": {"native": {"release-type": "simple", "version-file": "CMakeLists.txt"}}}"#,
        );
        assert!(matches!(
            cfg.packages[0].versioned_files[0].format,
            FileFormat::Cmake
        ));
    }

    #[test]
    fn cabal_file_is_inferred_from_its_extension() {
        let (cfg, _) = build_ok(
            r#"{"packages": {"hs": {"release-type": "simple", "version-file": "my-package.cabal"}}}"#,
        );
        assert!(matches!(
            cfg.packages[0].versioned_files[0].format,
            FileFormat::Cabal
        ));
    }

    #[test]
    fn version_file_with_unknown_extension_warns() {
        // A Ruby version.rb needs a Txt regex selector, which we can't infer.
        let (cfg, report) = build_ok(
            r#"{"packages": {".": {"release-type": "ruby", "version-file": "lib/foo/version.rb"}}}"#,
        );
        assert!(cfg.packages[0].versioned_files.is_empty());
        assert!(report.warnings.iter().any(|w| w.contains("version.rb")));
    }

    #[test]
    fn linked_versions_plugin_becomes_a_linked_group() {
        let (cfg, report) = build_ok(
            r#"{"packages": {".": {"release-type": "node"}},
                "plugins": [{"type": "linked-versions", "groupName": "main", "components": ["@acme/a", "@acme/b"]}]}"#,
        );
        assert_eq!(cfg.workspace.linked, vec![vec!["@acme/a", "@acme/b"]]);
        assert!(report.mapped.iter().any(|m| m.contains("linked-versions")));
    }

    #[test]
    fn linked_versions_without_components_warns() {
        let (cfg, report) = build_ok(
            r#"{"packages": {".": {"release-type": "node"}}, "plugins": ["linked-versions"]}"#,
        );
        assert!(cfg.workspace.linked.is_empty());
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("linked-versions"))
        );
    }

    #[test]
    fn node_workspace_plugin_is_reported_as_ignored() {
        let (_, report) = build_ok(
            r#"{"packages": {".": {"release-type": "node"}}, "plugins": ["node-workspace"]}"#,
        );
        assert!(report.ignored.iter().any(|i| i.contains("node-workspace")));
    }
}
