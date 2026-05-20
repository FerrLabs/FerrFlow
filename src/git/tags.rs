use anyhow::Result;
use git2::Repository;
use std::cell::RefCell;
use std::collections::HashSet;

use crate::config::OrphanedTagStrategy;
use crate::error_code::{self, ErrorCodeExt};

use super::commits::signature;

/// Build the set of commit OIDs reachable from HEAD via a single revwalk.
///
/// On a monorepo with N packages, `find_last_tag` (and its siblings) is
/// called once per package. Each call used to invoke
/// `repo.graph_descendant_of(head, oid)` per matching tag, which itself
/// walks the commit graph until it hits the target. On dense histories
/// (mono-large: 200 pkgs × 10k commits), this turns into N independent
/// O(commits) walks — 1815 ms for `ferrflow tag` was almost all this.
///
/// Building the ancestor set up front collapses that into one walk +
/// O(1) hash lookups per tag. Callers in the per-package loop pass
/// `Some(&set)`; single-shot callers (status, query, single-package
/// release) pass `None` and pay the original cost.
pub fn build_head_ancestors(repo: &Repository) -> Result<HashSet<git2::Oid>> {
    let head_oid = repo.head()?.peel_to_commit()?.id();
    let mut walk = repo.revwalk()?;
    walk.push(head_oid)?;
    let mut set: HashSet<git2::Oid> = HashSet::new();
    for oid in walk.flatten() {
        set.insert(oid);
    }
    Ok(set)
}

/// Pre-collected tag index with HEAD reachability information.
///
/// Built once at the start of a multi-package operation (monorepo
/// release, `ferrflow tag` listing, etc.), then queried per package.
/// Avoids both the per-tag `graph_descendant_of` walk (already addressed
/// by `build_head_ancestors`) AND the per-call `tag_foreach` scan that
/// was still dominating on dense histories: 200 packages × ~200 tags ×
/// callback overhead ≈ several hundred ms on mono-large.
///
/// The fast path covers `OrphanedTagStrategy::Warn` (default and most
/// common). Callers needing tree-hash or message recovery for orphan
/// tags fall back to the per-call `find_*_tag_with_cache` path which
/// still uses the ancestor set but doesn't benefit from the pre-scan.
pub struct TagIndex {
    entries: Vec<TagIndexEntry>,
    pub ancestors: HashSet<git2::Oid>,
}

struct TagIndexEntry {
    name: String,
    commit_oid: git2::Oid,
    time: i64,
    reachable: bool,
}

impl TagIndex {
    /// Build the per-package tag index.
    ///
    /// Intentionally stays on libgit2 even though `collect_all_tags`
    /// migrated to gix. Local benches in
    /// `tests/gix_parity.rs::tag_index_smoke_perf_200_tags` showed the
    /// gix variant at **22 ms vs 14 ms libgit2** on 200 tags — because
    /// `TagIndex::build` does many per-tag object lookups (peel + read
    /// time) on top of the enumeration, and gix's object DB lookups
    /// were slower than libgit2's batched equivalents in this shape,
    /// AND we'd pay a second `gix::open()` per call.
    ///
    /// The gix implementation is preserved as [`build_gix`] for the
    /// parity suite and ready to flip back once a long-lived
    /// `gix::ThreadSafeRepository` is cached across the run.
    pub fn build(repo: &Repository) -> Result<Self> {
        Self::build_libgit2(repo)
    }

