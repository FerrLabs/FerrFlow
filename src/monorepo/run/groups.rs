use std::collections::HashMap;
use std::path::Path;

use crate::config::Config;
use crate::conventional_commits::BumpType;
use crate::formats::read_version;

use super::super::util::pick_higher_semver;
use super::plan::{PackageBump, PackagePlan};

pub(super) fn apply_groups(config: &Config, root: &Path, plans: &mut [Option<PackagePlan>]) {
    let groups = config.package_groups();
    if groups.is_empty() {
        return;
    }

    let index_of: HashMap<&str, usize> = config
        .packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();
    let is_monorepo = config.is_monorepo();

    for group in &groups {
        let member_idxs: Vec<usize> = group
            .members
            .iter()
            .filter_map(|name| index_of.get(name.as_str()).copied())
            .collect();

        let Some((target_version, target_is_prerelease)) = group_target(plans, &member_idxs) else {
            continue;
        };

        for idx in member_idxs {
            let pkg = &config.packages[idx];
            let target_tag = pkg.tag_for_version(&config.workspace, is_monorepo, &target_version);

            match plans[idx].take() {
                Some(PackagePlan::Bump(mut bump)) => {
                    if bump.new_version != target_version {
                        bump.new_version = target_version.clone();
                        bump.tag = target_tag;
                        bump.is_prerelease = target_is_prerelease;
                    }
                    plans[idx] = Some(PackagePlan::Bump(bump));
                }
                _ => {
                    let current = pkg
                        .versioned_files
                        .first()
                        .and_then(|vf| read_version(vf, root).ok())
                        .unwrap_or_else(|| target_version.clone());
                    plans[idx] = Some(PackagePlan::Bump(Box::new(PackageBump {
                        recovered: false,
                        current_version: current,
                        new_version: target_version.clone(),
                        is_prerelease: target_is_prerelease,
                        last_tag: None,
                        commits: Vec::new(),
                        bump: BumpType::None,
                        strategy_label: group.kind.label().to_string(),
                        tag: target_tag,
                    })));
                }
            }
        }
    }
}

fn group_target(plans: &[Option<PackagePlan>], member_idxs: &[usize]) -> Option<(String, bool)> {
    let mut best: Option<(String, bool)> = None;
    for &idx in member_idxs {
        let Some(Some(PackagePlan::Bump(bump))) = plans.get(idx) else {
            continue;
        };
        best = match best {
            None => Some((bump.new_version.clone(), bump.is_prerelease)),
            Some((current, _))
                if pick_higher_semver(&current, &bump.new_version) == bump.new_version =>
            {
                Some((bump.new_version.clone(), bump.is_prerelease))
            }
            keep => keep,
        };
    }
    best
}

#[cfg(test)]
mod tests {
    use super::super::plan::SkipReason;
    use super::*;
    use crate::changelog::GitLog;

    fn write_pkg(root: &Path, name: &str, version: &str) {
        std::fs::create_dir_all(root.join(name)).unwrap();
        std::fs::write(
            root.join(name).join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .unwrap();
    }

    fn config(root: &Path, names: &[&str], linked: &str, fixed: &str) -> Config {
        for n in names {
            write_pkg(root, n, "1.0.0");
        }
        let packages: Vec<String> = names
            .iter()
            .map(|n| {
                format!(
                    r#"{{"name":"{n}","path":"{n}","versionedFiles":[{{"path":"{n}/Cargo.toml","format":"toml"}}]}}"#
                )
            })
            .collect();
        serde_json::from_str(&format!(
            r#"{{"workspace":{{"linked":{linked},"fixed":{fixed}}},"package":[{}]}}"#,
            packages.join(",")
        ))
        .unwrap()
    }

    fn bump(current: &str, new: &str, tag: &str, bump: BumpType) -> Option<PackagePlan> {
        Some(PackagePlan::Bump(Box::new(PackageBump {
            recovered: false,
            current_version: current.to_string(),
            new_version: new.to_string(),
            is_prerelease: false,
            last_tag: None,
            commits: vec![GitLog {
                hash: "abc1234".to_string(),
                message: "feat: x".to_string(),
            }],
            bump,
            strategy_label: bump.to_string(),
            tag: tag.to_string(),
        })))
    }

    fn as_bump(plan: &Option<PackagePlan>) -> &PackageBump {
        match plan {
            Some(PackagePlan::Bump(b)) => b,
            _ => panic!("expected a bump plan, got a skip"),
        }
    }

    #[test]
    fn linked_pulls_an_unchanged_member_to_the_same_version() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let config = config(root, &["a", "b"], r#"[["a","b"]]"#, "[]");

        let mut plans = vec![
            bump("1.0.0", "1.1.0", "a@v1.1.0", BumpType::Minor),
            Some(PackagePlan::Skipped {
                reason: SkipReason::NotTouched,
                recovered: false,
            }),
        ];
        apply_groups(&config, root, &mut plans);

        let a = as_bump(&plans[0]);
        let b = as_bump(&plans[1]);
        assert_eq!(a.new_version, "1.1.0");
        assert_eq!(b.new_version, "1.1.0", "unchanged member follows the group");
        assert_eq!(b.tag, "b@v1.1.0");
        assert_eq!(
            b.current_version, "1.0.0",
            "current read from the version file"
        );
        assert!(b.commits.is_empty(), "no commits of its own");
    }

    #[test]
    fn fixed_aligns_every_member_to_the_highest_bump() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let config = config(root, &["a", "b", "c"], "[]", r#"[["a","b","c"]]"#);

        let mut plans = vec![
            bump("1.0.0", "1.1.0", "a@v1.1.0", BumpType::Minor),
            bump("1.0.0", "1.0.1", "b@v1.0.1", BumpType::Patch),
            Some(PackagePlan::Skipped {
                reason: SkipReason::NoNewCommits,
                recovered: false,
            }),
        ];
        apply_groups(&config, root, &mut plans);

        for (idx, name) in ["a", "b", "c"].iter().enumerate() {
            let plan = as_bump(&plans[idx]);
            assert_eq!(
                plan.new_version, "1.1.0",
                "{name} must land on the max bump"
            );
            assert_eq!(plan.tag, format!("{name}@v1.1.0"));
        }
        // The patch member was raised to the group minor.
        assert_eq!(as_bump(&plans[1]).new_version, "1.1.0");
    }

    #[test]
    fn group_with_no_releasable_member_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let config = config(root, &["a", "b"], r#"[["a","b"]]"#, "[]");

        let mut plans = vec![
            Some(PackagePlan::Skipped {
                reason: SkipReason::NotTouched,
                recovered: false,
            }),
            Some(PackagePlan::Skipped {
                reason: SkipReason::NotTouched,
                recovered: false,
            }),
        ];
        apply_groups(&config, root, &mut plans);

        assert!(matches!(plans[0], Some(PackagePlan::Skipped { .. })));
        assert!(matches!(plans[1], Some(PackagePlan::Skipped { .. })));
    }
}
