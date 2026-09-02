use super::VersionFile;
use crate::error_code::{self, ErrorCode, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

/// Writes the top-level `version:` of a YAML manifest, leaving the rest of the
/// file byte-identical.
///
/// `Chart.yaml`, `pubspec.yaml` and `galaxy.yml` all carry the version the same
/// way and differ only in which error codes they report, so they share one
/// editor instead of a copy each.
pub struct YamlTopLevelVersion {
    read: ErrorCode,
    write: ErrorCode,
    invalid_utf8: ErrorCode,
    not_found: ErrorCode,
}

impl YamlTopLevelVersion {
    pub const CHART: Self = Self {
        read: error_code::CHART_YAML_READ,
        write: error_code::CHART_YAML_WRITE,
        invalid_utf8: error_code::CHART_YAML_INVALID_UTF8,
        not_found: error_code::CHART_YAML_VERSION_NOT_FOUND,
    };

    pub const PUBSPEC: Self = Self {
        read: error_code::PUBSPEC_READ,
        write: error_code::PUBSPEC_WRITE,
        invalid_utf8: error_code::PUBSPEC_INVALID_UTF8,
        not_found: error_code::PUBSPEC_VERSION_NOT_FOUND,
    };

    pub const GALAXY: Self = Self {
        read: error_code::GALAXY_YAML_READ,
        write: error_code::GALAXY_YAML_WRITE,
        invalid_utf8: error_code::GALAXY_YAML_INVALID_UTF8,
        not_found: error_code::GALAXY_YAML_VERSION_NOT_FOUND,
    };
}

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| {
        Regex::new(r#"(?m)^(version:\s*)(["']?)([^"'\s#]+)(["']?)\s*(?:#.*)?$"#).unwrap()
    })
}

impl VersionFile for YamlTopLevelVersion {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(self.read)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        super::splice::write_via_splice(self, file_path, version, None)
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(self.invalid_utf8)?;
        version_re()
            .captures(text)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No top-level version: key found in {filename}"))
            .error_code(self.not_found)
    }
}

impl super::splice::FormatPreservingEditor for YamlTopLevelVersion {
    fn locate_version(
        &self,
        content: &str,
        _selector: Option<&str>,
    ) -> Result<std::ops::Range<usize>> {
        version_re()
            .captures(content)
            .map(|c| c.get(3).unwrap().range())
            .ok_or_else(|| anyhow::anyhow!("No top-level version: key found"))
            .error_code(self.not_found)
    }

    fn read_error(&self) -> ErrorCode {
        self.read
    }

    fn write_error(&self) -> ErrorCode {
        self.write
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

    const CHART: &str = "apiVersion: v2\nname: my-chart\ndescription: A Helm chart\ntype: application\nversion: 0.1.0\nappVersion: \"1.16.0\"\n";

    const GALAXY: &str =
        "---\nnamespace: kubernetes_sigs\nname: kubespray\nversion: 2.32.0\nreadme: README.md\n";

    const NESTED: &str =
        "name: my_app\nversion: 1.0.0\ndependencies:\n  some_pkg:\n    version: 2.0.0\n";

    #[test]
    fn chart_reads_version_not_app_version() {
        let f = write_temp(CHART);
        assert_eq!(
            YamlTopLevelVersion::CHART.read_version(f.path()).unwrap(),
            "0.1.0"
        );
    }

    #[test]
    fn chart_write_leaves_app_version_untouched() {
        let f = write_temp(CHART);
        YamlTopLevelVersion::CHART
            .write_version(f.path(), "0.2.0")
            .unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("version: 0.2.0"));
        assert!(out.contains("appVersion: \"1.16.0\""));
    }

