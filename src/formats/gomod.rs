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
            // Bootstrap case: no matching tag yet. We error with the
            // dedicated `GOMOD_NO_TAG` code so the caller in `monorepo.rs`
            // can distinguish "no version available on disk" from a genuine
            // failure. The caller then picks the right bootstrap value for
            // the package's versioning strategy (semver → `0.0.0`,
            // sequential → `0`, calver → today's date, …).
            Err(anyhow::anyhow!(
                "No git tag matching '*@v*' or 'v*' found for this package"
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
    use std::process::Command;

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

    #[test]
    fn read_version_errors_when_no_tag() {
        // When no matching tag exists, `read_version` surfaces a
        // `GOMOD_NO_TAG` error so the caller can apply a strategy-aware
        // bootstrap (see `versioning::bootstrap_version`).
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();

        for (args, err_msg) in &[
            (vec!["init", "-b", "main"], "git init"),
            (
                vec!["config", "user.email", "test@example.com"],
                "config email",
            ),
            (vec!["config", "user.name", "Test"], "config name"),
            (vec!["commit", "--allow-empty", "-m", "initial"], "commit"),
        ] {
            let out = Command::new("git")
                .args(args)
                .current_dir(repo)
                .output()
                .unwrap_or_else(|e| panic!("spawn {err_msg}: {e}"));
            assert!(
                out.status.success(),
                "{err_msg} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }

        let handler = GoModVersionFile;
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo).unwrap();
        let result = handler.read_version(Path::new("go.mod"));
        std::env::set_current_dir(original_cwd).unwrap();

        assert!(
            result.is_err(),
            "expected error when no tag, got {result:?}"
        );
    }
}
