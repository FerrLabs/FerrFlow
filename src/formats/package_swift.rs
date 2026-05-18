use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct PackageSwiftVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| {
        Regex::new(r#"(?m)^(\s*let\s+(?:[A-Za-z_][A-Za-z0-9_]*[Vv]ersion|version)\s*=\s*)(["'])([^"']+)(["'])"#).unwrap()
    })
}

impl VersionFile for PackageSwiftVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::PACKAGE_SWIFT_READ)?;
        self.read_version_from_bytes(content.as_bytes(), &file_path.display().to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::PACKAGE_SWIFT_READ)?;
        if !version_re().is_match(&content) {
            Err(anyhow::anyhow!(
                "No `let <name>Version = \"…\"` declaration found in {}",
                file_path.display()
            ))
            .error_code(error_code::PACKAGE_SWIFT_VERSION_NOT_FOUND)?;
        }
        let new_content = version_re().replace(&content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], version, &caps[4])
        });
        std::fs::write(file_path, new_content.as_ref())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::PACKAGE_SWIFT_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::PACKAGE_SWIFT_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[3].to_string())
            .ok_or_else(|| {
                anyhow::anyhow!("No `let <name>Version = \"…\"` declaration found in {filename}")
            })
            .error_code(error_code::PACKAGE_SWIFT_VERSION_NOT_FOUND)
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

    const FIXTURE: &str = r#"// swift-tools-version:5.9
import PackageDescription

let packageVersion = "0.1.0"

let package = Package(
    name: "MyPackage",
    dependencies: [
        .package(url: "https://github.com/apple/swift-log", from: "1.5.0"),
    ],
    targets: [
        .target(name: "MyPackage"),
    ]
)
"#;

    #[test]
    fn read_canonical_package() {
        let f = write_temp(FIXTURE);
        assert_eq!(
            PackageSwiftVersionFile.read_version(f.path()).unwrap(),
            "0.1.0"
        );
    }

    #[test]
    fn write_leaves_dependency_versions_untouched() {
        let f = write_temp(FIXTURE);
        PackageSwiftVersionFile
            .write_version(f.path(), "0.2.0")
            .unwrap();
        let out = std::fs::read_to_string(f.path()).unwrap();
        assert!(out.contains("let packageVersion = \"0.2.0\""));
        assert!(out.contains("from: \"1.5.0\""));
    }

    #[test]
    fn read_accepts_lowercase_version_name() {
        let f = write_temp("let version = \"1.2.3\"\n");
        assert_eq!(
            PackageSwiftVersionFile.read_version(f.path()).unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn read_accepts_prefixed_version_name() {
        let f = write_temp("let AppVersion = \"1.2.3\"\n");
        assert_eq!(
            PackageSwiftVersionFile.read_version(f.path()).unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn read_rejects_file_without_version_let() {
        let f = write_temp(
            "import PackageDescription\n\
             let package = Package(name: \"X\")\n",
        );
        assert!(PackageSwiftVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn read_ignores_dep_from_arg() {
        let f = write_temp(".package(url: \"https://example.com\", from: \"9.9.9\")\n");
        assert!(PackageSwiftVersionFile.read_version(f.path()).is_err());
    }
}
