use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct XmlVersionFile;

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

    const POM: &str = r#"<?xml version="1.0"?>
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>myapp</artifactId>
  <version>1.0.0</version>
</project>"#;

    #[test]
    fn read_pom_version() {
        let f = write_temp(POM);
        assert_eq!(XmlVersionFile.read_version(f.path()).unwrap(), "1.0.0");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("<project><groupId>com.example</groupId></project>");
        assert!(XmlVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_pom_version() {
        let f = write_temp(POM);
        XmlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(XmlVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_replaces_first_version_tag_only() {
        let xml = "<project><version>1.0.0</version><dependencies><dependency><version>3.0</version></dependency></dependencies></project>";
        let f = write_temp(xml);
        XmlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        // Only the first <version> should be replaced
        assert!(content.contains("<version>2.0.0</version>"));
    }
}

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| Regex::new(r"<version>([^<]+)</version>").unwrap())
}

impl VersionFile for XmlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::XML_READ)?;

        version_re()
            .captures(&content)
            .map(|c| c[1].trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("No <version> tag found in {}", file_path.display()))
            .error_code(error_code::XML_VERSION_NOT_FOUND)
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::XML_READ)?;

        let mut count = 0;
        let new_content = version_re().replace(&content, |_: &regex::Captures| {
            count += 1;
            format!("<version>{version}</version>")
        });

        if count == 0 {
            Err(anyhow::anyhow!(
                "No <version> tag found to update in {}",
                file_path.display()
            ))
            .error_code(error_code::XML_VERSION_NOT_FOUND)?;
        }

        std::fs::write(file_path, new_content.as_ref())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::XML_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::XML_INVALID_UTF8)?;
        version_re()
            .captures(text)
            .map(|c| c[1].trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("No <version> tag found in {filename}"))
            .error_code(error_code::XML_VERSION_NOT_FOUND)
    }
}
