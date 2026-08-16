use anyhow::{Context, Result, anyhow};
use gix::ObjectId;
use std::collections::HashMap;
use std::path::Path;

use crate::error_code::{self, ErrorCodeExt};

use super::auth::{configure_git_command, get_remote_url};
use super::repo::Repository;
use super::retry::retry_transient;
use super::shell::run_git;

pub(super) fn local_tag_target_sha(repo: &Repository, tag: &str) -> Result<String> {
    let reference = repo
        .find_reference(&format!("refs/tags/{tag}"))
        .with_context(|| format!("could not resolve tag '{tag}' to a commit"))?;
    let id = reference
        .into_fully_peeled_id()
        .with_context(|| format!("could not peel tag '{tag}' to a commit"))?;
    Ok(id.to_string())
}

pub(super) fn parse_ls_remote_tags(stdout: &str) -> HashMap<String, String> {
    let mut tag_objects: HashMap<String, String> = HashMap::new();
    let mut dereferenced: HashMap<String, String> = HashMap::new();
    for line in stdout.lines() {
        let Some((sha, refname)) = line.split_once('\t') else {
            continue;
        };
        let Some(name) = refname.strip_prefix("refs/tags/") else {
            continue;
        };
        if let Some(base) = name.strip_suffix("^{}") {
            dereferenced.insert(base.to_string(), sha.trim().to_string());
        } else {
            tag_objects.insert(name.to_string(), sha.trim().to_string());
        }
    }
    let mut out = tag_objects;
    for (name, sha) in dereferenced {
        out.insert(name, sha);
    }
    out
}

pub(super) fn remote_tag_target_shas(
    workdir: &Path,
    push_url: &str,
    tags: &[&str],
) -> Result<HashMap<String, String>> {
    if tags.is_empty() {
        return Ok(HashMap::new());
    }
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(workdir);
    configure_git_command(&mut cmd, push_url);
    cmd.arg("ls-remote").arg("--tags").arg(push_url);
    for tag in tags {
        cmd.arg(format!("refs/tags/{tag}"));
    }
    let output = cmd
        .output()
        .with_context(|| "spawn `git ls-remote --tags` failed (is git in PATH?)")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git ls-remote --tags failed: {}", stderr.trim()));
    }
    Ok(parse_ls_remote_tags(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

pub fn force_push_tags(repo: &Repository, remote_name: &str, tags: &[&str]) -> Result<()> {
    if tags.is_empty() {
        return Ok(());
    }
    retry_transient("force-push floating tags", || {
        try_force_push_tags_once(repo, remote_name, tags)
    })
}

fn try_force_push_tags_once(repo: &Repository, remote_name: &str, tags: &[&str]) -> Result<()> {
    shell_push_tags(repo, remote_name, tags, true).error_code(error_code::GIT_FLOATING_TAGS)
}

pub fn verify_remote_branch(
    repo: &Repository,
    remote_name: &str,
    branch: &str,
    expected_oid: ObjectId,
) -> Result<()> {
    super::validate::ensure_safe_refname_fragment(remote_name, "remote name")?;
    super::validate::ensure_safe_refname_fragment(branch, "branch name")?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("Bare repositories are not supported"))?;
    let url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow!("Remote '{remote_name}' has no URL"))?;

    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(workdir);
    configure_git_command(&mut cmd, &url);
    cmd.args([
        "ls-remote",
        "--heads",
        &url,
        &format!("refs/heads/{branch}"),
    ]);

    let output = cmd
        .output()
        .with_context(|| "spawn `git ls-remote --heads` failed (is git in PATH?)")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git ls-remote --heads failed: {}", stderr.trim()));
    }

    let expected_ref = format!("refs/heads/{branch}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let Some((sha, refname)) = line.split_once('\t') else {
            continue;
        };
        if refname.trim() == expected_ref {
            let actual = ObjectId::from_hex(sha.trim().as_bytes())
                .with_context(|| format!("invalid sha from remote: {sha}"))?;
            if actual == expected_oid {
                return Ok(());
            }
            Err(anyhow!(
                "Remote branch '{}' points to {} but expected {}",
                branch,
                actual,
                expected_oid,
            ))
            .error_code(error_code::GIT_PUSH_VERIFY_FAILED)?;
        }
    }
    Err(anyhow!("Remote branch '{}' not found after push", branch))
        .error_code(error_code::GIT_REMOTE_BRANCH_NOT_FOUND)?;
    Ok(())
}

fn resolve_push_source(repo: &Repository, branch: &str) -> String {
    let local_ref = format!("refs/heads/{branch}");
    if repo.find_reference(&local_ref).is_ok() {
        local_ref
    } else {
        "HEAD".to_string()
    }
}

pub fn force_push_branch(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    super::validate::ensure_safe_refname_fragment(remote_name, "remote name")?;
    super::validate::ensure_safe_refname_fragment(branch, "branch name")?;
    retry_transient(&format!("force-push branch '{branch}'"), || {
        try_force_push_branch_once(repo, remote_name, branch)
    })
}

