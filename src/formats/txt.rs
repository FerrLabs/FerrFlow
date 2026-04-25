use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

pub struct TxtVersionFile;

/// Compile a user-supplied selector into a regex. The selector must contain
/// exactly one capture group whose match is the version string. We surface
/// a clear error rather than panicking on regex compile failures or wrong
/// arity, since the selector ships from user-authored config.
fn compile_selector(selector: &str) -> Result<Regex> {
    let re = Regex::new(selector)
        .with_context(|| format!("invalid regex selector: {selector:?}"))
        .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
    if re.captures_len() != 2 {
        Err(anyhow::anyhow!(
            "regex selector must contain exactly one capture group: {selector:?}"
        ))
        .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
    }
    Ok(re)
}

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

    #[test]
    fn read_with_regex_selector_picks_capture_group() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "name=foo\nVERSION=4.5.6\nother=ignored\n").unwrap();
        let v = TxtVersionFile
            .read_version_with_selector(f.path(), Some(r"(?m)^VERSION=(.+)$"))
            .unwrap();
        assert_eq!(v, "4.5.6");
    }

    #[test]
    fn write_with_regex_selector_replaces_capture_only() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "name=foo\nVERSION=1.0.0\nother=ignored\n").unwrap();
        TxtVersionFile
            .write_version_with_selector(f.path(), "2.0.0", Some(r"(?m)^VERSION=(.+)$"))
            .unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        // Only the version part of the matched line changes.
        assert_eq!(content, "name=foo\nVERSION=2.0.0\nother=ignored\n");
    }

    #[test]
    fn selector_with_no_match_errors() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "no version here").unwrap();
        let result = TxtVersionFile.read_version_with_selector(f.path(), Some(r"^VERSION=(.+)$"));
        assert!(result.is_err());
    }

    #[test]
    fn selector_with_wrong_capture_count_errors() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "VERSION=1.0.0").unwrap();
        // Zero capture groups.
        let result = TxtVersionFile.read_version_with_selector(f.path(), Some(r"VERSION=.+"));
        assert!(result.is_err());
        // Two capture groups.
        let result = TxtVersionFile.read_version_with_selector(f.path(), Some(r"(VERSION)=(.+)"));
        assert!(result.is_err());
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

    fn read_version_with_selector(
        &self,
        file_path: &Path,
        selector: Option<&str>,
    ) -> Result<String> {
        let Some(sel) = selector else {
            return self.read_version(file_path);
        };
        let re = compile_selector(sel)?;
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))
            .error_code(error_code::TXT_READ)?;
        let cap = re
            .captures(&content)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "selector {sel:?} did not match anything in {}",
                    file_path.display()
                )
            })
            .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
        let m = cap.get(1).ok_or_else(|| {
            anyhow::anyhow!("selector {sel:?} matched but capture group 1 is empty")
        })?;
        Ok(m.as_str().to_string())
    }

    fn write_version_with_selector(
        &self,
        file_path: &Path,
        version: &str,
        selector: Option<&str>,
    ) -> Result<()> {
        let Some(sel) = selector else {
            return self.write_version(file_path, version);
        };
        let re = compile_selector(sel)?;
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))
            .error_code(error_code::TXT_READ)?;
        // Replace only the capture group, leaving the rest of the matched
        // line (prefix, suffix, surrounding whitespace) untouched. We need
        // the absolute byte range of capture group 1 — `replacen` would
        // operate on the whole match.
        let cap = re
            .captures(&content)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "selector {sel:?} did not match anything in {}",
                    file_path.display()
                )
            })
            .error_code(error_code::TXT_VERSION_NOT_FOUND)?;
        let m = cap.get(1).ok_or_else(|| {
            anyhow::anyhow!("selector {sel:?} matched but capture group 1 is empty")
        })?;
        let mut new_content = String::with_capacity(content.len() + version.len());
        new_content.push_str(&content[..m.start()]);
        new_content.push_str(version);
        new_content.push_str(&content[m.end()..]);
        std::fs::write(file_path, new_content)
            .with_context(|| format!("failed to write {}", file_path.display()))
            .error_code(error_code::TXT_WRITE)?;
        Ok(())
    }
}
