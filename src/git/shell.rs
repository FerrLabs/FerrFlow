use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::process::Command;

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
        let combined = format!("{stdout}{stderr}").trim().to_string();
        return Err(anyhow!("git {} failed: {}", args.join(" "), combined));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
