use super::format::{format_handler, snake_to_camel, to_camel_case_keys};
#[cfg(feature = "cli")]
use super::loader_js::path_to_file_url;
use super::*;

// -----------------------------------------------------------------------
// Config parsing (all formats)
// -----------------------------------------------------------------------

#[test]
fn parse_json_config() {
    let json = r#"{
            "workspace": { "remote": "origin", "branch": "main" },
            "package": [{
                "name": "app",
                "path": ".",
                "versioned_files": [{ "path": "package.json", "format": "json" }]
            }]
        }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].name, "app");
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Json
    );
}

#[test]
fn parse_json_camel_case() {
    let json = r#"{
            "workspace": { "remote": "origin", "tagTemplate": "v{version}", "recoverMissedReleases": true, "releaseCommitMode": "pr", "autoMergeReleases": false },
            "package": [{
                "name": "app",
                "path": ".",
                "versionedFiles": [{ "path": "package.json", "format": "json" }],
                "sharedPaths": ["shared/"],
                "tagTemplate": "{name}@v{version}"
            }]
        }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.workspace.tag_template.as_deref(), Some("v{version}"));
    assert!(config.workspace.recover_missed_releases);
    assert_eq!(config.workspace.release_commit_mode, ReleaseCommitMode::Pr);
    assert!(!config.workspace.auto_merge_releases);
    assert_eq!(config.packages[0].versioned_files.len(), 1);
    assert_eq!(config.packages[0].shared_paths, vec!["shared/"]);
    assert_eq!(
        config.packages[0].tag_template.as_deref(),
        Some("{name}@v{version}")
    );
}

#[test]
fn parse_json5_config() {
    let json5 = r#"{
            workspace: { remote: "origin" },
            package: [{
                name: "app",
                path: ".",
                versioned_files: [{ path: "Cargo.toml", format: "toml" }],
            }],
        }"#;
    let config: Config = json5::from_str(json5).unwrap();
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Toml
    );
}

#[test]
fn parse_toml_config() {
    let toml = r#"
[workspace]
remote = "origin"
branch = "main"

[[package]]
name = "api"
path = "packages/api"
shared_paths = ["packages/shared/"]

[[package.versioned_files]]
path = "packages/api/Cargo.toml"
format = "toml"
"#;
    let config: Config = toml_edit::de::from_str(toml).unwrap();
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].shared_paths, vec!["packages/shared/"]);
}

#[test]
fn parse_changelog_config_toml() {
    let toml = r#"
[workspace.changelog]
sections = { feat = "Features", fix = "Bug Fixes", perf = "Performance", docs = false }
group_by_scope = true
include_commit_links = true
include_compare_link = true

[[package]]
name = "app"
path = "."
"#;
    let config: Config = toml_edit::de::from_str(toml).unwrap();
    let cl = config
        .workspace
        .changelog
        .expect("changelog config present");
    assert!(cl.group_by_scope);
    assert!(cl.include_commit_links);
    assert!(cl.include_compare_link);
    let sections = cl.sections.expect("sections present");
    assert_eq!(
        sections.get("feat").and_then(|s| s.label()),
        Some("Features")
    );
    assert!(sections.get("docs").unwrap().is_hidden());
    assert!(!sections.get("feat").unwrap().is_hidden());
}

#[test]
fn parse_changelog_config_camel_case() {
    let json = r#"{
        "workspace": { "changelog": { "groupByScope": true, "includeCommitLinks": true, "includeCompareLink": true } },
        "package": []
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    let cl = config
        .workspace
        .changelog
        .expect("changelog config present");
    assert!(cl.group_by_scope);
    assert!(cl.include_commit_links);
    assert!(cl.include_compare_link);
    assert!(cl.sections.is_none());
}

#[test]
fn changelog_config_defaults_when_absent() {
    let json = r#"{ "workspace": {}, "package": [] }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.workspace.changelog.is_none());
}

#[test]
fn parse_versioning_strategies() {
    let json = r#"{
            "workspace": { "versioning": "calver" },
            "package": [
                { "name": "a", "path": "a", "versioning": "zerover" },
                { "name": "b", "path": "b" }
            ]
        }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(
        config.workspace.versioning,
        Some(VersioningStrategy::Calver)
    );
    assert_eq!(
        config.packages[0].versioning,
        Some(VersioningStrategy::Zerover)
    );
    assert_eq!(config.packages[1].versioning, None);
}

#[test]
fn workspace_versioning_defaults_to_none() {
    // Unset `versioning` in config should deserialize to None so callers
    // can tell "user said nothing" apart from "user said semver".
    let json = r#"{ "workspace": {}, "package": [] }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.workspace.versioning, None);
}

#[test]
fn parse_all_versioning_variants() {
    for (s, expected) in [
        ("semver", VersioningStrategy::Semver),
        ("calver", VersioningStrategy::Calver),
        ("calver-short", VersioningStrategy::CalverShort),
        ("calver-seq", VersioningStrategy::CalverSeq),
        ("sequential", VersioningStrategy::Sequential),
        ("zerover", VersioningStrategy::Zerover),
    ] {
        let json = format!(r#"{{ "workspace": {{ "versioning": "{s}" }}, "package": [] }}"#);
        let config: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.workspace.versioning,
            Some(expected),
            "failed for {s}"
        );
    }
}

// -----------------------------------------------------------------------
// Effective versioning
// -----------------------------------------------------------------------

