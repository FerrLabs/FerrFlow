use crate::monorepo::run::checkpoint::{Checkpoint, Phase};

/// One undo step, decided before anything is touched so `--dry-run` and the
/// real run print exactly the same list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    DeleteTag { name: String, sha: String },
    DeleteRelease { tag: String, id: u64 },
    RevertCommit { sha: String },
    RestoreManifest,
}

/// A package that cannot be rolled back, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocked {
    pub package: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RollbackPlan {
    pub steps: Vec<Step>,
    pub blocked: Vec<Blocked>,
}

impl RollbackPlan {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// Decides what a rollback would do, from the checkpoint alone.
///
/// `packages` narrows the rollback to a subset; empty means everything the run
/// touched. A package that published to an immutable registry is reported as
/// blocked and contributes no steps, so a partially-published monorepo run
/// rolls back the packages it can and refuses the rest rather than doing half
/// of each.
pub fn plan(
    checkpoint: &Checkpoint,
    packages: &[String],
    has_manifest: bool,
    tag_owner: impl Fn(&str) -> Option<String>,
) -> RollbackPlan {
    let mut out = RollbackPlan::default();

    let blocked_packages = immutable_packages(checkpoint);
    for package in &blocked_packages {
        if wanted(packages, package) {
            out.blocked.push(Blocked {
                package: package.clone(),
                reason: immutable_reason(checkpoint, package),
            });
        }
    }

    let skip = |tag: &str| {
        tag_owner(tag)
            .map(|pkg| blocked_packages.contains(&pkg) || !wanted(packages, &pkg))
            .unwrap_or(!packages.is_empty())
    };

    for release in &checkpoint.forge_releases {
        if !skip(&release.tag) {
            out.steps.push(Step::DeleteRelease {
                tag: release.tag.clone(),
                id: release.id,
            });
        }
    }

    for tag in &checkpoint.created_tags {
        if !skip(&tag.name) {
            out.steps.push(Step::DeleteTag {
                name: tag.name.clone(),
                sha: tag.sha.clone(),
            });
        }
    }

    // The release commit carries every package's bump, so reverting it while
    // some packages stay released would undo their version files too. Only
    // revert on a whole-run rollback with nothing blocked.
    let whole_run = packages.is_empty() && out.blocked.is_empty();
    if whole_run
        && checkpoint.phase >= Phase::CommitDone
        && let Some(sha) = &checkpoint.commit_sha
    {
        out.steps.push(Step::RevertCommit { sha: sha.clone() });
        // Only when the repo actually keeps a manifest; planning it otherwise
        // makes the rollback fail at the last step, after it has already
        // deleted the tags, and leaves the checkpoint behind.
        if has_manifest {
            out.steps.push(Step::RestoreManifest);
        }
    }

    out
}

fn wanted(packages: &[String], package: &str) -> bool {
    packages.is_empty() || packages.iter().any(|p| p == package)
}

fn immutable_packages(checkpoint: &Checkpoint) -> Vec<String> {
    let mut names: Vec<String> = checkpoint
        .published
        .iter()
        .filter(|p| p.immutable)
        .map(|p| p.package.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

fn immutable_reason(checkpoint: &Checkpoint, package: &str) -> String {
    let mut kinds: Vec<&str> = checkpoint
        .published
        .iter()
        .filter(|p| p.immutable && p.package == package)
        .map(|p| p.kind.as_str())
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    format!(
        "already published to {}, which cannot be unpublished; release a new patch version instead",
        kinds.join(" and ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monorepo::run::checkpoint::{RecordedPublish, RecordedRelease, RecordedTag};

    fn checkpoint(phase: Phase) -> Checkpoint {
        let mut cp = Checkpoint::new("head".to_string(), Vec::new());
        cp.phase = phase;
        cp.commit_sha = Some("commit".to_string());
        cp
    }

    fn tag(cp: &mut Checkpoint, name: &str, sha: &str) {
        cp.created_tags.push(RecordedTag {
            name: name.to_string(),
            sha: sha.to_string(),
        });
    }

    fn release(cp: &mut Checkpoint, tag: &str, id: u64) {
        cp.forge_releases.push(RecordedRelease {
            tag: tag.to_string(),
            id,
        });
    }

    fn published(cp: &mut Checkpoint, package: &str, kind: &str, immutable: bool) {
        cp.published.push(RecordedPublish {
            package: package.to_string(),
            kind: kind.to_string(),
            immutable,
        });
    }

    /// Maps `pkg@v1.0.0` back to `pkg`, which is what the real caller derives
    /// from the config.
    fn owner(tag: &str) -> Option<String> {
        tag.split_once('@').map(|(p, _)| p.to_string())
    }

    #[test]
    fn a_clean_run_rolls_back_releases_then_tags_then_the_commit() {
        let mut cp = checkpoint(Phase::ReleasesCreated);
        tag(&mut cp, "api@v1.1.0", "sha-api");
        release(&mut cp, "api@v1.1.0", 42);

        let out = plan(&cp, &[], true, owner);

        assert_eq!(
            out.steps,
            vec![
                Step::DeleteRelease {
                    tag: "api@v1.1.0".to_string(),
                    id: 42
                },
                Step::DeleteTag {
                    name: "api@v1.1.0".to_string(),
                    sha: "sha-api".to_string()
                },
                Step::RevertCommit {
                    sha: "commit".to_string()
                },
                Step::RestoreManifest,
            ]
        );
        assert!(out.blocked.is_empty());
    }

    #[test]
    fn a_monorepo_failing_at_package_three_rolls_back_one_to_three_only() {
        let mut cp = checkpoint(Phase::ReleasesCreated);
        for name in ["a", "b", "c"] {
            tag(&mut cp, &format!("{name}@v1.0.0"), name);
            release(&mut cp, &format!("{name}@v1.0.0"), 1);
        }

        let out = plan(&cp, &[], true, owner);
        let tags: Vec<&str> = out
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::DeleteTag { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(tags, vec!["a@v1.0.0", "b@v1.0.0", "c@v1.0.0"]);
    }

    #[test]
    fn an_immutable_publish_blocks_that_package_and_spares_the_others() {
        let mut cp = checkpoint(Phase::ReleasesCreated);
        tag(&mut cp, "api@v1.1.0", "sha-api");
        tag(&mut cp, "web@v2.0.0", "sha-web");
        published(&mut cp, "api", "cargo", true);

        let out = plan(&cp, &[], true, owner);

        assert_eq!(out.blocked.len(), 1);
        assert_eq!(out.blocked[0].package, "api");
        assert!(out.blocked[0].reason.contains("cargo"));
        assert!(out.blocked[0].reason.contains("new patch version"));

        let tags: Vec<&str> = out
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::DeleteTag { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            tags,
            vec!["web@v2.0.0"],
            "api's tag must survive its publish"
        );
    }

    #[test]
    fn a_blocked_package_stops_the_commit_from_being_reverted() {
        let mut cp = checkpoint(Phase::ReleasesCreated);
        tag(&mut cp, "api@v1.1.0", "sha-api");
        published(&mut cp, "api", "npm", true);

        let out = plan(&cp, &[], true, owner);

        assert!(
            !out.steps.contains(&Step::RevertCommit {
                sha: "commit".to_string()
            }),
            "reverting would undo the version bump of a package that stays released"
        );
    }

    #[test]
    fn a_mutable_publish_does_not_block_anything() {
        let mut cp = checkpoint(Phase::ReleasesCreated);
        tag(&mut cp, "api@v1.1.0", "sha-api");
        published(&mut cp, "api", "docker", false);

        let out = plan(&cp, &[], true, owner);

        assert!(out.blocked.is_empty(), "a docker tag can be overwritten");
        assert!(
            out.steps
                .iter()
                .any(|s| matches!(s, Step::DeleteTag { .. }))
        );
    }

    #[test]
    fn naming_a_package_narrows_the_rollback_to_it() {
        let mut cp = checkpoint(Phase::ReleasesCreated);
        tag(&mut cp, "api@v1.1.0", "sha-api");
        tag(&mut cp, "web@v2.0.0", "sha-web");

        let out = plan(&cp, &["web".to_string()], true, owner);

        let tags: Vec<&str> = out
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::DeleteTag { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(tags, vec!["web@v2.0.0"]);
        assert!(
            !out.steps.contains(&Step::RestoreManifest),
            "a partial rollback must not rewrite the whole manifest"
        );
    }

    #[test]
    fn a_run_that_never_committed_has_nothing_to_revert() {
        let mut cp = checkpoint(Phase::Pending);
        cp.commit_sha = None;
        tag(&mut cp, "api@v1.1.0", "sha-api");

        let out = plan(&cp, &[], true, owner);

        assert!(
            !out.steps
                .iter()
                .any(|s| matches!(s, Step::RevertCommit { .. }))
        );
        assert!(
            out.steps
                .iter()
                .any(|s| matches!(s, Step::DeleteTag { .. }))
        );
    }

    #[test]
    fn a_repo_without_a_manifest_does_not_plan_to_restore_one() {
        let mut cp = checkpoint(Phase::CommitDone);
        tag(&mut cp, "api@v1.1.0", "sha-api");

        let out = plan(&cp, &[], false, owner);

        assert!(
            !out.steps.contains(&Step::RestoreManifest),
            "planning it would fail the run after the tags were already deleted"
        );
        assert!(out.steps.contains(&Step::RevertCommit {
            sha: "commit".to_string()
        }));
    }

    #[test]
    fn a_checkpoint_with_nothing_recorded_plans_nothing() {
        let out = plan(&checkpoint(Phase::Pending), &[], true, owner);
        assert!(out.is_empty());
    }
}
