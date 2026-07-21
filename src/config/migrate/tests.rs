use super::*;
use crate::config::types::{ChannelValue, ForgeKind};

fn build(raw: &str) -> (Config, MigrationReport) {
    build_config_from_releaserc(raw).expect("valid releaserc")
}

fn branch<'a>(cfg: &'a Config, name: &str) -> &'a BranchChannelConfig {
    cfg.workspace
        .branches
        .as_ref()
        .expect("branches present")
        .iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("branch {name} not converted"))
}

#[test]
fn tag_format_version_token_is_rewritten() {
    let (cfg, report) = build(r#"{"tagFormat": "v${version}"}"#);
    assert_eq!(cfg.workspace.tag_template.as_deref(), Some("v{{version}}"));
    assert!(report.mapped.iter().any(|m| m.contains("tagTemplate")));
}

#[test]
fn tag_format_with_prefix_and_suffix() {
    let (cfg, _) = build(r#"{"tagFormat": "release-${version}-stable"}"#);
    assert_eq!(
        cfg.workspace.tag_template.as_deref(),
        Some("release-{{version}}-stable")
    );
}

// main/master are the stable line; a prerelease:true branch becomes a channel.
#[test]
fn branches_map_stable_and_prerelease() {
    let (cfg, _) = build(r#"{"branches": ["main", {"name": "beta", "prerelease": true}]}"#);
    assert!(matches!(
        branch(&cfg, "main").channel,
        ChannelValue::Stable(true)
    ));
    assert!(matches!(
        &branch(&cfg, "beta").channel,
        ChannelValue::Named(c) if c == "beta"
    ));
}

// `prerelease` can name the identifier directly, distinct from the branch name.
#[test]
fn prerelease_string_names_the_channel() {
    let (cfg, _) = build(r#"{"branches": [{"name": "next-major", "prerelease": "next"}]}"#);
    assert!(matches!(
        &branch(&cfg, "next-major").channel,
        ChannelValue::Named(c) if c == "next"
    ));
}

// prerelease:false is a maintenance branch, not a channel.
#[test]
fn prerelease_false_is_not_a_channel() {
    let (cfg, _) = build(r#"{"branches": [{"name": "1.x", "prerelease": false}]}"#);
    assert!(matches!(
        branch(&cfg, "1.x").channel,
        ChannelValue::Stable(false)
    ));
}

#[test]
fn a_single_branch_string_is_accepted() {
    let (cfg, _) = build(r#"{"branches": "main"}"#);
    assert!(matches!(
        branch(&cfg, "main").channel,
        ChannelValue::Stable(true)
    ));
}

#[test]
fn changelog_plugin_sets_the_package_changelog() {
    let (cfg, _) = build(
        r#"{"plugins": [["@semantic-release/changelog", {"changelogFile": "docs/CHANGES.md"}]]}"#,
    );
    assert_eq!(
        cfg.packages[0].changelog.as_deref(),
        Some("docs/CHANGES.md")
    );
}

#[test]
fn changelog_plugin_without_options_defaults_the_path() {
    let (cfg, _) = build(r#"{"plugins": ["@semantic-release/changelog"]}"#);
    assert_eq!(cfg.packages[0].changelog.as_deref(), Some("CHANGELOG.md"));
}

#[test]
fn github_plugin_sets_the_forge() {
    let (cfg, _) = build(r#"{"plugins": ["@semantic-release/github"]}"#);
    assert!(matches!(cfg.workspace.forge, ForgeKind::Github));
}

#[test]
fn gitlab_plugin_sets_the_forge() {
    let (cfg, _) = build(r#"{"plugins": ["@semantic-release/gitlab"]}"#);
    assert!(matches!(cfg.workspace.forge, ForgeKind::Gitlab));
}

// Every exec command maps to the hook that runs at the equivalent phase.
#[test]
fn exec_commands_map_to_hooks() {
    let (cfg, _) = build(
        r#"{"plugins": [["@semantic-release/exec", {
            "prepareCmd": "npm run build",
            "publishCmd": "npm publish",
            "successCmd": "echo done",
            "failCmd": "echo failed",
            "verifyConditionsCmd": "npm run lint"
        }]]}"#,
    );
    let hooks = cfg.workspace.hooks.as_ref().expect("hooks present");
    assert_eq!(hooks.pre_bump.as_deref(), Some("npm run build"));
    assert_eq!(hooks.post_publish.as_deref(), Some("npm publish"));
    assert_eq!(hooks.on_success.as_deref(), Some("echo done"));
    assert_eq!(hooks.on_error.as_deref(), Some("echo failed"));
    assert_eq!(hooks.pre_release.as_deref(), Some("npm run lint"));
}

// The bump rules are fixed, so custom releaseRules can't be honoured — the user
// must be told, not silently ignored.
#[test]
fn custom_release_rules_warn() {
    let (_, report) = build(
        r#"{"plugins": [["@semantic-release/commit-analyzer", {
            "releaseRules": [{"type": "docs", "release": "patch"}]
        }]]}"#,
    );
    assert!(
        report.warnings.iter().any(|w| w.contains("releaseRules")),
        "expected a warning about custom releaseRules, got {:?}",
        report.warnings
    );
}

#[test]
fn default_commit_analyzer_does_not_warn() {
    let (_, report) = build(r#"{"plugins": ["@semantic-release/commit-analyzer"]}"#);
    assert!(
        !report.warnings.iter().any(|w| w.contains("releaseRules")),
        "a plain commit-analyzer should not warn about rules"
    );
}

// npm publishing semantics differ; a silent map would mislead.
#[test]
fn npm_plugin_warns_rather_than_mapping() {
    let (_, report) = build(r#"{"plugins": ["@semantic-release/npm"]}"#);
    assert!(report.warnings.iter().any(|w| w.contains("npm")));
}

#[test]
fn unknown_plugin_warns() {
    let (_, report) = build(r#"{"plugins": ["@some/custom-plugin"]}"#);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("@some/custom-plugin"))
    );
}

// repositoryUrl has no field — it must be reported, not dropped in silence.
#[test]
fn repository_url_is_reported_as_ignored() {
    let (cfg, report) = build(r#"{"repositoryUrl": "https://github.com/o/r.git"}"#);
    assert!(report.ignored.iter().any(|i| i.contains("repositoryUrl")));
    // and it does not leak into the config anywhere
    assert!(cfg.workspace.tag_template.is_none());
}

#[test]
fn empty_releaserc_still_produces_a_usable_config() {
    let (cfg, report) = build("{}");
    assert_eq!(cfg.packages.len(), 1);
    assert_eq!(cfg.packages[0].path, ".");
    // the single-package caveat is always surfaced
    assert!(report.warnings.iter().any(|w| w.contains("single-package")));
}

#[test]
fn malformed_json_is_an_error() {
    assert!(build_config_from_releaserc("{ not json").is_err());
}

// JSON5 tolerance: real .releaserc files sometimes carry comments/trailing commas.
#[test]
fn json5_features_are_tolerated() {
    let (cfg, _) = build(
        r#"{
            // stable only
            "tagFormat": "v${version}",
        }"#,
    );
    assert_eq!(cfg.workspace.tag_template.as_deref(), Some("v{{version}}"));
}

#[test]
fn source_label_is_stable() {
    assert_eq!(Source::SemanticRelease.label(), "semantic-release");
}

// A YAML `.releaserc` converts to JSON that the existing converter accepts.
#[test]
fn yaml_config_converts_then_migrates() {
    let yaml = "\
tagFormat: \"v${version}\"
branches:
  - main
  - name: beta
    prerelease: true
plugins:
  - \"@semantic-release/github\"
";
    let json = yaml_to_json(yaml).expect("yaml converts to json");
    let (cfg, _) = build_config_from_releaserc(&json).expect("converted json is valid");
    assert_eq!(cfg.workspace.tag_template.as_deref(), Some("v{{version}}"));
    assert!(matches!(cfg.workspace.forge, ForgeKind::Github));
    let beta = cfg
        .workspace
        .branches
        .as_ref()
        .unwrap()
        .iter()
        .find(|b| b.name == "beta")
        .unwrap();
    assert!(matches!(&beta.channel, ChannelValue::Named(c) if c == "beta"));
}

#[test]
fn yaml_to_json_produces_parseable_json() {
    let json = yaml_to_json("a: 1\nb: [x, y]\n").expect("valid yaml");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json out");
    assert_eq!(value["a"], 1);
    assert_eq!(value["b"][1], "y");
}

#[test]
fn malformed_yaml_is_an_error() {
    // An unclosed flow sequence is not valid YAML.
    assert!(yaml_to_json("plugins: [unclosed").is_err());
}