#[test]
fn effective_versioning_inherits_workspace() {
    let ws = WorkspaceConfig {
        versioning: Some(VersioningStrategy::Calver),
        ..WorkspaceConfig::default()
    };
    let pkg = PackageConfig {
        name: "a".into(),
        path: ".".into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    assert_eq!(
        pkg.effective_versioning(&ws, Vec::new),
        VersioningStrategy::Calver
    );
}

#[test]
fn effective_versioning_package_overrides() {
    let ws = WorkspaceConfig {
        versioning: Some(VersioningStrategy::Calver),
        ..WorkspaceConfig::default()
    };
    let pkg = PackageConfig {
        name: "a".into(),
        path: ".".into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: Some(VersioningStrategy::Zerover),
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    assert_eq!(
        pkg.effective_versioning(&ws, Vec::new),
        VersioningStrategy::Zerover
    );
}

#[test]
fn effective_versioning_does_not_read_tags_when_strategy_is_configured() {
    let ws = WorkspaceConfig {
        versioning: Some(VersioningStrategy::Calver),
        ..WorkspaceConfig::default()
    };
    let pkg = PackageConfig {
        name: "a".into(),
        path: ".".into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    let strategy =
        pkg.effective_versioning(&ws, || panic!("tags scanned despite configured strategy"));
    assert_eq!(strategy, VersioningStrategy::Calver);
}

#[test]
fn effective_versioning_autodetects_from_tags_when_unset() {
    let ws = WorkspaceConfig::default();
    let pkg = PackageConfig {
        name: "a".into(),
        path: ".".into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    let tags = vec!["v2024.04.18", "v2024.05.01"];
    assert_eq!(
        pkg.effective_versioning(&ws, || tags.clone()),
        VersioningStrategy::Calver
    );
}

#[test]
fn effective_versioning_falls_back_to_semver_without_tags() {
    let ws = WorkspaceConfig::default();
    let pkg = PackageConfig {
        name: "a".into(),
        path: ".".into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    assert_eq!(
        pkg.effective_versioning(&ws, Vec::new),
        VersioningStrategy::Semver
    );
}

// -----------------------------------------------------------------------
// Tag template
// -----------------------------------------------------------------------

fn make_pkg(name: &str, tag_template: Option<&str>) -> PackageConfig {
    PackageConfig {
        name: name.into(),
        path: ".".into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: tag_template.map(String::from),
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    }
}

#[test]
fn tag_default_single_repo() {
    let ws = WorkspaceConfig::default();
    let pkg = make_pkg("myapp", None);
    assert_eq!(pkg.tag_for_version(&ws, false, "1.2.3"), "v1.2.3");
    assert_eq!(pkg.tag_prefix(&ws, false), "v");
}

#[test]
fn tag_default_monorepo() {
    let ws = WorkspaceConfig::default();
    let pkg = make_pkg("api", None);
    assert_eq!(pkg.tag_for_version(&ws, true, "1.2.3"), "api@v1.2.3");
    assert_eq!(pkg.tag_prefix(&ws, true), "api@v");
}

#[test]
fn tag_custom_workspace_template() {
    let ws = WorkspaceConfig {
        tag_template: Some("release-{version}".into()),
        ..WorkspaceConfig::default()
    };
    let pkg = make_pkg("myapp", None);
    assert_eq!(pkg.tag_for_version(&ws, false, "1.0.0"), "release-1.0.0");
    assert_eq!(pkg.tag_prefix(&ws, false), "release-");
}

#[test]
fn tag_package_overrides_workspace() {
    let ws = WorkspaceConfig {
        tag_template: Some("v{version}".into()),
        ..WorkspaceConfig::default()
    };
    let pkg = make_pkg("api", Some("{name}/v{version}"));
    assert_eq!(pkg.tag_for_version(&ws, true, "2.0.0"), "api/v2.0.0");
    assert_eq!(pkg.tag_prefix(&ws, true), "api/v");
}

#[test]
fn update_lockfiles_defaults_off() {
    let ws = WorkspaceConfig::default();
    let pkg = make_pkg("api", None);
    assert!(!pkg.effective_update_lockfiles(&ws));
}

#[test]
fn update_lockfiles_inherits_workspace_when_unset() {
    let ws = WorkspaceConfig {
        update_lockfiles: true,
        ..WorkspaceConfig::default()
    };
    let pkg = make_pkg("api", None);
    assert!(pkg.effective_update_lockfiles(&ws));
}

#[test]
fn update_lockfiles_package_opts_out() {
    let ws = WorkspaceConfig {
        update_lockfiles: true,
        ..WorkspaceConfig::default()
    };
    let mut pkg = make_pkg("api", None);
    pkg.update_lockfiles = Some(false);
    assert!(!pkg.effective_update_lockfiles(&ws));
}

#[test]
fn update_lockfiles_alias_parses() {
    let cfg: Config = serde_json::from_str(
        r#"{"workspace": {"updateLockfiles": true}, "package": [
            {"name": "api", "path": ".", "updateLockfiles": false}
        ]}"#,
    )
    .unwrap();
    assert!(cfg.workspace.update_lockfiles);
    assert_eq!(cfg.packages[0].update_lockfiles, Some(false));
}

#[test]
fn tag_template_name_placeholder() {
    let ws = WorkspaceConfig::default();
    let pkg = make_pkg("frontend", Some("{name}-v{version}"));
    assert_eq!(pkg.tag_for_version(&ws, true, "3.0.0"), "frontend-v3.0.0");
}

// -----------------------------------------------------------------------
// is_monorepo
// -----------------------------------------------------------------------

#[test]
fn is_monorepo_single() {
    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages: vec![make_pkg("a", None)],
    };
    assert!(!config.is_monorepo());
}

#[test]
fn is_monorepo_multi() {
    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages: vec![make_pkg("a", None), make_pkg("b", None)],
    };
    assert!(config.is_monorepo());
}

// -----------------------------------------------------------------------
// Auto-detect
// -----------------------------------------------------------------------

#[test]
fn auto_detect_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::auto_detect(dir.path());
    assert!(config.packages.is_empty());
}

#[test]
fn auto_detect_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(config.packages.len(), 1);
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Toml
    );
}

#[test]
fn auto_detect_package_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Json
    );
}

