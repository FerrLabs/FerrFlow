use serde::{Deserialize, Serialize};

use super::package::{FloatingTagLevel, VersioningStrategy};
use super::types::{
    BranchChannelConfig, ForgeKind, HooksConfig, OrphanedTagStrategy, RegistryConfig,
    ReleaseCommitMode, ReleaseCommitScope,
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
    #[serde(default, alias = "recoverMissedReleases")]
    pub recover_missed_releases: bool,
    #[serde(default, alias = "releaseCommitMode")]
    pub release_commit_mode: ReleaseCommitMode,
    #[serde(default, alias = "releaseCommitScope")]
    pub release_commit_scope: ReleaseCommitScope,
    #[serde(default = "default_auto_merge", alias = "autoMergeReleases")]
    pub auto_merge_releases: bool,
    #[serde(default, alias = "skipCi")]
    pub skip_ci: Option<bool>,
    #[serde(default, alias = "commitSkipMarkers")]
    pub commit_skip_markers: Option<Vec<String>>,
    #[serde(default, alias = "floatingTags")]
    pub floating_tags: Vec<FloatingTagLevel>,
    #[serde(default, alias = "orphanedTagStrategy")]
    pub orphaned_tag_strategy: OrphanedTagStrategy,
    #[serde(default)]
    pub forge: ForgeKind,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
    #[serde(default)]
    pub branches: Option<Vec<BranchChannelConfig>>,
    /// Named registry credentials shared by `package.publishers[]`.
    /// Keyed by a short identifier (e.g. `"kellnr"`, `"gh-packages"`)
    /// that publishers reference by name. Lets users declare auth
    /// once and reuse it across every cargo / npm / docker entry. See
    /// the publisher RFC.
    #[serde(default)]
    pub registries: BTreeMap<String, RegistryConfig>,
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
