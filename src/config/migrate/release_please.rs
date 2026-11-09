use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::Config;
use crate::config::package::{FileFormat, PackageConfig, VersionedFile, VersioningStrategy};
use crate::config::types::ReleaseCommitMode;
use crate::config::workspace::WorkspaceConfig;
use crate::error_code::{self, ErrorCodeExt};
use crate::formats::lockfiles::Manifest;

use super::{Migration, MigrationReport, Source};

mod extra_files;
use extra_files::Mapped;

pub(super) const CONFIG_FILE: &str = "release-please-config.json";

pub(super) fn detect() -> Option<PathBuf> {
    let p = PathBuf::from(CONFIG_FILE);
    p.exists().then_some(p)
}

pub(super) fn run() -> Result<Migration> {
    let path = detect().ok_or_else(|| anyhow::anyhow!("no {CONFIG_FILE} found"))?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("could not read {}: {e}", path.display()))?;
    let (config, report) = build(&raw)?;
    Ok(Migration::new(Source::ReleasePlease, path, config, report))
}

#[derive(Debug, Deserialize, Default)]
struct ReleasePleaseConfig {
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
    #[serde(default, rename = "bump-minor-pre-major")]
    bump_minor_pre_major: Option<bool>,
    #[serde(default, rename = "bump-patch-for-minor-pre-major")]
    bump_patch_for_minor_pre_major: Option<bool>,
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
    #[serde(default, rename = "version-file")]
    version_file: Option<String>,
    #[serde(default, rename = "extra-files")]
    extra_files: Vec<serde_json::Value>,
    #[serde(default, rename = "bump-minor-pre-major")]
    bump_minor_pre_major: Option<bool>,
    #[serde(default, rename = "bump-patch-for-minor-pre-major")]
    bump_patch_for_minor_pre_major: Option<bool>,
}

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
    let mut workspace = WorkspaceConfig {
        release_commit_mode: ReleaseCommitMode::Pr,
        ..Default::default()
    };
    report
        .mapped
        .push("release-please PR flow → releaseCommitMode: pr".to_string());
    workspace.auto_merge_releases = false;
    report
        .mapped
        .push("release-please never merges its release PR → autoMergeReleases: false".to_string());

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
    let fallback = PackageEntry::default();
    let entries: Vec<(&str, &PackageEntry)> = if rp.packages.is_empty() {
        report.warnings.push(
            "no `packages` in the config. Scaffolded a single root package. Add entries if this \
             is a monorepo."
                .to_string(),
        );
        vec![(".", &fallback)]
    } else {
        rp.packages.iter().map(|(p, e)| (p.as_str(), e)).collect()
    };

    let mut packages = Vec::new();
    let mut pre_major = Vec::new();
    for (path, entry) in entries {
        packages.push(build_package(path, entry, default_type, &mut report));
        pre_major.push(report_pre_major_flags(path, entry, &rp, &mut report));
    }
    apply_zerover(&mut packages, &pre_major, &mut workspace);
    enable_lockfile_updates(&packages, &mut workspace, &mut report);

    Ok((
        Config {
            include: Vec::new(),
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

    let mut versioned_files = versioned_files;
    for extra in &entry.extra_files {
        add_extra_file(path, extra, &mut versioned_files, report);
    }

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
        ..Default::default()
    }
}

fn add_extra_file(
    pkg_path: &str,
    extra: &serde_json::Value,
    versioned_files: &mut Vec<VersionedFile>,
    report: &mut MigrationReport,
) {
    let (vf, generic) = match extra_files::map(pkg_path, extra) {
        Mapped::Generic(vf) => (vf, true),
        Mapped::Direct(vf) => (vf, false),
        Mapped::Manual(msg) => {
            report.warnings.push(msg);
            return;
        }
    };
    if versioned_files.contains(&vf) {
        report.mapped.push(format!(
            "package {pkg_path}: extra-files {} already covered",
            vf.path
        ));
        return;
    }
    if generic {
        report.mapped.push(format!(
            "package {pkg_path}: extra-files {} (generic) → txt selector on `x-release-please-version`",
            vf.path
        ));
        report.warnings.push(format!(
            "package {pkg_path}: {} is matched on single lines carrying `x-release-please-version`. \
             The `x-release-please-start-version` ... `x-release-please-end` block form is not \
             supported, review manually if the file uses it.",
            vf.path
        ));
    } else {
        report.mapped.push(format!(
            "package {pkg_path}: extra-files {} → {:?}",
            vf.path, vf.format
        ));
    }
    versioned_files.push(vf);
}

fn report_pre_major_flags(
    path: &str,
    entry: &PackageEntry,
    rp: &ReleasePleaseConfig,
    report: &mut MigrationReport,
) -> bool {
    let minor = entry
        .bump_minor_pre_major
        .or(rp.bump_minor_pre_major)
        .unwrap_or(false);
    let patch = entry
        .bump_patch_for_minor_pre_major
        .or(rp.bump_patch_for_minor_pre_major)
        .unwrap_or(false);
    if minor {
        report.mapped.push(format!(
            "package {path}: bump-minor-pre-major → versioning: zerover"
        ));
        report.warnings.push(format!(
            "package {path}: zerover is the closest match for bump-minor-pre-major but not the \
             same: release-please applies the rule only while the version is below 1.0.0, zerover \
             keeps the major at 0 for good and never reaches 1.0.0."
        ));
    }
    if patch {
        report.warnings.push(format!(
            "package {path}: bump-patch-for-minor-pre-major has no ferrflow equivalent yet \
             (tracked in #1376). Features will bump the same as other pre-1.0 changes."
        ));
    }
    minor
}

fn apply_zerover(packages: &mut [PackageConfig], flags: &[bool], workspace: &mut WorkspaceConfig) {
    if !flags.is_empty() && flags.iter().all(|f| *f) {
        workspace.versioning = Some(VersioningStrategy::Zerover);
        return;
    }
    for (package, flagged) in packages.iter_mut().zip(flags) {
        if *flagged {
            package.versioning = Some(VersioningStrategy::Zerover);
        }
    }
}

fn enable_lockfile_updates(
    packages: &[PackageConfig],
    workspace: &mut WorkspaceConfig,
    report: &mut MigrationReport,
) {
    let has_lockfile = packages
        .iter()
        .flat_map(|p| &p.versioned_files)
        .filter_map(|vf| vf.path.rsplit('/').next())
        .any(|name| Manifest::from_manifest_filename(name).is_some());
    if has_lockfile {
        workspace.update_lockfiles = true;
        report
            .mapped
            .push("versioned manifest with a lockfile → updateLockfiles: true".to_string());
    }
}

fn name_from_path(path: &str) -> String {
    if path == "." || path.is_empty() {
        super::default_package_name()
    } else {
        path.rsplit('/').next().unwrap_or(path).to_string()
    }
}

fn versioned_file_for(release_type: &str, pkg_path: &str) -> Option<VersionedFile> {
    let (file, format) = match release_type {
        "node" => ("package.json", FileFormat::Json),
        "rust" => ("Cargo.toml", FileFormat::Toml),
        "python" => ("pyproject.toml", FileFormat::Toml),
        "helm" => ("Chart.yaml", FileFormat::ChartYaml),
        "dart" => ("pubspec.yaml", FileFormat::PubspecYaml),
        "elixir" => ("mix.exs", FileFormat::MixExs),
        "expo" => ("app.json", FileFormat::Json),
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
    use crate::config::package::VersioningStrategy;

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

    #[test]
    fn extra_files_map_alongside_the_release_type_file() {
        let (cfg, report) = build_ok(
            r#"{"packages": {".": {"release-type": "rust", "extra-files": [
                {"type": "generic", "path": "README.md"},
                {"type": "json", "path": "manifest.json", "jsonpath": "$.version"}
            ]}}}"#,
        );
        let files = &cfg.packages[0].versioned_files;
        assert_eq!(files.len(), 3);
        assert_eq!(files[1].path, "README.md");
        assert!(matches!(files[1].format, FileFormat::Txt));
        assert!(files[1].selector.is_some());
        assert_eq!(files[2].path, "manifest.json");
        assert!(matches!(files[2].format, FileFormat::Json));
        assert!(report.mapped.iter().any(|m| m.contains("README.md")));
    }

    #[test]
    fn generic_extra_file_warns_about_the_block_form() {
        let (_, report) = build_ok(
            r#"{"packages": {".": {"release-type": "simple", "extra-files": ["README.md"]}}}"#,
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("x-release-please-start-version") && w.contains("README.md"))
        );
    }

    #[test]
    fn extra_file_duplicating_the_release_type_file_is_not_added_twice() {
        let (cfg, _) = build_ok(
            r#"{"packages": {".": {"release-type": "rust", "extra-files": [
                {"type": "toml", "path": "Cargo.toml", "jsonpath": "$.package.version"}
            ]}}}"#,
        );
        assert_eq!(cfg.packages[0].versioned_files.len(), 1);
    }

    #[test]
    fn unsupported_extra_files_are_reported_with_the_exact_entry() {
        let (cfg, report) = build_ok(
            r#"{"packages": {"svc": {"release-type": "simple", "extra-files": [
                {"type": "yaml", "path": "chart.yaml", "jsonpath": "$.version"}
            ]}}}"#,
        );
        assert_eq!(cfg.packages[0].versioned_files.len(), 1);
        assert!(report.warnings.iter().any(|w| {
            w.contains(r#"{"type":"yaml","path":"chart.yaml","jsonpath":"$.version"}"#)
        }));
    }

    #[test]
    fn bump_minor_pre_major_on_the_only_package_sets_workspace_zerover() {
        let (cfg, report) = build_ok(
            r#"{"packages": {".": {"release-type": "node", "bump-minor-pre-major": true}}}"#,
        );
        assert_eq!(cfg.workspace.versioning, Some(VersioningStrategy::Zerover));
        assert!(cfg.packages[0].versioning.is_none());
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("zerover") && w.contains("1.0.0"))
        );
    }

    #[test]
    fn bump_minor_pre_major_on_some_packages_sets_package_zerover() {
        let (cfg, _) = build_ok(
            r#"{"packages": {
                "a": {"release-type": "node", "bump-minor-pre-major": true},
                "b": {"release-type": "node"}
            }}"#,
        );
        assert_eq!(cfg.workspace.versioning, None);
        assert_eq!(
            cfg.packages[0].versioning,
            Some(VersioningStrategy::Zerover)
        );
        assert_eq!(cfg.packages[1].versioning, None);
    }

    #[test]
    fn top_level_bump_minor_pre_major_applies_to_every_package() {
        let (cfg, _) = build_ok(
            r#"{"bump-minor-pre-major": true, "packages": {"a": {"release-type": "node"}, "b": {"release-type": "node"}}}"#,
        );
        assert_eq!(cfg.workspace.versioning, Some(VersioningStrategy::Zerover));
    }

    #[test]
    fn package_flag_false_overrides_the_top_level_flag() {
        let (cfg, _) = build_ok(
            r#"{"bump-minor-pre-major": true, "packages": {"a": {"release-type": "node", "bump-minor-pre-major": false}}}"#,
        );
        assert_eq!(cfg.workspace.versioning, None);
        assert_eq!(cfg.packages[0].versioning, None);
    }

    #[test]
    fn bump_patch_for_minor_pre_major_warns_and_points_to_1376() {
        let (cfg, report) = build_ok(
            r#"{"packages": {".": {"release-type": "node", "bump-patch-for-minor-pre-major": true}}}"#,
        );
        assert_eq!(cfg.workspace.versioning, None);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("bump-patch-for-minor-pre-major") && w.contains("#1376"))
        );
    }

    #[test]
    fn release_please_disables_auto_merge() {
        let (cfg, report) = build_ok(r#"{"packages": {".": {"release-type": "node"}}}"#);
        assert!(!cfg.workspace.auto_merge_releases);
        assert!(
            report
                .mapped
                .iter()
                .any(|m| m.contains("autoMergeReleases"))
        );
    }

    #[test]
    fn manifests_with_a_lockfile_enable_lockfile_updates() {
        let (cfg, _) = build_ok(r#"{"packages": {".": {"release-type": "rust"}}}"#);
        assert!(cfg.workspace.update_lockfiles);
    }

    #[test]
    fn lockfile_updates_stay_off_without_a_lockfile_manifest() {
        let (cfg, _) = build_ok(
            r#"{"packages": {".": {"release-type": "simple", "extra-files": ["README.md"]}}}"#,
        );
        assert!(!cfg.workspace.update_lockfiles);
    }

    #[test]
    fn package_name_wins_over_component_and_path() {
        let (cfg, _) = build_ok(
            r#"{"packages": {"x": {"release-type": "node", "package-name": "pn", "component": "c"}}}"#,
        );
        assert_eq!(cfg.packages[0].name, "pn");
        let (cfg, _) =
            build_ok(r#"{"packages": {"x": {"release-type": "node", "component": "c"}}}"#);
        assert_eq!(cfg.packages[0].name, "c");
    }
}