#[test]
fn auto_detect_pom_xml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("pom.xml"),
        "<project><version>1.0</version></project>",
    )
    .unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Xml
    );
}

#[test]
fn auto_detect_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(config.packages[0].versioned_files.len(), 2);
}

// -----------------------------------------------------------------------
// Config load with explicit path
// -----------------------------------------------------------------------

#[test]
fn load_explicit_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.json");
    std::fs::write(&path, r#"{"package":[{"name":"x","path":"."}]}"#).unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "x");
}

#[test]
fn load_explicit_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.toml");
    std::fs::write(&path, "[[package]]\nname = \"x\"\npath = \".\"\n").unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "x");
}

#[test]
fn load_explicit_dotfile() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".ferrflow");
    std::fs::write(&path, r#"{"package":[{"name":"x","path":"."}]}"#).unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "x");
}

#[test]
fn load_explicit_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nope.json");
    assert!(Config::load_explicit(&path).is_err());
}

// -----------------------------------------------------------------------
// Config serialization roundtrip
// -----------------------------------------------------------------------

#[test]
fn json_roundtrip() {
    let handler = JsonFormat;
    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages: vec![make_pkg("test", None)],
    };
    let serialized = handler.serialize(&config).unwrap();
    let parsed = handler.parse(&serialized).unwrap();
    assert_eq!(parsed.packages[0].name, "test");
}

#[test]
fn json_serializes_camel_case() {
    let handler = JsonFormat;
    let config = Config {
        workspace: WorkspaceConfig {
            tag_template: Some("v{version}".into()),
            recover_missed_releases: true,
            ..WorkspaceConfig::default()
        },
        packages: vec![PackageConfig {
            name: "app".into(),
            path: ".".into(),
            versioned_files: vec![VersionedFile {
                path: "Cargo.toml".into(),
                format: FileFormat::Toml,
                selector: None,
            }],
            changelog: None,
            shared_paths: vec!["shared/".into()],
            depends_on: vec![],
            versioning: None,
            tag_template: Some("{name}@v{version}".into()),
            hooks: None,
            floating_tags: None,
            publishers: vec![],
            update_lockfiles: None,
        }],
    };
    let serialized = handler.serialize(&config).unwrap();
    assert!(serialized.contains("tagTemplate"));
    assert!(serialized.contains("versionedFiles"));
    assert!(serialized.contains("sharedPaths"));
    assert!(serialized.contains("recoverMissedReleases"));
    assert!(serialized.contains("releaseCommitMode"));
    assert!(serialized.contains("autoMergeReleases"));
    assert!(!serialized.contains("tag_template"));
    assert!(!serialized.contains("versioned_files"));
    assert!(!serialized.contains("shared_paths"));
    assert!(!serialized.contains("recover_missed_releases"));
    assert!(!serialized.contains("release_commit_mode"));
    assert!(!serialized.contains("auto_merge_releases"));

    let parsed = handler.parse(&serialized).unwrap();
    assert_eq!(parsed.workspace.tag_template.as_deref(), Some("v{version}"));
    assert_eq!(parsed.packages[0].shared_paths, vec!["shared/"]);
    assert!(parsed.workspace.recover_missed_releases);
}

#[test]
fn toml_keeps_snake_case() {
    let handler = TomlFormat;
    let config = Config {
        workspace: WorkspaceConfig {
            tag_template: Some("v{version}".into()),
            recover_missed_releases: true,
            ..WorkspaceConfig::default()
        },
        packages: vec![PackageConfig {
            name: "app".into(),
            path: ".".into(),
            versioned_files: vec![VersionedFile {
                path: "Cargo.toml".into(),
                format: FileFormat::Toml,
                selector: None,
            }],
            changelog: None,
            shared_paths: vec!["shared/".into()],
            depends_on: vec![],
            versioning: None,
            tag_template: Some("{name}@v{version}".into()),
            hooks: None,
            floating_tags: None,
            publishers: vec![],
            update_lockfiles: None,
        }],
    };
    let serialized = handler.serialize(&config).unwrap();
    assert!(serialized.contains("tag_template"));
    assert!(serialized.contains("versioned_files"));
    assert!(serialized.contains("shared_paths"));
    assert!(serialized.contains("recover_missed_releases"));
    assert!(serialized.contains("release_commit_mode"));
    assert!(serialized.contains("auto_merge_releases"));
    assert!(!serialized.contains("tagTemplate"));
    assert!(!serialized.contains("versionedFiles"));
    assert!(!serialized.contains("sharedPaths"));
    assert!(!serialized.contains("recoverMissedReleases"));
    assert!(!serialized.contains("releaseCommitMode"));
    assert!(!serialized.contains("autoMergeReleases"));
}

#[test]
fn toml_roundtrip() {
    let handler = TomlFormat;
    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages: vec![make_pkg("test", None)],
    };
    let serialized = handler.serialize(&config).unwrap();
    let parsed = handler.parse(&serialized).unwrap();
    assert_eq!(parsed.packages[0].name, "test");
}

// -----------------------------------------------------------------------
// effective_skip_ci
// -----------------------------------------------------------------------

#[test]
fn effective_skip_ci_defaults_true_for_commit_mode() {
    let ws = WorkspaceConfig {
        release_commit_mode: ReleaseCommitMode::Commit,
        skip_ci: None,
        ..WorkspaceConfig::default()
    };
    assert!(ws.effective_skip_ci());
}

#[test]
fn effective_skip_ci_defaults_false_for_pr_mode() {
    let ws = WorkspaceConfig {
        release_commit_mode: ReleaseCommitMode::Pr,
        skip_ci: None,
        ..WorkspaceConfig::default()
    };
    assert!(!ws.effective_skip_ci());
}

#[test]
fn effective_skip_ci_defaults_false_for_none_mode() {
    let ws = WorkspaceConfig {
        release_commit_mode: ReleaseCommitMode::None,
        skip_ci: None,
        ..WorkspaceConfig::default()
    };
    assert!(!ws.effective_skip_ci());
}