    pub fn build_libgit2(repo: &Repository) -> Result<Self> {
        let head = repo.head()?.peel_to_commit()?.id();
        let mut walk = repo.revwalk()?;
        walk.push(head)?;
        let mut ancestors: HashSet<git2::Oid> = HashSet::new();
        for oid in walk.flatten() {
            ancestors.insert(oid);
        }

        let entries: RefCell<Vec<TagIndexEntry>> = RefCell::new(Vec::new());
        repo.tag_foreach(|oid, name| {
            let name = String::from_utf8_lossy(name);
            let tag_name = name.trim_start_matches("refs/tags/").to_string();
            let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
                tag_obj.target_id()
            } else {
                oid
            };
            if let Ok(commit) = repo.find_commit(commit_oid) {
                let reachable = head == commit_oid || ancestors.contains(&commit_oid);
                entries.borrow_mut().push(TagIndexEntry {
                    name: tag_name,
                    commit_oid,
                    time: commit.time().seconds(),
                    reachable,
                });
            }
            true
        })?;
        Ok(Self {
            entries: entries.into_inner(),
            ancestors,
        })
    }

    /// Fast-path version of `find_last_tag_name` for the Warn strategy.
    /// Returns None when the caller asked for orphan recovery — caller
    /// should fall back to `find_last_tag_name_with_cache` in that case.
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

    /// Fast-path version of `find_highest_semver_tag` for the Warn strategy.
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

    /// Fast-path lookup of the commit OID of the most recent matching tag.
    pub fn find_last_tag_commit(
        &self,
        prefix: &str,
        strategy: OrphanedTagStrategy,
    ) -> Option<git2::Oid> {
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

    /// Fast-path lookup of the most recent stable (non-prerelease) tag's commit.
    pub fn find_last_stable_tag_commit(
        &self,
        prefix: &str,
        strategy: OrphanedTagStrategy,
    ) -> Option<git2::Oid> {
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

fn is_reachable(
    repo: &Repository,
    head: git2::Oid,
    commit_oid: git2::Oid,
    cache: Option<&HashSet<git2::Oid>>,
) -> bool {
    if let Some(set) = cache {
        return set.contains(&commit_oid);
    }
    head == commit_oid || repo.graph_descendant_of(head, commit_oid).unwrap_or(false)
}

pub(super) struct TagMatch {
    pub name: String,
    pub commit_oid: git2::Oid,
    pub time: i64,
}

pub(super) fn find_matching_commit(
    repo: &Repository,
    orphaned_commit: &git2::Commit,
    strategy: &OrphanedTagStrategy,
) -> Option<git2::Oid> {
    let mut walk = repo.revwalk().ok()?;
    walk.push_head().ok()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .ok()?;

    let limit = 1000;

    for (count, oid) in walk.enumerate() {
        if count >= limit {
            break;
        }
        let oid = match oid {
            Ok(o) => o,
            Err(_) => continue,
        };
        let candidate = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let matched = match strategy {
            OrphanedTagStrategy::TreeHash => candidate.tree_id() == orphaned_commit.tree_id(),
            OrphanedTagStrategy::Message => candidate.message() == orphaned_commit.message(),
            OrphanedTagStrategy::Warn => return None,
        };
        if matched {
            return Some(oid);
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
    ancestors: Option<&HashSet<git2::Oid>>,
) -> Result<Option<TagMatch>> {
    let head = repo.head()?.peel_to_commit()?.id();
    let latest: RefCell<Option<TagMatch>> = RefCell::new(None);
    let warnings: RefCell<Vec<String>> = RefCell::new(Vec::new());

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if !tag_name.starts_with(prefix) || is_floating_tag(tag_name, prefix) {
            return true;
        }

        let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
            tag_obj.target_id()
        } else {
            oid
        };

        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => {
                warnings.borrow_mut().push(format!(
                    "Warning: tag '{}' points to missing commit {} (likely garbage-collected). Skipping.\n  \
                     Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                     See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                    tag_name,
                    &commit_oid.to_string()[..7]
                ));
                return true;
            }
        };

        let reachable = is_reachable(repo, head, commit_oid, ancestors);

        let (effective_oid, effective_time) = if reachable {
            (commit_oid, commit.time().seconds())
        } else {
            let short = &commit_oid.to_string()[..7];
            if strategy == OrphanedTagStrategy::Warn {
                warnings.borrow_mut().push(format!(
                    "Warning: tag '{}' points to orphaned commit {} (not reachable from HEAD).\n  \
                     Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                     See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                    tag_name, short
                ));
                return true;
            }
            match find_matching_commit(repo, &commit, &strategy) {
                Some(matched_oid) => {
                    let strategy_name = match strategy {
                        OrphanedTagStrategy::TreeHash => "tree-hash",
                        OrphanedTagStrategy::Message => "message",
                        OrphanedTagStrategy::Warn => unreachable!(),
                    };
                    warnings.borrow_mut().push(format!(
                        "Info: tag '{}' was orphaned but matched commit {} on current branch via {}.",
                        tag_name,
                        &matched_oid.to_string()[..7],
                        strategy_name
                    ));
                    let matched_commit = match repo.find_commit(matched_oid) {
                        Ok(c) => c,
                        Err(_) => return true,
                    };
                    (matched_oid, matched_commit.time().seconds())
                }
                None => {
                    let strategy_name = match strategy {
                        OrphanedTagStrategy::TreeHash => "tree-hash",
                        OrphanedTagStrategy::Message => "message",
                        OrphanedTagStrategy::Warn => unreachable!(),
                    };
                    warnings.borrow_mut().push(format!(
                        "Warning: tag '{}' points to orphaned commit {}. No match found via {}. Skipping.\n  \
                         Hint: re-tag manually with 'git tag -f {} <correct-commit>'",
                        tag_name, short, strategy_name, tag_name
                    ));
                    return true;
                }
            }
        };

        let mut latest_ref = latest.borrow_mut();
        if latest_ref.is_none() || effective_time > latest_ref.as_ref().unwrap().time {
            *latest_ref = Some(TagMatch {
                name: tag_name.to_string(),
                commit_oid: effective_oid,
                time: effective_time,
            });
        }
        true
    })?;

    for w in warnings.borrow().iter() {
        eprintln!("{}", w);
    }

    Ok(latest.into_inner())
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
    ancestors: Option<&HashSet<git2::Oid>>,
) -> Result<Option<String>> {
    Ok(find_last_tag_with_cache(repo, prefix, strategy, ancestors)?.map(|t| t.name))
}

