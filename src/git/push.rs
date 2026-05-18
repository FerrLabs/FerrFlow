use anyhow::{Context, Result};
use git2::{PushOptions, RemoteCallbacks, Repository, Sort};
use std::cell::RefCell;
use std::rc::Rc;

use crate::error_code::{self, ErrorCodeExt};

use super::auth::{authenticated_remote_url, credentials_callback, get_authenticated_remote};
use super::fetch::make_fetch_options;
use super::retry::retry_transient;

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

fn make_push_options(push_errors: Rc<RefCell<Vec<String>>>) -> PushOptions<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);
    let errors = push_errors.clone();
    callbacks.push_update_reference(move |refname, status| {
        if let Some(msg) = status {
            errors.borrow_mut().push(format!("{refname}: {msg}"));
        }
        Ok(())
    });
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);
    push_options
}

fn check_push_errors(errors: &RefCell<Vec<String>>) -> Result<()> {
    let errs = errors.borrow();
    if errs.is_empty() {
        return Ok(());
    }
    let joined = errs.join("; ");
    Err(anyhow::anyhow!("Push rejected by remote: {joined}"))
        .error_code(error_code::GIT_PUSH_REJECTED)?;
    Ok(())
}

pub fn verify_remote_branch(
    repo: &Repository,
    remote_name: &str,
    branch: &str,
    expected_oid: git2::Oid,
) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);

    let connection = remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

    let expected_ref = format!("refs/heads/{branch}");
    for head in connection.list()? {
        if head.name() == expected_ref {
            if head.oid() == expected_oid {
                return Ok(());
            }
            Err(anyhow::anyhow!(
                "Remote branch '{}' points to {} but expected {}",
                branch,
                head.oid(),
                expected_oid,
            ))
            .error_code(error_code::GIT_PUSH_VERIFY_FAILED)?;
        }
    }
    Err(anyhow::anyhow!(
        "Remote branch '{}' not found after push",
        branch
    ))
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
        .ok_or_else(|| anyhow::anyhow!("bare repos are not supported"))?;
    let remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("Remote '{remote_name}' not found"))?;
    let raw_url = remote
        .url()
        .ok_or_else(|| anyhow::anyhow!("Remote '{remote_name}' has no URL"))?
        .to_string();
    let push_url = authenticated_remote_url(&raw_url).unwrap_or(raw_url);

    let prefix = if force { "+" } else { "" };
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(workdir).arg("push").arg(&push_url);
    for tag in tags {
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
        return Err(anyhow::anyhow!("{label}: {detail}"));
    }
    Ok(())
}

fn try_push_branch(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    retry_transient(&format!("push branch '{branch}'"), || {
        try_push_branch_once(repo, remote_name, branch)
    })
}

fn try_push_branch_once(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;
    let push_errors = Rc::new(RefCell::new(Vec::new()));
    let mut opts = make_push_options(push_errors.clone());
    let source = resolve_push_source(repo, branch);
    let branch_refspec = format!("{source}:refs/heads/{branch}");
    remote
        .push(&[&branch_refspec], Some(&mut opts))
        .with_context(|| format!("Failed to push branch '{branch}'"))
        .error_code(error_code::GIT_PUSH_BRANCH)?;
    check_push_errors(&push_errors)
        .with_context(|| format!("Branch push rejected for '{branch}'"))
        .error_code(error_code::GIT_PUSH_REJECTED)?;
    Ok(())
}