#[test]
fn effective_skip_ci_explicit_override() {
    let ws = WorkspaceConfig {
        release_commit_mode: ReleaseCommitMode::Commit,
        skip_ci: Some(false),
        ..WorkspaceConfig::default()
    };
    assert!(!ws.effective_skip_ci());

    let ws2 = WorkspaceConfig {
        release_commit_mode: ReleaseCommitMode::Pr,
        skip_ci: Some(true),
        ..WorkspaceConfig::default()
    };
    assert!(ws2.effective_skip_ci());
}

// -----------------------------------------------------------------------
// Config::load — discovery logic
// -----------------------------------------------------------------------

#[test]
fn load_discovers_json_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ferrflow.json"),
        r#"{"package":[{"name":"app","path":"."}]}"#,
    )
    .unwrap();
    let config = Config::load(dir.path(), None).unwrap();
    assert_eq!(config.packages[0].name, "app");
}

#[test]
fn load_discovers_toml_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ferrflow.toml"),
        "[[package]]\nname = \"myapp\"\npath = \".\"\n",
    )
    .unwrap();
    let config = Config::load(dir.path(), None).unwrap();
    assert_eq!(config.packages[0].name, "myapp");
}

#[test]
fn load_discovers_dotfile_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".ferrflow"),
        r#"{"package":[{"name":"dot","path":"."}]}"#,
    )
    .unwrap();
    let config = Config::load(dir.path(), None).unwrap();
    assert_eq!(config.packages[0].name, "dot");
}

#[test]
fn load_fails_on_multiple_config_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ferrflow.json"),
        r#"{"package":[{"name":"a","path":"."}]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("ferrflow.toml"),
        "[[package]]\nname = \"b\"\npath = \".\"\n",
    )
    .unwrap();
    let result = Config::load(dir.path(), None);
    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    assert!(err.contains("multiple config files"));
}

#[test]
fn load_falls_back_to_auto_detect() {
    let dir = tempfile::tempdir().unwrap();
    // No config file, but a Cargo.toml exists
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let config = Config::load(dir.path(), None).unwrap();
    assert_eq!(config.packages.len(), 1);
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Toml
    );
}

#[test]
fn load_with_explicit_path_overrides_discovery() {
    let dir = tempfile::tempdir().unwrap();
    // Put a decoy in the root
    std::fs::write(
        dir.path().join("ferrflow.json"),
        r#"{"package":[{"name":"decoy","path":"."}]}"#,
    )
    .unwrap();
    // Put the real config elsewhere
    let sub = dir.path().join("custom");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("my.json"),
        r#"{"package":[{"name":"real","path":"."}]}"#,
    )
    .unwrap();
    let config = Config::load(dir.path(), Some(&sub.join("my.json"))).unwrap();
    assert_eq!(config.packages[0].name, "real");
}

// -----------------------------------------------------------------------
// Auto-detect edge cases
// -----------------------------------------------------------------------

#[test]
fn auto_detect_version_txt() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("VERSION.txt"), "1.0.0\n").unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(config.packages.len(), 1);
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Txt
    );
    assert_eq!(config.packages[0].versioned_files[0].path, "VERSION.txt");
}

#[test]
fn auto_detect_version_no_ext() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("VERSION"), "1.0.0\n").unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].versioned_files[0].path, "VERSION");
}

#[test]
fn auto_detect_prefers_version_over_version_txt() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("VERSION"), "1.0.0\n").unwrap();
    std::fs::write(dir.path().join("VERSION.txt"), "1.0.0\n").unwrap();
    let config = Config::auto_detect(dir.path());
    // Should only pick one (VERSION, the first checked)
    let txt_files: Vec<_> = config.packages[0]
        .versioned_files
        .iter()
        .filter(|vf| vf.format == FileFormat::Txt)
        .collect();
    assert_eq!(txt_files.len(), 1);
    assert_eq!(txt_files[0].path, "VERSION");
}

#[test]
fn auto_detect_go_mod() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("go.mod"), "module example.com/foo\n").unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::GoMod
    );
}

#[test]
fn auto_detect_gradle() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle"), "version = \"1.0.0\"\n").unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Gradle
    );
}

#[test]
fn auto_detect_gradle_kts_preferred() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle"), "version = \"1.0.0\"\n").unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "version = \"1.0.0\"\n").unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(
        config.packages[0].versioned_files[0].path,
        "build.gradle.kts"
    );
}

#[test]
fn auto_detect_pyproject() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let config = Config::auto_detect(dir.path());
    assert_eq!(config.packages[0].versioned_files[0].path, "pyproject.toml");
    assert_eq!(
        config.packages[0].versioned_files[0].format,
        FileFormat::Toml
    );
}

#[test]
fn auto_detect_uses_dir_name_as_package_name() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let config = Config::auto_detect(dir.path());
    let dir_name = dir
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(config.packages[0].name, dir_name);
}

// -----------------------------------------------------------------------
// snake_to_camel
// -----------------------------------------------------------------------

#[test]
fn snake_to_camel_basic() {
    assert_eq!(snake_to_camel("tag_template"), "tagTemplate");
    assert_eq!(snake_to_camel("versioned_files"), "versionedFiles");
    assert_eq!(
        snake_to_camel("recover_missed_releases"),
        "recoverMissedReleases"
    );
}

#[test]
fn snake_to_camel_no_underscores() {
    assert_eq!(snake_to_camel("name"), "name");
    assert_eq!(snake_to_camel(""), "");
}

// -----------------------------------------------------------------------
// to_camel_case_keys
// -----------------------------------------------------------------------

