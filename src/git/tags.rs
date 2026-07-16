use anyhow::{Context, Result};
use gix::ObjectId;
use gix::revision::walk::Sorting;
use gix_traverse::commit::simple::CommitTimeOrder;
use std::collections::HashSet;

use super::repo::Repository;
use super::shell::run_git;
use crate::config::OrphanedTagStrategy;
use crate::error_code::{self, ErrorCodeExt};

pub fn build_head_ancestors(repo: &Repository) -> Result<HashSet<ObjectId>> {
    let head_id = repo.head_id()?.detach();
    let walk = repo
        .rev_walk([head_id])
        .use_commit_graph(true)
        .sorting(Sorting::BreadthFirst)
        .all()?;
    let mut set: HashSet<ObjectId> = HashSet::new();
    for info in walk {
        let info = info?;
        set.insert(info.id);
    }
    Ok(set)
}

pub struct TagIndex {
    entries: Vec<TagIndexEntry>,
    pub ancestors: HashSet<ObjectId>,
}

struct TagIndexEntry {
    name: String,
    commit_oid: ObjectId,
    time: i64,
    reachable: bool,
}

impl TagIndex {
    pub fn build(repo: &Repository) -> Result<Self> {
        let head_id = repo.head_id()?.detach();
        let walk = repo
            .rev_walk([head_id])
            .use_commit_graph(true)
            .sorting(Sorting::BreadthFirst)
            .all()?;
        let mut ancestors: HashSet<ObjectId> = HashSet::new();
        for info in walk {
            let info = info?;
            ancestors.insert(info.id);
        }

        let mut entries: Vec<TagIndexEntry> = Vec::new();
        let references = repo.references()?;
        for reference in references.tags()?.flatten() {
            let tag_name = String::from_utf8_lossy(reference.name().shorten()).into_owned();
            let commit_oid = match resolve_tag_to_commit(repo, reference.id().detach()) {
                Some(oid) => oid,
                None => continue,
            };
            let commit = match repo.find_commit(commit_oid) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let time = commit.time().map(|t| t.seconds).unwrap_or(0);
            let reachable = head_id == commit_oid || ancestors.contains(&commit_oid);
            entries.push(TagIndexEntry {
                name: tag_name,
                commit_oid,
                time,
                reachable,
            });
        }

        Ok(Self { entries, ancestors })
    }

    pub fn find_last_tag_name(
        &self,
        prefix: &str,
        strategy: OrphanedTagStrategy,
    ) -> Option<String> {
        if !matches!(strategy, OrphanedTagStrategy::Warn) {
            return None;
        }
        self.entries
            .iter()
            .filter(|e| {
                e.reachable && e.name.starts_with(prefix) && !is_floating_tag(&e.name, prefix)
            })
            .max_by_key(|e| e.time)
            .map(|e| e.name.clone())
    }

    pub fn find_highest_semver_tag(
        &self,
        prefix: &str,
        strategy: OrphanedTagStrategy,
    ) -> Option<(String, String)> {
        if !matches!(strategy, OrphanedTagStrategy::Warn) {
            return None;
        }
        let mut best: Option<(&str, semver::Version)> = None;
        for entry in &self.entries {
            if !entry.reachable
                || !entry.name.starts_with(prefix)
                || is_prerelease_tag(&entry.name, prefix)
                || is_floating_tag(&entry.name, prefix)
            {
                continue;
            }
            let version_str = entry
                .name
                .strip_prefix(prefix)
                .map(|s| s.strip_prefix('v').unwrap_or(s))
                .unwrap_or(&entry.name);
            let parsed = match semver::Version::parse(version_str) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match &best {
                Some((_, existing)) if *existing >= parsed => {}
                _ => best = Some((entry.name.as_str(), parsed)),
            }
        }
        best.map(|(name, version)| (name.to_string(), version.to_string()))
    }

    pub fn find_last_tag_commit(
        &self,
        prefix: &str,
        strategy: OrphanedTagStrategy,
    ) -> Option<ObjectId> {
        if !matches!(strategy, OrphanedTagStrategy::Warn) {
            return None;
        }
        self.entries
            .iter()
            .filter(|e| {
                e.reachable && e.name.starts_with(prefix) && !is_floating_tag(&e.name, prefix)
            })
            .max_by_key(|e| e.time)
            .map(|e| e.commit_oid)
    }

