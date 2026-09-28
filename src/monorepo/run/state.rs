use colored::Colorize;
use std::collections::HashMap;

use crate::conventional_commits::BumpType;
use crate::forge::ReleaseResult;
use crate::hooks::HookContext;

use super::super::types::CheckPackage;
use super::cascade::CascadeSink;
use super::context::RunFlags;
use super::plan::SkipReason;
use super::release_json::{ReleasedPackage, SkippedPackage};
use super::summary::PlannedTag;

#[derive(Default)]
pub(super) struct ReleaseState {
    pub any_bumped: bool,
    pub json_packages: Vec<CheckPackage>,
    pub released: Vec<ReleasedPackage>,
    pub skipped: Vec<SkippedPackage>,
    pub files_to_commit: Vec<String>,
    pub files_per_package: HashMap<String, Vec<String>>,
    pub tags_to_create: Vec<PlannedTag>,
    pub hook_contexts: Vec<(HookContext, usize)>,
    pub bumped: HashMap<String, BumpType>,
    pub bumped_versions: HashMap<String, String>,
    pub pkg_outputs: Vec<(String, Vec<String>)>,
    pub shared_outputs: Vec<String>,
    pub untouched_skipped: usize,
    pub forge_results: Vec<(String, ReleaseResult)>,
}

impl ReleaseState {
    pub fn record_finalize_tags(&mut self, finalize_tags: Vec<PlannedTag>) {
        for tag in &finalize_tags {
            self.pkg_outputs.push((
                tag.package.clone(),
                vec![format!(
                    "{} {}  {}  ({})",
                    "●".green().bold(),
                    tag.package.bold(),
                    tag.version.green().bold(),
                    "release merged, tagging now".cyan()
                )],
            ));
            self.released.push(ReleasedPackage {
                package: tag.package.clone(),
                previous_version: String::new(),
                new_version: tag.version.clone(),
                bump_type: "finalize".to_string(),
                tag: tag.tag.clone(),
                commit_count: tag.commit_count as usize,
                prerelease: tag.is_prerelease,
                version_source: None,
                forge_release_url: None,
                forge_release_id: None,
            });
        }
        self.tags_to_create = finalize_tags;
        self.any_bumped = true;
    }

    pub fn record_skip(&mut self, package: &str, reason: SkipReason, flags: RunFlags) {
        if matches!(reason, SkipReason::NotTouched) {
            self.untouched_skipped += 1;
        }
        if !flags.quiet()
            && let Some(line) = skip_output(&reason, package)
        {
            self.shared_outputs.push(line);
        }
        if flags.release_json {
            self.skipped.push(SkippedPackage {
                package: package.to_string(),
                reason: reason.json_label().to_string(),
            });
        }
    }

    pub fn push_package_line(&mut self, package: &str, line: String) {
        if let Some((_, lines)) = self
            .pkg_outputs
            .iter_mut()
            .rev()
            .find(|(n, _)| n == package)
        {
            lines.push(line);
        }
    }

    pub fn track_file(&mut self, package: &str, path: &str) {
        self.files_to_commit.push(path.to_string());
        self.files_per_package
            .entry(package.to_string())
            .or_default()
            .push(path.to_string());
    }

    pub fn cascade_sink(&mut self) -> CascadeSink<'_> {
        CascadeSink {
            any_bumped: &mut self.any_bumped,
            json_packages: &mut self.json_packages,
            released: &mut self.released,
            files_to_commit: &mut self.files_to_commit,
            files_per_package: &mut self.files_per_package,
            tags_to_create: &mut self.tags_to_create,
            pkg_outputs: &mut self.pkg_outputs,
            bumped: &mut self.bumped,
            bumped_versions: &mut self.bumped_versions,
        }
    }
}

fn skip_output(reason: &SkipReason, package: &str) -> Option<String> {
    match reason {
        SkipReason::Excluded => {
            tracing::info!(
                "{} {} — excluded by --exclude",
                "○".dimmed(),
                package.dimmed()
            );
            None
        }
        SkipReason::NotTouched => {
            tracing::debug!(
                "{} {} — not touched, skipping",
                "○".dimmed(),
                package.dimmed()
            );
            None
        }
        SkipReason::NoNewCommits => {
            tracing::debug!("{} {} — no new commits", "○".dimmed(), package.dimmed());
            None
        }
        SkipReason::NoReleasableCommits => Some(format!(
            "{} {} — no releasable commits",
            "○".dimmed(),
            package.dimmed()
        )),
        SkipReason::VersionUnchanged { version } => Some(
            format!(
                "{} {} has releasable commits but stays at {version}, so nothing was published",
                "!".yellow().bold(),
                package.bold()
            )
            .yellow()
            .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests;
