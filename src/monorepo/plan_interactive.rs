use anyhow::Result;
use colored::Colorize;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::config::Config;
use crate::conventional_commits::BumpType;
use crate::git::{get_repo_root, open_repo};
use crate::versioning::compute_next_version;

/// Read off the release JSON rather than the internal plan types, so the
/// session consumes the same contract external tooling does: if that
/// shape changes, this breaks visibly instead of drifting.
#[derive(Deserialize)]
struct PlanJson {
    packages: Vec<PlanEntry>,
}

#[derive(Deserialize)]
struct PlanEntry {
    name: String,
    current_version: String,
    next_version: String,
    bump_type: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Override {
    Bump(BumpType),
    Excluded,
}

struct Row {
    package: String,
    current: String,
    planned: Option<String>,
    reason: String,
}

/// The decisions a session produced, rendered as the flags that reproduce them.
pub fn command_for(overrides: &BTreeMap<String, String>, excluded: &[String]) -> String {
    let mut parts = vec!["ferrflow release".to_string()];
    for (pkg, version) in overrides {
        parts.push(format!("--force-version {pkg}@{version}"));
    }
    for pkg in excluded {
        parts.push(format!("--exclude {pkg}"));
    }
    parts.join(" ")
}

fn parse_bump(word: &str) -> Option<BumpType> {
    match word {
        "major" => Some(BumpType::Major),
        "minor" => Some(BumpType::Minor),
        "patch" => Some(BumpType::Patch),
        _ => None,
    }
}

pub fn run(config_path: Option<&Path>, channel: Option<&str>) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    let json = super::plan_json(config_path, channel)?;
    let plan: PlanJson = serde_json::from_str(&json)?;

    let mut rows: Vec<Row> = plan
        .packages
        .iter()
        .map(|p| Row {
            package: p.name.clone(),
            current: p.current_version.clone(),
            planned: Some(p.next_version.clone()),
            reason: p.bump_type.clone(),
        })
        .collect();
    rows.sort_by(|a, b| a.package.cmp(&b.package));

    if rows.is_empty() {
        println!("Nothing to release.");
        return Ok(());
    }

    let mut overrides: BTreeMap<String, Override> = BTreeMap::new();
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        render(&rows, &overrides, &config)?;
        print!("{} ", ">".cyan());
        std::io::stdout().flush()?;

        let Some(line) = lines.next() else { break };
        let line = line?;
        let words: Vec<&str> = line.split_whitespace().collect();

        match words.as_slice() {
            [] => continue,
            ["done"] => break,
            ["quit"] | ["q"] => {
                println!("{}", "no command emitted".dimmed());
                return Ok(());
            }
            [n, bump] if n.parse::<usize>().is_ok() && parse_bump(bump).is_some() => {
                match index(&rows, n) {
                    Some(pkg) => {
                        overrides.insert(pkg, Override::Bump(parse_bump(bump).unwrap()));
                    }
                    None => println!("  {} no package {n}", "✗".red()),
                }
            }
            ["exclude", n] => match index(&rows, n) {
                Some(pkg) => {
                    overrides.insert(pkg, Override::Excluded);
                }
                None => println!("  {} no package {n}", "✗".red()),
            },
            ["include", n] => match index(&rows, n) {
                Some(pkg) => {
                    overrides.remove(&pkg);
                }
                None => println!("  {} no package {n}", "✗".red()),
            },
            _ => println!(
                "  {} commands: <n> major|minor|patch, exclude <n>, include <n>, done, quit",
                "?".yellow()
            ),
        }
    }

    let mut forced = BTreeMap::new();
    let mut excluded = Vec::new();
    for (pkg, ov) in &overrides {
        match ov {
            Override::Excluded => excluded.push(pkg.clone()),
            Override::Bump(bump) => {
                if let Some(version) = resolved_version(&config, &rows, pkg, *bump) {
                    forced.insert(pkg.clone(), version);
                }
            }
        }
    }

    println!();
    if forced.is_empty() && excluded.is_empty() {
        println!("  {} plan unchanged, run:", "→".cyan());
        println!("    ferrflow release");
    } else {
        println!("  {} run:", "→".cyan());
        println!("    {}", command_for(&forced, &excluded).bold());
    }
    Ok(())
}

