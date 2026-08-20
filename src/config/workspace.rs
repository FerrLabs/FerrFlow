use serde::{Deserialize, Serialize};

use super::commit_formats::CommitFormats;
use super::package::{FloatingTagLevel, VersioningStrategy};
use super::types::{
    BranchChannelConfig, ForgeKind, HooksConfig, OrphanedTagStrategy, RegistryConfig,
    ReleaseCommitBody, ReleaseCommitMode, ReleaseCommitScope, VersionSourcePolicy,
};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct WorkspaceConfig {
    #[serde(default = "default_remote")]
    pub remote: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_telemetry", alias = "telemetry")]
    pub anonymous_telemetry: bool,
    #[serde(default)]
    pub versioning: Option<VersioningStrategy>,
    #[serde(alias = "tagTemplate")]
    pub tag_template: Option<String>,
    #[serde(alias = "versionTemplate")]
    pub version_template: Option<String>,
    #[serde(default, alias = "recoverMissedReleases")]
    pub recover_missed_releases: bool,
    #[serde(default, alias = "releaseCommitMode")]
    pub release_commit_mode: ReleaseCommitMode,
    #[serde(default, alias = "releaseCommitScope")]
    pub release_commit_scope: ReleaseCommitScope,
    #[serde(default, alias = "releaseCommitBody")]
    pub release_commit_body: ReleaseCommitBody,
    #[serde(default, alias = "commitFormats")]
    pub commit_formats: CommitFormats,
    #[serde(default = "default_auto_merge", alias = "autoMergeReleases")]
    pub auto_merge_releases: bool,
    #[serde(default, alias = "skipCi")]
    pub skip_ci: Option<bool>,
    #[serde(default, alias = "commitSkipMarkers")]
    pub commit_skip_markers: Option<Vec<String>>,
    #[serde(default, alias = "floatingTags")]
    pub floating_tags: Vec<FloatingTagLevel>,
    #[serde(default, alias = "latestTag")]
    pub latest_tag: Option<String>,
    #[serde(default, alias = "orphanedTagStrategy")]
    pub orphaned_tag_strategy: OrphanedTagStrategy,
    #[serde(default, alias = "versionSource")]
    pub version_source: VersionSourcePolicy,
    #[serde(default)]
    pub forge: ForgeKind,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
    #[serde(default)]
    pub branches: Option<Vec<BranchChannelConfig>>,
    #[serde(default)]
    pub registries: BTreeMap<String, RegistryConfig>,
    #[serde(default, alias = "deferPublish")]
    pub defer_publish: bool,
    #[serde(default)]
    pub changelog: Option<ChangelogConfig>,
    #[serde(default, alias = "manifestFile")]
    pub manifest_file: Option<String>,
    #[serde(default, alias = "updateLockfiles")]
    pub update_lockfiles: bool,
    #[serde(default, alias = "updateDependents")]
    pub update_dependents: bool,
    #[serde(default)]
    pub linked: Vec<Vec<String>>,
    #[serde(default)]
    pub fixed: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChangelogConfig {
    #[serde(default)]
    pub sections: Option<BTreeMap<String, SectionSetting>>,
    #[serde(default, alias = "groupByScope")]
    pub group_by_scope: bool,
    #[serde(default, alias = "includeCommitLinks")]
    pub include_commit_links: bool,
    #[serde(default, alias = "includeCompareLink")]
    pub include_compare_link: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SectionSetting {
    Label(String),
    Enabled(bool),
}

impl SectionSetting {
    pub fn label(&self) -> Option<&str> {
        match self {
            SectionSetting::Label(label) => Some(label),
            SectionSetting::Enabled(_) => None,
        }
    }

    pub fn is_hidden(&self) -> bool {
        matches!(self, SectionSetting::Enabled(false))
    }
}

impl WorkspaceConfig {
    pub fn effective_skip_ci(&self) -> bool {
        self.skip_ci
            .unwrap_or(self.release_commit_mode == ReleaseCommitMode::Commit)
    }

    pub fn effective_commit_skip_markers(&self) -> Vec<String> {
        self.commit_skip_markers
            .clone()
            .unwrap_or_else(default_commit_skip_markers)
    }
}

pub fn default_commit_skip_markers() -> Vec<String> {
    vec![
        "[skip ci]".to_string(),
        "[ci skip]".to_string(),
        "[no ci]".to_string(),
        "[skip actions]".to_string(),
        "[actions skip]".to_string(),
    ]
}

fn default_auto_merge() -> bool {
    true
}

fn default_telemetry() -> bool {
    true
}

fn default_remote() -> String {
    "origin".to_string()
}

fn default_branch() -> String {
    #[cfg(feature = "cli")]
    {
        let detected = (|| {
            let repo = gix::discover(".").ok()?;
            let reference = repo.find_reference("refs/remotes/origin/HEAD").ok()?;
            let target = reference.target();
            let target_name = match target {
                gix::refs::TargetRef::Symbolic(name) => name,
                _ => return None,
            };
            let full = target_name.as_bstr().to_string();
            let branch = full.strip_prefix("refs/remotes/origin/").unwrap_or(&full);
            if branch.is_empty() {
                None
            } else {
                Some(branch.to_string())
            }
        })();

        if let Some(branch) = detected {
            return branch;
        }
    }

    "main".to_string()
}
