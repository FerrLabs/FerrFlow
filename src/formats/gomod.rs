use super::VersionFile;
use crate::error_code::{self, ErrorCodeExt};
use anyhow::{Context, Result};
use std::path::Path;

pub struct GoModVersionFile;

impl VersionFile for GoModVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let mut cmd = std::process::Command::new("git");
        cmd.args([
            "describe",
            "--tags",
            "--match",
            "*@v*",
            "--match",
            "v*",
            "--abbrev=0",
        ]);
        let parent = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!(
                "go.mod file path has no parent directory: {}",
                file_path.display()
            )
        })?;
        if parent.as_os_str().is_empty() {
            cmd.current_dir(".");
        } else {
            cmd.current_dir(parent);
        }
        let output = cmd
            .output()
            .context("Failed to run git describe")
            .error_code(error_code::GOMOD_GIT_DESCRIBE)?;

        if !output.status.success() {
            Err(anyhow::anyhow!(
                "No git tag matching '*@v*' or 'v*' found for this package"
            ))
            .error_code(error_code::GOMOD_NO_TAG)?;
        }

        let tag = String::from_utf8_lossy(&output.stdout);
        let tag = tag.trim();

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

    fn init_repo(repo: &Path) {
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
    }

    #[test]
    fn read_version_errors_when_no_tag() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init_repo(repo);

        let handler = GoModVersionFile;
        let result = handler.read_version(&repo.join("go.mod"));

        assert!(
            result.is_err(),
            "expected error when no tag, got {result:?}"
        );
    }

    #[test]
    fn read_version_uses_file_path_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init_repo(repo);

        let tag_out = Command::new("git")
            .args(["tag", "v1.2.3"])
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(tag_out.status.success());

        let other_dir = tempfile::tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(other_dir.path()).unwrap();

        let handler = GoModVersionFile;
        let result = handler.read_version(&repo.join("go.mod"));

        std::env::set_current_dir(original_cwd).unwrap();

        let v = result.expect("should resolve version via file_path parent");
        assert_eq!(v, "1.2.3");
    }
}
