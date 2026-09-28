use super::*;
use crate::config::types::{ChannelValue, ForgeKind};

fn build(raw: &str) -> (Config, MigrationReport) {
    let _cwd = crate::test_utils::CWD_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
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
    assert_eq!(cfg.workspace.tag_template.as_deref(), Some("v{version}"));
    assert!(report.mapped.iter().any(|m| m.contains("tagTemplate")));
}

#[test]
fn tag_format_with_prefix_and_suffix() {
    let (cfg, _) = build(r#"{"tagFormat": "release-${version}-stable"}"#);
    assert_eq!(
        cfg.workspace.tag_template.as_deref(),
        Some("release-{version}-stable")
    );
}

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

#[test]
fn prerelease_string_names_the_channel() {
    let (cfg, _) = build(r#"{"branches": [{"name": "next-major", "prerelease": "next"}]}"#);
    assert!(matches!(
        &branch(&cfg, "next-major").channel,
        ChannelValue::Named(c) if c == "next"
    ));
}

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

#[test]
fn repository_url_is_reported_as_ignored() {
    let (cfg, report) = build(r#"{"repositoryUrl": "https://github.com/o/r.git"}"#);
    assert!(report.ignored.iter().any(|i| i.contains("repositoryUrl")));
    assert!(cfg.workspace.tag_template.is_none());
}

#[test]
fn empty_releaserc_still_produces_a_usable_config() {
    let (cfg, report) = build("{}");
    assert_eq!(cfg.packages.len(), 1);
    assert_eq!(cfg.packages[0].path, ".");
    assert!(report.warnings.iter().any(|w| w.contains("single-package")));
}

#[test]
fn malformed_json_is_an_error() {
    assert!(build_config_from_releaserc("{ not json").is_err());
}

#[test]
fn json5_features_are_tolerated() {
    let (cfg, _) = build(
        r#"{
            "tagFormat": "v${version}",
        }"#,
    );
    assert_eq!(cfg.workspace.tag_template.as_deref(), Some("v{version}"));
}

#[test]
fn source_label_is_stable() {
    assert_eq!(Source::SemanticRelease.label(), "semantic-release");
}

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
    assert_eq!(cfg.workspace.tag_template.as_deref(), Some("v{version}"));
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
    assert!(yaml_to_json("plugins: [unclosed").is_err());
}

fn sample_migration() -> Migration {
    let (config, report) = build(r#"{"tagFormat": "v${version}"}"#);
    Migration::new(
        Source::SemanticRelease,
        PathBuf::from(".releaserc"),
        config,
        report,
    )
}

#[test]
fn dry_run_returns_the_config_without_writing_it() {
    let dir = tempfile::tempdir().unwrap();
    let (filename, content) = emit(&sample_migration(), dir.path(), true).unwrap();
    assert_eq!(filename, "ferrflow.json");
    assert!(content.contains("v{version}"));
    assert!(!dir.path().join(&filename).exists());
}

#[test]
fn real_run_writes_exactly_what_the_dry_run_shows() {
    let dir = tempfile::tempdir().unwrap();
    let (_, preview) = emit(&sample_migration(), dir.path(), true).unwrap();
    let (filename, content) = emit(&sample_migration(), dir.path(), false).unwrap();
    assert_eq!(content, preview);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(filename)).unwrap(),
        content
    );
}

