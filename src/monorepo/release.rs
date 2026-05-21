use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::Config;
use crate::git::{get_repo_root, open_repo};

use super::run::run_release_logic;

const MAX_RELEASE_REGENERATE_ATTEMPTS: usize = 3;

pub fn release(
    config_path: Option<&Path>,
    dry_run: bool,
    verbose: bool,
    force: bool,
    force_version: Option<&str>,
    channel: Option<&str>,
    draft: bool,
) -> Result<()> {
    crate::bot_token::ensure_bot_token()?;
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;
    drop(repo);

    if dry_run {
        println!("{}", "FerrFlow — Release (dry run)".bold().blue());
    } else {
        println!("{}", "FerrFlow — Release".bold().green());
    }
    println!();

    let single_shot = dry_run
        || matches!(
            config.workspace.release_commit_mode,
            crate::config::ReleaseCommitMode::Pr | crate::config::ReleaseCommitMode::None
        );

    if single_shot {
        return run_release_logic(
            &root,
            &config,
            dry_run,
            verbose,
            false,
            force,
            force_version,
            channel,
            draft,
        );
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
            force,
            force_version,
            channel,
            draft,
        );

        match result {
            Ok(()) => return Ok(()),
            Err(e)
                if attempt < MAX_RELEASE_REGENERATE_ATTEMPTS
                    && crate::git::is_push_rejected_error(&e) =>
            {
                eprintln!();
                eprintln!(
                    "{}",
                    format!(
                        "Release attempt {attempt}/{MAX_RELEASE_REGENERATE_ATTEMPTS} \
                         pushed onto a stale '{}': {e}",
                        config.workspace.branch,
                    )
                    .yellow()
                );
                eprintln!(
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
