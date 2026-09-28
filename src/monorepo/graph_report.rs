use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use crate::config::{Config, PackageConfig};
use crate::conventional_commits::BumpType;
use crate::git::{collect_all_tags, get_repo_root, open_repo};
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

pub fn run(config_path: Option<&Path>, json: bool, impact: Option<&str>, bump: &str) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if let Some(seed) = impact {
        let all_tags = collect_all_tags(&repo);
        let report = crate::monorepo::impact::build_report(
            &config,
            &root,
            &all_tags,
            seed,
            parse_bump(bump)?,
        )?;
        if json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            crate::monorepo::impact::print_text(&report);
        }
        return Ok(());
    }

    let report = build_report(&config.packages);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        write_text(&mut std::io::stdout().lock(), &report)?;
    }

    if report.cycle.is_some() {
        anyhow::bail!("dependency cycle detected");
    }
    Ok(())
}

fn parse_bump(raw: &str) -> Result<BumpType> {
    match raw {
        "major" => Ok(BumpType::Major),
        "minor" => Ok(BumpType::Minor),
        "patch" => Ok(BumpType::Patch),
        other => anyhow::bail!("unknown bump {other:?}; expected major, minor or patch"),
    }
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

fn write_text(out: &mut impl Write, report: &GraphReport) -> std::io::Result<()> {
    writeln!(out, "{}", "FerrFlow — Dependency graph".bold())?;
    writeln!(out)?;

    for pkg in &report.packages {
        writeln!(out, "  {}", pkg.name.bold())?;
        if pkg.depends_on.is_empty() {
            writeln!(out, "    depends on  {}", "—".dimmed())?;
        } else {
            writeln!(out, "    depends on  {}", pkg.depends_on.join(", "))?;
        }
        if pkg.dependents.is_empty() {
            writeln!(out, "    required by {}", "—".dimmed())?;
        } else {
            writeln!(out, "    required by {}", pkg.dependents.join(", "))?;
        }
    }

    writeln!(out)?;
    match &report.cycle {
        Some(path) => {
            writeln!(
                out,
                "  {} cycle detected: {}",
                "✗".red(),
                path.join(" → ").red()
            )?;
            writeln!(
                out,
                "  {}",
                "no release order exists while this cycle stands".dimmed()
            )?;
        }
        None => {
            writeln!(
                out,
                "  {} {}",
                "release order".bold(),
                report.release_order.join(" → ")
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
