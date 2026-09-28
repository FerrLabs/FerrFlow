use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::git::Repository;

use super::super::types::{CheckResult, RunOutput};
use super::context::RunFlags;
use super::publish::short_head;
use super::release_json::{GitInfo, ReleaseJson};
use super::state::ReleaseState;
use super::summary::{self, collect_outputs};

pub(super) fn empty_config(flags: RunFlags) -> Result<RunOutput> {
    if flags.release_json {
        return Ok(RunOutput {
            json: Some(
                ReleaseJson {
                    dry_run: flags.dry_run,
                    ..Default::default()
                }
                .to_json()?,
            ),
            text_lines: Vec::new(),
        });
    }
    if flags.json {
        return Ok(RunOutput {
            json: Some(serde_json::to_string(&CheckResult { packages: vec![] })?),
            text_lines: Vec::new(),
        });
    }
    Ok(RunOutput {
        json: None,
        text_lines: vec![
            "No packages configured. Run `ferrflow init` to create a ferrflow config."
                .yellow()
                .to_string(),
        ],
    })
}

pub(super) fn check(state: ReleaseState) -> Result<RunOutput> {
    Ok(RunOutput {
        json: Some(serde_json::to_string(&CheckResult {
            packages: state.json_packages,
        })?),
        text_lines: Vec::new(),
    })
}

pub(super) fn release_json(
    repo: &Repository,
    state: ReleaseState,
    target_branch: &str,
    dry_run: bool,
) -> Result<RunOutput> {
    let mut released = state.released;
    for (tag, result) in &state.forge_results {
        if let Some(rp) = released.iter_mut().find(|rp| &rp.tag == tag) {
            rp.forge_release_url = result.url.clone();
            rp.forge_release_id = result.id;
        }
    }
    let tags_pushed = if dry_run {
        Vec::new()
    } else {
        state.tags_to_create.iter().map(|t| t.tag.clone()).collect()
    };
    let payload = ReleaseJson {
        released,
        skipped: state.skipped,
        git: GitInfo {
            commit: short_head(repo),
            tags_pushed,
            branch: target_branch.to_string(),
        },
        dry_run,
    };
    Ok(RunOutput {
        json: Some(payload.to_json()?),
        text_lines: Vec::new(),
    })
}

pub(super) fn text(mut state: ReleaseState, config: &Config, flags: RunFlags) -> RunOutput {
    // Only on a dry run. A per-push release skips untouched packages every
    // time by design, so the hint would be noise there; `check` is where a
    // plan that looks complete and is not costs someone an afternoon.
    if flags.dry_run
        && !flags.quiet()
        && let Some(hint) = summary::untouched_hint(
            state.untouched_skipped,
            config.workspace.recover_missed_releases,
        )
    {
        state.shared_outputs.push(hint);
    }

    let mut text_lines = collect_outputs(&state.pkg_outputs, &state.shared_outputs);
    if !state.any_bumped && !flags.verbose {
        text_lines.push("Nothing to release.".dimmed().to_string());
    }

    RunOutput {
        json: None,
        text_lines,
    }
}

#[cfg(test)]
mod tests;
