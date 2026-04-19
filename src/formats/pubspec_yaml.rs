//! `pubspec.yaml` (Dart / Flutter) version handler.
//!
//! Matches the top-level `version:` key, regardless of quoting. The regex is
//! anchored to line start with a multiline flag so inline `version:` values
//! inside dependency maps or `flutter:` blocks are ignored. Anchors and
//! comments elsewhere in the file are preserved verbatim — we rewrite only
//! the captured version substring.
//!
//! Pub uses SemVer with an optional `+<build>` suffix (e.g. `1.2.3+42`).
//! That fits any string the version bumper would produce; no special casing
//! needed here.

use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct PubspecYamlVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    // `^version:` — top-level YAML key, no indentation. Captures:
    //   1. prefix including `:` + whitespace
    //   2. optional opening quote (`'`, `"`, or empty)
    //   3. version value
    //   4. optional closing quote (same rule)
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
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::PUBSPEC_READ)?;
        if !version_re().is_match(&content) {
            Err(anyhow::anyhow!(
                "No top-level version: key found in {}",
                file_path.display()
            ))
            .error_code(error_code::PUBSPEC_VERSION_NOT_FOUND)?;
        }
        let new_content = version_re().replace(&content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], version, &caps[4])
        });
        std::fs::write(file_path, new_content.as_ref())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::PUBSPEC_WRITE)?;
        Ok(())
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
