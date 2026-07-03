use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct ChartYamlVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
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
        super::splice::write_via_splice(self, file_path, version, None)
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

impl super::splice::FormatPreservingEditor for ChartYamlVersionFile {
    fn locate_version(
        &self,
        content: &str,
        _selector: Option<&str>,
    ) -> Result<std::ops::Range<usize>> {
        version_re()
            .captures(content)
            .map(|c| c.get(3).unwrap().range())
            .ok_or_else(|| anyhow::anyhow!("No top-level version: key found"))
            .error_code(error_code::CHART_YAML_VERSION_NOT_FOUND)
    }

    fn read_error(&self) -> crate::error_code::ErrorCode {
        error_code::CHART_YAML_READ
    }

    fn write_error(&self) -> crate::error_code::ErrorCode {
        error_code::CHART_YAML_WRITE
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
