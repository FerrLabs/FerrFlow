use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::config::{Config, PackageConfig};
use crate::git::{get_repo_root, open_repo};
use crate::monorepo::run::graph::release_order;

#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
struct GraphPackage {
    name: String,
    depends_on: Vec<String>,
    dependents: Vec<String>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
struct GraphReport {
    packages: Vec<GraphPackage>,
    release_order: Vec<String>,
    cycle: Option<Vec<String>>,
}

pub fn run(config_path: Option<&Path>, json: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;
    let report = build_report(&config.packages);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }

    if report.cycle.is_some() {
        anyhow::bail!("dependency cycle detected");
    }
    Ok(())
}

fn build_report(packages: &[PackageConfig]) -> GraphReport {
    let known: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();

    let mut dependents: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for pkg in packages {
        for dep in &pkg.depends_on {
            if known.contains(&dep.name()) {
                dependents
                    .entry(dep.name())
                    .or_default()
                    .push(pkg.name.clone());
            }
        }
    }

    let entries: Vec<GraphPackage> = packages
        .iter()
        .map(|pkg| GraphPackage {
            name: pkg.name.clone(),
            depends_on: pkg
                .depends_on
                .iter()
                .filter(|dep| known.contains(&dep.name()))
                .map(|dep| dep.name().to_string())
                .collect(),
            dependents: dependents
                .get(pkg.name.as_str())
                .cloned()
                .unwrap_or_default(),
        })
        .collect();

    let (order, cycle) = match release_order(packages) {
        Ok(order) => (
            order.iter().map(|&i| packages[i].name.clone()).collect(),
            None,
        ),
        Err(found) => {
            let mut path = found.path().to_vec();
            if let Some(first) = path.first().cloned() {
                path.push(first);
            }
            (Vec::new(), Some(path))
        }
    };

    GraphReport {
        packages: entries,
        release_order: order,
        cycle,
    }
}

fn print_text(report: &GraphReport) {
    println!("{}", "FerrFlow — Dependency graph".bold());
    println!();

    for pkg in &report.packages {
        println!("  {}", pkg.name.bold());
        if pkg.depends_on.is_empty() {
            println!("    depends on  {}", "—".dimmed());
        } else {
            println!("    depends on  {}", pkg.depends_on.join(", "));
        }
        if pkg.dependents.is_empty() {
            println!("    required by {}", "—".dimmed());
        } else {
            println!("    required by {}", pkg.dependents.join(", "));
        }
    }

    println!();
    match &report.cycle {
        Some(path) => {
            println!("  {} cycle detected: {}", "✗".red(), path.join(" → ").red());
            println!(
                "  {}",
                "no release order exists while this cycle stands".dimmed()
            );
        }
        None => {
            println!(
                "  {} {}",
                "release order".bold(),
                report.release_order.join(" → ")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, deps: &[&str]) -> PackageConfig {
        PackageConfig {
            version_source: None,
            name: name.to_string(),
            path: name.to_string(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: deps
                .iter()
                .map(|s| crate::config::Dependency::Name(s.to_string()))
                .collect(),
            update_lockfiles: None,
            versioning: None,
            tag_template: None,
            version_template: None,
            floating_tags: None,
            latest_tag: None,
            hooks: None,
            publishers: vec![],
        }
    }

    fn find<'a>(report: &'a GraphReport, name: &str) -> &'a GraphPackage {
        report
            .packages
            .iter()
            .find(|p| p.name == name)
            .expect("package in report")
    }

    #[test]
    fn dependents_are_the_reverse_of_depends_on() {
        let report = build_report(&[
            pkg("core", &[]),
            pkg("api", &["core"]),
            pkg("cli", &["api", "core"]),
        ]);

        assert_eq!(find(&report, "core").depends_on, Vec::<String>::new());
        assert_eq!(find(&report, "core").dependents, vec!["api", "cli"]);
        assert_eq!(find(&report, "cli").depends_on, vec!["api", "core"]);
        assert_eq!(find(&report, "cli").dependents, Vec::<String>::new());
    }

    #[test]
    fn release_order_puts_dependencies_before_dependents() {
        let report = build_report(&[
            pkg("cli", &["api"]),
            pkg("api", &["core"]),
            pkg("core", &[]),
        ]);

        assert_eq!(report.cycle, None);
        let order = &report.release_order;
        let at = |name: &str| order.iter().position(|n| n == name).expect("in order");
        assert!(at("core") < at("api"), "{order:?}");
        assert!(at("api") < at("cli"), "{order:?}");
    }

    #[test]
    fn a_cycle_is_reported_with_a_closed_path_and_no_order() {
        let report = build_report(&[pkg("a", &["b"]), pkg("b", &["a"])]);

        let cycle = report.cycle.expect("cycle detected");
        assert_eq!(cycle.first(), cycle.last(), "path should close: {cycle:?}");
        assert!(cycle.contains(&"a".to_string()) && cycle.contains(&"b".to_string()));
        assert!(report.release_order.is_empty());
    }

    #[test]
    fn a_dependency_outside_the_workspace_is_left_out_rather_than_invented() {
        let report = build_report(&[pkg("api", &["serde", "core"]), pkg("core", &[])]);

        assert_eq!(find(&report, "api").depends_on, vec!["core"]);
        assert_eq!(report.cycle, None);
    }
}