#[test]
fn to_camel_case_keys_transforms_known_keys() {
    let input = serde_json::json!({
        "tag_template": "v{version}",
        "name": "test"
    });
    let output = to_camel_case_keys(input);
    assert!(output.get("tagTemplate").is_some());
    assert!(output.get("name").is_some());
    assert!(output.get("tag_template").is_none());
}

#[test]
fn to_camel_case_keys_nested() {
    let input = serde_json::json!({
        "package": [{
            "versioned_files": [],
            "shared_paths": []
        }]
    });
    let output = to_camel_case_keys(input);
    let pkg = &output["package"][0];
    assert!(pkg.get("versionedFiles").is_some());
    assert!(pkg.get("sharedPaths").is_some());
}

// -----------------------------------------------------------------------
// JSON5 roundtrip
// -----------------------------------------------------------------------

#[test]
fn json5_roundtrip() {
    let handler = Json5Format;
    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages: vec![make_pkg("test", None)],
    };
    let serialized = handler.serialize(&config).unwrap();
    let parsed = handler.parse(&serialized).unwrap();
    assert_eq!(parsed.packages[0].name, "test");
}

// -----------------------------------------------------------------------
// Dotfile roundtrip
// -----------------------------------------------------------------------

#[test]
fn dotfile_roundtrip() {
    let handler = DotfileFormat;
    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages: vec![make_pkg("test", None)],
    };
    let serialized = handler.serialize(&config).unwrap();
    let parsed = handler.parse(&serialized).unwrap();
    assert_eq!(parsed.packages[0].name, "test");
}

// -----------------------------------------------------------------------
// ReleaseCommitMode parsing
// -----------------------------------------------------------------------

#[test]
fn parse_release_commit_modes() {
    for (s, expected) in [
        ("commit", ReleaseCommitMode::Commit),
        ("pr", ReleaseCommitMode::Pr),
        ("none", ReleaseCommitMode::None),
    ] {
        let json = format!(r#"{{ "workspace": {{ "releaseCommitMode": "{s}" }}, "package": [] }}"#);
        let config: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.workspace.release_commit_mode, expected,
            "failed for {s}"
        );
    }
}

#[test]
fn parse_release_commit_scopes() {
    for (s, expected) in [
        ("grouped", ReleaseCommitScope::Grouped),
        ("per-package", ReleaseCommitScope::PerPackage),
    ] {
        let json =
            format!(r#"{{ "workspace": {{ "releaseCommitScope": "{s}" }}, "package": [] }}"#);
        let config: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.workspace.release_commit_scope, expected,
            "failed for {s}"
        );
    }
}

#[test]
fn release_commit_scope_defaults_to_grouped() {
    let json = r#"{ "workspace": {}, "package": [] }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(
        config.workspace.release_commit_scope,
        ReleaseCommitScope::Grouped
    );
}

#[test]
fn release_commit_scope_camel_case_alias() {
    let json = r#"{ "workspace": { "releaseCommitScope": "per-package" }, "package": [] }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(
        config.workspace.release_commit_scope,
        ReleaseCommitScope::PerPackage
    );
}

#[test]
fn defer_publish_defaults_false() {
    let json = r#"{ "workspace": {}, "package": [] }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(!config.workspace.defer_publish);
}

#[test]
fn defer_publish_parses_camel_case_alias() {
    let json = r#"{ "workspace": { "deferPublish": true }, "package": [] }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.workspace.defer_publish);
}

// -----------------------------------------------------------------------
// load_explicit with json5
// -----------------------------------------------------------------------

#[test]
fn load_explicit_json5() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.json5");
    std::fs::write(&path, "{ package: [{ name: \"x\", path: \".\" }] }").unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "x");
}

// -----------------------------------------------------------------------
// format_handler
// -----------------------------------------------------------------------

#[test]
fn format_handler_returns_correct_filenames() {
    assert_eq!(
        format_handler(ConfigFileFormat::Json).filename(),
        "ferrflow.json"
    );
    assert_eq!(
        format_handler(ConfigFileFormat::Json5).filename(),
        "ferrflow.json5"
    );
    assert_eq!(
        format_handler(ConfigFileFormat::Toml).filename(),
        "ferrflow.toml"
    );
    assert_eq!(
        format_handler(ConfigFileFormat::Dotfile).filename(),
        ".ferrflow"
    );
}

// -----------------------------------------------------------------------
// Config::is_monorepo edge case
// -----------------------------------------------------------------------

#[test]
fn is_monorepo_empty() {
    let config = Config {
        workspace: WorkspaceConfig::default(),
        packages: vec![],
    };
    assert!(!config.is_monorepo());
}

#[test]
fn load_fails_on_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ferrflow.json"), "{ invalid json").unwrap();
    assert!(Config::load(dir.path(), None).is_err());
}

#[test]
fn load_fails_on_invalid_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ferrflow.toml"), "[[[invalid").unwrap();
    assert!(Config::load(dir.path(), None).is_err());
}

#[test]
fn load_explicit_nonexistent_file() {
    let result = Config::load_explicit(std::path::Path::new("/nonexistent/ferrflow.json"));
    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    assert!(err.contains("not found") || err.contains("No such file"));
}

#[test]
fn load_explicit_unknown_extension_defaults_to_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.xyz");
    std::fs::write(&path, r#"{"package":[{"name":"x","path":"."}]}"#).unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "x");
}

#[test]
fn parse_json_ignores_unknown_fields() {
    let json = r#"{
            "workspace": { "remote": "origin", "unknown_field": true },
            "package": [{ "name": "app", "path": ".", "extra": "ignored" }]
        }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.packages[0].name, "app");
}

#[test]
fn default_workspace_config_values() {
    // Default trait gives empty strings; serde defaults give "origin"/"main"
    let ws = WorkspaceConfig::default();
    assert_eq!(ws.versioning, None);
    assert!(ws.tag_template.is_none());
    assert!(!ws.recover_missed_releases);
    assert_eq!(ws.release_commit_mode, ReleaseCommitMode::Commit);
    assert!(ws.skip_ci.is_none());
}

