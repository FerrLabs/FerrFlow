use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use crate::config::{Config, PackageConfig};
use crate::conventional_commits::BumpType;
use crate::formats::read_version;
use crate::versioning::compute_next_version;

use super::run::graph::cascade_round;
use super::util::tags_for_package;

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(Debug))]
pub(super) enum Joined {
    Seed,
    Group,
    Dependency,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
pub(super) struct ImpactPackage {
    pub name: String,
    pub bump: String,
    pub current_version: Option<String>,
    pub next_version: Option<String>,
    pub joined: Joined,
    pub depth: usize,
    pub through: Vec<String>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
pub(super) struct ImpactReport {
    pub seed: String,
    pub seed_bump: String,
    pub packages: Vec<ImpactPackage>,
}

pub(super) struct Versions<'a> {
    config: &'a Config,
    root: &'a Path,
    all_tags: &'a [String],
}

impl Versions<'_> {
    fn current(&self, pkg: &PackageConfig) -> Option<String> {
        let vf = pkg.versioned_files.first()?;
        read_version(vf, self.root).ok()
    }

    fn next(&self, pkg: &PackageConfig, bump: BumpType) -> Option<String> {
        let current = self.current(pkg)?;
        let prefix = pkg.tag_prefix(&self.config.workspace, self.config.is_monorepo());
        let strategy = pkg.effective_versioning(&self.config.workspace, || {
            tags_for_package(self.all_tags, &prefix)
        });
        let template = pkg.effective_version_template(&self.config.workspace);
        compute_next_version(&current, bump, strategy, template).ok()
    }
}

pub(super) fn build_report(
    config: &Config,
    root: &Path,
    all_tags: &[String],
    seed: &str,
    seed_bump: BumpType,
) -> Result<ImpactReport> {
    let Some(seed_pkg) = config.packages.iter().find(|p| p.name == seed) else {
        let known: Vec<&str> = config.packages.iter().map(|p| p.name.as_str()).collect();
        bail!(
            "unknown package {seed:?}; the config declares {}",
            known.join(", ")
        );
    };

    let versions = Versions {
        config,
        root,
        all_tags,
    };

    let mut bumped: HashMap<String, BumpType> = HashMap::new();
    bumped.insert(seed.to_string(), seed_bump);

    let seed_next = versions.next(seed_pkg, seed_bump);
    let mut packages = vec![ImpactPackage {
        name: seed.to_string(),
        bump: seed_bump.to_string(),
        current_version: versions.current(seed_pkg),
        next_version: seed_next.clone(),
        joined: Joined::Seed,
        depth: 0,
        through: Vec::new(),
    }];

    for member in group_siblings(config, seed) {
        let Some(pkg) = config.packages.iter().find(|p| p.name == member) else {
            continue;
        };
        bumped.insert(member.clone(), BumpType::None);
        packages.push(ImpactPackage {
            name: member,
            bump: BumpType::None.to_string(),
            current_version: versions.current(pkg),
            next_version: seed_next.clone(),
            joined: Joined::Group,
            depth: 0,
            through: Vec::new(),
        });
    }

    let mut depth = 0;
    loop {
        depth += 1;
        if depth > config.packages.len() {
            break;
        }
        let joined = cascade_round(&config.packages, &bumped);
        if joined.is_empty() {
            break;
        }
        let round: Vec<ImpactPackage> = joined
            .iter()
            .map(|(idx, bump)| {
                let pkg = &config.packages[*idx];
                ImpactPackage {
                    name: pkg.name.clone(),
                    bump: bump.to_string(),
                    current_version: versions.current(pkg),
                    next_version: versions.next(pkg, *bump),
                    joined: Joined::Dependency,
                    depth,
                    through: triggers(pkg, &bumped),
                }
            })
            .collect();
        for (idx, bump) in joined {
            bumped.insert(config.packages[idx].name.clone(), bump);
        }
        packages.extend(round);
    }

    Ok(ImpactReport {
        seed: seed.to_string(),
        seed_bump: seed_bump.to_string(),
        packages,
    })
}

