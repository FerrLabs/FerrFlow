use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct TomlVersionFile;

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
    fn read_cargo_toml() {
        let f = write_temp("[package]\nname = \"foo\"\nversion = \"1.2.3\"\n");
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn read_pyproject_toml() {
        let f = write_temp("[project]\nname = \"foo\"\nversion = \"0.5.0\"\n");
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "0.5.0");
    }

    #[test]
    fn read_poetry_toml() {
        let f = write_temp("[tool.poetry]\nname = \"foo\"\nversion = \"3.1.0\"\n");
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "3.1.0");
    }

    #[test]
    fn read_no_version_fails() {
        let f = write_temp("[package]\nname = \"foo\"\n");
        assert!(TomlVersionFile.read_version(f.path()).is_err());
    }

    #[test]
    fn write_cargo_toml() {
        let f = write_temp("[package]\nname = \"foo\"\nversion = \"1.0.0\"\n");
        TomlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_pyproject_toml() {
        let f = write_temp("[project]\nname = \"foo\"\nversion = \"1.0.0\"\n");
        TomlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(TomlVersionFile.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_preserves_formatting() {
        let input = "[package]\nname = \"foo\"\nversion = \"1.0.0\"\nedition = \"2021\"\n";
        let f = write_temp(input);
        TomlVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("name = \"foo\""));
        assert!(content.contains("edition = \"2021\""));
    }
}

impl VersionFile for TomlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::TOML_READ)?;
        let doc: toml_edit::DocumentMut = content
            .parse()
            .with_context(|| format!("Invalid TOML in {}", file_path.display()))
            .error_code(error_code::TOML_PARSE)?;

        if let Some(v) = doc
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }

        if let Some(v) = doc
            .get("project")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }

        if let Some(v) = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }

        Err(anyhow::anyhow!(
            "No version found in {}",
            file_path.display()
        ))
        .error_code(error_code::TOML_VERSION_NOT_FOUND)?
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::TOML_READ)?;
        let mut doc: toml_edit::DocumentMut = content
            .parse()
            .with_context(|| format!("Invalid TOML in {}", file_path.display()))
            .error_code(error_code::TOML_PARSE)?;

        let mut written = false;

        if let Some(pkg) = doc.get_mut("package")
            && let Some(v) = pkg.get_mut("version")
            && v.is_str()
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(proj) = doc.get_mut("project")
            && let Some(v) = proj.get_mut("version")
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(tool) = doc.get_mut("tool")
            && let Some(poetry) = tool.get_mut("poetry")
            && let Some(v) = poetry.get_mut("version")
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written {
            Err(anyhow::anyhow!(
                "Could not find version field to update in {}",
                file_path.display()
            ))
            .error_code(error_code::TOML_VERSION_NOT_FOUND)?;
        }

        std::fs::write(file_path, doc.to_string())
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::TOML_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::TOML_INVALID_UTF8)?;
        let doc: toml_edit::DocumentMut = text
            .parse()
            .with_context(|| format!("Invalid TOML in {filename}"))
            .error_code(error_code::TOML_PARSE)?;
        if let Some(v) = doc
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }
        if let Some(v) = doc
            .get("project")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }
        if let Some(v) = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }
        Err(anyhow::anyhow!("No version found in {filename}"))
            .error_code(error_code::TOML_VERSION_NOT_FOUND)?
    }
}