    pub fn find_last_stable_tag_commit(
        &self,
        prefix: &str,
        strategy: OrphanedTagStrategy,
    ) -> Option<ObjectId> {
        if !matches!(strategy, OrphanedTagStrategy::Warn) {
            return None;
        }
        self.entries
            .iter()
            .filter(|e| {
                e.reachable
                    && e.name.starts_with(prefix)
                    && !is_prerelease_tag(&e.name, prefix)
                    && !is_floating_tag(&e.name, prefix)
            })
            .max_by_key(|e| e.time)
            .map(|e| e.commit_oid)
    }
}

fn resolve_tag_to_commit(repo: &Repository, oid: ObjectId) -> Option<ObjectId> {
    let object = repo.find_object(oid).ok()?;
    match object.kind {
        gix::object::Kind::Commit => Some(oid),
        gix::object::Kind::Tag => {
            let tag = object.try_into_tag().ok()?;
            let decoded = tag.decode().ok()?;
            let target_id: ObjectId = decoded.target();
            if matches!(decoded.target_kind, gix::object::Kind::Commit) {
                Some(target_id)
            } else {
                resolve_tag_to_commit(repo, target_id)
            }
        }
        _ => None,
    }
}

fn is_reachable(
    repo: &Repository,
    head: ObjectId,
    commit_oid: ObjectId,
    cache: Option<&HashSet<ObjectId>>,
) -> bool {
    if let Some(set) = cache {
        return set.contains(&commit_oid);
    }
    if head == commit_oid {
        return true;
    }
    let walk = match repo
        .rev_walk([head])
        .use_commit_graph(true)
        .sorting(Sorting::BreadthFirst)
        .all()
    {
        Ok(w) => w,
        Err(_) => return false,
    };
    for info in walk.flatten() {
        if info.id == commit_oid {
            return true;
        }
    }
    false
}

pub(super) struct TagMatch {
    pub name: String,
    pub commit_oid: ObjectId,
    pub time: i64,
}

pub(super) fn find_matching_commit(
    repo: &Repository,
    orphaned_commit: &gix::Commit<'_>,
    strategy: &OrphanedTagStrategy,
) -> Option<ObjectId> {
    // Warn never recovers a commit; bail before paying for the walk.
    if matches!(strategy, OrphanedTagStrategy::Warn) {
        return None;
    }

    let head = repo.head_id().ok()?.detach();
    let walk = repo
        .rev_walk([head])
        .use_commit_graph(true)
        .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
        .all()
        .ok()?;

    let limit = 1000;
    let orphan_tree = orphaned_commit.tree_id().ok()?.detach();
    let orphan_message = orphaned_commit
        .message_raw()
        .ok()
        .map(|m| m.to_vec())
        .unwrap_or_default();

    for (count, info) in walk.enumerate() {
        if count >= limit {
            break;
        }
        let info = match info {
            Ok(i) => i,
            Err(_) => continue,
        };
        let candidate = match repo.find_commit(info.id) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let matched = match strategy {
            OrphanedTagStrategy::TreeHash => candidate
                .tree_id()
                .ok()
                .map(|t| t.detach() == orphan_tree)
                .unwrap_or(false),
            OrphanedTagStrategy::Message => candidate
                .message_raw()
                .ok()
                .map(|m| {
                    let bytes: &[u8] = m;
                    bytes == orphan_message.as_slice()
                })
                .unwrap_or(false),
            OrphanedTagStrategy::Warn => return None,
        };
        if matched {
            return Some(info.id);
        }
    }
    None
}

pub(super) fn is_floating_tag(tag_name: &str, prefix: &str) -> bool {
    let version_part = tag_name.strip_prefix(prefix).unwrap_or(tag_name);
    if version_part.is_empty() {
        return false;
    }
    let is_numeric = version_part.chars().all(|c| c.is_ascii_digit() || c == '.');
    let dot_count = version_part.chars().filter(|&c| c == '.').count();
    is_numeric && dot_count <= 1
}

pub(super) fn is_prerelease_tag(tag_name: &str, prefix: &str) -> bool {
    let version_part = tag_name.strip_prefix(prefix).unwrap_or(tag_name);
    version_part.contains('-')
}

