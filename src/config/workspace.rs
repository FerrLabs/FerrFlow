use serde::{Deserialize, Serialize};

use super::package::{FloatingTagLevel, VersioningStrategy};
use super::types::{
    BranchChannelConfig, ForgeKind, HooksConfig, OrphanedTagStrategy, RegistryConfig,
    ReleaseCommitBody, ReleaseCommitMode, ReleaseCommitScope,
};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct WorkspaceConfig {
    #[serde(default = "default_remote")]
    pub remote: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    // Telemetry was removed in v5.33; the key stays accepted so existing
    // configs neither warn nor fail schema validation. Nothing reads it.
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
    #[serde(default, alias = "releaseCommitBody")]
    pub release_commit_body: ReleaseCommitBody,
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
    /// When `true`, `ferrflow release` does NOT run the configured
    /// `publishers` inline — it defers them to a separate `ferrflow
    /// publish` invocation. Default `false` (publishers run at the end
    /// of `release`, as usual). Set `true` when the release job is
    /// minimal but publishing needs a build toolchain (docker buildx,
    /// helm, a built npm dist) that lives in another CI job. `ferrflow
    /// publish` always runs the publishers regardless of this flag.
    #[serde(default, alias = "deferPublish")]
    pub defer_publish: bool,
    #[serde(default)]
    pub changelog: Option<ChangelogConfig>,
    /// Opt-in single-file source of truth for per-package versions.
    /// When set (e.g. `".ferrflow.manifest.json"`), `ferrflow release`
    /// writes a full version snapshot to this path (relative to the repo
    /// root) as part of the release commit, validates it against the
    /// versioned files at release start, and `status` / `version` read
    /// from it. `None` (default) leaves all behaviour unchanged.
    #[serde(default, alias = "manifestFile")]
    pub manifest_file: Option<String>,
    /// Opt-in lockfile auto-refresh. When `true`, after `ferrflow
    /// release` bumps a manifest (`Cargo.toml`, `package.json`,
    /// `pyproject.toml`, `Gemfile`, `mix.exs`) it refreshes the sibling
    /// lockfile (`Cargo.lock`, `package-lock.json` / `pnpm-lock.yaml` /
    /// `yarn.lock`, `poetry.lock` / `uv.lock`, `Gemfile.lock`,
    /// `mix.lock`) with the package manager's offline / lockfile-only
    /// mode and stages it in the same release commit. Default `false`
    /// (no behaviour change). A missing package manager or an
    /// unresolvable offline update is warned about, never fatal. Set
    /// `package.updateLockfiles = false` to opt a single package out.
    #[serde(default, alias = "updateLockfiles")]
    pub update_lockfiles: bool,
    /// Rewrite the version constraint a dependent declares for a package
    /// that was just bumped, so consumers stop pinning the old range.
    /// Only `json` (package.json) and `toml` (Cargo.toml) manifests are
    /// rewritten, and only plain operator + version constraints — a
    /// `workspace:*`, `file:` or multi-part range is left for a human.
    /// Default `false`: this writes into manifests, so it is opt-in.
    #[serde(default, alias = "updateDependents")]
    pub update_dependents: bool,
    /// Packages that share a version line when co-released. When any
    /// member of a group has a releasable commit, every member is bumped
    /// to the same version (the highest resulting version across the
    /// group). Names stay distinct. Each inner array is one group.
    #[serde(default)]
    pub linked: Vec<Vec<String>>,
    /// Packages locked to an identical version forever. Behaves like
    /// [`linked`](Self::linked) but is intended for packages that must
    /// never diverge; `ferrflow validate` warns when their versions have
    /// drifted apart. Each inner array is one group.
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