#[test]
fn serde_default_workspace_values() {
    // When deserialized from JSON with explicit workspace, serde defaults fill missing fields
    let json = r#"{"workspace":{"remote":"origin"},"package":[]}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.workspace.remote, "origin");
    assert!(config.workspace.anonymous_telemetry);
    assert!(config.workspace.auto_merge_releases);
}

#[test]
fn file_format_serde_all_variants() {
    for (s, expected) in [
        ("json", FileFormat::Json),
        ("toml", FileFormat::Toml),
        ("xml", FileFormat::Xml),
        ("gradle", FileFormat::Gradle),
        ("gomod", FileFormat::GoMod),
        ("txt", FileFormat::Txt),
    ] {
        let json = format!(r#"{{ "path": "test", "format": "{s}" }}"#);
        let vf: VersionedFile = serde_json::from_str(&json).unwrap();
        assert_eq!(vf.format, expected, "failed for format {s}");
    }
}

#[test]
fn depends_on_deserializes_from_json() {
    let json =
        r#"{"package":[{"name":"cli","path":"cli","dependsOn":["core"],"versionedFiles":[]}]}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.packages[0].depends_on[0].name(), "core");
}

#[test]
fn depends_on_defaults_to_empty() {
    let json = r#"{"package":[{"name":"cli","path":"cli","versionedFiles":[]}]}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.packages[0].depends_on.is_empty());
}

#[test]
fn depends_on_deserializes_snake_case() {
    let json =
        r#"{"package":[{"name":"cli","path":"cli","depends_on":["core"],"versionedFiles":[]}]}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.packages[0].depends_on[0].name(), "core");
}

#[test]
fn tag_prefix_no_version_placeholder() {
    let ws = WorkspaceConfig::default();
    let pkg = PackageConfig {
        name: "app".to_string(),
        path: ".".to_string(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: Some("release-latest".to_string()),
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    // When template has no {version}, prefix is the entire template
    assert_eq!(pkg.tag_prefix(&ws, false), "release-latest");
}

#[test]
fn tag_for_version_replaces_placeholders() {
    let ws = WorkspaceConfig::default();
    let pkg = PackageConfig {
        name: "api".to_string(),
        path: ".".to_string(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: Some("{name}/v{version}".to_string()),
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    };
    assert_eq!(pkg.tag_for_version(&ws, true, "1.2.3"), "api/v1.2.3");
}

#[test]
fn config_default_is_empty() {
    let config = Config::default();
    assert!(config.packages.is_empty());
}

#[test]
fn load_discovers_json5_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ferrflow.json5"),
        "{ package: [{ name: \"j5\", path: \".\" }] }",
    )
    .unwrap();
    let config = Config::load(dir.path(), None).unwrap();
    assert_eq!(config.packages[0].name, "j5");
}

#[test]
fn load_with_relative_explicit_path() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("custom.json"),
        r#"{"package":[{"name":"rel","path":"."}]}"#,
    )
    .unwrap();
    let config = Config::load(dir.path(), Some(std::path::Path::new("custom.json"))).unwrap();
    assert_eq!(config.packages[0].name, "rel");
}

#[test]
fn auto_detect_no_version_files() {
    let dir = tempfile::tempdir().unwrap();
    // Empty dir, no recognizable version files
    let config = Config::auto_detect(dir.path());
    assert!(config.packages.is_empty());
}

#[test]
fn snake_to_camel_multiple_underscores() {
    assert_eq!(snake_to_camel("a_b_c_d"), "aBCD");
}

#[test]
fn snake_to_camel_trailing_underscore() {
    assert_eq!(snake_to_camel("trailing_"), "trailing");
}

#[test]
fn to_camel_case_keys_preserves_non_object_values() {
    let input = serde_json::json!("string_value");
    assert_eq!(to_camel_case_keys(input.clone()), input);

    let input = serde_json::json!(42);
    assert_eq!(to_camel_case_keys(input.clone()), input);

    let input = serde_json::json!(true);
    assert_eq!(to_camel_case_keys(input.clone()), input);

    let input = serde_json::json!(null);
    assert_eq!(to_camel_case_keys(input.clone()), input);
}

#[test]
fn deserialize_branches_json() {
    let json = r#"{
            "workspace": {
                "branches": [
                    { "name": "main", "channel": false },
                    { "name": "develop", "channel": "dev" },
                    { "name": "beta", "channel": "beta", "prereleaseIdentifier": "timestamp" }
                ]
            },
            "package": []
        }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    let branches = config.workspace.branches.unwrap();
    assert_eq!(branches.len(), 3);
    assert!(matches!(branches[0].channel, ChannelValue::Stable(false)));
    assert!(matches!(&branches[1].channel, ChannelValue::Named(s) if s == "dev"));
    assert_eq!(
        branches[1].prerelease_identifier,
        PrereleaseIdentifier::Increment
    );
    assert_eq!(
        branches[2].prerelease_identifier,
        PrereleaseIdentifier::Timestamp
    );
}

#[test]
fn deserialize_branches_toml() {
    let toml_str = r#"
            [[workspace.branches]]
            name = "main"
            channel = false

            [[workspace.branches]]
            name = "develop"
            channel = "dev"
            prereleaseIdentifier = "short-hash"

            [[package]]
            name = "test"
            path = "."
        "#;
    let config: Config = toml_edit::de::from_str(toml_str).unwrap();
    let branches = config.workspace.branches.unwrap();
    assert_eq!(branches.len(), 2);
    assert!(matches!(branches[0].channel, ChannelValue::Stable(false)));
    assert_eq!(
        branches[1].prerelease_identifier,
        PrereleaseIdentifier::ShortHash
    );
}

