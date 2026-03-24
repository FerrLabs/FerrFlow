use super::VersionFile;
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct GradleVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE
        .get_or_init(|| Regex::new(r#"(?m)^(\s*version\s*=\s*)(["'])([^"']+)(["'])"#).unwrap())
}

impl VersionFile for GradleVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;

        version_re()
            .captures(&content)
            .map(|c| c[3].to_string())
            .ok_or_else(|| anyhow::anyhow!("No version field found in {}", file_path.display()))
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;

        if !version_re().is_match(&content) {
            anyhow::bail!(
                "No version field found to update in {}",
                file_path.display()
            );
        }

        let new_content = version_re().replace(&content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], version, &caps[4])
        });

        std::fs::write(file_path, new_content.as_ref())?;
        Ok(())
    }
}
