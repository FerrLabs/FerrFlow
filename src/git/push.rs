use anyhow::{Context, Result, anyhow};
use gix::ObjectId;
use std::collections::HashMap;
use std::path::Path;

use crate::error_code::{self, ErrorCodeExt};

use super::auth::{configure_git_command, get_remote_url};
use super::repo::Repository;
use super::retry::retry_transient;
use super::shell::run_git;

fn local_tag_target_sha(repo: &Repository, tag: &str) -> Result<String> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("Bare repositories are not supported"))?;
    let out = run_git(
        workdir,
        &["rev-list", "-n", "1", &format!("refs/tags/{tag}")],
    )
    .with_context(|| format!("could not resolve tag '{tag}' to a commit"))?;
    Ok(out.trim().to_string())
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

pub fn push_branch(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    try_push_branch(repo, remote_name, branch)
}

pub fn push_tags(repo: &Repository, remote_name: &str, tags: &[&str]) -> Result<()> {
    if tags.is_empty() {
        return Ok(());
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
        eprintln!(
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
        eprintln!(
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

pub(super) fn fetch_and_rebase(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
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
        .with_context(|| "spawn `git fetch` failed")?;
    if !fetch_out.status.success() {
        let stderr = String::from_utf8_lossy(&fetch_out.stderr);
        return Err(anyhow!("git fetch failed: {}", stderr.trim()));
    }

    let remote_oid_str = run_git(
        workdir,
        &["rev-parse", &format!("refs/remotes/{remote_name}/{branch}")],
    )
    .with_context(|| format!("could not resolve refs/remotes/{remote_name}/{branch}"))?
    .trim()
    .to_string();
    let local_oid_str = run_git(workdir, &["rev-parse", "HEAD"])
        .with_context(|| "could not resolve HEAD")?
        .trim()
        .to_string();

    if remote_oid_str == local_oid_str {
        return Ok(());
    }

    if run_git(
        workdir,
        &[
            "merge-base",
            "--is-ancestor",
            &remote_oid_str,
            &local_oid_str,
        ],
    )
    .is_ok()
    {
        return Ok(());
    }

    let local_ref = format!("refs/heads/{branch}");
    // Distinguish "detached HEAD" (legit state — bail with a clear error
    // instead of silently rewriting a branch we may not even be tracking)
    // from "on a different branch" (still a hard error: we shouldn't try
    // to fast-forward a branch the user isn't on). The previous code
    // collapsed both into a destructive `update-ref` + `checkout -f`.
    let current_head = run_git(workdir, &["symbolic-ref", "-q", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string());
    match current_head.as_deref() {
        Some(r) if r == local_ref => {
            let rebase_result = run_git(
                workdir,
                &["rebase", &format!("refs/remotes/{remote_name}/{branch}")],
            );
            if let Err(err) = rebase_result {
                let _ = run_git(workdir, &["rebase", "--abort"]);
                return Err(anyhow!(
                    "Rebase conflict: cannot rebase release commits on top of remote '{branch}'. \
                     Run manually or use releaseCommitMode = \"pr\".\n{err}"
                ));
            }
        }
        Some(other) => {
            return Err(anyhow!(
                "Refusing to rebase: HEAD is on '{other}', not '{local_ref}'. \
                 Check out '{branch}' before retrying."
            ));
        }
        None => {
            return Err(anyhow!(
                "Refusing to rebase: HEAD is detached. Check out '{branch}' before retrying. \
                 (To recover an in-progress release, use `ferrflow release --recover` or reset \
                 the branch manually.)"
            ));
        }
    }

    Ok(())
}

pub fn reset_branch_to_remote(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
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

const MAX_PUSH_RETRIES: usize = 3;

pub fn push(repo: &Repository, remote_name: &str, branch: &str, tags: &[&str]) -> Result<()> {
    for attempt in 1..=MAX_PUSH_RETRIES {
        match try_push_branch(repo, remote_name, branch) {
            Ok(()) => break,
            Err(e) => {
                let is_non_ff = e.chain().any(|cause| {
                    let msg = cause.to_string().to_lowercase();
                    msg.contains("non-fastforward")
                        || msg.contains("not fast forward")
                        || msg.contains("non-fast-forward")
                        || msg.contains("push rejected")
                        || msg.contains("rejected")
                });

                if !is_non_ff || attempt == MAX_PUSH_RETRIES {
                    return Err(e)
                        .with_context(|| {
                            format!("Failed to push branch '{branch}' after {attempt} attempt(s)")
                        })
                        .error_code(error_code::GIT_PUSH_BRANCH);
                }

                eprintln!(
                    "Push rejected (non-fast-forward), rebasing on remote and retrying ({attempt}/{MAX_PUSH_RETRIES})..."
                );
                fetch_and_rebase(repo, remote_name, branch)?;
            }
        }
    }

    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow!("bare repos are not supported"))?;
    let head_str = run_git(workdir, &["rev-parse", "HEAD"])?.trim().to_string();
    let head_oid = ObjectId::from_hex(head_str.as_bytes())
        .with_context(|| format!("invalid HEAD sha: {head_str}"))?;
    verify_remote_branch(repo, remote_name, branch, head_oid)
        .with_context(|| "Post-push verification failed: release commit not on remote branch")
        .error_code(error_code::GIT_PUSH_VERIFY_FAILED)?;

    push_tags(repo, remote_name, tags)?;

    Ok(())
}
