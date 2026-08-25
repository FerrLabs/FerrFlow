use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

use crate::config::Config;
use crate::config::package::{FileFormat, PackageConfig, VersionedFile};
use crate::config::workspace::WorkspaceConfig;
use crate::error_code::{self, ErrorCodeExt};

use super::{MigrationReport, Source, read_source_as_json, write_and_report};

pub(super) const CONFIG_FILES: &[&str] = &[
    ".versionrc",
    ".versionrc.json",
    ".versionrc.yaml",
    ".versionrc.yml",
    ".versionrc.js",
    ".versionrc.cjs",
];

pub(super) fn detect() -> Option<PathBuf> {
    CONFIG_FILES.iter().map(PathBuf::from).find(|p| p.exists())
}

pub(super) fn run() -> Result<()> {
    let path = detect().ok_or_else(|| anyhow::anyhow!("no .versionrc found"))?;
    let raw = read_source_as_json(&path)?;
    let (config, report) = build(&raw)?;
    write_and_report(Source::StandardVersion, &path, &config, &report)
}

#[derive(Debug, Deserialize, Default)]
struct VersionRc {
    #[serde(default, rename = "tagPrefix")]
    tag_prefix: Option<String>,
    #[serde(default, rename = "bumpFiles")]
    bump_files: Vec<BumpFile>,
    #[serde(default, rename = "packageFiles")]
    package_files: Vec<BumpFile>,
    #[serde(default)]
    types: Option<serde_json::Value>,
    #[serde(default)]
    skip: Option<Skip>,
    #[serde(default, rename = "releaseCommitMessageFormat")]
    release_commit_message_format: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BumpFile {
    filename: String,
    #[serde(default, rename = "type")]
    type_hint: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Skip {
    #[serde(default)]
    tag: bool,
    #[serde(default)]
    changelog: bool,
}

pub(super) fn build(raw: &str) -> Result<(Config, MigrationReport)> {
    let rc: VersionRc = json5::from_str(raw)
        .map_err(|e| anyhow::anyhow!("could not parse .versionrc as JSON: {e}"))
        .error_code(error_code::CONFIG_INVALID_JSON)?;

    let mut report = MigrationReport::default();
    let mut workspace = WorkspaceConfig::default();

    if let Some(prefix) = &rc.tag_prefix {
        let template = format!("{prefix}{{version}}");
        workspace.tag_template = Some(template.clone());
        report
            .mapped
            .push(format!("tagPrefix → tagTemplate {template}"));
    }

    let mut versioned_files: Vec<VersionedFile> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for bf in rc.bump_files.iter().chain(rc.package_files.iter()) {
        if seen.contains(&bf.filename) {
            continue;
        }
        seen.push(bf.filename.clone());
        match format_for(bf.type_hint.as_deref(), &bf.filename) {
            Some(format) => {
                report
                    .mapped
                    .push(format!("version file {} → versionedFiles", bf.filename));
                versioned_files.push(VersionedFile {
                    path: bf.filename.clone(),
                    format,
                    selector: None,
                });
            }
            None => report.warnings.push(format!(
                "version file {} has an unrecognised type — add it to `versionedFiles` by hand.",
                bf.filename
            )),
        }
    }
    if versioned_files.is_empty() {
        versioned_files.push(VersionedFile {
            path: "package.json".to_string(),
            format: FileFormat::Json,
            selector: None,
        });
        report
            .mapped
            .push("no bumpFiles → defaulted to package.json".to_string());
    }

    if rc.types.is_some() {
        report.warnings.push(
            "custom changelog `types` don't map — ferrflow generates changelog sections from \
             conventional-commit types with its own layout."
                .to_string(),
        );
    }
    if let Some(skip) = &rc.skip {
        if skip.tag {
            report.warnings.push(
                "skip.tag isn't supported — ferrflow always creates the release tag.".to_string(),
            );
        }
        if skip.changelog {
            report.ignored.push(
                "skip.changelog — omit the package `changelog` field to skip changelog generation."
                    .to_string(),
            );
        }
    }
    if rc.release_commit_message_format.is_some() {
        report.ignored.push(
            "releaseCommitMessageFormat — ferrflow uses a fixed `chore(release):` commit message."
                .to_string(),
        );
    }

    let package = PackageConfig {
        name: super::default_package_name(),
        path: ".".to_string(),
        versioned_files,
        changelog: Some("CHANGELOG.md".to_string()),
        shared_paths: Vec::new(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        version_template: None,
        hooks: None,
        floating_tags: None,
        latest_tag: None,
        publishers: vec![],
        update_lockfiles: None,
        version_source: None,
    };

    report.warnings.push(
        "standard-version is single-package; scaffolded one package. Add more for a monorepo."
            .to_string(),
    );

    Ok((
        Config {
            include: Vec::new(),
            workspace,
            packages: vec![package],
        },
        report,
    ))
}

fn format_for(type_hint: Option<&str>, filename: &str) -> Option<FileFormat> {
    if let Some(t) = type_hint {
        match t {
            "json" => return Some(FileFormat::Json),
            "plain-text" => return Some(FileFormat::Txt),
            _ => {}
        }
    }
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".json") {
        Some(FileFormat::Json)
    } else if lower.ends_with(".toml") {
        Some(FileFormat::Toml)
    } else if lower.ends_with(".txt") || lower == "version" {
        Some(FileFormat::Txt)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_ok(raw: &str) -> (Config, MigrationReport) {
        build(raw).expect("valid .versionrc")
    }

    #[test]
    fn tag_prefix_becomes_a_tag_template() {
        let (cfg, _) = build_ok(r#"{"tagPrefix": "release-"}"#);
        assert_eq!(
            cfg.workspace.tag_template.as_deref(),
            Some("release-{version}")
        );
    }

    #[test]
    fn bump_files_map_to_versioned_files_with_types() {
        let (cfg, _) = build_ok(
            r#"{"bumpFiles": [
                {"filename": "package.json", "type": "json"},
                {"filename": "VERSION", "type": "plain-text"}
            ]}"#,
        );
        let paths: Vec<&str> = cfg.packages[0]
            .versioned_files
            .iter()
            .map(|v| v.path.as_str())
            .collect();
        assert_eq!(paths, vec!["package.json", "VERSION"]);
        assert!(matches!(
            cfg.packages[0].versioned_files[1].format,
            FileFormat::Txt
        ));
    }

    #[test]
    fn package_files_and_bump_files_are_deduped() {
        let (cfg, _) = build_ok(
            r#"{
                "packageFiles": [{"filename": "package.json", "type": "json"}],
                "bumpFiles": [{"filename": "package.json", "type": "json"}, {"filename": "manifest.json", "type": "json"}]
            }"#,
        );
        assert_eq!(cfg.packages[0].versioned_files.len(), 2);
    }

    #[test]
    fn format_inferred_from_extension_when_type_absent() {
        let (cfg, _) = build_ok(r#"{"bumpFiles": [{"filename": "Cargo.toml"}]}"#);
        assert!(matches!(
            cfg.packages[0].versioned_files[0].format,
            FileFormat::Toml
        ));
    }

    #[test]
    fn no_bump_files_defaults_to_package_json() {
        let (cfg, _) = build_ok("{}");
        assert_eq!(cfg.packages[0].versioned_files[0].path, "package.json");
    }

    #[test]
    fn custom_types_warn() {
        let (_, report) = build_ok(r#"{"types": [{"type": "feat", "section": "Features"}]}"#);
        assert!(report.warnings.iter().any(|w| w.contains("types")));
    }

    #[test]
    fn skip_tag_warns() {
        let (_, report) = build_ok(r#"{"skip": {"tag": true}}"#);
        assert!(report.warnings.iter().any(|w| w.contains("skip.tag")));
    }

    #[test]
    fn release_commit_message_format_is_reported() {
        let (_, report) = build_ok(r#"{"releaseCommitMessageFormat": "chore: {{currentTag}}"}"#);
        assert!(
            report
                .ignored
                .iter()
                .any(|i| i.contains("releaseCommitMessageFormat"))
        );
    }

    #[test]
    fn unrecognised_file_type_warns_and_is_skipped() {
        let (cfg, report) =
            build_ok(r#"{"bumpFiles": [{"filename": "weird.xyz", "type": "mystery"}]}"#);
        assert_eq!(cfg.packages[0].versioned_files[0].path, "package.json");
        assert!(report.warnings.iter().any(|w| w.contains("weird.xyz")));
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(build("{ not json").is_err());
    }
}
