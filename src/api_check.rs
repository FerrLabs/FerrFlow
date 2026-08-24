use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{Config, PackageConfig};
use crate::conventional_commits::{BumpType, determine_bump};
use crate::error_code::{self, ErrorCodeExt};
use crate::git::{find_last_tag_name, get_commits_since_last_tag, get_repo_root, open_repo};

/// What comparing the public API against the baseline concluded.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "verdict", content = "detail")]
pub enum ApiVerdict {
    /// No deny-level violations: the API is backwards compatible.
    Compatible,
    /// The analyser found breaking changes.
    Breaking,
    /// The analyser ran but could not reach a conclusion.
    Inconclusive(String),
    /// No analyser applies to this package, or it is not installed.
    NotChecked(String),
}

#[derive(Serialize)]
struct PackageReport {
    package: String,
    baseline: Option<String>,
    commit_bump: String,
    #[serde(flatten)]
    api: ApiVerdict,
    disagrees: bool,
}

#[derive(Serialize)]
struct Report {
    packages: Vec<PackageReport>,
    disagreements: usize,
}

/// `cargo semver-checks` exit codes, from its documented contract.
const EXIT_VIOLATIONS_FOUND: i32 = 100;
const EXIT_COULD_NOT_COMPLETE: i32 = 101;

const NOT_INSTALLED: &str =
    "cargo-semver-checks is not installed: cargo install cargo-semver-checks";

/// `cargo` answers an unknown subcommand with the same exit code it uses
/// for a check that could not complete, so the verdict has to come from
/// the message rather than the status.
fn is_missing_subcommand(stderr: &[u8]) -> bool {
    String::from_utf8_lossy(stderr).contains("no such command")
}

fn is_rust_package(path: &Path) -> bool {
    path.join("Cargo.toml").is_file()
}

fn check_rust_api(package_name: &str, root: &Path, path: &PathBuf, baseline: &str) -> ApiVerdict {
    if !is_rust_package(&root.join(path)) {
        return ApiVerdict::NotChecked("no analyser for this package type".to_string());
    }

    let output = Command::new("cargo")
        .current_dir(root)
        .args(["semver-checks", "--package", package_name, "--baseline-rev"])
        .arg(baseline)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            return ApiVerdict::NotChecked(NOT_INSTALLED.to_string());
        }
    };

    if is_missing_subcommand(&output.stderr) {
        return ApiVerdict::NotChecked(NOT_INSTALLED.to_string());
    }

    match output.status.code() {
        Some(0) => ApiVerdict::Compatible,
        Some(EXIT_VIOLATIONS_FOUND) => ApiVerdict::Breaking,
        Some(EXIT_COULD_NOT_COMPLETE) => {
            ApiVerdict::Inconclusive(last_meaningful_line(&output.stderr))
        }
        _ => ApiVerdict::NotChecked(last_meaningful_line(&output.stderr)),
    }
}

fn last_meaningful_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .rfind(|l| !l.trim().is_empty())
        .unwrap_or("(no output)")
        .trim()
        .to_string()
}

/// A verdict of `Breaking` only disagrees with the commits when those
/// commits did not already ask for a major.
pub fn disagrees(commit_bump: BumpType, api: &ApiVerdict) -> bool {
    *api == ApiVerdict::Breaking && commit_bump != BumpType::Major
}

