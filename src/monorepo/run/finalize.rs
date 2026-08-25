use std::path::Path;

use crate::changelog::{ChangelogRender, GitLog, build_section_with};
use crate::config::{Config, PackageConfig};
use crate::formats::read_version;
use crate::git::{Repository, find_last_tag_name, get_commits_since_last_tag};

use super::summary::PlannedTag;

const RELEASE_SUBJECT_PREFIX: &str = "chore(release):";

pub(super) fn merged_release_tags(
    repo: &Repository,
    config: &Config,
    root: &Path,
    all_tags: &[String],
    forge_base: Option<String>,
) -> Vec<PlannedTag> {
    let is_monorepo = config.is_monorepo();
    let mut pending = Vec::new();

    for pkg in &config.packages {
        let Some(version) = file_version(pkg, root) else {
            continue;
        };
        let tag = pkg.tag_for_version(&config.workspace, is_monorepo, &version);
        if all_tags.iter().any(|t| t == &tag) {
            continue;
        }

        let prefix = pkg.tag_prefix(&config.workspace, is_monorepo);
        let strategy = config.workspace.orphaned_tag_strategy;
        let last_tag = find_last_tag_name(repo, &prefix, strategy).ok().flatten();
        let skip_markers = config.workspace.effective_commit_skip_markers();
        let Ok(commits) = get_commits_since_last_tag(repo, &prefix, strategy, &skip_markers, None)
        else {
            continue;
        };
        if !carries_release_commit(&commits) {
            continue;
        }

        let render = ChangelogRender {
            formats: Some(&config.workspace.commit_formats),
            config: config.workspace.changelog.as_ref(),
            forge_base: forge_base.clone(),
            last_tag: last_tag.clone(),
            new_tag: Some(tag.clone()),
        };

        let is_prerelease = version_is_prerelease(&version);
        pending.push(PlannedTag {
            message: format!("Release {tag}"),
            body: build_section_with(&version, &commits, &render),
            tag,
            package: pkg.name.clone(),
            version,
            commit_count: commits.len() as i32,
            is_prerelease,
        });
    }

    pending
}

fn version_is_prerelease(version: &str) -> bool {
    semver::Version::parse(version.trim_start_matches('v'))
        .map(|v| !v.pre.is_empty())
        .unwrap_or(false)
}

fn file_version(pkg: &PackageConfig, root: &Path) -> Option<String> {
    let vf = pkg.versioned_files.first()?;
    read_version(vf, root).ok()
}

fn carries_release_commit(commits: &[GitLog]) -> bool {
    commits.iter().any(|c| {
        c.message
            .lines()
            .next()
            .is_some_and(|subject| subject.trim_start().starts_with(RELEASE_SUBJECT_PREFIX))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(message: &str) -> GitLog {
        GitLog {
            hash: String::new(),
            message: message.to_string(),
        }
    }

    #[test]
    fn a_semver_prerelease_version_is_flagged_as_one() {
        assert!(version_is_prerelease("1.1.0-beta.1"));
        assert!(version_is_prerelease("v2.0.0-rc.3"));
        assert!(!version_is_prerelease("1.1.0"));
        assert!(!version_is_prerelease("2026.8.25"));
    }

    #[test]
    fn a_release_commit_in_the_window_is_recognised() {
        assert!(carries_release_commit(&[
            log("feat: something"),
            log("chore(release): app v1.1.0"),
        ]));
    }

    #[test]
    fn a_merged_release_commit_with_a_body_is_recognised() {
        assert!(carries_release_commit(&[log(
            "chore(release): app v1.1.0\n\n- app 1.1.0 (3 commits)"
        )]));
    }

    #[test]
    fn ordinary_commits_alone_do_not_trigger_finalisation() {
        assert!(!carries_release_commit(&[
            log("feat: something"),
            log("fix: something else"),
            log("chore: unrelated housekeeping"),
        ]));
    }

    #[test]
    fn a_commit_merely_mentioning_a_release_does_not_count() {
        assert!(!carries_release_commit(&[log(
            "fix: do not break chore(release): parsing"
        )]));
    }
}
