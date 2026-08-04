use serde::{Deserialize, Serialize};

use crate::conventional_commits::BumpType;

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
    pub depends_on: Vec<Dependency>,
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

/// An upstream package this one depends on.
///
/// Accepts both the plain name (`"core"`) and the detailed form
/// (`{ name = "core", propagate = "major-on-major" }`); the plain form is
/// equivalent to the detailed one with the default policy.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Dependency {
    Name(String),
    Detailed {
        name: String,
        #[serde(default)]
        propagate: PropagatePolicy,
    },
}

impl Dependency {
    pub fn name(&self) -> &str {
        match self {
            Dependency::Name(name) => name,
            Dependency::Detailed { name, .. } => name,
        }
    }

    pub fn propagate(&self) -> PropagatePolicy {
        match self {
            Dependency::Name(_) => PropagatePolicy::default(),
            Dependency::Detailed { propagate, .. } => *propagate,
        }
    }
}

/// How an upstream package's bump translates into its dependents' bump.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PropagatePolicy {
    /// The dependent gets the same bump the upstream got. A breaking upstream
    /// makes the dependent breaking too, which is the honest signal: the
    /// dependent now builds against a breaking API.
    #[default]
    Same,
    /// A major upstream makes the dependent major; anything else is a patch.
    MajorOnMajor,
    /// Always a patch, whatever the upstream did. FerrFlow's behaviour before
    /// propagation existed.
    Patch,
    /// The dependent is not bumped at all for this upstream.
    None,
}

