use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct GradleVersionFile;

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
    fn read_gradle_double_quotes() {
        let f = write_temp("plugins { id 'java' }\nversion = \"1.2.3\"\n");
        assert_eq!(GradleVersionFile.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn read_gradle_single_quotes() {
        let f = write_temp("version = '0.5.0'\n");
        assert_eq!(GradleVersionFile.read_version(f.path()).unwrap(), "0.5.0");
    }

    #[test]
    fn read_gradle_with_spaces() {
        let f = write_temp("version  =  \"1.0.0\"\n");
        assert_eq!(GradleVersionFile.read_version(f.path()).unwrap(), "1.0.0");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("plugins { id 'java' }\n");
        assert!(GradleVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_gradle_version() {
        let f = write_temp("version = \"1.0.0\"\n");
        GradleVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(GradleVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_preserves_quote_style() {
        let f = write_temp("version = '1.0.0'\n");
        GradleVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version = '2.0.0'"));
    }
}

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE
        .get_or_init(|| Regex::new(r#"(?m)^(\s*version\s*=\s*)(["'])([^"']+)(["'])"#).unwrap())
}

impl VersionFile for GradleVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::GRADLE_READ)?;

        version_re()
            .captures(&content)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No version field found in {}", file_path.display()))
            .error_code(error_code::GRADLE_VERSION_NOT_FOUND)
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        super::splice::write_via_splice(self, file_path, version, None)
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::GRADLE_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No version field found in {filename}"))
            .error_code(error_code::GRADLE_VERSION_NOT_FOUND)
    }
}

impl super::splice::FormatPreservingEditor for GradleVersionFile {
    fn locate_version(
        &self,
        content: &str,
        _selector: Option<&str>,
    ) -> Result<std::ops::Range<usize>> {
        version_re()
            .captures(content)
            .map(|c| c.get(3).unwrap().range())
            .ok_or_else(|| anyhow::anyhow!("No version field found to update"))
            .error_code(error_code::GRADLE_VERSION_NOT_FOUND)
    }

    fn read_error(&self) -> crate::error_code::ErrorCode {
        error_code::GRADLE_READ
    }

    fn write_error(&self) -> crate::error_code::ErrorCode {
        error_code::GRADLE_WRITE
    }
}