fn try_force_push_branch_once(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("bare repos are not supported"))?;
    let push_url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow!("Remote '{remote_name}' has no URL"))?;
    let source = resolve_push_source(repo, branch);
    let refspec = format!("+{source}:refs/heads/{branch}");

    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(workdir);
    configure_git_command(&mut cmd, &push_url);
    cmd.arg("push").arg(&push_url).arg(&refspec);

    let output = cmd
        .output()
        .with_context(|| format!("spawn `git push --force` for branch '{branch}' failed"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = format!("{stdout}{stderr}").trim().to_string();
        return Err(anyhow!("Failed to force-push branch '{branch}': {detail}"))
            .error_code(error_code::GIT_FORCE_PUSH_BRANCH);
    }
    Ok(())
}

// SAFETY GUARD for the persistent release PR: before force-pushing the
pub fn release_branch_foreign_commit(
    repo: &Repository,
    remote_name: &str,
    branch: &str,
    base: &str,
) -> Result<Option<String>> {
    super::validate::ensure_safe_refname_fragment(remote_name, "remote name")?;
    super::validate::ensure_safe_refname_fragment(branch, "branch name")?;
    super::validate::ensure_safe_refname_fragment(base, "target branch")?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("bare repos are not supported"))?;
    let url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow!("Remote '{remote_name}' has no URL"))?;

    let mut ls = std::process::Command::new("git");
    ls.current_dir(workdir);
    configure_git_command(&mut ls, &url);
    ls.args([
        "ls-remote",
        "--heads",
        &url,
        &format!("refs/heads/{branch}"),
    ]);
    let ls_out = ls
        .output()
        .with_context(|| "spawn `git ls-remote` failed")
        .error_code(error_code::GIT_INSPECT_RELEASE_BRANCH)?;
    if !ls_out.status.success() {
        return Err(anyhow!(
            "git ls-remote failed: {}",
            String::from_utf8_lossy(&ls_out.stderr).trim()
        ))
        .error_code(error_code::GIT_INSPECT_RELEASE_BRANCH);
    }
    if String::from_utf8_lossy(&ls_out.stdout).trim().is_empty() {
        return Ok(None);
    }

    let mut fetch = std::process::Command::new("git");
    fetch.current_dir(workdir);
    configure_git_command(&mut fetch, &url);
    fetch.args(["fetch", "--quiet", &url, &format!("refs/heads/{branch}")]);
    let fetch_out = fetch
        .output()
        .with_context(|| "spawn `git fetch` for release branch failed")
        .error_code(error_code::GIT_INSPECT_RELEASE_BRANCH)?;
    if !fetch_out.status.success() {
        return Err(anyhow!(
            "git fetch of release branch failed: {}",
            String::from_utf8_lossy(&fetch_out.stderr).trim()
        ))
        .error_code(error_code::GIT_INSPECT_RELEASE_BRANCH);
    }

    let mut log = std::process::Command::new("git");
    log.current_dir(workdir);
    log.args(["log", "--format=%s", &format!("{base}..FETCH_HEAD")]);
    let log_out = log
        .output()
        .with_context(|| "spawn `git log` for release branch failed")
        .error_code(error_code::GIT_INSPECT_RELEASE_BRANCH)?;
    if !log_out.status.success() {
        return Err(anyhow!(
            "git log of release branch failed: {}",
            String::from_utf8_lossy(&log_out.stderr).trim()
        ))
        .error_code(error_code::GIT_INSPECT_RELEASE_BRANCH);
    }
    for subject in String::from_utf8_lossy(&log_out.stdout).lines() {
        let subject = subject.trim();
        if !subject.is_empty() && !subject.starts_with("chore(release):") {
            return Ok(Some(subject.to_string()));
        }
    }
    Ok(None)
}

pub fn push_tags(repo: &Repository, remote_name: &str, tags: &[&str]) -> Result<()> {
    if tags.is_empty() {
        return Ok(());
    }
    super::validate::ensure_safe_refname_fragment(remote_name, "remote name")?;
    for tag in tags {
        super::validate::ensure_safe_refname_fragment(tag, "tag name")?;
    }
    retry_transient("push tags", || try_push_tags_once(repo, remote_name, tags))
}

fn try_push_tags_once(repo: &Repository, remote_name: &str, tags: &[&str]) -> Result<()> {
    shell_push_tags(repo, remote_name, tags, false).error_code(error_code::GIT_PUSH_TAGS)
}

