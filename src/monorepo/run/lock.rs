use anyhow::{Context, Result, anyhow};
use gix::Repository;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error_code::{self, ErrorCodeExt};

const STALE_LOCK_TTL: Duration = Duration::from_secs(30 * 60);

/// RAII lock guard for `ferrflow release`. Acquires `ferrflow.lock` in the
/// repository's common git dir atomically via O_CREAT|O_EXCL, and releases
/// the file on drop. The common dir is shared by every linked worktree, so
/// one repository has one lock however many worktrees are checked out.
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
    pub fn acquire(repo: &Repository) -> Result<Self> {
        let path = lock_path(repo)?;

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
                    return Self::acquire(repo);
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
    pub fn acquire_force(repo: &Repository) -> Result<Self> {
        let path = lock_path(repo)?;
        if path.exists() {
            let _ = std::fs::remove_file(&path);
            tracing::warn!(
                "Warning: --force-unlock removed existing lockfile at {}",
                path.display()
            );
        }
        Self::acquire(repo)
    }
}

fn lock_path(repo: &Repository) -> Result<PathBuf> {
    if repo.workdir().is_none() {
        return Err(anyhow!(
            "release lock cannot acquire — {} is a bare repository, which has \
             nothing to release from",
            repo.common_dir().display()
        ))
        .error_code(error_code::GIT_NOT_A_REPO);
    }
    Ok(repo.common_dir().join("ferrflow.lock"))
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

    fn git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("git should be on PATH");
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q", "-b", "main"]);
        git(dir.path(), &["config", "user.email", "t@example.com"]);
        git(dir.path(), &["config", "user.name", "t"]);
        git(dir.path(), &["commit", "-q", "--allow-empty", "-m", "root"]);
        let repo = crate::git::open_repo(dir.path()).unwrap();
        (dir, repo)
    }

    fn add_worktree(main: &Path, name: &str) -> Repository {
        let path = main.join(name);
        git(
            main,
            &["worktree", "add", "-q", "-b", name, path.to_str().unwrap()],
        );
        crate::git::open_repo(&path).unwrap()
    }

    #[test]
    fn acquire_in_clean_repo_succeeds() {
        let (dir, repo) = init_test_repo();
        let _lock = ReleaseLock::acquire(&repo).expect("first acquire");
        assert!(dir.path().join(".git/ferrflow.lock").exists());
    }

    #[test]
    fn drop_removes_the_lockfile() {
        let (dir, repo) = init_test_repo();
        {
            let _lock = ReleaseLock::acquire(&repo).unwrap();
        }
        assert!(!dir.path().join(".git/ferrflow.lock").exists());
    }

    #[test]
    fn second_acquire_fails_while_first_held() {
        let (_dir, repo) = init_test_repo();
        let _first = ReleaseLock::acquire(&repo).unwrap();
        let err = ReleaseLock::acquire(&repo).expect_err("second acquire should fail");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("already running"),
            "expected lock-busy error, got: {msg}"
        );
    }

    #[test]
    fn force_unlock_takes_over_active_lock() {
        let (_dir, repo) = init_test_repo();
        let first = ReleaseLock::acquire(&repo).unwrap();
        let _second =
            ReleaseLock::acquire_force(&repo).expect("force-unlock should succeed even if held");
        drop(first);
    }

    #[test]
    fn a_bare_repo_errors_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q", "--bare", "bare.git"]);
        let repo = crate::git::open_repo(&dir.path().join("bare.git")).unwrap();
        let err = ReleaseLock::acquire(&repo).expect_err("bare repo should error");
        let msg = format!("{err:?}");
        assert!(msg.contains("bare repository"), "{msg}");
        assert!(
            !msg.contains("worktree"),
            "the bare error should not blame worktrees: {msg}"
        );
    }

    #[test]
    fn a_worktree_can_acquire_and_shares_the_main_repository_lock() {
        let (dir, main) = init_test_repo();
        let worktree = add_worktree(dir.path(), "wt");

        let lock = ReleaseLock::acquire(&worktree).expect("a worktree should be able to release");
        assert!(
            dir.path().join(".git/ferrflow.lock").exists(),
            "the lock belongs in the common dir, not the per-worktree git dir"
        );
        assert!(
            !dir.path().join("wt/.git").is_dir(),
            "the worktree's .git should stay a file, so this proves the fallback is not in play"
        );

        let err = ReleaseLock::acquire(&main)
            .expect_err("the main checkout must not release while a worktree holds the lock");
        assert!(format!("{err:?}").contains("already running"), "{err:?}");
        drop(lock);
    }

    #[test]
    fn lockfile_content_includes_pid() {
        let (dir, repo) = init_test_repo();
        let _lock = ReleaseLock::acquire(&repo).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".git/ferrflow.lock")).unwrap();
        let expected = std::process::id().to_string();
        assert!(
            content.starts_with(&expected),
            "expected lock content to start with PID {expected}, got: {content:?}"
        );
    }
}