#[test]
fn a_migrated_tag_format_renders_the_same_tags_semantic_release_created() {
    let (cfg, _) = build(r#"{"tagFormat": "release-${version}"}"#);
    let pkg = &cfg.packages[0];
    assert_eq!(
        pkg.tag_for_version(&cfg.workspace, false, "1.2.0"),
        "release-1.2.0"
    );
    assert_eq!(pkg.tag_prefix(&cfg.workspace, false), "release-");
}

mod end_to_end {
    use super::super::{Source, migrate};
    use crate::config::Config;
    use crate::error_code::code_from_error;
    use crate::test_utils::with_cwd;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn run(dir: &Path, from: Option<Source>, dry_run: bool) -> anyhow::Result<()> {
        let mut outcome = None;
        with_cwd(dir, || {
            outcome = Some(migrate(from, dry_run));
            Ok(())
        })
        .unwrap();
        outcome.unwrap()
    }

    fn migrated(dir: &Path) -> Config {
        Config::load(dir, Some(&dir.join("ferrflow.json"))).expect("migration output must load")
    }

    #[test]
    fn a_yaml_releaserc_is_detected_and_written_as_a_loadable_config() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".releaserc.yml",
            "tagFormat: \"v${version}\"\nbranches:\n  - main\n  - name: next\n    prerelease: true\n",
        );
        write(dir.path(), "package.json", r#"{"name":"my-lib"}"#);

        run(dir.path(), None, false).unwrap();

        let cfg = migrated(dir.path());
        assert_eq!(cfg.workspace.tag_template.as_deref(), Some("v{version}"));
        assert_eq!(cfg.packages[0].name, "my-lib");
        let branches: Vec<&str> = cfg
            .workspace
            .branches
            .as_ref()
            .unwrap()
            .iter()
            .map(|b| b.name.as_str())
            .collect();
        assert_eq!(branches, ["main", "next"]);
    }

    #[test]
    fn a_releaserc_without_extension_is_read_as_json5() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".releaserc",
            "{\n  // comment\n  tagFormat: 'r${version}',\n}\n",
        );

        run(dir.path(), None, false).unwrap();

        assert_eq!(
            migrated(dir.path()).workspace.tag_template.as_deref(),
            Some("r{version}")
        );
    }

    #[test]
    fn a_dry_run_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".releaserc.json",
            r#"{"tagFormat":"v${version}"}"#,
        );

        run(dir.path(), None, true).unwrap();

        assert!(!dir.path().join("ferrflow.json").exists());
    }

    #[test]
    fn semantic_release_wins_detection_over_other_tools() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".releaserc.json",
            r#"{"tagFormat":"sr-${version}"}"#,
        );
        write(dir.path(), ".versionrc.json", r#"{"tagPrefix":"sv-"}"#);

        run(dir.path(), None, false).unwrap();

        assert_eq!(
            migrated(dir.path()).workspace.tag_template.as_deref(),
            Some("sr-{version}")
        );
    }

    #[test]
    fn from_overrides_detection() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".releaserc.json",
            r#"{"tagFormat":"sr-${version}"}"#,
        );
        write(dir.path(), ".versionrc.json", r#"{"tagPrefix":"sv-"}"#);

        run(dir.path(), Some(Source::StandardVersion), false).unwrap();

        assert_eq!(
            migrated(dir.path()).workspace.tag_template.as_deref(),
            Some("sv-{version}")
        );
    }

    #[test]
    fn a_yaml_versionrc_is_detected_when_nothing_else_is_there() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".versionrc.yaml", "tagPrefix: rel-\n");

        run(dir.path(), None, false).unwrap();

        assert_eq!(
            migrated(dir.path()).workspace.tag_template.as_deref(),
            Some("rel-{version}")
        );
    }

    #[test]
    fn forcing_a_source_whose_config_is_missing_fails() {
        let dir = tempfile::tempdir().unwrap();

        let err = run(dir.path(), Some(Source::SemanticRelease), false).unwrap_err();

        assert!(
            format!("{err:#}").contains("no semantic-release config found"),
            "{err:#}"
        );
        assert!(!dir.path().join("ferrflow.json").exists());
    }

    #[test]
    fn no_known_config_is_a_not_found_error_naming_what_was_looked_for() {
        let dir = tempfile::tempdir().unwrap();

        let err = run(dir.path(), None, false).unwrap_err();

        assert_eq!(code_from_error(&err).as_deref(), Some("E1001"));
        let msg = format!("{err:#}");
        for looked_for in [
            ".releaserc.json",
            ".changeset/config.json",
            "release-please-config.json",
            ".versionrc",
            "--from",
        ] {
            assert!(msg.contains(looked_for), "missing {looked_for}: {msg}");
        }
    }

    #[test]
    fn an_existing_ferrflow_config_is_never_overwritten() {
        for existing in ["ferrflow.json", "ferrflow.toml", ".ferrflow", "ferrflow.ts"] {
            let dir = tempfile::tempdir().unwrap();
            write(
                dir.path(),
                ".releaserc.json",
                r#"{"tagFormat":"v${version}"}"#,
            );
            write(dir.path(), existing, "original");

            let err = run(dir.path(), None, false).unwrap_err();

            assert_eq!(
                code_from_error(&err).as_deref(),
                Some("E1017"),
                "{existing}: {err:#}"
            );
            assert_eq!(
                std::fs::read_to_string(dir.path().join(existing)).unwrap(),
                "original"
            );
            if existing != "ferrflow.json" {
                assert!(!dir.path().join("ferrflow.json").exists(), "{existing}");
            }
        }
    }

    #[test]
    fn an_unparseable_releaserc_fails_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".releaserc.json", "{ not json");

        let err = run(dir.path(), None, false).unwrap_err();

        assert_eq!(code_from_error(&err).as_deref(), Some("E1014"));
        assert!(!dir.path().join("ferrflow.json").exists());
    }
}
