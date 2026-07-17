pub mod changelog;
pub mod config;
pub mod conventional_commits;
pub mod error_code;
pub mod formats;
pub mod prerelease;
pub mod versioning;

#[cfg(feature = "cli")]
pub mod cache;
#[cfg(feature = "cli")]
pub mod diff;
#[cfg(feature = "cli")]
pub mod forge;
#[cfg(feature = "cli")]
pub mod git;
#[cfg(feature = "cli")]
pub mod manifest;
#[cfg(feature = "cli")]
pub mod publishers;
#[cfg(feature = "cli")]
pub mod validate;

#[cfg(all(test, feature = "cli"))]
pub mod test_utils {
    use std::path::Path;
    use std::process::Command;
    use std::sync::Mutex;

    pub static CWD_LOCK: Mutex<()> = Mutex::new(());

    pub fn with_cwd<F: FnOnce() -> anyhow::Result<()>>(
        dir: &std::path::Path,
        f: F,
    ) -> anyhow::Result<()> {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        let result = f();
        std::env::set_current_dir(&saved).unwrap();
        result
    }

    pub fn git(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("git {} failed to spawn: {}", args.join(" "), e));
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            panic!("git {} failed: {}{}", args.join(" "), stdout, stderr);
        }
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn git_with_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
        let mut cmd = Command::new("git");
        cmd.current_dir(dir).args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let out = cmd
            .output()
            .unwrap_or_else(|e| panic!("git {} failed to spawn: {}", args.join(" "), e));
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            panic!("git {} failed: {}{}", args.join(" "), stdout, stderr);
        }
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn init_repo() -> (tempfile::TempDir, crate::git::Repository) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let repo = init_repo_at(&path);
        (dir, repo)
    }

    pub fn init_repo_at(path: &Path) -> crate::git::Repository {
        git(path, &["init", "-b", "main"]);
        git(path, &["config", "user.name", "Test"]);
        git(path, &["config", "user.email", "test@test.com"]);
        git(path, &["config", "commit.gpgsign", "false"]);
        git(path, &["config", "tag.gpgsign", "false"]);
        crate::git::open_repo(path).expect("open_repo after init")
    }

    pub fn commit_file(dir: &Path, filename: &str, content: &str, message: &str, ts: i64) {
        std::fs::write(dir.join(filename), content).unwrap();
        git(dir, &["add", "--", filename]);
        let date = format!("{ts} +0000");
        git_with_env(
            dir,
            &["commit", "-m", message],
            &[("GIT_AUTHOR_DATE", &date), ("GIT_COMMITTER_DATE", &date)],
        );
    }
}
