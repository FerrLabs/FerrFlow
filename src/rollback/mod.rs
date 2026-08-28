mod plan;

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use crate::config::Config;
use crate::error_code::{self, ErrorCodeExt};
use crate::git::{Repository, get_repo_root, open_repo};
use crate::monorepo::run::checkpoint::Checkpoint;

pub use plan::{RollbackPlan, Step};

pub fn run(packages: &[String], yes: bool, config_path: Option<&Path>) -> Result<()> {
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    let Some(checkpoint) = Checkpoint::load(&root)? else {
        tracing::info!(
            "{}",
            "No release checkpoint found: there is no failed run to roll back.".dimmed()
        );
        return Ok(());
    };

    let has_manifest = crate::manifest::manifest_path(&config, &root).is_some();
    let plan = plan::plan(&checkpoint, packages, has_manifest, |tag| {
        tag_owner(&config, tag)
    });

    print_plan(&plan);

    if !plan.blocked.is_empty() && plan.is_empty() {
        return Err(anyhow::anyhow!(
            "nothing can be rolled back: every package in this run published to a registry that cannot be unpublished"
        ))
        .error_code(error_code::ROLLBACK_BLOCKED);
    }

    if plan.is_empty() {
        return Ok(());
    }

    if !yes {
        tracing::info!("");
        tracing::info!(
            "{}",
            "This was a dry run. Re-run with --yes to apply it.".dimmed()
        );
        return Ok(());
    }

    apply(&plan, &repo, &config, config_path)?;
    Checkpoint::delete(&root)?;
    tracing::info!("");
    tracing::info!("{}", "✓ Rolled back".green());
    Ok(())
}

/// Which package a tag belongs to, by matching the tag against each package's
/// rendered prefix. Returns None when no package claims it, which keeps a
/// stray tag out of a narrowed rollback rather than guessing.
fn tag_owner(config: &Config, tag: &str) -> Option<String> {
    let is_monorepo = config.is_monorepo();
    config
        .packages
        .iter()
        .filter_map(|pkg| {
            let prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
            (!prefix.is_empty() && tag.starts_with(&prefix)).then(|| pkg.name.clone())
        })
        .next()
}

fn print_plan(plan: &RollbackPlan) {
    for blocked in &plan.blocked {
        tracing::warn!(
            "{}",
            format!("  ! {} {}", blocked.package.bold(), blocked.reason).yellow()
        );
    }
    if !plan.blocked.is_empty() {
        tracing::info!("");
    }
    for step in &plan.steps {
        tracing::info!("  {}", describe(step));
    }
    if plan.steps.is_empty() && plan.blocked.is_empty() {
        tracing::info!("{}", "Nothing to roll back.".dimmed());
    }
}

fn describe(step: &Step) -> String {
    match step {
        Step::DeleteTag { name, sha } => {
            format!("delete tag {} ({})", name.cyan(), &sha[..sha.len().min(7)])
        }
        Step::DeleteRelease { tag, id } => {
            format!("delete release {} (#{id})", tag.cyan())
        }
        Step::RevertCommit { sha } => {
            format!("revert commit {}", &sha[..sha.len().min(7)])
        }
        Step::RestoreManifest => "restore the manifest".to_string(),
    }
}

fn apply(
    plan: &RollbackPlan,
    repo: &Repository,
    config: &Config,
    config_path: Option<&Path>,
) -> Result<()> {
    let remote = &config.workspace.remote;
    let forge = crate::monorepo::preview::build_forge_instance(repo, config);

    for step in &plan.steps {
        match step {
            Step::DeleteRelease { tag, id } => match forge.as_ref() {
                Some(f) => f.delete_release(*id).with_context(|| {
                    format!("Failed to delete the release for {tag}; delete it by hand and rerun")
                })?,
                None => tracing::warn!(
                    "{}",
                    format!("  skipped release for {tag}: no forge token available").yellow()
                ),
            },
            Step::DeleteTag { name, sha } => {
                crate::git::delete_tag_if_unchanged(repo, remote, name, sha)?;
            }
            Step::RevertCommit { sha } => {
                crate::git::revert_commit(repo, sha)?;
            }
            Step::RestoreManifest => {
                crate::manifest::sync_cwd(config_path)?;
            }
        }
    }
    Ok(())
}