// Kept as a public no-cache convenience for single-shot callers (and the
// existing test suite). The cached variant below is the perf path.
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
    ancestors: Option<&HashSet<git2::Oid>>,
) -> Result<Option<(String, String)>> {
    let head = repo.head()?.peel_to_commit()?.id();
    let highest: RefCell<Option<(String, semver::Version)>> = RefCell::new(None);

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if !tag_name.starts_with(prefix)
            || is_prerelease_tag(tag_name, prefix)
            || is_floating_tag(tag_name, prefix)
        {
            return true;
        }

        let version_str = tag_name
            .strip_prefix(prefix)
            .map(|s| s.strip_prefix('v').unwrap_or(s))
            .unwrap_or(tag_name);
        let parsed = match semver::Version::parse(version_str) {
            Ok(v) => v,
            Err(_) => return true,
        };

        let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
            tag_obj.target_id()
        } else {
            oid
        };
        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => return true,
        };
        let reachable = is_reachable(repo, head, commit_oid, ancestors);
        if !reachable {
            match strategy {
                OrphanedTagStrategy::Warn => return true,
                OrphanedTagStrategy::TreeHash | OrphanedTagStrategy::Message => {
                    if find_matching_commit(repo, &commit, &strategy).is_none() {
                        return true;
                    }
                }
            }
        }

        let mut highest_ref = highest.borrow_mut();
        match highest_ref.as_ref() {
            Some((_, existing)) if existing >= &parsed => {}
            _ => {
                *highest_ref = Some((tag_name.to_string(), parsed));
            }
        }
        true
    })?;

    Ok(highest
        .into_inner()
        .map(|(name, version)| (name, version.to_string())))
}

pub(super) fn find_last_tag_commit(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<git2::Oid>> {
    Ok(find_last_tag(repo, prefix, strategy)?.map(|t| t.commit_oid))
}

pub(super) fn find_last_stable_tag(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<TagMatch>> {
    find_last_stable_tag_with_cache(repo, prefix, strategy, None)
}

pub(super) fn find_last_stable_tag_with_cache(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
    ancestors: Option<&HashSet<git2::Oid>>,
) -> Result<Option<TagMatch>> {
    let head = repo.head()?.peel_to_commit()?.id();
    let latest: RefCell<Option<TagMatch>> = RefCell::new(None);

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if !tag_name.starts_with(prefix)
            || is_prerelease_tag(tag_name, prefix)
            || is_floating_tag(tag_name, prefix)
        {
            return true;
        }

        let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
            tag_obj.target_id()
        } else {
            oid
        };

        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => return true,
        };

        let reachable = is_reachable(repo, head, commit_oid, ancestors);

        let (effective_oid, effective_time) = if reachable {
            (commit_oid, commit.time().seconds())
        } else {
            if strategy == OrphanedTagStrategy::Warn {
                return true;
            }
            match find_matching_commit(repo, &commit, &strategy) {
                Some(matched_oid) => {
                    let matched_commit = match repo.find_commit(matched_oid) {
                        Ok(c) => c,
                        Err(_) => return true,
                    };
                    (matched_oid, matched_commit.time().seconds())
                }
                None => return true,
            }
        };

        let mut latest_ref = latest.borrow_mut();
        if latest_ref.is_none() || effective_time > latest_ref.as_ref().unwrap().time {
            *latest_ref = Some(TagMatch {
                name: tag_name.to_string(),
                commit_oid: effective_oid,
                time: effective_time,
            });
        }
        true
    })?;

    Ok(latest.into_inner())
}

/// Route through gitoxide for ~2.7× faster tag enumeration on dense
/// repos. Falls back to libgit2 if gix can't open the workdir (e.g. a
/// transient partial-config state during a release commit). See
/// `tests/gix_parity.rs` for the byte-for-byte equivalence test suite.
pub fn collect_all_tags(repo: &Repository) -> Vec<String> {
    if let Some(workdir) = repo.workdir()
        && let Ok(tags) = collect_all_tags_gix(workdir)
    {
        return tags;
    }
    collect_all_tags_libgit2(repo)
}

pub fn collect_all_tags_libgit2(repo: &Repository) -> Vec<String> {
    let mut tags = Vec::new();
    let _ = repo.tag_foreach(|_oid, name| {
        let name = String::from_utf8_lossy(name);
        tags.push(name.trim_start_matches("refs/tags/").to_string());
        true
    });
    tags
}