#[test]
fn deserialize_no_branches_backward_compatible() {
    let json = r#"{ "workspace": {}, "package": [] }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.workspace.branches.is_none());
}

#[test]
fn channel_value_rejects_true() {
    let json = r#"{ "name": "main", "channel": true }"#;
    let config: BranchChannelConfig = serde_json::from_str(json).unwrap();
    assert!(matches!(config.channel, ChannelValue::Stable(true)));
}

// -----------------------------------------------------------------------
// JS/TS config loading (requires node/tsx on PATH)
// -----------------------------------------------------------------------

#[cfg(feature = "cli")]
#[test]
fn load_explicit_js_config() {
    // Skip if node is not available
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: node not found");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.js");
    std::fs::write(
            &path,
            r#"export default {
                workspace: { remote: "origin", branch: "main" },
                package: [{ name: "js-app", path: ".", versionedFiles: [{ path: "package.json", format: "json" }] }]
            };"#,
        )
        .unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "js-app");
}

#[cfg(feature = "cli")]
#[test]
fn load_explicit_js_async_function() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: node not found");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.js");
    std::fs::write(
        &path,
        r#"export default async () => ({
                workspace: { remote: "origin" },
                package: [{ name: "async-app", path: "." }]
            });"#,
    )
    .unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "async-app");
}

#[cfg(feature = "cli")]
#[test]
fn load_discovers_js_config() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: node not found");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ferrflow.js"),
        r#"export default { package: [{ name: "discovered-js", path: "." }] };"#,
    )
    .unwrap();
    let config = Config::load(dir.path(), None).unwrap();
    assert_eq!(config.packages[0].name, "discovered-js");
}

#[cfg(feature = "cli")]
#[test]
fn load_js_and_json_fails_multiple() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: node not found");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ferrflow.json"),
        r#"{"package":[{"name":"a","path":"."}]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("ferrflow.js"),
        r#"export default { package: [{ name: "b", path: "." }] };"#,
    )
    .unwrap();
    let result = Config::load(dir.path(), None);
    assert!(result.is_err());
    assert!(format!("{:?}", result.unwrap_err()).contains("multiple config files"));
}

#[test]
fn load_explicit_js_not_found() {
    let path = std::path::Path::new("/nonexistent/ferrflow.js");
    assert!(Config::load_explicit(path).is_err());
}

#[cfg(feature = "cli")]
#[test]
fn load_explicit_ts_config() {
    // Skip if tsx cannot actually execute a TS file (not just --version)
    let dir = tempfile::tempdir().unwrap();
    let probe = dir.path().join("probe.mts");
    std::fs::write(&probe, "process.stdout.write('ok');").unwrap();
    let tsx_works = std::process::Command::new("tsx")
        .arg(&probe)
        .output()
        .or_else(|_| {
            std::process::Command::new("npx")
                .args(["tsx"])
                .arg(&probe)
                .output()
        })
        .map(|o| o.status.success() && o.stdout == b"ok")
        .unwrap_or(false);

    if !tsx_works {
        eprintln!("Skipping: tsx cannot execute TS files");
        return;
    }

    let path = dir.path().join("ferrflow.ts");
    std::fs::write(
        &path,
        r#"const config = { package: [{ name: "ts-app", path: "." }] };
export default config;"#,
    )
    .unwrap();
    let config = match Config::load_explicit(&path) {
        Ok(c) => c,
        Err(e) => panic!("load_explicit failed: {e:?}"),
    };
    assert!(
        !config.packages.is_empty(),
        "Expected packages but got none. Config: {:?}",
        config
    );
    assert_eq!(config.packages[0].name, "ts-app");
}

#[cfg(feature = "cli")]
#[test]
fn load_explicit_js_function_hooks() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: node not found");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.js");
    std::fs::write(
        &path,
        r#"export default {
                workspace: {
                    hooks: {
                        postBump: (ctx) => { console.log(ctx.newVersion); },
                        preBump: "echo hello"
                    }
                },
                package: [{ name: "hook-app", path: "." }]
            };"#,
    )
    .unwrap();
    let config = Config::load_explicit(&path).unwrap();
    assert_eq!(config.packages[0].name, "hook-app");
    // String hook should remain as-is
    let hooks = config.workspace.hooks.unwrap();
    assert_eq!(hooks.pre_bump.as_deref(), Some("echo hello"));
    // Function hook should be converted to a node command
    let post_bump = hooks.post_bump.unwrap();
    assert!(
        post_bump.contains("node"),
        "function hook should be reified as a node command: {post_bump}"
    );
    assert!(post_bump.contains("postBump"));
}

#[cfg(feature = "cli")]
#[test]
fn path_to_file_url_unix_style() {
    // Test the URL conversion with a temp path
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.js");
    std::fs::write(&path, "").unwrap();
    let url = path_to_file_url(&path).unwrap();
    assert!(url.starts_with("file:///"));
    assert!(url.contains("test.js"));
    assert!(!url.contains('\\'));
}

// -----------------------------------------------------------------------
// publishers + registries (RFC v1 — parse/dry-run preview only)
// -----------------------------------------------------------------------

#[test]
fn registries_parse_in_workspace_section() {
    let json = r#"{
        "workspace": {
            "registries": {
                "kellnr": { "url": "https://kellnr.example.com", "tokenEnv": "CARGO_REGISTRIES_KELLNR_TOKEN" },
                "gh-packages": { "tokenEnv": "NODE_AUTH_TOKEN" }
            }
        },
        "package": []
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    let kellnr = cfg.workspace.registries.get("kellnr").unwrap();
    assert_eq!(kellnr.url.as_deref(), Some("https://kellnr.example.com"));
    assert_eq!(
        kellnr.token_env.as_deref(),
        Some("CARGO_REGISTRIES_KELLNR_TOKEN")
    );
    let gh = cfg.workspace.registries.get("gh-packages").unwrap();
    assert_eq!(gh.url, None);
    assert_eq!(gh.token_env.as_deref(), Some("NODE_AUTH_TOKEN"));
}

