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

/// Resolve a tag name to the commit it points at, peeling annotated tags.
/// Returns None when the tag doesn't exist or points at a non-commit.
pub fn resolve_tag_name_to_commit(repo: &Repository, tag_name: &str) -> Option<ObjectId> {
    let reference = repo.find_reference(&format!("refs/tags/{tag_name}")).ok()?;
    resolve_tag_to_commit(repo, reference.id().detach())
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
    let walk = TagWalk::new(repo, strategy, ancestors, true)?;
    let mut latest: Option<TagMatch> = None;
    let mut unreachable_tags: Vec<String> = Vec::new();

    for (tag_name, raw_oid) in prefixed_tags(repo, prefix, false)? {
        match walk.place(&tag_name, raw_oid) {
            Placement::At { oid, time } => keep_latest(&mut latest, tag_name, oid, time),
            Placement::Unreachable => unreachable_tags.push(tag_name),
            Placement::Skipped => {}
        }
    }

    warn_unreachable_tags(unreachable_tags);

    Ok(latest)
}

enum Placement {
    At { oid: ObjectId, time: i64 },
    Unreachable,
    Skipped,
}

struct TagWalk<'a> {
    repo: &'a Repository,
    head: ObjectId,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&'a HashSet<ObjectId>>,
    report: bool,
}

impl<'a> TagWalk<'a> {
    fn new(
        repo: &'a Repository,
        strategy: OrphanedTagStrategy,
        ancestors: Option<&'a HashSet<ObjectId>>,
        report: bool,
    ) -> Result<Self> {
        Ok(Self {
            repo,
            head: repo.head_id()?.detach(),
            strategy,
            ancestors,
            report,
        })
    }

    fn place(&self, tag_name: &str, raw_oid: ObjectId) -> Placement {
        let Some(commit_oid) = resolve_tag_to_commit(self.repo, raw_oid) else {
            self.warn_missing_commit(tag_name, raw_oid);
            return Placement::Skipped;
        };
        let Ok(commit) = self.repo.find_commit(commit_oid) else {
            self.warn_missing_commit(tag_name, commit_oid);
            return Placement::Skipped;
        };
        if is_reachable(self.repo, self.head, commit_oid, self.ancestors) {
            return Placement::At {
                oid: commit_oid,
                time: commit_time(&commit),
            };
        }
        if self.strategy == OrphanedTagStrategy::Warn {
            return Placement::Unreachable;
        }
        self.recover(tag_name, &commit, commit_oid)
    }

    fn recover(&self, tag_name: &str, commit: &gix::Commit<'_>, commit_oid: ObjectId) -> Placement {
        let via = strategy_label(self.strategy);
        let Some(matched_oid) = find_matching_commit(self.repo, commit, &self.strategy) else {
            if self.report {
                tracing::warn!(
                    "Warning: tag '{}' points to orphaned commit {}. No match found via {}. Skipping.\n  \
                     Hint: re-tag manually with 'git tag -f {} <correct-commit>'",
                    tag_name,
                    &commit_oid.to_string()[..7],
                    via,
                    tag_name
                );
            }
            return Placement::Skipped;
        };
        if self.report {
            tracing::info!(
                "Info: tag '{}' was orphaned but matched commit {} on current branch via {}.",
                tag_name,
                &matched_oid.to_string()[..7],
                via
            );
        }
        match self.repo.find_commit(matched_oid) {
            Ok(matched) => Placement::At {
                oid: matched_oid,
                time: commit_time(&matched),
            },
            Err(_) => Placement::Skipped,
        }
    }

    fn warn_missing_commit(&self, tag_name: &str, oid: ObjectId) {
        if self.report {
            tracing::warn!(
                "Warning: tag '{}' points to missing commit {} (likely garbage-collected). Skipping.\n  \
                 Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                 See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                tag_name,
                &oid.to_string()[..7]
            );
        }
    }
}

