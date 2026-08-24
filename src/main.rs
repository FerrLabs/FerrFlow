mod api_check;
mod bot_token;
mod cache;
mod changelog;
mod cli;
mod concurrency;
mod config;
mod conventional_commits;
mod diff;
mod doctor;
mod error_code;
mod error_report;
mod forge;
mod formats;
mod git;
mod hooks;
mod http;
mod logging;
mod manifest;
mod monorepo;
mod prerelease;
mod publish;
mod publishers;
mod query;
mod schema;
mod status;
mod timing;
mod validate;
mod version_diff;
mod versioning;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    logging::init_logging(cli.verbose, cli.log_format);

    if cli.command.needs_bot_token()
        && let Err(err) = bot_token::ensure_bot_token()
    {
        for line in error_report::error_report_lines(&err) {
            tracing::error!("{line}");
        }
        std::process::exit(1);
    }

    concurrency::init(cli.jobs);

    let result = cli.run();

    if let Err(err) = result {
        for line in error_report::error_report_lines(&err) {
            tracing::error!("{line}");
        }

        std::process::exit(1);
    }
}

#[cfg(test)]
mod test_utils {
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