pub(super) fn fetch_and_rebase(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;
    let mut opts = make_fetch_options();
    remote.fetch(
        &[&format!(
            "refs/heads/{branch}:refs/remotes/{remote_name}/{branch}"
        )],
        Some(&mut opts),
        None,
    )?;
    drop(remote);

    let remote_ref = format!("refs/remotes/{remote_name}/{branch}");
    let remote_oid = repo
        .refname_to_id(&remote_ref)
        .with_context(|| format!("Could not find remote ref {remote_ref} after fetch"))?;

    let local_commit = repo.head()?.peel_to_commit()?;
    let local_oid = local_commit.id();

    if remote_oid == local_oid || repo.graph_descendant_of(local_oid, remote_oid)? {
        return Ok(());
    }

    let merge_base = repo
        .merge_base(local_oid, remote_oid)
        .with_context(|| "No common ancestor between local and remote branch")?;

    let mut local_commits = Vec::new();
    let mut walk = repo.revwalk()?;
    walk.push(local_oid)?;
    walk.hide(merge_base)?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    for oid in walk {
        local_commits.push(oid?);
    }

    if local_commits.is_empty() {
        return Ok(());
    }

    let mut current_parent = repo.find_commit(remote_oid)?;
    for commit_oid in &local_commits {
        let commit = repo.find_commit(*commit_oid)?;
        let commit_parent_tree = commit.parent(0)?.tree()?;
        let commit_tree = commit.tree()?;
        let new_base_tree = current_parent.tree()?;

        let mut merge_index =
            repo.merge_trees(&commit_parent_tree, &new_base_tree, &commit_tree, None)?;
        if merge_index.has_conflicts() {
            let paths: Vec<String> = merge_index
                .conflicts()
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|c| c.ok())
                .filter_map(|c| {
                    c.our
                        .as_ref()
                        .or(c.their.as_ref())
                        .or(c.ancestor.as_ref())
                        .map(|e| String::from_utf8_lossy(&e.path).into_owned())
                })
                .collect();
            let path_list = if paths.is_empty() {
                String::new()
            } else {
                format!("\nConflicting paths:\n  - {}", paths.join("\n  - "))
            };
            anyhow::bail!(
                "Rebase conflict: cannot rebase release commits on top of remote '{branch}'. \
                 Run manually or use releaseCommitMode = \"pr\".{path_list}"
            );
        }

        let new_tree_oid = merge_index.write_tree_to(repo)?;
        let new_tree = repo.find_tree(new_tree_oid)?;

        let new_oid = repo.commit(
            None,
            &commit.author(),
            &commit.committer(),
            commit.message().unwrap_or(""),
            &new_tree,
            &[&current_parent],
        )?;
        current_parent = repo.find_commit(new_oid)?;
    }

    let local_ref = format!("refs/heads/{branch}");
    if repo.find_reference(&local_ref).is_ok() {
        repo.reference(
            &local_ref,
            current_parent.id(),
            true,
            "ferrflow: rebase on push",
        )?;
    }
    repo.set_head_detached(current_parent.id())?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;

    Ok(())
}

pub fn reset_branch_to_remote(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;
    let mut opts = make_fetch_options();
    remote
        .fetch(
            &[&format!(
                "refs/heads/{branch}:refs/remotes/{remote_name}/{branch}"
            )],
            Some(&mut opts),
            None,
        )
        .with_context(|| format!("Failed to fetch '{remote_name}/{branch}' for reset"))?;
    drop(remote);

    let remote_ref = format!("refs/remotes/{remote_name}/{branch}");
    let remote_oid = repo
        .refname_to_id(&remote_ref)
        .with_context(|| format!("Could not find remote ref {remote_ref} after fetch"))?;

    let local_ref = format!("refs/heads/{branch}");
    if repo.find_reference(&local_ref).is_ok() {
        repo.reference(
            &local_ref,
            remote_oid,
            true,
            "ferrflow: reset to remote for release retry",
        )?;
    }

    repo.set_head_detached(remote_oid)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::new()
            .force()
            .remove_untracked(true),
    ))?;

    if repo.find_reference(&local_ref).is_ok() {
        repo.set_head(&local_ref)?;
    }

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

    let head_oid = repo.head()?.peel_to_commit()?.id();
    verify_remote_branch(repo, remote_name, branch, head_oid)
        .with_context(|| "Post-push verification failed: release commit not on remote branch")
        .error_code(error_code::GIT_PUSH_VERIFY_FAILED)?;

    push_tags(repo, remote_name, tags)?;

    Ok(())
}
