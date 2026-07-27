use serde::{Deserialize, Serialize};

use super::types::{HooksConfig, PublisherConfig};
use super::workspace::WorkspaceConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub path: String,
    #[serde(default, alias = "versionedFiles")]
    pub versioned_files: Vec<VersionedFile>,
    pub changelog: Option<String>,
    #[serde(default, alias = "sharedPaths")]
    pub shared_paths: Vec<String>,
    #[serde(default, alias = "dependsOn")]
    pub depends_on: Vec<String>,
    pub versioning: Option<VersioningStrategy>,
    #[serde(alias = "tagTemplate")]
    pub tag_template: Option<String>,
    #[serde(default, alias = "floatingTags")]
    pub floating_tags: Option<Vec<FloatingTagLevel>>,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
    /// Declarative publish targets. Evaluated in declaration order
    /// after the git push + GitHub Release create the new tag. v1
    /// only emits dry-run preview lines; per-kind execution lands in
    /// follow-up PRs.
    #[serde(default)]
    pub publishers: Vec<PublisherConfig>,
    #[serde(default, alias = "updateLockfiles")]
    pub update_lockfiles: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VersioningStrategy {
    #[default]
    Semver,
    Calver,
    CalverShort,
    CalverSeq,
    Sequential,
    Zerover,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FloatingTagLevel {
    Major,
    Minor,
}

impl PackageConfig {
    /// Resolve the effective versioning strategy for this package. Priority:
    ///   1. package.versioning if explicitly set
    ///   2. workspace.versioning if explicitly set
    ///   3. auto-detect from `tags` (filtered to tags relevant to this
    ///      package — caller's job)
    ///   4. fallback to [`VersioningStrategy::Semver`]
    ///
    /// Note: zerover is intentionally excluded from auto-detection because it
    /// is ambiguous with semver (both use `X.Y.Z`). Users must opt-in
    /// explicitly via config.
    pub fn effective_versioning<'t>(
        &self,
        workspace: &WorkspaceConfig,
        tags: impl FnOnce() -> Vec<&'t str>,
    ) -> VersioningStrategy {
        self.versioning
            .or(workspace.versioning)
            .or_else(|| crate::versioning::detect_strategy_from_tags(&tags()))
            .unwrap_or_default()
    }

    fn effective_template<'a>(
        &'a self,
        workspace: &'a WorkspaceConfig,
        is_monorepo: bool,
    ) -> &'a str {
        self.tag_template
            .as_deref()
            .or(workspace.tag_template.as_deref())
            .unwrap_or(if is_monorepo {
                "{name}@v{version}"
            } else {
                "v{version}"
            })
    }

    pub fn tag_for_version(
        &self,
        workspace: &WorkspaceConfig,
        is_monorepo: bool,
        version: &str,
    ) -> String {
        self.effective_template(workspace, is_monorepo)
            .replace("{name}", &self.name)
            .replace("{version}", version)
    }

    pub fn tag_prefix(&self, workspace: &WorkspaceConfig, is_monorepo: bool) -> String {
        let template = self.effective_template(workspace, is_monorepo);
        let prefix = template.split("{version}").next().unwrap_or(template);
        prefix.replace("{name}", &self.name)
    }

    pub fn effective_floating_tags<'a>(
        &'a self,
        workspace: &'a WorkspaceConfig,
    ) -> &'a [FloatingTagLevel] {
        match &self.floating_tags {
            Some(tags) => tags,
            None => &workspace.floating_tags,
        }
    }

    pub fn effective_update_lockfiles(&self, workspace: &WorkspaceConfig) -> bool {
        self.update_lockfiles.unwrap_or(workspace.update_lockfiles)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionedFile {
    pub path: String,
    pub format: FileFormat,
    /// Optional selector to disambiguate which occurrence in the file is the
    /// version to bump. Syntax depends on the format:
    ///
    /// - `xml`: a slash-delimited path of tag names rooted at the document
    ///   element, e.g. `/project/version`. Without a selector the handler
    ///   targets the first `<version>` that is a direct child of the root
    ///   element — which fixes the common Maven `<parent>` pitfall.
    /// - `txt`: a regex with a single capture group that brackets the
    ///   version string, e.g. `^VERSION=(.+)$`.
    ///
    /// Other formats currently ignore this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Csproj,
    #[serde(rename = "gomod")]
    GoMod,
    Gradle,
    /// `values.yaml` templating for Helm charts. For the top-level
    /// `Chart.yaml` manifest use [`FileFormat::ChartYaml`] instead.
    Helm,
    Json,
    Toml,
    Txt,
    Xml,
    /// `pubspec.yaml` for Dart / Flutter packages.
    #[serde(rename = "pubspecyaml")]
    PubspecYaml,
    /// `mix.exs` for Elixir / Mix projects.
    #[serde(rename = "mixexs")]
    MixExs,
    /// `Chart.yaml` for Helm chart top-level manifests.
    #[serde(rename = "chartyaml")]
    ChartYaml,
    /// `*.gemspec` for Ruby gems.
    Gemspec,
    /// `Package.swift` for Swift packages.
    #[serde(rename = "packageswift")]
    PackageSwift,
    /// `*.cabal` for Haskell packages.
    Cabal,
    /// `CMakeLists.txt` — the `VERSION` argument of the `project()` call.
    #[serde(rename = "cmake")]
    Cmake,
}
