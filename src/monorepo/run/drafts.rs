use anyhow::Result;
use colored::Colorize;
use git2::Repository;
use std::path::Path;

use crate::config::Config;
use crate::formats::read_version;

use super::super::preview::build_forge_instance;

/// When the run didn't bump anything (no new commits since the last
/// release) but a previous run left draft releases dangling on the
/// forge, publish them now so the release cycle terminates.
///
/// Skipped on draft / dry-run mode (we don't want to publish anything
/// the user explicitly opted into keeping as a draft). Per-package
/// errors are logged but never abort the run.
pub(super) fn publish_pending_drafts(
    repo: &Repository,
    config: &Config,
    root: &Path,
    verbose: bool,
    shared_outputs: &mut Vec<String>,
) -> Result<()> {
    let Some(forge_instance) = build_forge_instance(repo, config) else {
        return Ok(());
    };
    for pkg in &config.packages {
        let Some(vf) = pkg.versioned_files.first() else {
            continue;
        };
        let Ok(version) = read_version(vf, root) else {
            continue;
        };
        let tag = pkg.tag_for_version(&config.workspace, config.is_monorepo(), &version);
        match forge_instance.find_draft_release(&tag) {
            Ok(Some(release_id)) => match forge_instance.publish_release(release_id) {
                Ok(()) => {
                    shared_outputs.push(format!(
                        "✓ Published draft {} {}",
                        forge_instance.release_noun(),
                        tag.cyan()
                    ));
                }
                Err(err) => eprintln!(
                    "{}",
                    format!("  Warning: failed to publish draft for {tag}: {err}").yellow()
                ),
            },
            Ok(None) => {}
            Err(err) => {
                if verbose {
                    eprintln!(
                        "{}",
                        format!("  Warning: failed to check draft release {tag}: {err}").yellow()
                    );
                }
            }
        }
    }
    Ok(())
}
