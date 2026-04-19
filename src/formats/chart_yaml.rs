//! `Chart.yaml` (Helm chart top-level manifest) version handler.
//!
//! Distinct from the existing [`super::helm::HelmVersionFile`] which targets
//! `values.yaml` templating (`{{ .Chart.Version }}`-style). Chart.yaml uses a
//! literal `version:` key at the top level, the same shape as `pubspec.yaml`
//! but with a different idiomatic layout and separate docs surface. We keep
//! them as distinct variants so users opting in to either don't need to
//! understand the internals of the sibling file.

use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct ChartYamlVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    // Top-level `version:` only — charts also define `appVersion:`, which is
    // a different concept (the app shipped by the chart, not the chart
    // itself). We leave `appVersion:` strictly alone.
    VERSION_RE.get_or_init(|| {
        Regex::new(r#"(?m)^(version:\s*)(["']?)([^"'\s#]+)(["']?)\s*(?:#.*)?$"#).unwrap()
    })
}

impl VersionFile for ChartYamlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::CHART_YAML_READ)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::CHART_YAML_READ)?;
        if !version_re().is_match(&content) {
            Err(anyhow::anyhow!(
                "No top-level version: key found in {}",
                file_path.display()
            ))
            .error_code(error_code::CHART_YAML_VERSION_NOT_FOUND)?;
        }
        let new_content = version_re().replace(&content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], version, &caps[4])
        });
        std::fs::write(file_path, new_content.as_ref())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::CHART_YAML_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::CHART_YAML_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No top-level version: key found in {filename}"))
            .error_code(error_code::CHART_YAML_VERSION_NOT_FOUND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    const FIXTURE: &str = "apiVersion: v2\n\
                            name: my-chart\n\
                            description: A Helm chart\n\
                            type: application\n\
                            version: 0.1.0\n\
                            appVersion: \"1.16.0\"\n";

    #[test]
    fn read_version_not_app_version() {
        let f = write_temp(FIXTURE);
        assert_eq!(
            ChartYamlVersionFile.read_version(f.path()).unwrap(),
            "0.1.0"
        );
    }

    #[test]
    fn write_leaves_app_version_untouched() {
        let f = write_temp(FIXTURE);
        ChartYamlVersionFile
            .write_version(f.path(), "0.2.0")
            .unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("version: 0.2.0"));
        // appVersion must stay exactly as it was — different concept.
        assert!(out.contains("appVersion: \"1.16.0\""));
    }

    #[test]
    fn read_quoted_version() {
        let f = write_temp("apiVersion: v2\nname: x\nversion: \"1.2.3\"\n");
        assert_eq!(
            ChartYamlVersionFile.read_version(f.path()).unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("apiVersion: v2\nname: x\n");
        assert!(ChartYamlVersionFile.read_version(f.path()).is_err());
    }
}
