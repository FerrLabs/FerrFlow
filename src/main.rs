mod bot_token;
mod cache;
mod changelog;
mod cli;
mod concurrency;
mod config;
mod conventional_commits;
mod diff;
mod error_code;
mod forge;
mod formats;
mod git;
mod hooks;
mod logging;
mod manifest;
mod monorepo;
mod prerelease;
mod publish;
mod publishers;
mod query;
mod status;
mod telemetry;
mod timing;
mod validate;
mod versioning;

// Allocator swap for the CLI hot paths. The default system allocator
// (glibc malloc / Windows HeapAlloc) is well-known to be suboptimal on
// alloc-heavy short-lived CLIs; mimalloc consistently wins 5-15% on
// commit-walk / tag-scan / regex-capture workloads — see #507. Disabled
// in tests so they don't drag in the dep when not needed.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();
    let command_name = cli.command.name();

    logging::init_logging(cli.verbose, cli.log_format);
    concurrency::init(cli.jobs);

    telemetry::maybe_print_first_run_notice();

    let result = cli.run();

    if let Err(err) = result {
        let code = err.downcast_ref::<error_code::ErrorCode>().copied();

        let error_code_str = code.map(|c| c.to_string());
        let mut metadata = serde_json::Map::new();
        metadata.insert("command".into(), command_name.into());
        if let Some(ref code_str) = error_code_str {
            metadata.insert("error_code".into(), code_str.clone().into());
        }
        telemetry::send_event(
            telemetry::EventType::Error,
            Some(serde_json::Value::Object(metadata)),
            None,
            None,
            None,
        );

        telemetry::flush();

        if let Some(code) = code {
            let msgs: Vec<String> = err
                .chain()
                .filter(|c| c.downcast_ref::<error_code::ErrorCode>().is_none())
                .map(|c| c.to_string())
                .collect();

            tracing::error!("error[{}]: {}", code, msgs[0]);
            for msg in &msgs[1..] {
                tracing::error!("  {msg}");
            }
            tracing::error!("");
            tracing::error!("  For help: {}", code.doc_url());
        } else {
            tracing::error!("Error: {err:?}");
        }

        std::process::exit(1);
    }

    telemetry::flush();
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