/// gitoxide-backed `TagIndex::build`. Kept for reference + the parity
/// suite even though it's currently NOT wired into the production
/// build path — see the comment on [`TagIndex::build`]. The 22 ms vs
/// 14 ms slowdown on 200 tags is preserved here for the parity tests
/// in `tests/gix_parity.rs`, and ready to be re-enabled once we cache
/// a long-lived gix::Repository across the run.
#[allow(dead_code)]
pub fn build_gix(workdir: &std::path::Path) -> anyhow::Result<TagIndex> {
    let repo = gix::open(workdir).map_err(|e| anyhow::anyhow!("gix open: {e}"))?;
    let head_id = repo
        .head_id()
        .map_err(|e| anyhow::anyhow!("gix head_id: {e}"))?
        .detach();

    // Build ancestor set via a single rev_walk from HEAD.
    let mut ancestors_gix: HashSet<gix::ObjectId> = HashSet::new();
    let walk = repo
        .rev_walk([head_id])
        .all()
        .map_err(|e| anyhow::anyhow!("gix rev_walk: {e}"))?;
    for info in walk {
        match info {
            Ok(info) => {
                ancestors_gix.insert(info.id);
            }
            Err(_) => continue,
        }
    }
    let ancestors: HashSet<git2::Oid> = ancestors_gix
        .iter()
        .filter_map(|o| git2::Oid::from_bytes(o.as_slice()).ok())
        .collect();

    // Iterate refs/tags/* and resolve each to its target commit.
    let refs = repo
        .references()
        .map_err(|e| anyhow::anyhow!("gix references: {e}"))?;
    let iter = refs
        .tags()
        .map_err(|e| anyhow::anyhow!("gix tags iter: {e}"))?;
    let mut entries = Vec::new();
    for r in iter {
        let mut r = match r {
            Ok(r) => r,
            Err(_) => continue,
        };
        let tag_name = r
            .name()
            .as_bstr()
            .to_string()
            .trim_start_matches("refs/tags/")
            .to_string();
        // peel_to_id walks annotated tag objects down to the commit
        // they point at; lightweight tags resolve to themselves.
        let commit_id = match r.peel_to_id() {
            Ok(id) => id.detach(),
            Err(_) => continue,
        };
        let commit = match repo.find_object(commit_id) {
            Ok(obj) => match obj.try_into_commit() {
                Ok(c) => c,
                Err(_) => continue,
            },
            Err(_) => continue,
        };
        let time = match commit.time() {
            Ok(t) => t.seconds,
            Err(_) => continue,
        };
        let commit_oid = match git2::Oid::from_bytes(commit_id.as_slice()) {
            Ok(o) => o,
            Err(_) => continue,
        };
        let reachable = ancestors_gix.contains(&commit_id);
        entries.push(TagIndexEntry {
            name: tag_name,
            commit_oid,
            time,
            reachable,
        });
    }

    Ok(TagIndex { entries, ancestors })
}

pub fn collect_all_tags_gix(workdir: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let repo = gix::open(workdir).map_err(|e| anyhow::anyhow!("gix open: {e}"))?;
    let refs = repo
        .references()
        .map_err(|e| anyhow::anyhow!("gix references: {e}"))?;
    let iter = refs
        .tags()
        .map_err(|e| anyhow::anyhow!("gix tags iter: {e}"))?;
    let mut tags = Vec::new();
    for r in iter {
        let r = r.map_err(|e| anyhow::anyhow!("gix tag ref: {e}"))?;
        tags.push(
            r.name()
                .as_bstr()
                .to_string()
                .trim_start_matches("refs/tags/")
                .to_string(),
        );
    }
    Ok(tags)
}

pub fn tag_exists(repo: &Repository, tag_name: &str) -> bool {
    repo.refname_to_id(&format!("refs/tags/{tag_name}")).is_ok()
}

pub fn create_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<()> {
    if tag_exists(repo, tag_name) {
        Err(anyhow::anyhow!("tag {tag_name} already exists"))
            .error_code(error_code::GIT_TAG_EXISTS)?;
    }
    let head = repo.head()?.peel_to_commit()?;
    let sig = signature(repo)?;
    repo.tag(tag_name, head.as_object(), &sig, message, false)?;
    Ok(())
}

pub fn create_or_move_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<bool> {
    let existed = tag_exists(repo, tag_name);
    if existed {
        repo.tag_delete(tag_name)?;
    }
    let head = repo.head()?.peel_to_commit()?;
    let sig = signature(repo)?;
    repo.tag(tag_name, head.as_object(), &sig, message, false)?;
    Ok(existed)
}

pub fn get_tag_message(repo: &Repository, tag_name: &str) -> Option<String> {
    let oid = repo.refname_to_id(&format!("refs/tags/{tag_name}")).ok()?;
    let obj = repo.find_object(oid, None).ok()?;
    let tag = obj.as_tag()?;
    tag.message().map(String::from)
}