pub(super) fn find_last_tag(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<TagMatch>> {
    find_last_tag_with_cache(repo, prefix, strategy, None)
}

pub(super) fn find_last_tag_with_cache(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Option<TagMatch>> {
    let head = repo.head_id()?.detach();
    let mut latest: Option<TagMatch> = None;
    let references = repo.references()?;

    for reference in references.tags()?.flatten() {
        let tag_name = String::from_utf8_lossy(reference.name().shorten()).into_owned();
        if !tag_name.starts_with(prefix) || is_floating_tag(&tag_name, prefix) {
            continue;
        }

        let raw_oid = reference.id().detach();
        let commit_oid = match resolve_tag_to_commit(repo, raw_oid) {
            Some(oid) => oid,
            None => {
                tracing::warn!(
                    "Warning: tag '{}' points to missing commit {} (likely garbage-collected). Skipping.\n  \
                     Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                     See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                    tag_name,
                    &raw_oid.to_string()[..7]
                );
                continue;
            }
        };

        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => {
                tracing::warn!(
                    "Warning: tag '{}' points to missing commit {} (likely garbage-collected). Skipping.\n  \
                     Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                     See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                    tag_name,
                    &commit_oid.to_string()[..7]
                );
                continue;
            }
        };

        let reachable = is_reachable(repo, head, commit_oid, ancestors);

        let (effective_oid, effective_time) = if reachable {
            (commit_oid, commit.time().map(|t| t.seconds).unwrap_or(0))
        } else {
            let short = &commit_oid.to_string()[..7].to_string();
            if strategy == OrphanedTagStrategy::Warn {
                tracing::warn!(
                    "Warning: tag '{}' points to orphaned commit {} (not reachable from HEAD).\n  \
                     Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                     See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                    tag_name,
                    short
                );
                continue;
            }
            match find_matching_commit(repo, &commit, &strategy) {
                Some(matched_oid) => {
                    let strategy_name = match strategy {
                        OrphanedTagStrategy::TreeHash => "tree-hash",
                        OrphanedTagStrategy::Message => "message",
                        OrphanedTagStrategy::Warn => unreachable!(),
                    };
                    tracing::info!(
                        "Info: tag '{}' was orphaned but matched commit {} on current branch via {}.",
                        tag_name,
                        &matched_oid.to_string()[..7],
                        strategy_name
                    );
                    let matched_commit = match repo.find_commit(matched_oid) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    (
                        matched_oid,
                        matched_commit.time().map(|t| t.seconds).unwrap_or(0),
                    )
                }
                None => {
                    let strategy_name = match strategy {
                        OrphanedTagStrategy::TreeHash => "tree-hash",
                        OrphanedTagStrategy::Message => "message",
                        OrphanedTagStrategy::Warn => unreachable!(),
                    };
                    tracing::warn!(
                        "Warning: tag '{}' points to orphaned commit {}. No match found via {}. Skipping.\n  \
                         Hint: re-tag manually with 'git tag -f {} <correct-commit>'",
                        tag_name,
                        short,
                        strategy_name,
                        tag_name
                    );
                    continue;
                }
            }
        };

        if latest.as_ref().is_none_or(|l| effective_time > l.time) {
            latest = Some(TagMatch {
                name: tag_name,
                commit_oid: effective_oid,
                time: effective_time,
            });
        }
    }

    Ok(latest)
}

pub fn find_last_tag_name(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<String>> {
    Ok(find_last_tag(repo, prefix, strategy)?.map(|t| t.name))
}

pub fn find_last_tag_name_with_cache(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Option<String>> {
    Ok(find_last_tag_with_cache(repo, prefix, strategy, ancestors)?.map(|t| t.name))
}

#[allow(dead_code)]
pub fn find_highest_semver_tag(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<(String, String)>> {
    find_highest_semver_tag_with_cache(repo, prefix, strategy, None)
}

pub fn find_highest_semver_tag_with_cache(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Option<(String, String)>> {
    let head = repo.head_id()?.detach();
    let mut highest: Option<(String, semver::Version)> = None;
    let references = repo.references()?;

    for reference in references.tags()?.flatten() {
        let tag_name = String::from_utf8_lossy(reference.name().shorten()).into_owned();
        if !tag_name.starts_with(prefix)
            || is_prerelease_tag(&tag_name, prefix)
            || is_floating_tag(&tag_name, prefix)
        {
            continue;
        }

        let version_str = tag_name
            .strip_prefix(prefix)
            .map(|s| s.strip_prefix('v').unwrap_or(s))
            .unwrap_or(&tag_name);
        let parsed = match semver::Version::parse(version_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let raw_oid = reference.id().detach();
        let commit_oid = match resolve_tag_to_commit(repo, raw_oid) {
            Some(oid) => oid,
            None => continue,
        };
        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let reachable = is_reachable(repo, head, commit_oid, ancestors);
        if !reachable {
            match strategy {
                OrphanedTagStrategy::Warn => continue,
                OrphanedTagStrategy::TreeHash | OrphanedTagStrategy::Message => {
                    if find_matching_commit(repo, &commit, &strategy).is_none() {
                        continue;
                    }
                }
            }
        }

        match highest.as_ref() {
            Some((_, existing)) if existing >= &parsed => {}
            _ => {
                highest = Some((tag_name, parsed));
            }
        }
    }

    Ok(highest.map(|(name, version)| (name, version.to_string())))
}

pub(super) fn find_last_tag_commit(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Option<ObjectId>> {
    Ok(find_last_tag_with_cache(repo, prefix, strategy, ancestors)?.map(|t| t.commit_oid))
}

pub(super) fn find_last_stable_tag_with_cache(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&HashSet<ObjectId>>,
) -> Result<Option<TagMatch>> {
    let head = repo.head_id()?.detach();
    let mut latest: Option<TagMatch> = None;
    let references = repo.references()?;

    for reference in references.tags()?.flatten() {
        let tag_name = String::from_utf8_lossy(reference.name().shorten()).into_owned();
        if !tag_name.starts_with(prefix)
            || is_prerelease_tag(&tag_name, prefix)
            || is_floating_tag(&tag_name, prefix)
        {
            continue;
        }

        let raw_oid = reference.id().detach();
        let commit_oid = match resolve_tag_to_commit(repo, raw_oid) {
            Some(oid) => oid,
            None => continue,
        };

        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let reachable = is_reachable(repo, head, commit_oid, ancestors);

        let (effective_oid, effective_time) = if reachable {
            (commit_oid, commit.time().map(|t| t.seconds).unwrap_or(0))
        } else {
            if strategy == OrphanedTagStrategy::Warn {
                continue;
            }
            match find_matching_commit(repo, &commit, &strategy) {
                Some(matched_oid) => {
                    let matched_commit = match repo.find_commit(matched_oid) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    (
                        matched_oid,
                        matched_commit.time().map(|t| t.seconds).unwrap_or(0),
                    )
                }
                None => continue,
            }
        };

        if latest.as_ref().is_none_or(|l| effective_time > l.time) {
            latest = Some(TagMatch {
                name: tag_name,
                commit_oid: effective_oid,
                time: effective_time,
            });
        }
    }

    Ok(latest)
}

pub fn collect_all_tags(repo: &Repository) -> Vec<String> {
    let references = match repo.references() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut tags = Vec::new();
    if let Ok(iter) = references.tags() {
        for reference in iter.flatten() {
            let name = reference.name().shorten();
            tags.push(String::from_utf8_lossy(name).into_owned());
        }
    }
    tags
}

pub fn tag_exists(repo: &Repository, tag_name: &str) -> bool {
    repo.find_reference(&format!("refs/tags/{tag_name}"))
        .is_ok()
}

pub fn create_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<()> {
    super::validate::ensure_safe_refname_fragment(tag_name, "tag name")?;
    if tag_exists(repo, tag_name) {
        Err(anyhow::anyhow!("tag {tag_name} already exists"))
            .error_code(error_code::GIT_TAG_EXISTS)?;
    }
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))?;
    run_git(workdir, &["tag", "-a", "-m", message, "--", tag_name])
        .with_context(|| format!("git tag -a {tag_name} failed"))?;
    Ok(())
}

pub fn create_or_move_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<bool> {
    super::validate::ensure_safe_refname_fragment(tag_name, "tag name")?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))?;
    let existed = tag_exists(repo, tag_name);
    if existed {
        run_git(workdir, &["tag", "-d", "--", tag_name])
            .with_context(|| format!("git tag -d {tag_name} failed"))?;
    }
    run_git(workdir, &["tag", "-a", "-m", message, "--", tag_name])
        .with_context(|| format!("git tag -a {tag_name} failed"))?;
    Ok(existed)
}

pub fn get_tag_message(repo: &Repository, tag_name: &str) -> Option<String> {
    let reference = repo.find_reference(&format!("refs/tags/{tag_name}")).ok()?;
    let oid = reference.id().detach();
    let object = repo.find_object(oid).ok()?;
    if !matches!(object.kind, gix::object::Kind::Tag) {
        return None;
    }
    let tag = object.try_into_tag().ok()?;
    let decoded = tag.decode().ok()?;
    Some(decoded.message.to_string())
}
