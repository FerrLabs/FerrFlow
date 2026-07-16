use anyhow::{Context, Result};
use gix::ObjectId;
use gix::objs::TreeRefIter;
use gix_diff::tree::recorder::Change;
use std::collections::HashSet;

use crate::config::OrphanedTagStrategy;

use super::repo::Repository;
use super::tags::find_last_tag_commit;

pub fn get_changed_files(repo: &Repository) -> Result<Vec<String>> {
    if repo.workdir().is_none() {
        return Ok(vec![]);
    }
    let head = match repo.head_id() {
        Ok(id) => id.detach(),
        Err(_) => return Ok(vec![]),
    };

    let mut parents: Vec<ObjectId> = repo
        .find_commit(head)
        .map(|c| c.parent_ids().map(|id| id.detach()).collect())
        .unwrap_or_default();
    // `git diff-tree` prints nothing for merge commits unless -m/-c is given;
    // the previous shell-out inherited that, so an empty list is the contract.
    if parents.len() > 1 {
        return Ok(vec![]);
    }
    changed_paths_between(repo, parents.pop(), head)
}

pub fn get_changed_files_since_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Vec<String>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix, strategy, ancestors)?;
    get_changed_files_since_oid(repo, last_tag_oid)
}

pub fn get_changed_files_since_oid(
    repo: &Repository,
    last_tag_oid: Option<ObjectId>,
) -> Result<Vec<String>> {
    if repo.workdir().is_none() {
        return Ok(vec![]);
    }
    let head = match repo.head_id() {
        Ok(id) => id.detach(),
        Err(_) => return Ok(vec![]),
    };
    changed_paths_between(repo, last_tag_oid, head)
}

fn changed_paths_between(
    repo: &Repository,
    from: Option<ObjectId>,
    to: ObjectId,
) -> Result<Vec<String>> {
    let to_tree = tree_bytes(repo, to)?;
    let from_tree = from.map(|oid| tree_bytes(repo, oid)).transpose()?;

    let hash_kind = repo.object_hash();
    let mut recorder = gix_diff::tree::Recorder::default();
    gix_diff::tree(
        TreeRefIter::from_bytes(from_tree.as_deref().unwrap_or_default(), hash_kind),
        TreeRefIter::from_bytes(&to_tree, hash_kind),
        gix_diff::tree::State::default(),
        &repo.objects,
        &mut recorder,
    )
    .with_context(|| format!("tree diff failed for {to}"))?;

    let mut paths: Vec<String> = recorder
        .records
        .into_iter()
        .filter_map(|change| {
            let (mode, path) = match change {
                Change::Addition {
                    entry_mode, path, ..
                } => (entry_mode, path),
                Change::Deletion {
                    entry_mode, path, ..
                } => (entry_mode, path),
                Change::Modification {
                    entry_mode, path, ..
                } => (entry_mode, path),
            };
            (!mode.is_tree()).then(|| path.to_string())
        })
        .collect();
    paths.sort_unstable();
    paths.dedup();
    Ok(paths)
}

fn tree_bytes(repo: &Repository, commit: ObjectId) -> Result<Vec<u8>> {
    let tree_id = repo
        .find_commit(commit)
        .with_context(|| format!("commit {commit} not found"))?
        .tree_id()
        .with_context(|| format!("no tree for commit {commit}"))?
        .detach();
    Ok(repo
        .find_object(tree_id)
        .with_context(|| format!("tree {tree_id} not found"))?
        .detach()
        .data)
}