fn shell_push_tags(repo: &Repository, remote_name: &str, tags: &[&str], force: bool) -> Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("bare repos are not supported"))?;
    let push_url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow!("Remote '{remote_name}' has no URL"))?;

    let remote_shas = remote_tag_target_shas(workdir, &push_url, tags).unwrap_or_else(|err| {
        tracing::warn!(
            "  Warning: could not enumerate remote tag state ({err}); falling back to a plain push"
        );
        HashMap::new()
    });

    let mut to_push: Vec<&str> = Vec::with_capacity(tags.len());
    let mut already_synced: Vec<&str> = Vec::new();
    let mut diverged: Vec<(String, String, String)> = Vec::new();
    for tag in tags {
        let local_sha = local_tag_target_sha(repo, tag)?;
        match remote_shas.get(*tag) {
            Some(remote_sha) if remote_sha == &local_sha => already_synced.push(*tag),
            Some(remote_sha) if !force => {
                diverged.push(((*tag).to_string(), local_sha, remote_sha.clone()));
            }
            _ => to_push.push(*tag),
        }
    }

    if !already_synced.is_empty() {
        tracing::info!(
            "  ↻ Already on remote at the same commit: {}",
            already_synced.join(", ")
        );
    }
    if !diverged.is_empty() {
        let joined = diverged
            .iter()
            .map(|(t, l, r)| {
                format!(
                    "{t} (local {} != remote {})",
                    &l[..7.min(l.len())],
                    &r[..7.min(r.len())]
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow!(
            "Tag(s) already exist on remote pointing to a different commit: {joined}. \
             This usually means a previous release run partially succeeded — \
             delete the divergent remote tag(s) and retry, or use --force if you really want to overwrite."
        ));
    }
    if to_push.is_empty() {
        return Ok(());
    }

    let prefix = if force { "+" } else { "" };
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(workdir);
    configure_git_command(&mut cmd, &push_url);
    cmd.arg("push").arg(&push_url);
    for tag in &to_push {
        cmd.arg(format!("{prefix}refs/tags/{tag}:refs/tags/{tag}"));
    }

    let output = cmd
        .output()
        .with_context(|| "spawn `git push` for tags failed (is git in PATH?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = format!("{stdout}{stderr}").trim().to_string();
        let label = if force {
            "Failed to force-push floating tags"
        } else {
            "Failed to push tags"
        };
        return Err(anyhow!("{label}: {detail}"));
    }
    Ok(())
}

fn try_push_branch(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    retry_transient(&format!("push branch '{branch}'"), || {
        try_push_branch_once(repo, remote_name, branch)
    })
}

fn try_push_branch_once(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("bare repos are not supported"))?;
    let push_url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow!("Remote '{remote_name}' has no URL"))?;
    let source = resolve_push_source(repo, branch);
    let refspec = format!("{source}:refs/heads/{branch}");

    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(workdir);
    configure_git_command(&mut cmd, &push_url);
    cmd.arg("push").arg(&push_url).arg(&refspec);

    let output = cmd
        .output()
        .with_context(|| format!("spawn `git push` for branch '{branch}' failed"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = format!("{stdout}{stderr}").trim().to_string();
        return Err(anyhow!("Failed to push branch '{branch}': {detail}"))
            .error_code(error_code::GIT_PUSH_BRANCH);
    }
    Ok(())
}

pub fn reset_branch_to_remote(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    super::validate::ensure_safe_refname_fragment(remote_name, "remote name")?;
    super::validate::ensure_safe_refname_fragment(branch, "branch name")?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("bare repos are not supported"))?;
    let push_url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow!("Remote '{remote_name}' has no URL"))?;

    let mut fetch_cmd = std::process::Command::new("git");
    fetch_cmd.current_dir(workdir);
    configure_git_command(&mut fetch_cmd, &push_url);
    fetch_cmd.arg("fetch").arg(remote_name).arg(format!(
        "+refs/heads/{branch}:refs/remotes/{remote_name}/{branch}"
    ));
    let fetch_out = fetch_cmd
        .output()
        .with_context(|| format!("Failed to fetch '{remote_name}/{branch}' for reset"))?;
    if !fetch_out.status.success() {
        let stderr = String::from_utf8_lossy(&fetch_out.stderr);
        return Err(anyhow!(
            "Failed to fetch '{remote_name}/{branch}' for reset: {}",
            stderr.trim()
        ));
    }

    let remote_oid = run_git(
        workdir,
        &["rev-parse", &format!("refs/remotes/{remote_name}/{branch}")],
    )
    .with_context(|| {
        format!("Could not find remote ref refs/remotes/{remote_name}/{branch} after fetch")
    })?
    .trim()
    .to_string();

    let local_ref = format!("refs/heads/{branch}");
    if repo.find_reference(&local_ref).is_ok() {
        run_git(workdir, &["update-ref", &local_ref, &remote_oid])?;
        run_git(workdir, &["checkout", "-f", branch])?;
    } else {
        run_git(workdir, &["checkout", "-f", &remote_oid])?;
    }
    run_git(workdir, &["clean", "-fd"]).ok();

    Ok(())
}

pub fn push(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    try_push_branch(repo, remote_name, branch)
        .with_context(|| format!("Failed to push branch '{branch}'"))
        .error_code(error_code::GIT_PUSH_BRANCH)?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("bare repos are not supported"))?;
    let head_str = run_git(workdir, &["rev-parse", "HEAD"])?.trim().to_string();
    let head_oid = ObjectId::from_hex(head_str.as_bytes())
        .with_context(|| format!("invalid HEAD sha: {head_str}"))?;
    verify_remote_branch(repo, remote_name, branch, head_oid)
        .with_context(|| "Post-push verification failed: release commit not on remote branch")
        .error_code(error_code::GIT_PUSH_VERIFY_FAILED)?;

    Ok(())
}
