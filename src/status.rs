use crate::config::Config;
use crate::conventional_commits::{BumpType, determine_bump};
use crate::formats::read_version;
use crate::git::{find_last_tag_name, get_commits_since_last_tag, get_repo_root, open_repo};
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Serialize)]
struct PackageStatus {
    name: String,
    version: String,
    last_tag: Option<String>,
    has_changes: bool,
}

pub fn run(config_path: Option<&std::path::Path>, output: &OutputFormat) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if config.packages.is_empty() {
        println!(
            "{}",
            "No packages configured. Run `ferrflow init` to create a ferrflow config.".yellow()
        );
        return Ok(());
    }

    let mut statuses: Vec<PackageStatus> = Vec::new();

    for pkg in &config.packages {
        let tag_search_prefix = pkg.tag_prefix(&config.workspace, config.is_monorepo());
        let last_tag = find_last_tag_name(&repo, &tag_search_prefix)?;

        let version = if let Some(vf) = pkg.versioned_files.first() {
            read_version(vf, &root).unwrap_or_else(|_| "unknown".to_string())
        } else {
            "unknown".to_string()
        };

        let commits = get_commits_since_last_tag(&repo, &tag_search_prefix)?;
        let has_changes = commits
            .iter()
            .map(|c| determine_bump(&c.message))
            .any(|b| b != BumpType::None);

        statuses.push(PackageStatus {
            name: pkg.name.clone(),
            version,
            last_tag,
            has_changes,
        });
    }

    match output {
        OutputFormat::Text => print_text(&statuses),
        OutputFormat::Json => print_json(&statuses)?,
    }

    Ok(())
}

fn print_text(statuses: &[PackageStatus]) {
    for s in statuses {
        let dot = if s.has_changes {
            "●".green().bold()
        } else {
            "○".dimmed()
        };
        let tag_info = match &s.last_tag {
            Some(tag) => format!("(tag: {})", tag),
            None => "(no tag)".to_string(),
        };
        println!(
            "{} {:<20} v{}   {}",
            dot,
            s.name,
            s.version,
            tag_info.dimmed()
        );
    }
}

fn print_json(statuses: &[PackageStatus]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&statuses)?);
    Ok(())
}
