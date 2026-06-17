use anyhow::{Context, Result};
use gix::ObjectId;
use gix::revision::walk::Sorting;
use std::collections::HashSet;

pub use crate::changelog::GitLog;
use crate::config::OrphanedTagStrategy;

use super::repo::Repository;
use super::shell::run_git;
use super::tags::{find_last_stable_tag_with_cache, find_last_tag_commit};

pub fn get_commits_since_last_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
    skip_markers: &[String],
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix, strategy, ancestors)?;
    get_commits_since_oid(repo, last_tag_oid, skip_markers)
}

pub fn get_commits_since_last_stable_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
    skip_markers: &[String],
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_stable_tag_with_cache(repo, tag_prefix, strategy, ancestors)?
        .map(|t| t.commit_oid);
    get_commits_since_oid(repo, last_tag_oid, skip_markers)
}

pub fn get_commits_since_oid(
    repo: &Repository,
    last_tag_oid: Option<ObjectId>,
    skip_markers: &[String],
) -> Result<Vec<GitLog>> {
    let head = repo.head_id()?.detach();
    let mut platform = repo.rev_walk([head]).sorting(Sorting::BreadthFirst);
    if let Some(stop) = last_tag_oid {
        platform = platform.with_hidden([stop]);
    }
    let walk = platform.all()?;

    let mut commits = Vec::new();
    for info in walk {
        let info = info?;
        let commit = match repo.find_commit(info.id) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let raw = match commit.message_raw() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let message = String::from_utf8_lossy(raw).into_owned();
        if subject_has_skip_marker(&message, skip_markers) {
            continue;
        }
        commits.push(GitLog {
            hash: info.id.to_string()[..8].to_string(),
            message,
        });
    }

    Ok(commits)
}

/// Returns true if the commit subject (first line) contains any of the
/// configured skip markers, matched case-insensitively.
///
/// We scope to the subject line so a commit *body* that quotes or
/// documents a marker (e.g. release notes referencing `[skip ci]`)
/// isn't silently dropped.
pub fn subject_has_skip_marker(message: &str, skip_markers: &[String]) -> bool {
    if skip_markers.is_empty() {
        return false;
    }
    let subject = message.lines().next().unwrap_or("").to_ascii_lowercase();
    if subject.is_empty() {
        return false;
    }
    skip_markers
        .iter()
        .any(|m| subject.contains(&m.to_ascii_lowercase()))
}

pub fn create_commit(repo: &Repository, files: &[&str], message: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))?;

    let mut add_args: Vec<&str> = vec!["add", "--"];
    add_args.extend_from_slice(files);
    run_git(workdir, &add_args).with_context(|| "git add failed")?;

    run_git(workdir, &["commit", "-m", message]).with_context(|| "git commit failed")?;
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
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))?;

    let original_head = run_git(workdir, &["rev-parse", "HEAD"])
        .with_context(|| "rev-parse HEAD failed")?
        .trim()
        .to_string();

    run_git(workdir, &["branch", branch_name])
        .with_context(|| format!("git branch {branch_name} failed"))?;

    for (files, message) in commits {
        let mut add_args: Vec<&str> = vec!["add", "--"];
        add_args.extend_from_slice(files);
        run_git(workdir, &add_args).with_context(|| "git add failed")?;

        let tree_sha = run_git(workdir, &["write-tree"])
            .with_context(|| "git write-tree failed")?
            .trim()
            .to_string();
        let parent_sha = run_git(
            workdir,
            &["rev-parse", &format!("refs/heads/{branch_name}")],
        )
        .with_context(|| format!("rev-parse {branch_name} failed"))?
        .trim()
        .to_string();
        let commit_sha = run_git(
            workdir,
            &["commit-tree", &tree_sha, "-p", &parent_sha, "-m", message],
        )
        .with_context(|| "git commit-tree failed")?
        .trim()
        .to_string();
        run_git(
            workdir,
            &[
                "update-ref",
                &format!("refs/heads/{branch_name}"),
                &commit_sha,
            ],
        )
        .with_context(|| "git update-ref failed")?;
    }

    let mut reset_args: Vec<&str> = vec!["reset", "--mixed", &original_head, "--"];
    for (files, _) in commits {
        for f in *files {
            reset_args.push(f);
        }
    }
    run_git(workdir, &reset_args).with_context(|| "git reset --mixed failed")?;

    let _ = repo;
    Ok(())
}
