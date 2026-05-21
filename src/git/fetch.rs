use anyhow::{Context, Result, anyhow};

use super::auth::{configure_git_command, get_remote_url};
use super::repo::Repository;

pub fn fetch_tags(repo: &Repository, remote_name: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("Bare repositories are not supported"))?;
    let url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow!("Remote '{remote_name}' not found"))?;

    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(workdir);
    configure_git_command(&mut cmd, &url);
    cmd.args(["fetch", "--tags", remote_name]);

    let output = cmd
        .output()
        .with_context(|| "spawn `git fetch --tags` failed (is git in PATH?)")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git fetch --tags failed: {}", stderr.trim()));
    }
    Ok(())
}
