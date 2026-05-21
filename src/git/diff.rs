use anyhow::Result;
use gix::ObjectId;

use crate::config::OrphanedTagStrategy;

use super::repo::Repository;
use super::shell::run_git;
use super::tags::find_last_tag_commit;

pub fn get_changed_files(repo: &Repository) -> Result<Vec<String>> {
    let workdir = match repo.workdir() {
        Some(p) => p,
        None => return Ok(vec![]),
    };
    let head = match repo.head_id() {
        Ok(id) => id.detach(),
        Err(_) => return Ok(vec![]),
    };

    let has_parent = repo
        .find_commit(head)
        .ok()
        .and_then(|c| c.parent_ids().next())
        .is_some();

    let args: Vec<String> = if has_parent {
        vec![
            "diff-tree".into(),
            "--no-commit-id".into(),
            "--name-only".into(),
            "-r".into(),
            head.to_string(),
        ]
    } else {
        vec![
            "ls-tree".into(),
            "-r".into(),
            "--name-only".into(),
            head.to_string(),
        ]
    };
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run_git(workdir, &arg_refs)?;
    Ok(out
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

pub fn get_changed_files_since_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Vec<String>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix, strategy)?;
    get_changed_files_since_oid(repo, last_tag_oid)
}

pub fn get_changed_files_since_oid(
    repo: &Repository,
    last_tag_oid: Option<ObjectId>,
) -> Result<Vec<String>> {
    let workdir = match repo.workdir() {
        Some(p) => p,
        None => return Ok(vec![]),
    };
    let head = match repo.head_id() {
        Ok(id) => id.detach(),
        Err(_) => return Ok(vec![]),
    };

    let head_str = head.to_string();
    let args: Vec<String> = match last_tag_oid {
        Some(tag_oid) => vec![
            "diff".into(),
            "--name-only".into(),
            tag_oid.to_string(),
            head_str,
        ],
        None => vec![
            "ls-tree".into(),
            "-r".into(),
            "--name-only".into(),
            head_str,
        ],
    };
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run_git(workdir, &arg_refs)?;
    Ok(out
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}