#[test]
fn publishers_cargo_kind_parses() {
    let json = r#"{
        "package": [{
            "name": "auth",
            "path": "crates/auth",
            "publishers": [
                { "kind": "cargo", "registry": "kellnr", "allowDirty": false, "noVerify": true }
            ]
        }]
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    let p = &cfg.packages[0].publishers[0];
    match p {
        PublisherConfig::Cargo {
            registry,
            allow_dirty,
            no_verify,
            ..
        } => {
            assert_eq!(registry.as_deref(), Some("kellnr"));
            assert!(!*allow_dirty);
            assert!(*no_verify);
        }
        _ => panic!("expected cargo kind"),
    }
}

#[test]
fn publishers_cargo_no_verify_defaults_false() {
    let json = r#"{
        "package": [{
            "name": "auth",
            "path": "crates/auth",
            "publishers": [{ "kind": "cargo" }]
        }]
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    match &cfg.packages[0].publishers[0] {
        PublisherConfig::Cargo { no_verify, .. } => assert!(!*no_verify),
        _ => panic!("expected cargo kind"),
    }
}

#[test]
fn publishers_args_parses_and_defaults_empty() {
    let json = r#"{
        "package": [{
            "name": "auth",
            "path": "crates/auth",
            "publishers": [
                { "kind": "cargo", "args": ["--locked", "--jobs", "2"] },
                { "kind": "npm" }
            ]
        }]
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    match &cfg.packages[0].publishers[0] {
        PublisherConfig::Cargo { args, .. } => {
            assert_eq!(args, &vec!["--locked", "--jobs", "2"]);
        }
        _ => panic!("expected cargo kind"),
    }
    match &cfg.packages[0].publishers[1] {
        PublisherConfig::Npm { args, .. } => assert!(args.is_empty()),
        _ => panic!("expected npm kind"),
    }
}

#[test]
fn publishers_docker_kind_has_defaults() {
    let json = r#"{
        "package": [{
            "name": "auth",
            "path": "crates/auth",
            "publishers": [
                { "kind": "docker", "image": "ghcr.io/ferrlabs/auth" }
            ]
        }]
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    match &cfg.packages[0].publishers[0] {
        PublisherConfig::Docker {
            image,
            tags,
            platforms,
            context,
            dockerfile,
            sign,
            ..
        } => {
            assert_eq!(image, "ghcr.io/ferrlabs/auth");
            assert_eq!(tags, &vec!["{version}".to_string()]);
            assert!(platforms.is_empty());
            assert_eq!(context, ".");
            assert_eq!(dockerfile, "Dockerfile");
            assert_eq!(*sign, crate::config::DockerSign::None);
        }
        _ => panic!("expected docker kind"),
    }
}

#[test]
fn publishers_describe_renders_dry_run_preview() {
    let cargo = PublisherConfig::Cargo {
        registry: Some("kellnr".to_string()),
        allow_dirty: false,
        no_verify: false,
        args: vec![],
    };
    assert_eq!(
        cargo.describe("ferrlabs-auth", "0.1.0"),
        "cargo publish ferrlabs-auth@0.1.0 → kellnr"
    );

    let docker = PublisherConfig::Docker {
        image: "ghcr.io/ferrlabs/auth".to_string(),
        tags: vec!["{version}".to_string(), "latest".to_string()],
        platforms: vec!["linux/amd64".to_string(), "linux/arm64".to_string()],
        context: ".".to_string(),
        dockerfile: "Dockerfile".to_string(),
        sign: crate::config::DockerSign::Sigstore,
        args: vec![],
    };
    let line = docker.describe("ferrlabs-auth", "0.1.0");
    assert!(line.contains("ghcr.io/ferrlabs/auth"));
    assert!(line.contains("0.1.0"));
    assert!(line.contains("latest"));
    assert!(line.contains("linux/amd64,linux/arm64"));
    assert!(line.contains("+sigstore"));

    let webhook = PublisherConfig::Webhook {
        url: "https://hooks.slack.com/x".to_string(),
        body: None,
        headers: Default::default(),
    };
    assert!(webhook.describe("auth", "0.1.0").contains("POST"));
    assert!(webhook.describe("auth", "0.1.0").contains("auth@0.1.0"));
}

#[test]
fn publishers_default_to_empty_vec_when_omitted() {
    let json = r#"{
        "package": [{ "name": "x", "path": "." }]
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert!(cfg.packages[0].publishers.is_empty());
}

#[test]
fn publishers_accepts_all_six_kinds_in_one_package() {
    let json = r#"{
        "package": [{
            "name": "kit",
            "path": "crates/kit",
            "publishers": [
                { "kind": "cargo" },
                { "kind": "npm", "tag": "beta", "access": "public" },
                { "kind": "docker", "image": "ghcr.io/x/kit" },
                { "kind": "helm", "registry": "oci://ghcr.io/x/charts" },
                { "kind": "github-release-asset", "path": "sbom.cdx.json" },
                { "kind": "webhook", "url": "https://example.com/x" }
            ]
        }]
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    let kinds: Vec<&str> = cfg.packages[0]
        .publishers
        .iter()
        .map(PublisherConfig::kind_name)
        .collect();
    assert_eq!(
        kinds,
        vec![
            "cargo",
            "npm",
            "docker",
            "helm",
            "github-release-asset",
            "webhook"
        ]
    );
}

#[test]
fn publishers_unknown_kind_is_rejected() {
    let json = r#"{
        "package": [{
            "name": "x",
            "path": ".",
            "publishers": [{ "kind": "telegram", "channel": "@foo" }]
        }]
    }"#;
    assert!(
        serde_json::from_str::<Config>(json).is_err(),
        "an unknown publisher kind must not silently parse"
    );
}
