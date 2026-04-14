use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct GoModVersionFile;

impl VersionFile for GoModVersionFile {
    fn read_version(&self, _file_path: &Path) -> Result<String> {
        let output = std::process::Command::new("git")
            .args([
                "describe",
                "--tags",
                "--match",
                "*@v*",
                "--match",
                "v*",
                "--abbrev=0",
            ])
            .output()
            .context("Failed to run git describe")
            .error_code(error_code::GOMOD_GIT_DESCRIBE)?;

        if !output.status.success() {
            Err(anyhow::anyhow!(
                "No git tag matching '*@v*' or 'v*' found. \
                Create an initial tag first (e.g. git tag mymodule@v0.1.0 or git tag v0.1.0)."
            ))
            .error_code(error_code::GOMOD_NO_TAG)?;
        }

        let tag = String::from_utf8_lossy(&output.stdout);
        let tag = tag.trim();

        // FerrFlow convention: <package>@v<version> — extract version after last "@v".
        let version = if let Some(idx) = tag.rfind("@v") {
            &tag[idx + 2..]
        } else if let Some(stripped) = tag.strip_prefix('v') {
            stripped
        } else {
            tag
        };

        Ok(version.to_string())
    }

    fn write_version(&self, _file_path: &Path, _version: &str) -> Result<()> {
        // Go modules are versioned via git tags only — no file to update.
        Ok(())
    }

    fn modifies_file(&self) -> bool {
        false
    }

    fn read_version_from_bytes(&self, _content: &[u8], filename: &str) -> Result<String> {
        Err(anyhow::anyhow!(
            "go.mod version is derived from git tags, cannot parse version from file content ({filename})"
        ))
        .error_code(error_code::GOMOD_UNSUPPORTED)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn write_version_is_noop() {
        let handler = GoModVersionFile;
        handler.write_version(Path::new("go.mod"), "1.0.0").unwrap();
    }

    #[test]
    fn modifies_file_returns_false() {
        let handler = GoModVersionFile;
        assert!(!handler.modifies_file());
    }
}
