use serde::{Deserialize, Serialize};

use crate::conventional_commits::BumpType;

use super::types::{BuildMetadata, HooksConfig, PublisherConfig, VersionSourcePolicy};
use super::workspace::WorkspaceConfig;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PackageConfig {
    pub name: String,
    #[serde(default)]
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
    #[serde(alias = "versionTemplate")]
    pub version_template: Option<String>,
    #[serde(default, alias = "floatingTags")]
    pub floating_tags: Option<Vec<FloatingTagLevel>>,
    #[serde(default, alias = "latestTag")]
    pub latest_tag: Option<String>,
    /// Overrides `workspace.buildMetadata` for this package. `false` releases
    /// it with a plain version while the rest of the workspace stays stamped.
    #[serde(default, alias = "buildMetadata")]
    pub build_metadata: Option<BuildMetadata>,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
    #[serde(default)]
    pub publishers: Vec<PublisherConfig>,
    #[serde(default, alias = "updateLockfiles")]
    pub update_lockfiles: Option<bool>,
    #[serde(default, alias = "versionSource")]
    pub version_source: Option<VersionSourcePolicy>,
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
    #[default]
    Same,
    MajorOnMajor,
    Patch,
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
    CalverShortSeq,
    Sequential,
    Zerover,
}

impl std::fmt::Display for VersioningStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Semver => "semver",
            Self::Calver => "calver",
            Self::CalverShort => "calver-short",
            Self::CalverSeq => "calver-seq",
            Self::CalverShortSeq => "calver-short-seq",
            Self::Sequential => "sequential",
            Self::Zerover => "zerover",
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FloatingTagLevel {
    Major,
    Minor,
}

impl PackageConfig {
    /// Whether this package owns any of `changed_files`: its own `path`, minus
    /// the `nested` package paths it contains, or one of its `shared_paths`.
    ///
    /// A single-package repo always owns everything, as does a package rooted
    /// at the repo root with no nested package excluded, so both short-circuit
    /// to true.
    pub fn is_touched_by(
        &self,
        changed_files: &[String],
        is_monorepo: bool,
        nested: &[String],
    ) -> bool {
        if !is_monorepo {
            return true;
        }
        if self.relative_path().is_none() && nested.is_empty() {
            return true;
        }
        if changed_files.iter().any(|f| self.owns_file(f, nested)) {
            return true;
        }
        self.shared_paths.iter().any(|shared| {
            let shared = shared.trim_end_matches('/');
            changed_files
                .iter()
                .any(|f| f.starts_with(shared) || f == shared)
        })
    }

    pub fn owns_file(&self, file: &str, nested: &[String]) -> bool {
        let inside = match self.relative_path() {
            None => true,
            Some(path) => file.starts_with(&format!("{path}/")),
        };
        inside && !nested.iter().any(|n| file.starts_with(&format!("{n}/")))
    }

    pub fn relative_path(&self) -> Option<&str> {
        let path = self.path.trim_start_matches("./").trim_end_matches('/');
        (!path.is_empty() && path != ".").then_some(path)
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

    pub fn effective_version_template<'a>(
        &'a self,
        workspace: &'a WorkspaceConfig,
    ) -> Option<&'a str> {
        self.version_template
            .as_deref()
            .or(workspace.version_template.as_deref())
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

    pub fn latest_tag_name(&self, workspace: &WorkspaceConfig) -> Option<String> {
        let template = self
            .latest_tag
            .as_deref()
            .or(workspace.latest_tag.as_deref())?;
        let rendered = template.replace("{name}", &self.name);
        (!rendered.trim().is_empty()).then_some(rendered)
    }

    pub fn effective_update_lockfiles(&self, workspace: &WorkspaceConfig) -> bool {
        self.update_lockfiles.unwrap_or(workspace.update_lockfiles)
    }