    #[test]
    fn chart_reads_a_quoted_version() {
        let f = write_temp("apiVersion: v2\nname: x\nversion: \"1.2.3\"\n");
        assert_eq!(
            YamlTopLevelVersion::CHART.read_version(f.path()).unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn chart_without_a_version_fails() {
        let f = write_temp("apiVersion: v2\nname: x\n");
        assert!(YamlTopLevelVersion::CHART.read_version(f.path()).is_err());
    }

    #[test]
    fn pubspec_reads_unquoted() {
        let f = write_temp("name: my_app\nversion: 1.2.3\n");
        assert_eq!(
            YamlTopLevelVersion::PUBSPEC.read_version(f.path()).unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn pubspec_reads_a_build_suffix() {
        let f = write_temp("name: my_app\nversion: 1.2.3+42\n");
        assert_eq!(
            YamlTopLevelVersion::PUBSPEC.read_version(f.path()).unwrap(),
            "1.2.3+42"
        );
    }

    #[test]
    fn pubspec_reads_double_quoted() {
        let f = write_temp("version: \"1.0.0\"\n");
        assert_eq!(
            YamlTopLevelVersion::PUBSPEC.read_version(f.path()).unwrap(),
            "1.0.0"
        );
    }

    #[test]
    fn pubspec_reads_single_quoted() {
        let f = write_temp("version: '0.5.0'\n");
        assert_eq!(
            YamlTopLevelVersion::PUBSPEC.read_version(f.path()).unwrap(),
            "0.5.0"
        );
    }

    #[test]
    fn a_nested_version_under_dependencies_is_ignored() {
        let f = write_temp(NESTED);
        assert_eq!(
            YamlTopLevelVersion::PUBSPEC.read_version(f.path()).unwrap(),
            "1.0.0"
        );
    }

    #[test]
    fn a_trailing_comment_is_not_part_of_the_version() {
        let f = write_temp("version: 9.9.9 # pinned for release\n");
        assert_eq!(
            YamlTopLevelVersion::PUBSPEC.read_version(f.path()).unwrap(),
            "9.9.9"
        );
    }

    #[test]
    fn pubspec_without_a_version_fails() {
        let f = write_temp("name: my_app\n");
        assert!(YamlTopLevelVersion::PUBSPEC.read_version(f.path()).is_err());
    }

    #[test]
    fn writing_preserves_quotes() {
        let f = write_temp("version: '1.0.0'\n");
        YamlTopLevelVersion::PUBSPEC
            .write_version(f.path(), "2.0.0")
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: '2.0.0'"));
    }

    #[test]
    fn writing_preserves_the_absence_of_quotes() {
        let f = write_temp("version: 1.0.0\n");
        YamlTopLevelVersion::PUBSPEC
            .write_version(f.path(), "1.2.3")
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: 1.2.3"));
        assert!(!content.contains("'1.2.3'"));
    }

    #[test]
    fn writing_leaves_dependency_versions_untouched() {
        let f = write_temp(NESTED);
        YamlTopLevelVersion::PUBSPEC
            .write_version(f.path(), "1.1.0")
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("version: 1.1.0"));
        assert!(content.contains("version: 2.0.0"));
    }

    #[test]
    fn writing_without_a_version_fails() {
        let f = write_temp("name: my_app\n");
        assert!(
            YamlTopLevelVersion::PUBSPEC
                .write_version(f.path(), "2.0.0")
                .is_err()
        );
    }

    #[test]
    fn galaxy_reads_the_collection_version() {
        let f = write_temp(GALAXY);
        assert_eq!(
            YamlTopLevelVersion::GALAXY.read_version(f.path()).unwrap(),
            "2.32.0"
        );
    }

    #[test]
    fn galaxy_writes_without_disturbing_the_rest() {
        let f = write_temp(GALAXY);
        YamlTopLevelVersion::GALAXY
            .write_version(f.path(), "2.33.0")
            .unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("version: 2.33.0"), "{out}");
        assert!(out.contains("namespace: kubernetes_sigs"), "{out}");
        assert!(out.contains("readme: README.md"), "{out}");
    }

    #[test]
    fn each_format_reports_its_own_error_code() {
        let f = write_temp("namespace: acme\nname: thing\n");
        let err = YamlTopLevelVersion::GALAXY
            .read_version(f.path())
            .expect_err("no version key");
        assert!(
            format!("{err:?}").contains("4872"),
            "sharing one editor must not make a galaxy.yml failure report a Chart.yaml code: {err:?}"
        );
    }
}
