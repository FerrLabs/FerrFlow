use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::process::Command;

use super::auth::scrub_trace_env;

pub(super) fn run_git(workdir: &Path, args: &[&str]) -> Result<String> {
    run_git_with_env(workdir, args, &[])
}

pub(super) fn run_git_with_env(
    workdir: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(workdir);
    scrub_trace_env(&mut cmd);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.args(args);
    let output = cmd
        .output()
        .with_context(|| format!("spawn `git {}` failed (is git in PATH?)", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = scrub_token_patterns(&format!("{stdout}{stderr}"))
            .trim()
            .to_string();
        return Err(anyhow!("git {} failed: {}", args.join(" "), combined));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn scrub_token_patterns(s: &str) -> String {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)(authorization:\s*\S+|(?:ghs|ghp|gho|ghu|github_pat|glpat)_[A-Za-z0-9_]{20,})",
        )
        .expect("scrub regex must compile")
    });
    re.replace_all(s, "[REDACTED]").into_owned()
}
