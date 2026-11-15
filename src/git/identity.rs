use anyhow::{Result, bail};
use std::path::Path;

use super::shell::run_git_with_env;

const ACTIONS_BOT_NAME: &str = "github-actions[bot]";
const ACTIONS_BOT_EMAIL: &str = "41898282+github-actions[bot]@users.noreply.github.com";

pub fn on_github_actions() -> bool {
    std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

pub fn commit_identity(workdir: &Path) -> Option<String> {
    commit_identity_with_env(workdir, &[])
}

pub fn ensure_commit_identity(workdir: &Path) -> Result<()> {
    ensure_commit_identity_with_env(workdir, on_github_actions(), &[])
}

fn commit_identity_with_env(workdir: &Path, env: &[(&str, &str)]) -> Option<String> {
    let author = run_git_with_env(workdir, &["var", "GIT_AUTHOR_IDENT"], env).ok()?;
    run_git_with_env(workdir, &["var", "GIT_COMMITTER_IDENT"], env).ok()?;
    let ident = author.trim();
    let name_and_email = ident.rsplitn(3, ' ').nth(2).unwrap_or(ident);
    Some(name_and_email.to_string())
}

fn ensure_commit_identity_with_env(
    workdir: &Path,
    on_actions: bool,
    env: &[(&str, &str)],
) -> Result<()> {
    if commit_identity_with_env(workdir, env).is_some() {
        return Ok(());
    }

    if !on_actions {
        bail!(
            "no git identity to author the release commit and tags.\n\
             Set one before releasing:\n  \
             git config user.name \"Your Name\"\n  \
             git config user.email \"you@example.com\""
        );
    }

    tracing::info!("No git identity configured, committing as {ACTIONS_BOT_NAME}.");
    run_git_with_env(
        workdir,
        &["config", "--local", "user.name", ACTIONS_BOT_NAME],
        env,
    )?;
    run_git_with_env(
        workdir,
        &["config", "--local", "user.email", ACTIONS_BOT_EMAIL],
        env,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::ENV_LOCK;

    const INHERITED_IDENTITY_ENV: [&str; 7] = [
        "GIT_AUTHOR_NAME",
        "GIT_AUTHOR_EMAIL",
        "GIT_COMMITTER_NAME",
        "GIT_COMMITTER_EMAIL",
        "EMAIL",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
    ];

    struct IdentityEnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        saved: Vec<(&'static str, std::ffi::OsString)>,
    }

    impl IdentityEnvGuard {
        fn clear() -> Self {
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let saved = INHERITED_IDENTITY_ENV
                .into_iter()
                .filter_map(|var| std::env::var_os(var).map(|v| (var, v)))
                .collect();
            for var in INHERITED_IDENTITY_ENV {
                unsafe { std::env::remove_var(var) };
            }
            Self { _lock: lock, saved }
        }
    }

    impl Drop for IdentityEnvGuard {
        fn drop(&mut self) {
            for (var, value) in &self.saved {
                unsafe { std::env::set_var(var, value) };
            }
        }
    }

    struct IsolatedRepo {
        dir: tempfile::TempDir,
        global_config: String,
        _env: IdentityEnvGuard,
    }

    impl IsolatedRepo {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let global = dir.path().join("empty-global.gitconfig");
            std::fs::write(&global, "").unwrap();
            let repo = Self {
                global_config: global.to_string_lossy().into_owned(),
                dir,
                _env: IdentityEnvGuard::clear(),
            };
            repo.git(&["init", "-q"]);
            repo.git(&["config", "--local", "user.useConfigOnly", "true"]);
            repo
        }

        fn env(&self) -> [(&str, &str); 2] {
            [
                ("GIT_CONFIG_GLOBAL", self.global_config.as_str()),
                ("GIT_CONFIG_NOSYSTEM", "1"),
            ]
        }

        fn git(&self, args: &[&str]) -> String {
            run_git_with_env(self.dir.path(), args, &self.env()).unwrap()
        }

        fn local_config(&self, key: &str) -> Option<String> {
            run_git_with_env(
                self.dir.path(),
                &["config", "--local", "--get", key],
                &self.env(),
            )
            .ok()
            .map(|v| v.trim().to_string())
        }
    }

    #[test]
    fn an_unconfigured_repo_has_no_identity() {
        let repo = IsolatedRepo::new();
        assert_eq!(commit_identity_with_env(repo.dir.path(), &repo.env()), None);
    }

    #[test]
    fn a_configured_identity_is_reported_without_the_timestamp() {
        let repo = IsolatedRepo::new();
        repo.git(&["config", "--local", "user.name", "Jane Dev"]);
        repo.git(&["config", "--local", "user.email", "jane@example.com"]);
        assert_eq!(
            commit_identity_with_env(repo.dir.path(), &repo.env()).as_deref(),
            Some("Jane Dev <jane@example.com>")
        );
    }

    #[test]
    fn a_name_without_an_email_is_not_an_identity() {
        let repo = IsolatedRepo::new();
        repo.git(&["config", "--local", "user.name", "Jane Dev"]);
        assert_eq!(commit_identity_with_env(repo.dir.path(), &repo.env()), None);
    }

    #[test]
    fn on_actions_a_missing_identity_falls_back_to_github_actions_bot() {
        let repo = IsolatedRepo::new();
        ensure_commit_identity_with_env(repo.dir.path(), true, &repo.env()).unwrap();

        assert_eq!(
            repo.local_config("user.name").as_deref(),
            Some("github-actions[bot]")
        );
        assert_eq!(
            repo.local_config("user.email").as_deref(),
            Some("41898282+github-actions[bot]@users.noreply.github.com")
        );
        repo.git(&[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "chore(release): v1.0.0",
        ]);
    }

    #[test]
    fn on_actions_an_existing_identity_is_left_alone() {
        let repo = IsolatedRepo::new();
        repo.git(&["config", "--local", "user.name", "ferrflow[bot]"]);
        repo.git(&[
            "config",
            "--local",
            "user.email",
            "278126555+ferrflow[bot]@users.noreply.github.com",
        ]);

        ensure_commit_identity_with_env(repo.dir.path(), true, &repo.env()).unwrap();

        assert_eq!(
            repo.local_config("user.name").as_deref(),
            Some("ferrflow[bot]")
        );
    }

    #[test]
    fn off_actions_a_missing_identity_is_an_actionable_error_and_writes_nothing() {
        let repo = IsolatedRepo::new();
        let err = ensure_commit_identity_with_env(repo.dir.path(), false, &repo.env())
            .unwrap_err()
            .to_string();

        assert!(err.contains("git config user.email"), "{err}");
        assert_eq!(repo.local_config("user.name"), None);
        assert_eq!(repo.local_config("user.email"), None);
    }
}
