pub mod chart_yaml;
pub mod csproj;
pub mod gemspec;
pub mod gomod;
pub mod gradle;
pub mod helm;
pub mod json;
pub mod mix_exs;
pub mod package_swift;
pub mod pubspec_yaml;
pub mod toml_format;
pub mod txt;
pub mod xml;

use anyhow::Result;
use std::path::Path;

use crate::config::{FileFormat, VersionedFile};

pub trait VersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String>;
    fn write_version(&self, file_path: &Path, version: &str) -> Result<()>;
    /// Returns false if this format tracks versions via git tags only and
    /// does not write any file. Callers should skip committing the path.
    fn modifies_file(&self) -> bool {
        true
    }
    /// Parse version from raw file content without filesystem access.
    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String>;

    /// Read with an optional per-file selector. Default delegates to the
    /// selector-less path; formats that support selectors override this.
    /// See [`crate::config::VersionedFile::selector`] for syntax.
    fn read_version_with_selector(
        &self,
        file_path: &Path,
        _selector: Option<&str>,
    ) -> Result<String> {
        self.read_version(file_path)
    }

    /// Write counterpart of [`Self::read_version_with_selector`].
    fn write_version_with_selector(
        &self,
        file_path: &Path,
        version: &str,
        _selector: Option<&str>,
    ) -> Result<()> {
        self.write_version(file_path, version)
    }
}

pub fn get_handler(format: &FileFormat) -> Box<dyn VersionFile> {
    match format {
        FileFormat::Csproj => Box::new(csproj::CsprojVersionFile),
        FileFormat::GoMod => Box::new(gomod::GoModVersionFile),
        FileFormat::Gradle => Box::new(gradle::GradleVersionFile),
        FileFormat::Helm => Box::new(helm::HelmVersionFile),
        FileFormat::Json => Box::new(json::JsonVersionFile),
        FileFormat::Toml => Box::new(toml_format::TomlVersionFile),
        FileFormat::Txt => Box::new(txt::TxtVersionFile),
        FileFormat::Xml => Box::new(xml::XmlVersionFile),
        FileFormat::PubspecYaml => Box::new(pubspec_yaml::PubspecYamlVersionFile),
        FileFormat::MixExs => Box::new(mix_exs::MixExsVersionFile),
        FileFormat::ChartYaml => Box::new(chart_yaml::ChartYamlVersionFile),
        FileFormat::Gemspec => Box::new(gemspec::GemspecVersionFile),
        FileFormat::PackageSwift => Box::new(package_swift::PackageSwiftVersionFile),
    }
}

pub fn read_version(vf: &VersionedFile, repo_root: &Path) -> Result<String> {
    let path = repo_root.join(&vf.path);
    let handler = get_handler(&vf.format);
    handler.read_version_with_selector(&path, vf.selector.as_deref())
}

pub fn write_version(vf: &VersionedFile, repo_root: &Path, version: &str) -> Result<()> {
    let path = repo_root.join(&vf.path);
    let handler = get_handler(&vf.format);
    handler.write_version_with_selector(&path, version, vf.selector.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FileFormat, VersionedFile};

    #[test]
    fn get_handler_returns_handler_for_each_format() {
        // Verify get_handler doesn't panic for any format variant
        for format in &[
            FileFormat::Csproj,
            FileFormat::GoMod,
            FileFormat::Gradle,
            FileFormat::Helm,
            FileFormat::Json,
            FileFormat::Toml,
            FileFormat::Txt,
            FileFormat::Xml,
            FileFormat::PubspecYaml,
            FileFormat::MixExs,
            FileFormat::ChartYaml,
            FileFormat::Gemspec,
            FileFormat::PackageSwift,
        ] {
            let _ = get_handler(format);
        }
    }

    #[test]
    fn gomod_handler_does_not_modify_file() {
        let handler = get_handler(&FileFormat::GoMod);
        assert!(!handler.modifies_file());
    }

    #[test]
    fn non_gomod_handlers_modify_file() {
        for format in &[
            FileFormat::Csproj,
            FileFormat::Gradle,
            FileFormat::Helm,
            FileFormat::Json,
            FileFormat::Toml,
            FileFormat::Txt,
            FileFormat::Xml,
            FileFormat::PubspecYaml,
            FileFormat::MixExs,
            FileFormat::ChartYaml,
            FileFormat::Gemspec,
            FileFormat::PackageSwift,
        ] {
            let handler = get_handler(format);
            assert!(
                handler.modifies_file(),
                "expected modifies_file=true for {:?}",
                format
            );
        }
    }

    #[test]
    fn read_version_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"test","version":"3.2.1"}"#,
        )
        .unwrap();
        let vf = VersionedFile {
            path: "package.json".to_string(),
            format: FileFormat::Json,
            selector: None,
        };
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "3.2.1");
    }

    #[test]
    fn write_then_read_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"test","version":"1.0.0"}"#,
        )
        .unwrap();
        let vf = VersionedFile {
            path: "package.json".to_string(),
            format: FileFormat::Json,
            selector: None,
        };
        write_version(&vf, dir.path(), "2.0.0").unwrap();
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn read_version_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.5.0\"\n",
        )
        .unwrap();
        let vf = VersionedFile {
            path: "Cargo.toml".to_string(),
            format: FileFormat::Toml,
            selector: None,
        };
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "0.5.0");
    }

    #[test]
    fn read_version_txt() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("VERSION"), "4.1.0\n").unwrap();
        let vf = VersionedFile {
            path: "VERSION".to_string(),
            format: FileFormat::Txt,
            selector: None,
        };
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "4.1.0");
    }

    #[test]
    fn write_then_read_txt() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("VERSION"), "1.0.0\n").unwrap();
        let vf = VersionedFile {
            path: "VERSION".to_string(),
            format: FileFormat::Txt,
            selector: None,
        };
        write_version(&vf, dir.path(), "1.1.0").unwrap();
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "1.1.0");
    }

    #[test]
    fn read_version_xml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project><version>2.3.4</version></project>",
        )
        .unwrap();
        let vf = VersionedFile {
            path: "pom.xml".to_string(),
            format: FileFormat::Xml,
            selector: None,
        };
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "2.3.4");
    }

    #[test]
    fn read_version_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let vf = VersionedFile {
            path: "nope.json".to_string(),
            format: FileFormat::Json,
            selector: None,
        };
        assert!(read_version(&vf, dir.path()).is_err());
    }

    #[test]
    fn read_version_gradle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle"), "version = '1.2.3'\n").unwrap();
        let vf = VersionedFile {
            path: "build.gradle".to_string(),
            format: FileFormat::Gradle,
            selector: None,
        };
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn path_joining_works() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("VERSION"), "5.0.0\n").unwrap();
        let vf = VersionedFile {
            path: "sub/VERSION".to_string(),
            format: FileFormat::Txt,
            selector: None,
        };
        assert_eq!(read_version(&vf, dir.path()).unwrap(), "5.0.0");
    }
}
