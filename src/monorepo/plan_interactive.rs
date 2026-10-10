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

    let Some(overrides) = collect_overrides(&rows, &config, std::io::stdin().lock().lines())?
    else {
        println!("{}", "no command emitted".dimmed());
        return Ok(());
    };
    print_command(&config, &rows, &overrides);
    Ok(())
}

enum Command<'a> {
    Skip,
    Done,
    Quit,
    Set {
        n: &'a str,
        choice: Option<Override>,
    },
    Unknown,
}

fn parse_command<'a>(words: &[&'a str]) -> Command<'a> {
    match words {
        [] => Command::Skip,
        ["done"] => Command::Done,
        ["quit"] | ["q"] => Command::Quit,
        ["exclude", n] => Command::Set {
            n,
            choice: Some(Override::Excluded),
        },
        ["include", n] => Command::Set { n, choice: None },
        [n, bump] => match (n.parse::<usize>(), parse_bump(bump)) {
            (Ok(_), Some(bump)) => Command::Set {
                n,
                choice: Some(Override::Bump(bump)),
            },
            _ => Command::Unknown,
        },
        _ => Command::Unknown,
    }
}

fn collect_overrides(
    rows: &[Row],
    config: &Config,
    mut lines: impl Iterator<Item = std::io::Result<String>>,
) -> Result<Option<BTreeMap<String, Override>>> {
    let mut overrides: BTreeMap<String, Override> = BTreeMap::new();

    loop {
        render(rows, &overrides, config)?;
        print!("{} ", ">".cyan());
        std::io::stdout().flush()?;

        let Some(line) = lines.next() else { break };
        let line = line?;
        let words: Vec<&str> = line.split_whitespace().collect();

        match parse_command(&words) {
            Command::Skip => continue,
            Command::Done => break,
            Command::Quit => return Ok(None),
            Command::Set { n, choice } => apply_choice(rows, &mut overrides, n, choice),
            Command::Unknown => println!(
                "  {} commands: <n> major|minor|patch, exclude <n>, include <n>, done, quit",
                "?".yellow()
            ),
        }
    }
    Ok(Some(overrides))
}

fn apply_choice(
    rows: &[Row],
    overrides: &mut BTreeMap<String, Override>,
    n: &str,
    choice: Option<Override>,
) {
    let Some(pkg) = index(rows, n) else {
        println!("  {} no package {n}", "✗".red());
        return;
    };
    match choice {
        Some(ov) => {
            overrides.insert(pkg, ov);
        }
        None => {
            overrides.remove(&pkg);
        }
    }
}

fn print_command(config: &Config, rows: &[Row], overrides: &BTreeMap<String, Override>) {
    println!();
    match changed_command(config, rows, overrides) {
        None => {
            println!("  {} plan unchanged, run:", "→".cyan());
            println!("    ferrflow release");
        }
        Some(command) => {
            println!("  {} run:", "→".cyan());
            println!("    {}", command.bold());
        }
    }
}

fn changed_command(
    config: &Config,
    rows: &[Row],
    overrides: &BTreeMap<String, Override>,
) -> Option<String> {
    let mut forced = BTreeMap::new();
    let mut excluded = Vec::new();
    for (pkg, ov) in overrides {
        match ov {
            Override::Excluded => excluded.push(pkg.clone()),
            Override::Bump(bump) => {
                if let Some(version) = resolved_version(config, rows, pkg, *bump) {
                    forced.insert(pkg.clone(), version);
                }
            }
        }
    }

    (!forced.is_empty() || !excluded.is_empty()).then(|| command_for(&forced, &excluded))
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
                    bump.to_string().green()
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
mod tests;