fn strategy_label(strategy: OrphanedTagStrategy) -> &'static str {
    match strategy {
        OrphanedTagStrategy::TreeHash => "tree-hash",
        OrphanedTagStrategy::Message => "message",
        OrphanedTagStrategy::Warn => "warn",
    }
}

fn commit_time(commit: &gix::Commit<'_>) -> i64 {
    commit.time().map(|t| t.seconds).unwrap_or(0)
}

fn keep_latest(latest: &mut Option<TagMatch>, name: String, oid: ObjectId, time: i64) {
    if latest.as_ref().is_none_or(|l| time > l.time) {
        *latest = Some(TagMatch {
            name,
            commit_oid: oid,
            time,
        });
    }
}

fn prefixed_tags(
    repo: &Repository,
    prefix: &str,
    stable_only: bool,
) -> Result<Vec<(String, ObjectId)>> {
    let references = repo.references()?;
    let tags = references
        .tags()?
        .flatten()
        .filter_map(|reference| {
            let name = String::from_utf8_lossy(reference.name().shorten()).into_owned();
            let wanted = name.starts_with(prefix)
                && !is_floating_tag(&name, prefix)
                && !(stable_only && is_prerelease_tag(&name, prefix));
            wanted.then(|| (name, reference.id().detach()))
        })
        .collect();
    Ok(tags)
}

fn parse_tag_semver(tag_name: &str, prefix: &str) -> Option<semver::Version> {
    let version = tag_name
        .strip_prefix(prefix)
        .map(|s| s.strip_prefix('v').unwrap_or(s))
        .unwrap_or(tag_name);
    semver::Version::parse(version).ok()
}

pub(super) fn take_unreported(tags: Vec<String>, seen: &mut HashSet<String>) -> Vec<String> {
    tags.into_iter()
        .filter(|t| seen.insert(t.clone()))
        .collect()
}

pub(super) fn unreachable_tags_message(tags: &[String]) -> String {
    const SHOWN: usize = 5;
    let listed = tags
        .iter()
        .take(SHOWN)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let names = if tags.len() > SHOWN {
        format!("{listed} and {} more", tags.len() - SHOWN)
    } else {
        listed
    };
    let subject = if tags.len() == 1 {
        "tag is"
    } else {
        "tags are"
    };

    format!(
        "Warning: {} {} not reachable from HEAD ({}), so they were ignored.\n  \
         Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
         See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
        tags.len(),
        subject,
        names
    )
}

fn warn_unreachable_tags(tags: Vec<String>) {
    if tags.is_empty() {
        return;
    }

    static REPORTED: std::sync::OnceLock<std::sync::Mutex<HashSet<String>>> =
        std::sync::OnceLock::new();
    let reported = REPORTED.get_or_init(|| std::sync::Mutex::new(HashSet::new()));

    let fresh = match reported.lock() {
        Ok(mut seen) => take_unreported(tags, &mut seen),
        Err(_) => tags,
    };
    if fresh.is_empty() {
        return;
    }

    tracing::warn!("{}", unreachable_tags_message(&fresh));
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
    let walk = TagWalk::new(repo, strategy, ancestors, false)?;
    let mut highest: Option<(String, semver::Version)> = None;

    for (tag_name, raw_oid) in prefixed_tags(repo, prefix, true)? {
        let Some(parsed) = parse_tag_semver(&tag_name, prefix) else {
            continue;
        };
        if !matches!(walk.place(&tag_name, raw_oid), Placement::At { .. }) {
            continue;
        }
        if highest
            .as_ref()
            .is_none_or(|(_, existing)| &parsed > existing)
        {
            highest = Some((tag_name, parsed));
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
    let walk = TagWalk::new(repo, strategy, ancestors, false)?;
    let mut latest: Option<TagMatch> = None;

    for (tag_name, raw_oid) in prefixed_tags(repo, prefix, true)? {
        if let Placement::At { oid, time } = walk.place(&tag_name, raw_oid) {
            keep_latest(&mut latest, tag_name, oid, time);
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
