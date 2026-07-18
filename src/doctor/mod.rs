use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::git::{get_repo_root, open_repo};

mod checks;
mod report;

use report::Report;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum DoctorFormat {
    Human,
    Json,
}

pub fn run(config_path: Option<&Path>, format: DoctorFormat, online: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = open_repo(&cwd).ok();
    let root = repo
        .as_ref()
        .and_then(|r| get_repo_root(r).ok())
        .unwrap_or_else(|| cwd.clone());

    let discovered = Config::discovered_config_paths(&root);
    let (config, config_error) = match Config::load(&root, config_path) {
        Ok(config) => (Some(config), None),
        Err(err) => (None, Some(format!("{err:#}"))),
    };

    let report = Report::build(vec![
        checks::repo_section(repo.as_ref(), config.as_ref(), &root),
        checks::config_section(config.as_ref(), config_error.as_deref(), &discovered, &root),
        checks::versioning_section(config.as_ref(), &root),
        checks::forge_section(repo.as_ref(), config.as_ref(), online),
        checks::ci_section(&root),
    ]);

    match format {
        DoctorFormat::Json => println!("{}", report.to_json()?),
        DoctorFormat::Human => report.print_human(),
    }

    if report.exit_code != 0 {
        std::process::exit(report.exit_code);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