fn index(rows: &[Row], n: &str) -> Option<String> {
    let i: usize = n.parse().ok()?;
    rows.get(i.checked_sub(1)?).map(|r| r.package.clone())
}

fn resolved_version(
    config: &Config,
    rows: &[Row],
    pkg_name: &str,
    bump: BumpType,
) -> Option<String> {
    let row = rows.iter().find(|r| r.package == pkg_name)?;
    let pkg = config.packages.iter().find(|p| p.name == pkg_name)?;
    let strategy = pkg.effective_versioning(&config.workspace, Vec::new);
    let template = pkg.effective_version_template(&config.workspace);
    let current = if row.current.is_empty() {
        "0.0.0"
    } else {
        &row.current
    };
    compute_next_version(current, bump, strategy, template).ok()
}

fn render(rows: &[Row], overrides: &BTreeMap<String, Override>, config: &Config) -> Result<()> {
    println!();
    println!("{}", "FerrFlow — Release plan (interactive)".bold());
    println!();
    for (i, row) in rows.iter().enumerate() {
        let n = i + 1;
        match overrides.get(&row.package) {
            Some(Override::Excluded) => {
                println!("  {n:>2}  {:<18} {}", row.package, "excluded".yellow());
            }
            Some(Override::Bump(bump)) => {
                let version = resolved_version(config, rows, &row.package, *bump)
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "  {n:>2}  {:<18} {} → {}  {}",
                    row.package,
                    row.current,
                    version.green(),
                    format!("{bump:?}").to_lowercase().green()
                );
            }
            None => match &row.planned {
                Some(next) => println!(
                    "  {n:>2}  {:<18} {} → {}  {}",
                    row.package, row.current, next, row.reason
                ),
                None => println!(
                    "  {n:>2}  {:<18} {}",
                    row.package,
                    format!("skipped ({})", row.reason).dimmed()
                ),
            },
        }
    }
    println!();
    println!(
        "  {}",
        "<n> major|minor|patch, exclude <n>, include <n>, done, quit".dimmed()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row> {
        vec![
            Row {
                package: "api".into(),
                current: "1.0.0".into(),
                planned: Some("1.1.0".into()),
                reason: "minor".into(),
            },
            Row {
                package: "core".into(),
                current: "2.3.4".into(),
                planned: Some("2.3.5".into()),
                reason: "patch".into(),
            },
        ]
    }

    #[test]
    fn the_emitted_command_carries_every_decision() {
        let mut forced = BTreeMap::new();
        forced.insert("api".to_string(), "2.0.0".to_string());
        forced.insert("core".to_string(), "3.0.0".to_string());
        let cmd = command_for(&forced, &["web".to_string(), "docs".to_string()]);
        assert_eq!(
            cmd,
            "ferrflow release --force-version api@2.0.0 --force-version core@3.0.0 \
             --exclude web --exclude docs"
        );
    }

    #[test]
    fn no_decisions_emit_no_flags() {
        assert_eq!(command_for(&BTreeMap::new(), &[]), "ferrflow release");
    }

    #[test]
    fn indices_are_one_based_and_reject_anything_outside_the_list() {
        let r = rows();
        assert_eq!(index(&r, "1").as_deref(), Some("api"));
        assert_eq!(index(&r, "2").as_deref(), Some("core"));
        assert_eq!(index(&r, "0"), None, "0 must not wrap to the last row");
        assert_eq!(index(&r, "3"), None);
        assert_eq!(index(&r, "x"), None);
    }

    #[test]
    fn only_the_three_bump_words_parse() {
        assert_eq!(parse_bump("major"), Some(BumpType::Major));
        assert_eq!(parse_bump("minor"), Some(BumpType::Minor));
        assert_eq!(parse_bump("patch"), Some(BumpType::Patch));
        assert_eq!(parse_bump("Major"), None);
        assert_eq!(parse_bump("none"), None);
    }
}
