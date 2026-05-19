use anyhow::Result;
use git2::{Repository, Sort};
use std::path::Path;

pub use crate::changelog::GitLog;
use crate::config::OrphanedTagStrategy;

use super::tags::{find_last_stable_tag, find_last_tag_commit};

pub fn get_commits_since_last_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix, strategy)?;
    get_commits_since_oid(repo, last_tag_oid)
}

pub fn get_commits_since_last_stable_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_stable_tag(repo, tag_prefix, strategy)?.map(|t| t.commit_oid);
    get_commits_since_oid(repo, last_tag_oid)
}

/// Walk commits from HEAD back to `last_tag_oid` (exclusive). Callers in
/// the multi-package monorepo loop resolve the OID once via `TagIndex`
/// and reuse this helper, sparing the per-package `tag_foreach` scan
/// that `find_last_tag_commit` would otherwise re-run.
pub fn get_commits_since_oid(
    repo: &Repository,
    last_tag_oid: Option<git2::Oid>,
) -> Result<Vec<GitLog>> {
    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in walk {
        let oid = oid?;
        if let Some(stop) = last_tag_oid
            && oid == stop
        {
            break;
        }
        if let Ok(commit) = repo.find_commit(oid) {
            let message = commit.message().unwrap_or("").to_string();
            if message.contains("[skip ci]") {
                continue;
            }
            commits.push(GitLog {
                hash: oid.to_string()[..8].to_string(),
                message,
            });
        }
    }

    Ok(commits)
}

pub(super) fn signature(repo: &Repository) -> Result<git2::Signature<'static>> {
    if let Ok(sig) = repo.signature() {
        return Ok(sig);
    }
    Ok(git2::Signature::now("FerrFlow", "contact@ferrflow.com")?)
}

pub fn create_commit(repo: &Repository, files: &[&str], message: &str) -> Result<()> {
    let mut index = repo.index()?;
    for file in files {
        index.add_path(Path::new(file))?;
    }
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = signature(repo)?;
    let parent = repo.head()?.peel_to_commit()?;

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;
    Ok(())
}

pub fn create_branch_and_commit(
    repo: &Repository,
    branch_name: &str,
    files: &[&str],
    message: &str,
) -> Result<()> {
    create_branch_and_commits(repo, branch_name, &[(files, message)])
}

pub fn create_branch_and_commits(
    repo: &Repository,
    branch_name: &str,
    commits: &[(&[&str], &str)],
) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(branch_name, &head, false)?;

    let refname = format!("refs/heads/{branch_name}");
    let sig = signature(repo)?;
    let mut parent = head;

    for (files, message) in commits {
        let mut index = repo.index()?;
        for file in *files {
            index.add_path(Path::new(file))?;
        }
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let oid = repo.commit(Some(&refname), &sig, &sig, message, &tree, &[&parent])?;
        parent = repo.find_commit(oid)?;
    }
    Ok(())
}
