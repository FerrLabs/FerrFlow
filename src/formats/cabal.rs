use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct CabalVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

// Cabal field names are case-insensitive and top-level fields sit at column 0,
// which is what keeps this off `version:` fields nested inside stanzas. The
// leading `^` also excludes `cabal-version:`, a different field that would
// otherwise match a looser pattern.
fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| Regex::new(r"(?im)^(version[ \t]*:[ \t]*)(\S+)").unwrap())
}

impl VersionFile for CabalVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::CABAL_READ)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        super::splice::write_via_splice(self, file_path, version, None)
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::CABAL_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[2].to_string())
            .ok_or_else(|| anyhow::anyhow!("No top-level `version:` field found in {filename}"))
            .error_code(error_code::CABAL_VERSION_NOT_FOUND)
    }
}

impl super::splice::FormatPreservingEditor for CabalVersionFile {
    fn locate_version(
        &self,
        content: &str,
        _selector: Option<&str>,
    ) -> Result<std::ops::Range<usize>> {
        version_re()
            .captures(content)
            .map(|c| c.get(2).unwrap().range())
            .ok_or_else(|| anyhow::anyhow!("No top-level `version:` field found"))
            .error_code(error_code::CABAL_VERSION_NOT_FOUND)
    }

    fn read_error(&self) -> crate::error_code::ErrorCode {
        error_code::CABAL_READ
    }

    fn write_error(&self) -> crate::error_code::ErrorCode {
        error_code::CABAL_WRITE
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

    const FIXTURE: &str = r#"cabal-version:      2.4
name:               my-package
version:            0.1.0.0
synopsis:           An example package
license:            BSD-3-Clause
build-type:         Simple

library
    exposed-modules:  MyLib
    build-depends:    base >=4.7 && <5
    default-language: Haskell2010
"#;

    #[test]
    fn read_canonical_cabal() {
        let f = write_temp(FIXTURE);
        assert_eq!(CabalVersionFile.read_version(f.path()).unwrap(), "0.1.0.0");
    }

    #[test]
    fn write_preserves_column_alignment() {
        let f = write_temp(FIXTURE);
        CabalVersionFile.write_version(f.path(), "0.2.0.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("version:            0.2.0.0"));
        assert!(out.contains("name:               my-package"));
    }

    // `cabal-version` declares the format of the file, not the package
    // version. Bumping it would change which Cabal features are available.
    #[test]
    fn cabal_version_field_is_not_mistaken_for_the_package_version() {
        let f = write_temp(FIXTURE);
        assert_eq!(CabalVersionFile.read_version(f.path()).unwrap(), "0.1.0.0");

        CabalVersionFile.write_version(f.path(), "9.9.9").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("cabal-version:      2.4"));
    }

    // Stanza fields are indented; only the column-0 field is the package
    // version. A nested `version:` must not win.
    #[test]
    fn indented_version_field_is_ignored() {
        let f = write_temp("name: pkg\nversion: 1.0.0\n\nlibrary\n    version: 9.9.9\n");
        assert_eq!(CabalVersionFile.read_version(f.path()).unwrap(), "1.0.0");

        CabalVersionFile.write_version(f.path(), "1.1.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("version: 1.1.0"));
        assert!(out.contains("    version: 9.9.9"));
    }

    #[test]
    fn read_uppercase_field_name() {
        let f = write_temp("Name: pkg\nVersion: 2.3.4\n");
        assert_eq!(CabalVersionFile.read_version(f.path()).unwrap(), "2.3.4");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("name: pkg\nsynopsis: nothing here\n");
        assert!(CabalVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn read_rejects_cabal_version_only_file() {
        let f = write_temp("cabal-version: 2.4\nname: pkg\n");
        assert!(CabalVersionFile.read_version(f.path()).is_err());
    }
}