    pub fn effective_version_source(&self, workspace: &WorkspaceConfig) -> VersionSourcePolicy {
        self.version_source.unwrap_or(workspace.version_source)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionedFile {
    pub path: String,
    pub format: FileFormat,
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
    Helm,
    Json,
    Toml,
    Txt,
    Xml,
    #[serde(rename = "pubspecyaml")]
    PubspecYaml,
    #[serde(rename = "mixexs")]
    MixExs,
    #[serde(rename = "chartyaml")]
    ChartYaml,
    #[serde(rename = "galaxyyml")]
    GalaxyYaml,
    Gemspec,
    #[serde(rename = "packageswift")]
    PackageSwift,
    Cabal,
    #[serde(rename = "cmake")]
    Cmake,
}

#[cfg(test)]
mod tests {
    #[test]
    fn strategy_display_matches_the_config_spelling() {
        for strategy in [
            VersioningStrategy::Semver,
            VersioningStrategy::Calver,
            VersioningStrategy::CalverShort,
            VersioningStrategy::CalverSeq,
            VersioningStrategy::CalverShortSeq,
            VersioningStrategy::Sequential,
            VersioningStrategy::Zerover,
        ] {
            let serialized = serde_json::to_value(strategy).unwrap();
            assert_eq!(serialized.as_str(), Some(strategy.to_string().as_str()));
        }
    }

    fn ws(tag_template: Option<&str>, latest: Option<&str>) -> WorkspaceConfig {
        WorkspaceConfig {
            tag_template: tag_template.map(str::to_string),
            version_template: None,
            latest_tag: latest.map(str::to_string),
            ..Default::default()
        }
    }

    fn named(name: &str) -> PackageConfig {
        let mut p = pkg(".", &[]);
        p.name = name.to_string();
        p
    }

    #[test]
    fn latest_tag_ignores_the_version_template_entirely() {
        let w = ws(Some("v{version}"), Some("latest"));
        assert_eq!(named("api").latest_tag_name(&w).as_deref(), Some("latest"));
    }

    #[test]
    fn latest_tag_never_inherits_a_v_prefix() {
        for template in ["v{version}", "{name}@v{version}", "release-{version}"] {
            let w = ws(Some(template), Some("latest"));
            let got = named("api").latest_tag_name(&w).unwrap();
            assert_eq!(got, "latest", "template {template:?} leaked into the alias");
            assert!(!got.starts_with('v'), "got {got:?}");
        }
    }

    #[test]
    fn latest_tag_namespaces_per_package_via_name() {
        let w = ws(Some("{name}@v{version}"), Some("{name}@latest"));
        assert_eq!(
            named("api").latest_tag_name(&w).as_deref(),
            Some("api@latest")
        );
        assert_eq!(
            named("web").latest_tag_name(&w).as_deref(),
            Some("web@latest")
        );
    }

    #[test]
    fn latest_tag_is_off_unless_configured() {
        assert_eq!(named("api").latest_tag_name(&ws(None, None)), None);
        assert_eq!(named("api").latest_tag_name(&ws(None, Some("  "))), None);
    }

    #[test]
    fn package_latest_tag_overrides_the_workspace_one() {
        let w = ws(None, Some("{name}@latest"));
        let mut p = named("api");
        p.latest_tag = Some("stable".to_string());
        assert_eq!(p.latest_tag_name(&w).as_deref(), Some("stable"));
    }
    use super::*;

    fn pkg(path: &str, shared: &[&str]) -> PackageConfig {
        PackageConfig {
            name: "api".to_string(),
            path: path.to_string(),
            shared_paths: shared.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn dep(json: &str) -> Dependency {
        serde_json::from_str(json).expect("valid dependency")
    }

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
        assert!(p.is_touched_by(&files(&["packages/api/src/main.rs"]), true, &[]));
    }

    #[test]
    fn ignores_files_of_a_sibling_package() {
        let p = pkg("packages/api", &[]);
        assert!(!p.is_touched_by(&files(&["packages/web/src/app.ts"]), true, &[]));
    }

    #[test]
    fn a_sibling_with_a_shared_prefix_does_not_match() {
        let p = pkg("packages/api", &[]);
        assert!(!p.is_touched_by(&files(&["packages/api-client/index.ts"]), true, &[]));
    }

    #[test]
    fn shared_paths_count_as_a_touch() {
        let p = pkg("packages/api", &["proto"]);
        assert!(p.is_touched_by(&files(&["proto/schema.proto"]), true, &[]));
    }

    #[test]
    fn a_shared_path_file_itself_counts() {
        let p = pkg("packages/api", &["Cargo.lock"]);
        assert!(p.is_touched_by(&files(&["Cargo.lock"]), true, &[]));
    }

    #[test]
    fn a_single_package_repo_owns_everything() {
        let p = pkg("packages/api", &[]);
        assert!(p.is_touched_by(&files(&["anywhere/else.txt"]), false, &[]));
    }

    #[test]
    fn a_root_package_owns_everything() {
        for path in [".", "", "./"] {
            let p = pkg(path, &[]);
            assert!(
                p.is_touched_by(&files(&["anywhere/else.txt"]), true, &[]),
                "path {path:?} should own the whole tree"
            );
        }
    }

    #[test]
    fn a_trailing_slash_on_the_package_path_is_tolerated() {
        let p = pkg("packages/api/", &[]);
        assert!(p.is_touched_by(&files(&["packages/api/src/main.rs"]), true, &[]));
    }

    #[test]
    fn no_changed_files_is_not_a_touch() {
        let p = pkg("packages/api", &[]);
        assert!(!p.is_touched_by(&[], true, &[]));
    }
    fn nested(list: &[&str]) -> Vec<String> {
        files(list)
    }

    #[test]
    fn a_root_package_skips_files_owned_by_a_nested_package() {
        let p = pkg(".", &[]);
        assert!(!p.is_touched_by(&files(&["cli/src/main.rs"]), true, &nested(&["cli"])));
        assert!(p.is_touched_by(
            &files(&["cli/src/main.rs", "go.mod"]),
            true,
            &nested(&["cli"])
        ));
    }

    #[test]
    fn a_nested_exclusion_does_not_swallow_a_sibling_with_a_shared_prefix() {
        let p = pkg(".", &[]);
        assert!(p.is_touched_by(&files(&["cli-tools/run.sh"]), true, &nested(&["cli"])));
    }

    #[test]
    fn an_outer_package_keeps_its_files_outside_the_nested_one() {
        let p = pkg("packages/app", &[]);
        let inner = nested(&["packages/app/plugin"]);
        assert!(!p.is_touched_by(&files(&["packages/app/plugin/lib.rs"]), true, &inner));
        assert!(p.is_touched_by(&files(&["packages/app/src/lib.rs"]), true, &inner));
    }

    #[test]
    fn a_shared_path_still_counts_inside_a_nested_package() {
        let p = pkg(".", &["cli/schema.json"]);
        assert!(p.is_touched_by(&files(&["cli/schema.json"]), true, &nested(&["cli"])));
    }

    #[test]
    fn a_root_package_with_nested_exclusions_needs_a_changed_file() {
        let p = pkg(".", &[]);
        assert!(!p.is_touched_by(&[], true, &nested(&["cli"])));
    }
}
