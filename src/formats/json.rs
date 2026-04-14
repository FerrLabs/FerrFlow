use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct JsonVersionFile;

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
    fn read_version_from_package_json() {
        let f = write_temp(r#"{"name":"foo","version":"1.2.3"}"#);
        let handler = JsonVersionFile;
        assert_eq!(handler.read_version(f.path()).unwrap(), "1.2.3");
    }

    #[test]
    fn read_version_missing_field() {
        let f = write_temp(r#"{"name":"foo"}"#);
        let handler = JsonVersionFile;
        assert!(handler.read_version(f.path()).is_err());
    }

    #[test]
    fn write_version_updates_field() {
        let f = write_temp(r#"{"name":"foo","version":"1.0.0"}"#);
        let handler = JsonVersionFile;
        handler.write_version(f.path(), "2.0.0").unwrap();
        assert_eq!(handler.read_version(f.path()).unwrap(), "2.0.0");
    }

    #[test]
    fn write_preserves_other_fields() {
        let f = write_temp(r#"{"name":"foo","version":"1.0.0","private":true}"#);
        let handler = JsonVersionFile;
        handler.write_version(f.path(), "2.0.0").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["name"], "foo");
        assert_eq!(v["private"], true);
        assert_eq!(v["version"], "2.0.0");
    }
}

impl VersionFile for JsonVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::JSON_READ)?;
        let v: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in {}", file_path.display()))
            .error_code(error_code::JSON_PARSE)?;
        v["version"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No 'version' field in {}", file_path.display()))
            .error_code(error_code::JSON_VERSION_NOT_FOUND)
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))
            .error_code(error_code::JSON_READ)?;
        let mut v: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in {}", file_path.display()))
            .error_code(error_code::JSON_PARSE)?;
        v["version"] = serde_json::Value::String(version.to_string());
        let new_content = serde_json::to_string_pretty(&v)
            .with_context(|| format!("Cannot serialize JSON for {}", file_path.display()))
            .error_code(error_code::JSON_WRITE)?
            + "\n";
        std::fs::write(file_path, new_content)
            .with_context(|| format!("Cannot write {}", file_path.display()))
            .error_code(error_code::JSON_WRITE)?;
        Ok(())
    }

    fn read_version_from_bytes(&self, content: &[u8], filename: &str) -> Result<String> {
        let text = std::str::from_utf8(content)
            .with_context(|| format!("Invalid UTF-8 in {filename}"))
            .error_code(error_code::JSON_INVALID_UTF8)?;
        let v: serde_json::Value = serde_json::from_str(text)
            .with_context(|| format!("Invalid JSON in {filename}"))
            .error_code(error_code::JSON_PARSE)?;
        v["version"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No 'version' field in {filename}"))
            .error_code(error_code::JSON_VERSION_NOT_FOUND)
    }
}
