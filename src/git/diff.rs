use anyhow::Result;
use git2::Repository;

use crate::config::OrphanedTagStrategy;

use super::tags::find_last_tag_commit;

pub fn get_changed_files(repo: &Repository) -> Result<Vec<String>> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => return Ok(vec![]),
    };
    let head_tree = head.tree()?;

    let files = if let Ok(parent) = head.parent(0) {
        let parent_tree = parent.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&head_tree), None)?;
        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files.push(path.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        )?;
        files
    } else {
        let mut files = Vec::new();
        head_tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
            if let Some(name) = entry.name() {
                files.push(name.to_string());
            }
            git2::TreeWalkResult::Ok
        })?;
        files
    };

    Ok(files)
}

pub fn get_changed_files_since_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Vec<String>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix, strategy)?;
    get_changed_files_since_oid(repo, last_tag_oid)
}

/// Same as [`get_changed_files_since_tag`] but skips the tag lookup —
/// callers in the multi-package monorepo loop resolve the OID once via
/// `TagIndex` instead of paying for an independent `tag_foreach` per
/// package.
pub fn get_changed_files_since_oid(
    repo: &Repository,
    last_tag_oid: Option<git2::Oid>,
) -> Result<Vec<String>> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => return Ok(vec![]),
    };
    let head_tree = head.tree()?;

    let old_tree = if let Some(tag_oid) = last_tag_oid {
        let tag_commit = repo.find_commit(tag_oid)?;
        Some(tag_commit.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(old_tree.as_ref(), Some(&head_tree), None)?;
    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                files.push(path.to_string_lossy().to_string());
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(files)
}
