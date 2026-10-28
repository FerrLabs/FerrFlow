use std::process::Command;

const REPO_LOCATION_ENV: [&str; 10] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_SHALLOW_FILE",
    "GIT_GRAFT_FILE",
    "GIT_PREFIX",
];

pub fn command() -> Command {
    let mut cmd = Command::new("git");
    for var in REPO_LOCATION_ENV {
        cmd.env_remove(var);
    }
    cmd
}

#[cfg(all(test, feature = "cli"))]
mod tests {
    use crate::test_utils::{ENV_LOCK, git, init_repo};
    use std::path::Path;

    struct GitDirGuard(Option<std::ffi::OsString>);

    impl GitDirGuard {
        fn set(path: &Path) -> Self {
            let previous = std::env::var_os("GIT_DIR");
            unsafe { std::env::set_var("GIT_DIR", path) };
            Self(previous)
        }
    }

    impl Drop for GitDirGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(v) => std::env::set_var("GIT_DIR", v),
                    None => std::env::remove_var("GIT_DIR"),
                }
            }
        }
    }

    #[test]
    fn an_inherited_git_dir_never_redirects_git_away_from_the_temp_repo() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let decoy = tempfile::tempdir().unwrap();
        git(decoy.path(), &["init", "-q"]);
        let decoy_config = decoy.path().join(".git/config");
        let config_before = std::fs::read_to_string(&decoy_config).unwrap();

        {
            let _guard = GitDirGuard::set(&decoy.path().join(".git"));
            let initialized = init_repo();
            let git_dir = super::command()
                .current_dir(initialized.0.path())
                .args(["rev-parse", "--absolute-git-dir"])
                .output()
                .unwrap();
            assert!(git_dir.status.success());
            assert_eq!(
                Path::new(String::from_utf8_lossy(&git_dir.stdout).trim())
                    .canonicalize()
                    .unwrap(),
                initialized.0.path().join(".git").canonicalize().unwrap()
            );
        }

        assert_eq!(
            std::fs::read_to_string(&decoy_config).unwrap(),
            config_before
        );
    }
}
