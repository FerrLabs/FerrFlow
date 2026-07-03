use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct PubspecYamlVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| {
        Regex::new(r#"(?m)^(version:\s*)(["']?)([^"'\s#]+)(["']?)\s*(?:#.*)?$"#).unwrap()
    })
}

impl VersionFile for PubspecYamlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::PUBSPEC_READ)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        super::splice::write_via_splice(self, file_path, version, None)
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::PUBSPEC_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No top-level version: key found in {filename}"))
            .error_code(error_code::PUBSPEC_VERSION_NOT_FOUND)
    }
}

impl super::splice::FormatPreservingEditor for PubspecYamlVersionFile {
    fn locate_version(
        &self,
        content: &str,
        _selector: Option<&str>,
    ) -> Result<std::ops::Range<usize>> {
        version_re()
            .captures(content)
            .map(|c| c.get(3).unwrap().range())
            .ok_or_else(|| anyhow::anyhow!("No top-level version: key found"))
            .error_code(error_code::PUBSPEC_VERSION_NOT_FOUND)
    }

    fn read_error(&self) -> crate::error_code::ErrorCode {
        error_code::PUBSPEC_READ
    }

    fn write_error(&self) -> crate::error_code::ErrorCode {
        error_code::PUBSPEC_WRITE
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

    #[test]
    fn read_unquoted() {
        let f = write_temp("name: my_app\nversion: 1.2.3\n");
        assert_eq!(
            PubspecYamlVersionFile.read_version(f.path()).unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn read_with_build_suffix() {
        let f = write_temp("name: my_app\nversion: 1.2.3+42\n");
        assert_eq!(
            PubspecYamlVersionFile.read_version(f.path()).unwrap(),
            "1.2.3+42"
        );
    }

    #[test]
    fn read_double_quoted() {
        let f = write_temp("version: \"1.0.0\"\n");
        assert_eq!(
            PubspecYamlVersionFile.read_version(f.path()).unwrap(),
            "1.0.0"
        );
    }

    #[test]
    fn read_single_quoted() {
        let f = write_temp("version: '0.5.0'\n");
        assert_eq!(
            PubspecYamlVersionFile.read_version(f.path()).unwrap(),
            "0.5.0"
        );
    }

    #[test]
    fn read_ignores_nested_version_under_dependencies() {
        let f = write_temp(
            "name: my_app\n\
             version: 1.0.0\n\
             dependencies:\n  some_pkg:\n    version: 2.0.0\n",
        );
        assert_eq!(
            PubspecYamlVersionFile.read_version(f.path()).unwrap(),
            "1.0.0"
        );
    }

    #[test]
    fn read_handles_trailing_comment() {
        let f = write_temp("version: 9.9.9 # pinned for release\n");
        assert_eq!(
            PubspecYamlVersionFile.read_version(f.path()).unwrap(),
            "9.9.9"
        );
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("name: my_app\n");
        assert!(PubspecYamlVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_preserves_quotes() {
        let f = write_temp("version: '1.0.0'\n");
        PubspecYamlVersionFile
            .write_version(f.path(), "2.0.0")
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: '2.0.0'"));
    }

    #[test]
    fn write_preserves_unquoted() {
        let f = write_temp("version: 1.0.0\n");
        PubspecYamlVersionFile
            .write_version(f.path(), "1.2.3")
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: 1.2.3"));
        assert!(!content.contains("'1.2.3'"));
    }

    #[test]
    fn write_leaves_dependency_versions_untouched() {
        let f = write_temp(
            "name: my_app\n\
             version: 1.0.0\n\
             dependencies:\n  some_pkg:\n    version: 2.0.0\n",
        );
        PubspecYamlVersionFile
            .write_version(f.path(), "1.1.0")
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: 1.1.0"));
        assert!(content.contains("version: 2.0.0"));
    }

    #[test]
    fn write_no_version_fails() {
        let f = write_temp("name: my_app\n");
        assert!(
            PubspecYamlVersionFile
                .write_version(f.path(), "2.0.0")
                .is_err()
        );
    }
}