fn group_siblings(config: &Config, seed: &str) -> Vec<String> {
    config
        .package_groups()
        .into_iter()
        .filter(|group| group.members.iter().any(|m| m == seed))
        .flat_map(|group| group.members)
        .filter(|m| m != seed)
        .collect()
}

fn triggers(pkg: &PackageConfig, bumped: &HashMap<String, BumpType>) -> Vec<String> {
    pkg.depends_on
        .iter()
        .filter(|dep| {
            bumped
                .get(dep.name())
                .is_some_and(|up| dep.propagate().resolve(*up) != BumpType::None)
        })
        .map(|dep| dep.name().to_string())
        .collect()
}

pub(super) fn print_text(report: &ImpactReport) {
    use colored::Colorize;

    println!("{}", "FerrFlow — Release impact".bold());
    println!();

    let seed = &report.packages[0];
    println!(
        "  {} {} ({}, assumed)",
        seed.name.bold(),
        version_arrow(seed),
        report.seed_bump
    );

    let rest = &report.packages[1..];
    if rest.is_empty() {
        println!();
        println!("  {}", "nothing else would be released".dimmed());
        return;
    }

    println!();
    println!("  {}", "would also release".bold());
    for pkg in rest {
        println!(
            "    {:<20} {:<18} {}",
            pkg.name,
            version_arrow(pkg),
            why(pkg).dimmed()
        );
    }

    println!();
    println!(
        "  {} packages in total",
        report.packages.len().to_string().bold()
    );
}

fn version_arrow(pkg: &ImpactPackage) -> String {
    match (&pkg.current_version, &pkg.next_version) {
        (Some(current), Some(next)) => format!("{current} → {next}"),
        _ => "—".to_string(),
    }
}