impl PropagatePolicy {
    /// The bump a dependent receives when its upstream got `upstream`.
    pub fn resolve(self, upstream: BumpType) -> BumpType {
        match self {
            PropagatePolicy::Same => upstream,
            PropagatePolicy::MajorOnMajor => match upstream {
                BumpType::Major => BumpType::Major,
                BumpType::None => BumpType::None,
                _ => BumpType::Patch,
            },
            PropagatePolicy::Patch => match upstream {
                BumpType::None => BumpType::None,
                _ => BumpType::Patch,
            },
            PropagatePolicy::None => BumpType::None,
        }
    }
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
    /// Whether this package owns any of `changed_files` — its own `path`, or
    /// one of its `shared_paths`.
    ///
    /// A single-package repo always owns everything, as does a package rooted
    /// at the repo root, so both short-circuit to true.
    pub fn is_touched_by(&self, changed_files: &[String], is_monorepo: bool) -> bool {
        if !is_monorepo {
            return true;
        }

        let pkg_path = self.path.trim_start_matches("./").trim_end_matches('/');
        if pkg_path == "." || pkg_path.is_empty() {
            return true;
        }

        let prefix = format!("{pkg_path}/");
        if changed_files.iter().any(|f| f.starts_with(&prefix)) {
            return true;
        }

        self.shared_paths.iter().any(|shared| {
            let shared = shared.trim_end_matches('/');
            changed_files
                .iter()
                .any(|f| f.starts_with(shared) || f == shared)
        })
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(path: &str, shared: &[&str]) -> PackageConfig {
        PackageConfig {
            name: "api".to_string(),
            path: path.to_string(),
            versioned_files: Vec::new(),
            changelog: None,
            shared_paths: shared.iter().map(|s| s.to_string()).collect(),
            depends_on: Vec::new(),
            versioning: None,
            tag_template: None,
            floating_tags: None,
            hooks: None,
            publishers: Vec::new(),
            update_lockfiles: None,
        }
    }

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn dep(json: &str) -> Dependency {
        serde_json::from_str(json).expect("valid dependency")
    }

    // The plain-string form predates policies and must keep working.
    #[test]
    fn a_plain_string_dependency_uses_the_default_policy() {
        let d = dep(r#""core""#);
        assert_eq!(d.name(), "core");
        assert_eq!(d.propagate(), PropagatePolicy::Same);
    }

    #[test]
    fn the_detailed_form_carries_its_policy() {
        let d = dep(r#"{"name":"core","propagate":"major-on-major"}"#);
        assert_eq!(d.name(), "core");
        assert_eq!(d.propagate(), PropagatePolicy::MajorOnMajor);
    }

    #[test]
    fn the_detailed_form_without_a_policy_defaults_like_the_string_form() {
        assert_eq!(dep(r#"{"name":"core"}"#).propagate(), PropagatePolicy::Same);
    }

    // The bug this feature exists for: a breaking upstream used to hand its
    // dependents a patch, so consumers upgraded straight into a breaking API.
    #[test]
    fn same_policy_forwards_the_upstream_bump_unchanged() {
        for bump in [BumpType::Major, BumpType::Minor, BumpType::Patch] {
            assert_eq!(PropagatePolicy::Same.resolve(bump), bump);
        }
    }

    #[test]
    fn major_on_major_downgrades_everything_below_major() {
        assert_eq!(
            PropagatePolicy::MajorOnMajor.resolve(BumpType::Major),
            BumpType::Major
        );
        assert_eq!(
            PropagatePolicy::MajorOnMajor.resolve(BumpType::Minor),
            BumpType::Patch
        );
    }

    #[test]
    fn patch_policy_reproduces_the_old_behaviour() {
        for bump in [BumpType::Major, BumpType::Minor, BumpType::Patch] {
            assert_eq!(PropagatePolicy::Patch.resolve(bump), BumpType::Patch);
        }
    }

    #[test]
    fn none_policy_opts_out_of_the_cascade() {
        assert_eq!(
            PropagatePolicy::None.resolve(BumpType::Major),
            BumpType::None
        );
    }

    // An upstream that did not move must never manufacture a downstream bump.
    #[test]
    fn no_upstream_bump_never_produces_one_downstream() {
        for p in [
            PropagatePolicy::Same,
            PropagatePolicy::MajorOnMajor,
            PropagatePolicy::Patch,
            PropagatePolicy::None,
        ] {
            assert_eq!(p.resolve(BumpType::None), BumpType::None, "policy {p:?}");
        }
    }

    #[test]
    fn matches_files_under_the_package_path() {
        let p = pkg("packages/api", &[]);
        assert!(p.is_touched_by(&files(&["packages/api/src/main.rs"]), true));
    }

    // The whole point of scoping: a sibling package's commit must not count.
    #[test]
    fn ignores_files_of_a_sibling_package() {
        let p = pkg("packages/api", &[]);
        assert!(!p.is_touched_by(&files(&["packages/web/src/app.ts"]), true));
    }

    // `packages/api-client` must not match the `packages/api` package just
    // because the string starts the same way.
    #[test]
    fn a_sibling_with_a_shared_prefix_does_not_match() {
        let p = pkg("packages/api", &[]);
        assert!(!p.is_touched_by(&files(&["packages/api-client/index.ts"]), true));
    }

    #[test]
    fn shared_paths_count_as_a_touch() {
        let p = pkg("packages/api", &["proto"]);
        assert!(p.is_touched_by(&files(&["proto/schema.proto"]), true));
    }

    #[test]
    fn a_shared_path_file_itself_counts() {
        let p = pkg("packages/api", &["Cargo.lock"]);
        assert!(p.is_touched_by(&files(&["Cargo.lock"]), true));
    }

    // A single-package repo owns every commit, so scoping must never filter.
    #[test]
    fn a_single_package_repo_owns_everything() {
        let p = pkg("packages/api", &[]);
        assert!(p.is_touched_by(&files(&["anywhere/else.txt"]), false));
    }

    #[test]
    fn a_root_package_owns_everything() {
        for path in [".", "", "./"] {
            let p = pkg(path, &[]);
            assert!(
                p.is_touched_by(&files(&["anywhere/else.txt"]), true),
                "path {path:?} should own the whole tree"
            );
        }
    }

    #[test]
    fn a_trailing_slash_on_the_package_path_is_tolerated() {
        let p = pkg("packages/api/", &[]);
        assert!(p.is_touched_by(&files(&["packages/api/src/main.rs"]), true));
    }

    #[test]
    fn no_changed_files_is_not_a_touch() {
        let p = pkg("packages/api", &[]);
        assert!(!p.is_touched_by(&[], true));
    }
}
