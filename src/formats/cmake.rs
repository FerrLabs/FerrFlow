use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct CmakeVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

// Anchored on the `project(` call so a `set(FOO_VERSION …)` elsewhere in the
// file can't match. `[^)]*?` keeps the search inside that call's parentheses,
// which is also what lets the common multi-line form work:
//
//     project(MyProj
//         VERSION 1.2.3
//         LANGUAGES CXX)
//
// Command names are case-insensitive in CMake; the `VERSION` keyword is
// conventionally uppercase but is matched case-insensitively for tolerance.
fn version_re() -> &'static Regex {
    VERSION_RE
        .get_or_init(|| Regex::new(r"(?is)\bproject\s*\([^)]*?\bVERSION\s+([^\s)]+)").unwrap())
}

impl VersionFile for CmakeVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::CMAKE_READ)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        super::splice::write_via_splice(self, file_path, version, None)
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::CMAKE_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[1].to_string())
            .ok_or_else(|| {
                anyhow::anyhow!("No `project(… VERSION …)` declaration found in {filename}")
            })
            .error_code(error_code::CMAKE_VERSION_NOT_FOUND)
    }
}

impl super::splice::FormatPreservingEditor for CmakeVersionFile {
    fn locate_version(
        &self,
        content: &str,
        _selector: Option<&str>,
    ) -> Result<std::ops::Range<usize>> {
        version_re()
            .captures(content)
            .map(|c| c.get(1).unwrap().range())
            .ok_or_else(|| anyhow::anyhow!("No `project(… VERSION …)` declaration found"))
            .error_code(error_code::CMAKE_VERSION_NOT_FOUND)
    }

    fn read_error(&self) -> crate::error_code::ErrorCode {
        error_code::CMAKE_READ
    }

    fn write_error(&self) -> crate::error_code::ErrorCode {
        error_code::CMAKE_WRITE
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

    const FIXTURE: &str = r#"cmake_minimum_required(VERSION 3.20)

project(MyProject VERSION 1.2.3 LANGUAGES CXX)

add_executable(app main.cpp)
"#;

    #[test]
    fn read_single_line_project() {
        let f = write_temp(FIXTURE);
        assert_eq!(CmakeVersionFile.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn write_preserves_the_rest_of_the_call() {
        let f = write_temp(FIXTURE);
        CmakeVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("project(MyProject VERSION 2.0.0 LANGUAGES CXX)"));
    }

    // `cmake_minimum_required(VERSION 3.20)` is the CMake tool version, not the
    // project version. Bumping it would raise the required toolchain.
    #[test]
    fn cmake_minimum_required_is_not_mistaken_for_the_project_version() {
        let f = write_temp(FIXTURE);
        CmakeVersionFile.write_version(f.path(), "9.9.9").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("cmake_minimum_required(VERSION 3.20)"));
        assert!(out.contains("VERSION 9.9.9 LANGUAGES"));
    }

    #[test]
    fn read_multiline_project() {
        let f = write_temp(
            "project(MyProject\n    VERSION 0.4.1\n    DESCRIPTION \"demo\"\n    LANGUAGES C CXX\n)\n",
        );
        assert_eq!(CmakeVersionFile.read_version(f.path()).unwrap(), "0.4.1");
    }

    #[test]
    fn write_multiline_project_keeps_layout() {
        let f = write_temp("project(MyProject\n    VERSION 0.4.1\n    LANGUAGES C CXX\n)\n");
        CmakeVersionFile.write_version(f.path(), "0.5.0").unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(
            out,
            "project(MyProject\n    VERSION 0.5.0\n    LANGUAGES C CXX\n)\n"
        );
    }

    #[test]
    fn read_lowercase_command_name() {
        let f = write_temp("PROJECT(Foo VERSION 3.1.4)\n");
        assert_eq!(CmakeVersionFile.read_version(f.path()).unwrap(), "3.1.4");
    }

    // A `set(..._VERSION ...)` variable is not the project version, and must not
    // be picked up when the project() call carries no VERSION of its own.
    #[test]
    fn set_version_variable_is_not_matched() {
        let f = write_temp("project(Foo LANGUAGES CXX)\nset(FOO_VERSION 7.7.7)\n");
        assert!(CmakeVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn read_no_project_fails() {
        let f = write_temp("cmake_minimum_required(VERSION 3.20)\nadd_library(x x.c)\n");
        assert!(CmakeVersionFile.read_version(f.path()).is_err());
    }
}
