use crate::git::Repository;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::Config;
use crate::formats::read_version;

use super::super::preview::build_forge_instance;

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
                Err(err) => tracing::warn!(
                    "{}",
                    format!("  Warning: failed to publish draft for {tag}: {err}").yellow()
                ),
            },
            Ok(None) => {}
            Err(err) => {
                if verbose {
                    tracing::warn!(
                        "{}",
                        format!("  Warning: failed to check draft release {tag}: {err}").yellow()
                    );
                }
            }
        }
    }
    Ok(())
}
