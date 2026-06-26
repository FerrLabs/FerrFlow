use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::Config;
use crate::git::{get_repo_root, open_repo};
use crate::timing::Timing;

use super::run::run_release_logic;

const MAX_RELEASE_REGENERATE_ATTEMPTS: usize = 3;

#[allow(clippy::too_many_arguments)]
pub fn release(
    config_path: Option<&Path>,
    dry_run: bool,
    verbose: bool,
    json: bool,
    force: bool,
    force_version: Option<&str>,
    channel: Option<&str>,
    draft: bool,
    force_unlock: bool,
    timing: &mut Timing,
) -> Result<()> {
    crate::bot_token::ensure_bot_token()?;
    let repo = timing.stage("open_repo", || open_repo(&std::env::current_dir()?))?;
    let root = get_repo_root(&repo)?;
    let config = timing.stage("load config", || Config::load(&root, config_path))?;
    drop(repo);

    if !dry_run {
        crate::manifest::validate_in_sync(&config, &root)?;
    }

    if !json {
        if dry_run {
            println!("{}", "FerrFlow — Release (dry run)".bold().blue());
        } else {
            println!("{}", "FerrFlow — Release".bold().green());
        }
        println!();
    }

    let single_shot = dry_run
        || matches!(
            config.workspace.release_commit_mode,
            crate::config::ReleaseCommitMode::Pr | crate::config::ReleaseCommitMode::None
        );

    if single_shot {
        let out = run_release_logic(
            &root,
            &config,
            dry_run,
            verbose,
            false,
            json,
            force,
            force_version,
            channel,
            draft,
            force_unlock,
            timing,
        )?;
        if let Some(out) = out {
            out.print();
        }
        return Ok(());
    }

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=MAX_RELEASE_REGENERATE_ATTEMPTS {
        let pre_attempt_tags: std::collections::HashSet<String> = {
            let repo = open_repo(&root)?;
            crate::git::collect_all_tags(&repo).into_iter().collect()
        };

        let result = run_release_logic(
            &root,
            &config,
            dry_run,
            verbose,
            false,
            json,
            force,
            force_version,
            channel,
            draft,
            force_unlock,
            timing,
        );

        match result {
            Ok(out) => {
                if let Some(out) = out {
                    out.print();
                }
                return Ok(());
            }
            Err(e)
                if attempt < MAX_RELEASE_REGENERATE_ATTEMPTS
                    && crate::git::is_push_rejected_error(&e) =>
            {
                eprintln!();
                tracing::warn!(
                    "{}",
                    format!(
                        "Release attempt {attempt}/{MAX_RELEASE_REGENERATE_ATTEMPTS} \
                         pushed onto a stale '{}': {e}",
                        config.workspace.branch,
                    )
                    .yellow()
                );
                tracing::warn!(
                    "{}",
                    "Resetting working tree to remote tip and regenerating the release commit \
                     against the latest history…"
                        .dimmed()
                );

                cleanup_failed_release_attempt(&root, &config, &pre_attempt_tags)?;
                last_err = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!(
            "release failed after {MAX_RELEASE_REGENERATE_ATTEMPTS} regenerate attempts"
        )
    }))
}

fn cleanup_failed_release_attempt(
    root: &Path,
    config: &Config,
    pre_attempt_tags: &std::collections::HashSet<String>,
) -> Result<()> {
    let repo = open_repo(root)?;
    let after: std::collections::HashSet<String> =
        crate::git::collect_all_tags(&repo).into_iter().collect();
    if let Some(workdir) = repo.workdir() {
        for tag in after.difference(pre_attempt_tags) {
            let _ = std::process::Command::new("git")
                .current_dir(workdir)
                .args(["tag", "-d", tag])
                .output();
        }
    }

    let target_branch = crate::git::resolve_current_branch(&repo, &config.workspace.branch);
    crate::git::reset_branch_to_remote(&repo, &config.workspace.remote, &target_branch)?;
    Ok(())
}
