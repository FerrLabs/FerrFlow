use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct HelmVersionFile;

impl VersionFile for HelmVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::HELM_READ)?;

        for line in content.lines() {
            if let Some(v) = line.strip_prefix("version:") {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Ok(v.to_string());
                }
            }
        }

        Err(anyhow::anyhow!(
            "No `version` field found in {}",
            file_path.display()
        ))
        .error_code(error_code::HELM_VERSION_NOT_FOUND)?
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::HELM_READ)?;

        let mut lines: Vec<String> = Vec::new();
        let mut found_version = false;

        for line in content.lines() {
            if line.starts_with("version:") {
                lines.push(format!("version: {version}"));
                found_version = true;
            } else if line.starts_with("appVersion:") {
                // Preserve quoting style: appVersion is typically quoted
                let old = line.strip_prefix("appVersion:").unwrap().trim();
                if old.starts_with('"') {
                    lines.push(format!("appVersion: \"{version}\""));
                } else if old.starts_with('\'') {
                    lines.push(format!("appVersion: '{version}'"));
                } else {
                    lines.push(format!("appVersion: \"{version}\""));
                }
            } else {
                lines.push(line.to_string());
            }
        }

        if !found_version {
            Err(anyhow::anyhow!(
                "No `version` field found in {}",
                file_path.display()
            ))
            .error_code(error_code::HELM_VERSION_NOT_FOUND)?;
        }

        let mut out = lines.join("\n");
        if content.ends_with('\n') {
            out.push('\n');
        }

        std::fs::write(file_path, out)
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::HELM_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::HELM_INVALID_UTF8)?;
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("version:") {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Ok(v.to_string());
                }
            }
        }
        Err(anyhow::anyhow!("No `version` field found in {filename}"))
            .error_code(error_code::HELM_VERSION_NOT_FOUND)?
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

    const CHART_YAML: &str = "\
apiVersion: v2
name: my-app
description: A Helm chart
type: application
version: 1.2.3
appVersion: \"1.2.3\"
";

    #[test]
    fn read_version() {
        let f = write_temp(CHART_YAML);
        assert_eq!(HelmVersionFile.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn read_version_no_app_version() {
        let f = write_temp("apiVersion: v2\nversion: 0.1.0\n");
        assert_eq!(HelmVersionFile.read_version(f.path()).unwrap(), "0.1.0");
    }

    #[test]
    fn read_version_missing_fails() {
        let f = write_temp("apiVersion: v2\nname: test\n");
        assert!(HelmVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_updates_both_fields() {
        let f = write_temp(CHART_YAML);
        HelmVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: 2.0.0"));
        assert!(content.contains("appVersion: \"2.0.0\""));
        assert!(content.contains("name: my-app"));
    }

    #[test]
    fn write_preserves_single_quote_style() {
        let f = write_temp("version: 1.0.0\nappVersion: '1.0.0'\n");
        HelmVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("appVersion: '2.0.0'"));
    }

    #[test]
    fn write_without_app_version() {
        let f = write_temp("apiVersion: v2\nversion: 1.0.0\n");
        HelmVersionFile.write_version(f.path(), "3.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: 3.0.0"));
        assert!(!content.contains("appVersion"));
    }

    #[test]
    fn write_no_version_fails() {
        let f = write_temp("apiVersion: v2\nname: test\n");
        assert!(HelmVersionFile.write_version(f.path(), "1.0.0").is_err());
    }

    #[test]
    fn roundtrip() {
        let f = write_temp(CHART_YAML);
        HelmVersionFile.write_version(f.path(), "5.0.0").unwrap();
        assert_eq!(HelmVersionFile.read_version(f.path()).unwrap(), "5.0.0");
    }
}
