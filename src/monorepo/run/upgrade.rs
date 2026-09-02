use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::conventional_commits::BumpType;
use crate::versioning::compute_next_version;

use super::super::util::tags_for_package;
use super::plan::PackagePlan;

/// Raise a package's own bump when a dependency it declares moves further than
/// its commits did.
///
/// A package with a `fix` of its own that depends on something taking a minor
/// has to ship the minor: the dependency it is being rebuilt against changed
/// by that much. Its commits decide the floor, not the answer.
///
/// This runs on the plans, before anything is written, for the same reason the
/// cascade settles before acting. The release loop writes each package once
/// from its plan, so raising the bump afterwards would mean rewriting the
/// version file, the changelog and the tag it had already produced.
pub(super) fn apply_cascade_upgrades(
    config: &Config,
    all_tags: &[String],
    plans: &mut [Option<PackagePlan>],
) {
    if !config.is_monorepo() {
        return;
    }

    let mut state: HashMap<String, BumpType> = HashMap::new();
    for (idx, plan) in plans.iter().enumerate() {
        if let Some(PackagePlan::Bump(bump)) = plan {
            state.insert(config.packages[idx].name.clone(), bump.bump);
        }
    }
    if state.is_empty() {
        return;
    }

    let mut raised: HashMap<usize, (BumpType, String)> = HashMap::new();
    let mut blocked: HashSet<usize> = HashSet::new();

    for _ in 0..config.packages.len().saturating_mul(4) {
        let moved: Vec<(usize, BumpType)> = super::graph::cascade_round(&config.packages, &state)
            .into_iter()
            .filter(|(idx, _)| !blocked.contains(idx))
            .collect();
        if moved.is_empty() {
            break;
        }
        let mut progressed = false;
        for (idx, bump) in moved {
            // A package with no plan of its own is left to the cascade proper,
            // which runs after the release loop. It still has to enter the
            // walk, or what depends on it never learns that it moved.
            if matches!(plans[idx], Some(PackagePlan::Bump(_))) {
                let Some(version) = raised_version(config, all_tags, plans, idx, bump) else {
                    // The raise cannot be applied, so it must not be recorded
                    // either: a dependent settling on it would cascade off a
                    // version this package never takes.
                    blocked.insert(idx);
                    continue;
                };
                raised.insert(idx, (bump, version));
            }
            state.insert(config.packages[idx].name.clone(), bump);
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    for (idx, (bump, new_version)) in raised {
        let pkg = &config.packages[idx];
        let Some(PackagePlan::Bump(plan)) = plans[idx].as_mut() else {
            continue;
        };
        // The label is the bump for semver, but a calendar strategy name or
        // "forced" otherwise, and those must survive the raise untouched.
        if plan.strategy_label == plan.bump.to_string() {
            plan.strategy_label = bump.to_string();
        }
        plan.bump = bump;
        plan.tag = pkg.tag_for_version(&config.workspace, true, &new_version);
        plan.new_version = new_version;
    }
}

/// The version a package would take at `bump`, or `None` when the strategy
/// cannot produce one.
fn raised_version(
    config: &Config,
    all_tags: &[String],
    plans: &[Option<PackagePlan>],
    idx: usize,
    bump: BumpType,
) -> Option<String> {
    let pkg = &config.packages[idx];
    let Some(PackagePlan::Bump(plan)) = plans.get(idx)? else {
        return None;
    };
    let prefix = pkg.tag_prefix(&config.workspace, true);
    let strategy =
        pkg.effective_versioning(&config.workspace, || tags_for_package(all_tags, &prefix));
    let template = pkg.effective_version_template(&config.workspace);
    compute_next_version(&plan.current_version, bump, strategy, template).ok()
}

#[cfg(test)]
mod tests {
    use super::super::plan::PackageBump;
    use super::*;
    use crate::monorepo::version_source::VersionSource;

    fn plan_for(current: &str, next: &str, bump: BumpType) -> Option<PackagePlan> {
        Some(PackagePlan::Bump(Box::new(PackageBump {
            recovered: false,
            current_version: current.to_string(),
            new_version: next.to_string(),
            is_prerelease: false,
            last_tag: None,
            commits: Vec::new(),
            bump,
            strategy_label: bump.to_string(),
            tag: format!("v{next}"),
            version_source: None::<VersionSource>,
        })))
    }

    fn bump_of(plans: &[Option<PackagePlan>], idx: usize) -> Option<(BumpType, String, String)> {
        match plans.get(idx)? {
            Some(PackagePlan::Bump(b)) => {
                Some((b.bump, b.new_version.clone(), b.strategy_label.clone()))
            }
            _ => None,
        }
    }

    const CHAIN: &str = r#"{
        "package": [
            { "name": "shared", "path": "shared" },
            { "name": "api", "path": "api", "dependsOn": ["shared"] },
            { "name": "web", "path": "web", "dependsOn": ["api"] }
        ]
    }"#;

    #[test]
    fn a_packages_own_patch_is_raised_by_a_dependency_taking_a_minor() {
        let config: Config = serde_json::from_str(CHAIN).expect("valid config");
        // api has no commits of its own; the cascade bumps it after the loop.
        let mut plans = vec![
            plan_for("1.0.0", "1.1.0", BumpType::Minor),
            None,
            plan_for("1.0.0", "1.0.1", BumpType::Patch),
        ];

        apply_cascade_upgrades(&config, &[], &mut plans);

        let (bump, version, label) = bump_of(&plans, 2).expect("web still has a plan");
        assert_eq!(bump, BumpType::Minor);
        assert_eq!(
            version, "1.1.0",
            "the version has to follow the raised bump, not stay at the patch"
        );
        assert_eq!(
            label, "minor",
            "the printed label is derived from the bump and must not go stale"
        );
    }

    #[test]
    fn a_package_with_no_plan_is_left_for_the_cascade() {
        let config: Config = serde_json::from_str(CHAIN).expect("valid config");
        let mut plans = vec![plan_for("1.0.0", "1.1.0", BumpType::Minor), None, None];

        apply_cascade_upgrades(&config, &[], &mut plans);

        assert!(
            plans[1].is_none(),
            "api gets its plan from the cascade, which runs after the release loop"
        );
    }

    #[test]
    fn a_bump_already_as_strong_is_left_alone() {
        let config: Config = serde_json::from_str(CHAIN).expect("valid config");
        let mut plans = vec![
            plan_for("1.0.0", "1.0.1", BumpType::Patch),
            None,
            plan_for("2.0.0", "3.0.0", BumpType::Major),
        ];

        apply_cascade_upgrades(&config, &[], &mut plans);

        let (bump, version, _) = bump_of(&plans, 2).expect("web still has a plan");
        assert_eq!(bump, BumpType::Major);
        assert_eq!(version, "3.0.0", "nothing weaker may overwrite it");
    }

    #[test]
    fn a_raise_whose_version_cannot_be_computed_is_not_passed_on() {
        let config: Config = serde_json::from_str(CHAIN).expect("valid config");
        // api holds a version no strategy can bump, so its raise cannot be
        // applied. web depends on it and must not settle on the raise either.
        let mut plans = vec![
            plan_for("1.0.0", "1.1.0", BumpType::Minor),
            plan_for("not-a-version", "not-a-version", BumpType::Patch),
            plan_for("1.0.0", "1.0.1", BumpType::Patch),
        ];

        apply_cascade_upgrades(&config, &[], &mut plans);

        let (api_bump, api_version, _) = bump_of(&plans, 1).expect("api still has a plan");
        assert_eq!(api_bump, BumpType::Patch, "the raise could not be applied");
        assert_eq!(api_version, "not-a-version", "so nothing was written to it");

        let (web_bump, web_version, _) = bump_of(&plans, 2).expect("web still has a plan");
        assert_eq!(
            web_bump,
            BumpType::Patch,
            "web must not cascade off a raise api never took"
        );
        assert_eq!(web_version, "1.0.1");
    }

    #[test]
    fn an_edge_that_declines_to_propagate_raises_nothing() {
        let config: Config = serde_json::from_str(
            r#"{
                "package": [
                    { "name": "shared", "path": "shared" },
                    { "name": "web", "path": "web",
                      "dependsOn": [{ "name": "shared", "propagate": "none" }] }
                ]
            }"#,
        )
        .expect("valid config");
        let mut plans = vec![
            plan_for("1.0.0", "2.0.0", BumpType::Major),
            plan_for("1.0.0", "1.0.1", BumpType::Patch),
        ];

        apply_cascade_upgrades(&config, &[], &mut plans);

        let (bump, version, _) = bump_of(&plans, 1).expect("web still has a plan");
        assert_eq!(bump, BumpType::Patch);
        assert_eq!(version, "1.0.1");
    }
}
