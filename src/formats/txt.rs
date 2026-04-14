use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct TxtVersionFile;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::VersionFile;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn read_version_from_txt() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "1.2.3").unwrap();
        let v = TxtVersionFile.read_version(f.path()).unwrap();
        assert_eq!(v, "1.2.3");
    }

    #[test]
    fn read_version_trims_whitespace() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "  0.4.1\n\n").unwrap();
        let v = TxtVersionFile.read_version(f.path()).unwrap();
        assert_eq!(v, "0.4.1");
    }

    #[test]
    fn read_empty_file_fails() {
        let f = NamedTempFile::new().unwrap();
        let result = TxtVersionFile.read_version(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn write_version_to_txt() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "1.0.0").unwrap();
        TxtVersionFile.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "2.0.0\n");
    }
}

impl super::VersionFile for TxtVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))
            .error_code(error_code::TXT_READ)?;
        let version = content.trim();
        if version.is_empty() {
            Err(anyhow::anyhow!(
                "no version found in {}",
                file_path.display()
            ))
            .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
        }
        Ok(version.to_string())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        std::fs::write(file_path, format!("{version}\n"))
            .with_context(|| format!("failed to write {}", file_path.display()))
            .error_code(error_code::TXT_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::TXT_INVALID_UTF8)?;
        let version = text.trim();
        if version.is_empty() {
            Err(anyhow::anyhow!("no version found in {filename}"))
                .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
        }
        Ok(version.to_string())
    }
}