pub fn run(config_path: Option<&Path>, package: Option<&str>, json: bool) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        Err(anyhow::anyhow!(
            "No packages configured. Run `ferrflow init` to create a config."
        ))
        .error_code(error_code::QUERY_NO_PACKAGES)?;
    }

    let selected: Vec<&PackageConfig> = match package {
        Some(name) => vec![
            config
                .packages
                .iter()
                .find(|p| p.name == name)
                .ok_or_else(|| anyhow::anyhow!("package '{name}' not found"))
                .error_code(error_code::QUERY_PACKAGE_NOT_FOUND)?,
        ],
        None => config.packages.iter().collect(),
    };

    let is_monorepo = config.is_monorepo();
    let strategy = config.workspace.orphaned_tag_strategy;
    let mut packages = Vec::new();

    for pkg in selected {
        let prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
        let baseline = find_last_tag_name(&repo, &prefix, strategy)?;

        let commit_bump = match &baseline {
            Some(_) => get_commits_since_last_tag(
                &repo,
                &prefix,
                strategy,
                &config.workspace.effective_commit_skip_markers(),
                None,
            )
            .unwrap_or_default()
            .iter()
            .map(|c| determine_bump(&c.message, &config.workspace.commit_formats))
            .max()
            .unwrap_or(BumpType::None),
            None => BumpType::None,
        };

        let api = match &baseline {
            Some(tag) => check_rust_api(&pkg.name, &root, &PathBuf::from(&pkg.path), tag),
            None => ApiVerdict::NotChecked("no baseline tag to compare against".to_string()),
        };

        packages.push(PackageReport {
            package: pkg.name.clone(),
            baseline,
            commit_bump: format!("{commit_bump:?}").to_lowercase(),
            disagrees: disagrees(commit_bump, &api),
            api,
        });
    }

    let disagreements = packages.iter().filter(|p| p.disagrees).count();
    let report = Report {
        packages,
        disagreements,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }

    if disagreements > 0 {
        anyhow::bail!(
            "{disagreements} package(s) have breaking API changes the commits did not ask for"
        );
    }
    Ok(())
}

fn print_text(report: &Report) {
    println!("{}", "FerrFlow — API compatibility".bold());
    println!();

    for p in &report.packages {
        let verdict = match &p.api {
            ApiVerdict::Compatible => "compatible".green().to_string(),
            ApiVerdict::Breaking => "breaking".red().to_string(),
            ApiVerdict::Inconclusive(why) => format!("{} ({why})", "inconclusive".yellow()),
            ApiVerdict::NotChecked(why) => format!("{} ({why})", "not checked".dimmed()),
        };
        println!("  {}", p.package.bold());
        println!(
            "    baseline     {}",
            p.baseline.as_deref().unwrap_or("none").dimmed()
        );
        println!("    commits say  {}", p.commit_bump);
        println!("    api says     {verdict}");
        if p.disagrees {
            println!(
                "    {} the API broke but the commits ask for {}, not major",
                "✗".red(),
                p.commit_bump
            );
        }
    }

    println!();
    if report.disagreements == 0 {
        println!("  {} no disagreement between commits and API", "✓".green());
    } else {
        println!(
            "  {} {} package(s) where the commits understate the change",
            "✗".red(),
            report.disagreements
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaking_only_disagrees_when_the_commits_asked_for_less_than_major() {
        assert!(disagrees(BumpType::Patch, &ApiVerdict::Breaking));
        assert!(disagrees(BumpType::Minor, &ApiVerdict::Breaking));
        assert!(disagrees(BumpType::None, &ApiVerdict::Breaking));
        assert!(!disagrees(BumpType::Major, &ApiVerdict::Breaking));
    }

    #[test]
    fn a_verdict_other_than_breaking_never_disagrees() {
        for verdict in [
            ApiVerdict::Compatible,
            ApiVerdict::Inconclusive("build failed".into()),
            ApiVerdict::NotChecked("not installed".into()),
        ] {
            assert!(
                !disagrees(BumpType::Patch, &verdict),
                "{verdict:?} should not disagree"
            );
        }
    }

    #[test]
    fn an_absent_analyser_is_not_checked_rather_than_compatible() {
        let dir = tempfile::tempdir().unwrap();
        let verdict = check_rust_api("x", dir.path(), &PathBuf::from("."), "v1.0.0");
        assert!(
            matches!(verdict, ApiVerdict::NotChecked(_)),
            "a package with no Cargo.toml must not read as compatible, got {verdict:?}"
        );
    }

    #[test]
    fn an_unknown_cargo_subcommand_reads_as_not_installed_not_inconclusive() {
        assert!(is_missing_subcommand(
            b"error: no such command: `semver-checks`"
        ));
        assert!(!is_missing_subcommand(
            b"error: failed to build rustdoc JSON"
        ));
    }

    #[test]
    fn the_last_meaningful_line_skips_trailing_blanks() {
        assert_eq!(
            last_meaningful_line(b"first\nerror: boom\n\n  \n"),
            "error: boom"
        );
        assert_eq!(last_meaningful_line(b""), "(no output)");
    }
}
