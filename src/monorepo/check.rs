use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use crate::cache::{self, CachedRun};
use crate::config::Config;
use crate::formats::{get_handler, read_version};
use crate::git::{get_repo_root, open_repo};
use crate::timing::Timing;

use super::preview::post_preview_comment;
use super::run::run_release_logic;

#[allow(clippy::too_many_arguments)]
pub fn check(
    config_path: Option<&Path>,
    verbose: bool,
    json: bool,
    channel: Option<&str>,
    comment: bool,
    timing: &mut Timing,
) -> Result<()> {
    let repo = timing.stage("open_repo", || open_repo(&std::env::current_dir()?))?;
    let root = get_repo_root(&repo)?;
    let config = timing.stage("load config", || Config::load(&root, config_path))?;

    verify_versioned_files(&config, &root)?;

    if !json {
        tracing::info!("{}", "FerrFlow — Check (dry run)".bold().blue());
        tracing::info!("");
    }

    let variant = if json { "json" } else { "text" };
    let cache_eligible = !verbose && channel.is_none() && !config.has_release_side_effects();
    let cache_key = cache_eligible
        .then(|| cache::compute_key(&repo, &root, &config, config_path, variant))
        .flatten();
    let cache_dir = cache::cache_dir(&repo);

    if let Some(key) = &cache_key
        && let Some(hit) = timing.stage("cache lookup", || cache::read(&cache_dir, key))
    {
        print_cached(&hit);
        if comment {
            post_preview_comment(&repo, &config, &root);
        }
        return Ok(());
    }

    let out = run_release_logic(
        &root,
        &config,
        true,
        verbose,
        json,
        false,
        false,
        &[],
        &[],
        channel,
        false,
        false,
        false,
        timing,
    )?;

    crate::git::write_commit_graph_if_absent(&repo);

    if let Some(key) = &cache_key {
        cache::write(
            &cache_dir,
            key,
            &CachedRun {
                json: out.json.clone(),
                text_lines: out.text_lines.clone(),
            },
        );
    }

    out.print();

    if comment {
        post_preview_comment(&repo, &config, &root);
    }

    Ok(())
}

fn verify_versioned_files(config: &Config, root: &Path) -> Result<()> {
    config
        .packages
        .iter()
        .flat_map(|pkg| pkg.versioned_files.iter().map(move |vf| (pkg, vf)))
        .filter(|(_, vf)| get_handler(&vf.format).modifies_file())
        .try_for_each(|(pkg, vf)| {
            read_version(vf, root)
                .map(drop)
                .with_context(|| format!("package {}: cannot read {}", pkg.name, vf.path))
        })
}

fn print_cached(hit: &CachedRun) {
    if let Some(json) = &hit.json {
        println!("{json}");
        return;
    }
    for line in &hit.text_lines {
        tracing::info!("{line}");
    }
}

/// Run the same computation `check --json` does and hand back the JSON
/// instead of printing it. Used by the interactive planner so it reads
/// the published contract rather than the internal plan types.
pub(super) fn plan_json(config_path: Option<&Path>, channel: Option<&str>) -> Result<String> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;
    plan_json_at(&root, &config, channel)
}

pub(super) fn plan_json_at(root: &Path, config: &Config, channel: Option<&str>) -> Result<String> {
    let out = run_release_logic(
        root,
        config,
        true,
        false,
        true,
        false,
        false,
        &[],
        &[],
        channel,
        false,
        false,
        false,
        &mut Timing::new(false),
    )?;

    Ok(out.json.unwrap_or_else(|| r#"{"packages":[]}"#.to_string()))
}