fn why(pkg: &ImpactPackage) -> String {
    match pkg.joined {
        Joined::Seed => String::new(),
        Joined::Group => "same release group".to_string(),
        Joined::Dependency => {
            let via = pkg.through.join(", ");
            if pkg.depth <= 1 {
                format!("{} bump, depends on {via}", pkg.bump)
            } else {
                format!("{} bump, through {via} ({} deep)", pkg.bump, pkg.depth)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(json: &str, seed: &str, bump: BumpType) -> ImpactReport {
        let config: Config = serde_json::from_str(json).expect("valid config");
        build_report(&config, Path::new("."), &[], seed, bump).expect("impact")
    }

    fn names(report: &ImpactReport) -> Vec<&str> {
        report.packages.iter().map(|p| p.name.as_str()).collect()
    }

    fn entry<'a>(report: &'a ImpactReport, name: &str) -> &'a ImpactPackage {
        report
            .packages
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} in impact, got {:?}", names(report)))
    }

    const CHAIN: &str = r#"{
        "package": [
            { "name": "shared", "path": "shared" },
            { "name": "api", "path": "api", "dependsOn": ["shared"] },
            { "name": "mobile", "path": "mobile", "dependsOn": ["api"] }
        ]
    }"#;

    #[test]
    fn a_transitive_dependent_is_reported_further_away_than_a_direct_one() {
        let report = report(CHAIN, "shared", BumpType::Minor);

        assert_eq!(entry(&report, "api").depth, 1);
        assert_eq!(entry(&report, "mobile").depth, 2);
        assert_eq!(
            entry(&report, "mobile").through,
            vec!["api"],
            "mobile is reached through api, not through the package that started it"
        );
    }

    #[test]
    fn an_edge_that_declines_to_propagate_keeps_its_package_out() {
        let report = report(
            r#"{
                "package": [
                    { "name": "shared", "path": "shared" },
                    { "name": "docs", "path": "docs",
                      "dependsOn": [{ "name": "shared", "propagate": "none" }] }
                ]
            }"#,
            "shared",
            BumpType::Major,
        );

        assert_eq!(
            names(&report),
            vec!["shared"],
            "propagate none means the dependent does not release, even on a major"
        );
    }

    #[test]
    fn an_edge_policy_caps_the_bump_it_passes_on() {
        let report = report(
            r#"{
                "package": [
                    { "name": "shared", "path": "shared" },
                    { "name": "web", "path": "web",
                      "dependsOn": [{ "name": "shared", "propagate": "patch" }] },
                    { "name": "api", "path": "api", "dependsOn": ["shared"] }
                ]
            }"#,
            "shared",
            BumpType::Major,
        );

        assert_eq!(entry(&report, "web").bump, "patch");
        assert_eq!(
            entry(&report, "api").bump,
            "major",
            "the default policy passes the upstream bump through unchanged"
        );
    }

    #[test]
    fn a_release_group_joins_without_any_dependency_edge() {
        let report = report(
            r#"{
                "workspace": { "linked": [["web", "docs"]] },
                "package": [
                    { "name": "web", "path": "web" },
                    { "name": "docs", "path": "docs" }
                ]
            }"#,
            "web",
            BumpType::Minor,
        );

        let docs = entry(&report, "docs");
        assert_eq!(docs.joined, Joined::Group);
        assert!(
            docs.through.is_empty(),
            "a group member is not reached through a dependency"
        );
    }

    #[test]
    fn a_group_sibling_does_not_cascade_to_what_depends_on_it() {
        let report = report(
            r#"{
                "workspace": { "linked": [["shared", "ui"]] },
                "package": [
                    { "name": "shared", "path": "shared" },
                    { "name": "ui", "path": "ui" },
                    { "name": "app", "path": "app", "dependsOn": ["ui"] }
                ]
            }"#,
            "shared",
            BumpType::Minor,
        );

        assert_eq!(
            names(&report),
            vec!["shared", "ui"],
            "ui is realigned to the group version without a bump of its own, so a release leaves app alone and the preview must too"
        );
        assert_eq!(
            entry(&report, "ui").bump,
            "none",
            "a release records no bump for a group member that had no plan"
        );
    }

    #[test]
    fn a_package_reached_by_two_edges_takes_the_first_bump_to_arrive() {
        let report = report(
            r#"{
                "package": [
                    { "name": "shared", "path": "shared" },
                    { "name": "api", "path": "api", "dependsOn": ["shared"] },
                    { "name": "web", "path": "web",
                      "dependsOn": [{ "name": "shared", "propagate": "patch" }, "api"] }
                ]
            }"#,
            "shared",
            BumpType::Minor,
        );

        assert_eq!(
            entry(&report, "web").bump,
            "patch",
            "web joins on the patch edge in the first round and is never revisited when the minor arrives through api in the second"
        );
        assert_eq!(
            entry(&report, "web").depth,
            1,
            "it joined in the first round, which is why the later minor cannot reach it"
        );
    }

    #[test]
    fn a_cycle_terminates_instead_of_looping() {
        let report = report(
            r#"{
                "package": [
                    { "name": "a", "path": "a", "dependsOn": ["b"] },
                    { "name": "b", "path": "b", "dependsOn": ["a"] }
                ]
            }"#,
            "a",
            BumpType::Minor,
        );

        assert_eq!(names(&report), vec!["a", "b"]);
    }

    #[test]
    fn an_unknown_package_is_rejected_rather_than_reported_as_harmless() {
        let config: Config = serde_json::from_str(CHAIN).expect("valid config");
        let err = build_report(&config, Path::new("."), &[], "nope", BumpType::Minor)
            .expect_err("unknown package should fail");

        let message = err.to_string();
        assert!(message.contains("nope"), "{message}");
        assert!(
            message.contains("shared"),
            "the error should list what is available: {message}"
        );
    }
}
