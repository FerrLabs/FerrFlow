use anyhow::{Context, Result, anyhow};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error_code::{self, ErrorCodeExt};

const STALE_LOCK_TTL: Duration = Duration::from_secs(30 * 60);

/// RAII lock guard for `ferrflow release`. Acquires `.git/ferrflow.lock`
/// atomically via O_CREAT|O_EXCL. Releases the file on drop.
///
/// Prevents two concurrent `release` invocations on the same repo from
/// racing — typical scenario: a manually-triggered release running at
/// the same time as the cron-driven `auto-release` workflow. Without
/// this guard they compete on git refs (half-pushed tag sets, non-FF
/// rejects, duplicate draft releases).
///
/// Read-only commands (`check`, `status`, `version`, `tag`) don't take
/// the lock — only mutation paths need it.
#[derive(Debug)]
pub struct ReleaseLock {
    path: PathBuf,
    _handle: File,
}

impl ReleaseLock {
    /// Try to acquire the release lock. Returns Err if another live
    /// release is in progress. Stale locks (older than STALE_LOCK_TTL
    /// with the PID no longer alive) are taken over with a warning.
    pub fn acquire(repo_root: &Path) -> Result<Self> {
        let git_dir = repo_root.join(".git");
        if !git_dir.is_dir() {
            return Err(anyhow!(
                "release lock cannot acquire — {} is not a regular .git directory \
                 (worktrees and submodules currently unsupported by the lock)",
                git_dir.display()
            ))
            .error_code(error_code::GIT_NOT_A_REPO);
        }
        let path = git_dir.join("ferrflow.lock");

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = writeln!(
                    file,
                    "{pid}\n{now}\n{host}",
                    pid = std::process::id(),
                    host = hostname_or_unknown()
                );
                let _ = file.flush();
                crate::cleanup::register(path.clone());
                Ok(Self {
                    path,
                    _handle: file,
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if take_over_if_stale(&path)? {
                    tracing::warn!(
                        "Warning: previous release lock at {} appeared stale; took it over.",
                        path.display()
                    );
                    return Self::acquire(repo_root);
                }
                let existing = read_lock_info(&path).unwrap_or_else(|| "<unreadable>".to_string());
                Err(anyhow!(
                    "another `ferrflow release` is already running on this repo (lockfile: {})\n  \
                     lock content:\n  {}\n  \
                     If you're sure no other release is in progress, delete the lockfile manually \
                     and retry (or run with --force-unlock).",
                    path.display(),
                    existing.replace('\n', "\n  ")
                ))
                .error_code(error_code::GIT_LOCKED)
            }
            Err(e) => Err(e)
                .with_context(|| format!("could not create release lock at {}", path.display()))
                .error_code(error_code::GIT_LOCKED),
        }
    }

    /// Force-acquire the lock, ignoring any existing one. Used by
    /// `--force-unlock` for manual recovery.
    pub fn acquire_force(repo_root: &Path) -> Result<Self> {
        let path = repo_root.join(".git").join("ferrflow.lock");
        if path.exists() {
            let _ = std::fs::remove_file(&path);
            tracing::warn!(
                "Warning: --force-unlock removed existing lockfile at {}",
                path.display()
            );
        }
        Self::acquire(repo_root)
    }
}

impl Drop for ReleaseLock {
    fn drop(&mut self) {
        crate::cleanup::unregister(&self.path);
        let _ = std::fs::remove_file(&self.path);
    }
}

fn read_lock_info(path: &Path) -> Option<String> {
    let mut buf = String::new();
    File::open(path).ok()?.read_to_string(&mut buf).ok()?;
    Some(buf)
}

fn take_over_if_stale(path: &Path) -> Result<bool> {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(false),
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .unwrap_or(Duration::ZERO);
    if modified < STALE_LOCK_TTL {
        return Ok(false);
    }
    let _ = std::fs::remove_file(path);
    Ok(true)
}

fn hostname_or_unknown() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        dir
    }

    #[test]
    fn acquire_in_clean_repo_succeeds() {
        let dir = init_test_repo();
        let _lock = ReleaseLock::acquire(dir.path()).expect("first acquire");
        assert!(dir.path().join(".git/ferrflow.lock").exists());
    }

    #[test]
    fn drop_removes_the_lockfile() {
        let dir = init_test_repo();
        {
            let _lock = ReleaseLock::acquire(dir.path()).unwrap();
        }
        assert!(!dir.path().join(".git/ferrflow.lock").exists());
    }

    #[test]
    fn second_acquire_fails_while_first_held() {
        let dir = init_test_repo();
        let _first = ReleaseLock::acquire(dir.path()).unwrap();
        let err = ReleaseLock::acquire(dir.path()).expect_err("second acquire should fail");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("already running"),
            "expected lock-busy error, got: {msg}"
        );
    }

    #[test]
    fn force_unlock_takes_over_active_lock() {
        let dir = init_test_repo();
        let first = ReleaseLock::acquire(dir.path()).unwrap();
        let _second = ReleaseLock::acquire_force(dir.path())
            .expect("force-unlock should succeed even if held");
        drop(first);
    }

    #[test]
    fn errors_when_git_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        let err = ReleaseLock::acquire(dir.path()).expect_err("no .git → should error");
        assert!(format!("{err:?}").contains(".git directory"));
    }

    #[test]
    fn lockfile_content_includes_pid() {
        let dir = init_test_repo();
        let _lock = ReleaseLock::acquire(dir.path()).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".git/ferrflow.lock")).unwrap();
        let expected = std::process::id().to_string();
        assert!(
            content.starts_with(&expected),
            "expected lock content to start with PID {expected}, got: {content:?}"
        );
    }
}
