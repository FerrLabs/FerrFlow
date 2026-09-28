use super::*;
use crate::config::{Config, FileFormat, PackageConfig, VersionedFile, WorkspaceConfig};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use tempfile::TempDir;

fn make_config(packages: Vec<PackageConfig>) -> Config {
    Config {
        include: Vec::new(),
        workspace: WorkspaceConfig::default(),
        packages,
    }
}

fn make_package(name: &str, path: &str) -> PackageConfig {
    PackageConfig {
        build_metadata: None,
        version_source: None,
        name: name.to_string(),
        path: path.to_string(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        version_template: None,
        floating_tags: None,
        latest_tag: None,
        publishers: vec![],
        update_lockfiles: None,
        hooks: None,
    }
}

#[test]
fn local_source_read_existing_file() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("test.txt"), "hello").unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    assert_eq!(
        source.read_file("test.txt").unwrap(),
        Some(b"hello".to_vec())
    );
}

#[test]
fn local_source_read_missing_file() {
    let tmp = TempDir::new().unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    assert_eq!(source.read_file("nope.txt").unwrap(), None);
}

#[test]
fn local_source_path_exists() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("file.txt"), "x").unwrap();
    fs::create_dir(tmp.path().join("subdir")).unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    assert!(source.path_exists("file.txt").unwrap());
    assert!(source.path_exists("subdir").unwrap());
    assert!(!source.path_exists("nope.txt").unwrap());
}

#[test]
fn parse_repo_spec_github_short() {
    let (p, o, r) = parse_repo_spec("owner/repo").unwrap();
    assert_eq!(p, RemoteProvider::GitHub);
    assert_eq!(o, "owner");
    assert_eq!(r, "repo");
}

#[test]
fn parse_repo_spec_github_full() {
    let (p, o, r) = parse_repo_spec("github.com/owner/repo").unwrap();
    assert_eq!(p, RemoteProvider::GitHub);
    assert_eq!(o, "owner");
    assert_eq!(r, "repo");
}

#[test]
fn parse_repo_spec_gitlab() {
    let (p, o, r) = parse_repo_spec("gitlab.com/owner/repo").unwrap();
    assert_eq!(p, RemoteProvider::GitLab);
    assert_eq!(o, "owner");
    assert_eq!(r, "repo");
}

#[test]
fn parse_repo_spec_invalid() {
    assert!(parse_repo_spec("just-a-name").is_err());
}

#[test]
fn validation_result_valid_when_no_errors() {
    let result = ValidationResult::from_entries(vec![ValidationEntry {
        level: ValidationLevel::Warning,
        path: "test".to_string(),
        message: "just a warning".to_string(),
    }]);
    assert!(result.valid);
}

#[test]
fn validation_result_invalid_when_errors() {
    let result = ValidationResult::from_entries(vec![ValidationEntry {
        level: ValidationLevel::Error,
        path: "test".to_string(),
        message: "broken".to_string(),
    }]);
    assert!(!result.valid);
}

#[test]
fn load_config_local() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("ferrflow.json"),
        r#"{"package": [{"name": "app", "path": "."}]}"#,
    )
    .unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    let (config, filename) = load_config_from_source(&source, None).unwrap();
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].name, "app");
    assert_eq!(filename, "ferrflow.json");
}

#[test]
fn load_config_priority_order() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("ferrflow.json"),
        r#"{"package": [{"name": "json", "path": "."}]}"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join(".ferrflow"),
        r#"{"package": [{"name": "dotfile", "path": "."}]}"#,
    )
    .unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    let (config, _) = load_config_from_source(&source, None).unwrap();
    assert_eq!(config.packages[0].name, "json");
}

#[test]
fn load_config_not_found() {
    let tmp = TempDir::new().unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    assert!(load_config_from_source(&source, None).is_err());
}

#[test]
fn load_config_explicit_path() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("custom.json"),
        r#"{"package": [{"name": "custom", "path": "."}]}"#,
    )
    .unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    let (config, filename) = load_config_from_source(&source, Some("custom.json")).unwrap();
    assert_eq!(config.packages[0].name, "custom");
    assert_eq!(filename, "custom.json");
}

#[test]
fn pass_duplicate_names() {
    let config = make_config(vec![
        make_package("app", "packages/a"),
        make_package("app", "packages/b"),
    ]);
    let entries = check_duplicate_names(&config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Error);
    assert!(entries[0].message.contains("app"));
}

#[test]
fn pass_no_duplicate_names() {
    let config = make_config(vec![
        make_package("api", "packages/api"),
        make_package("web", "packages/web"),
    ]);
    assert!(check_duplicate_names(&config).is_empty());
}

#[test]
fn pass_duplicate_paths() {
    let config = make_config(vec![
        make_package("a", "packages/app"),
        make_package("b", "packages/app"),
    ]);
    let entries = check_duplicate_paths(&config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Error);
}

#[test]
fn pass_tag_template_missing_version() {
    let mut config = make_config(vec![make_package("app", ".")]);
    config.workspace.tag_template = Some("{name}-release".to_string());
    let entries = check_tag_templates(&config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Error);
    assert!(entries[0].message.contains("{version}"));
}

#[test]
fn pass_tag_template_missing_name_monorepo() {
    let mut config = make_config(vec![
        make_package("api", "packages/api"),
        make_package("web", "packages/web"),
    ]);
    config.workspace.tag_template = Some("v{version}".to_string());
    let entries = check_tag_templates(&config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Warning);
    assert!(entries[0].message.contains("{name}"));
}

#[test]
fn latest_tag_with_version_placeholder_is_an_error() {
    let mut config = make_config(vec![make_package("app", ".")]);
    config.workspace.latest_tag = Some("{name}@v{version}-latest".to_string());
    let entries = check_tag_templates(&config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Error);
    assert!(entries[0].message.contains("{version}"));
}

#[test]
fn latest_tag_without_name_in_a_monorepo_warns() {
    let mut config = make_config(vec![
        make_package("api", "packages/api"),
        make_package("web", "packages/web"),
    ]);
    config.workspace.latest_tag = Some("latest".to_string());
    let entries = check_tag_templates(&config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Warning);
    assert!(entries[0].message.contains("overwrite the same ref"));
}

#[test]
fn latest_tag_is_fine_bare_in_a_single_package_repo() {
    let mut config = make_config(vec![make_package("app", ".")]);
    config.workspace.latest_tag = Some("latest".to_string());
    assert!(check_tag_templates(&config).is_empty());
}

#[test]
fn pass_package_paths_exist() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("packages/api")).unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    let config = make_config(vec![make_package("api", "packages/api")]);
    assert!(check_package_paths(&config, &source).is_empty());
}

#[test]
fn pass_package_paths_missing() {
    let tmp = TempDir::new().unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    let config = make_config(vec![make_package("api", "packages/api")]);
    let entries = check_package_paths(&config, &source);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Error);
}

#[test]
fn pass_versioned_files_parseable() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("packages/api")).unwrap();
    fs::write(
        tmp.path().join("packages/api/package.json"),
        r#"{"name": "api", "version": "1.0.0"}"#,
    )
    .unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    let mut pkg = make_package("api", "packages/api");
    pkg.versioned_files = vec![VersionedFile {
        path: "packages/api/package.json".to_string(),
        format: FileFormat::Json,
        selector: None,
    }];
    let config = make_config(vec![pkg]);
    let (entries, versions) = check_versioned_files(&config, &source);
    assert!(entries.is_empty());
    assert_eq!(versions["api"].len(), 1);
    assert_eq!(versions["api"][0].1, "1.0.0");
}

#[test]
fn pass_version_consistency_mismatch() {
    let mut versions = HashMap::new();
    versions.insert(
        "app".to_string(),
        vec![
            ("package.json".to_string(), "1.0.0".to_string()),
            ("Cargo.toml".to_string(), "1.1.0".to_string()),
        ],
    );
    let entries = check_version_consistency(&versions);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, ValidationLevel::Error);
    assert!(entries[0].message.contains("1.0.0"));
    assert!(entries[0].message.contains("1.1.0"));
}

#[test]
fn pass_version_consistency_ok() {
    let mut versions = HashMap::new();
    versions.insert(
        "app".to_string(),
        vec![
            ("package.json".to_string(), "1.0.0".to_string()),
            ("Cargo.toml".to_string(), "1.0.0".to_string()),
        ],
    );
    assert!(check_version_consistency(&versions).is_empty());
}

#[test]
fn run_ref_without_repo_errors() {
    let result = run(None, false, None, Some("main"));
    assert!(result.is_err());
    assert!(format!("{:?}", result.unwrap_err()).contains("--ref"));
}

fn app_with_two_versioned_files() -> Config {
    let mut pkg = make_package("app", ".");
    pkg.versioned_files = vec![
        VersionedFile {
            path: "package.json".to_string(),
            format: FileFormat::Json,
            selector: None,
        },
        VersionedFile {
            path: "Cargo.toml".to_string(),
            format: FileFormat::Toml,
            selector: None,
        },
    ];
    make_config(vec![pkg])
}

#[test]
fn validate_files_flags_version_mismatch_across_provided_files() {
    let config = app_with_two_versioned_files();
    let mut files = BTreeMap::new();
    files.insert(
        "package.json".to_string(),
        br#"{"name":"app","version":"1.0.0"}"#.to_vec(),
    );
    files.insert(
        "Cargo.toml".to_string(),
        b"[package]\nname = \"app\"\nversion = \"1.1.0\"\n".to_vec(),
    );

    let result = validate_files(&config, files);

    assert!(!result.valid);
    assert_eq!(result.package_count, 1);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("1.0.0") && e.message.contains("1.1.0")),
        "expected a version-consistency error, got {:?}",
        result.errors
    );
}

#[test]
fn validate_files_passes_when_provided_versions_agree() {
    let config = app_with_two_versioned_files();
    let mut files = BTreeMap::new();
    files.insert(
        "package.json".to_string(),
        br#"{"name":"app","version":"2.3.4"}"#.to_vec(),
    );
    files.insert(
        "Cargo.toml".to_string(),
        b"[package]\nname = \"app\"\nversion = \"2.3.4\"\n".to_vec(),
    );

    let result = validate_files(&config, files);

    assert!(
        result.valid,
        "expected valid, got errors: {:?}",
        result.errors
    );
    assert_eq!(result.package_count, 1);
}

#[test]
fn validate_files_treats_a_package_dir_as_present_when_files_live_under_it() {
    let mut pkg = make_package("api", "packages/api");
    pkg.versioned_files = vec![VersionedFile {
        path: "packages/api/package.json".to_string(),
        format: FileFormat::Json,
        selector: None,
    }];
    let config = make_config(vec![pkg]);
    let mut files = BTreeMap::new();
    files.insert(
        "packages/api/package.json".to_string(),
        br#"{"name":"api","version":"1.0.0"}"#.to_vec(),
    );

    let result = validate_files(&config, files);

    assert!(
        result.valid,
        "expected valid, got errors: {:?}",
        result.errors
    );
    assert!(
        !result
            .errors
            .iter()
            .any(|e| e.message.contains("does not exist")),
        "package dir should count as present via its files, got {:?}",
        result.errors
    );
}

#[test]
fn validate_files_reports_missing_versioned_file() {
    let config = app_with_two_versioned_files();
    let mut files = BTreeMap::new();
    files.insert(
        "package.json".to_string(),
        br#"{"name":"app","version":"1.0.0"}"#.to_vec(),
    );

    let result = validate_files(&config, files);

    assert!(!result.valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("Cargo.toml") && e.message.contains("does not exist")),
        "expected a missing-file error, got {:?}",
        result.errors
    );
}

#[test]
fn validate_files_honours_a_txt_selector() {
    let mut pkg = make_package("probe", ".");
    pkg.versioned_files = vec![
        VersionedFile {
            path: "VERSION".to_string(),
            format: FileFormat::Txt,
            selector: None,
        },
        VersionedFile {
            path: "Settings.asset".to_string(),
            format: FileFormat::Txt,
            selector: Some("(?m)^bundleVersion: (.+)$".to_string()),
        },
    ];
    let config = make_config(vec![pkg]);

    let mut files = BTreeMap::new();
    files.insert(
        "VERSION".to_string(),
        b"1.2.3
"
        .to_vec(),
    );
    files.insert(
        "Settings.asset".to_string(),
        b"a: 1
bundleVersion: 1.2.3
b: 2
"
        .to_vec(),
    );

    let result = validate_files(&config, files);

    assert!(
        result.valid,
        "the selector picks 1.2.3 out of the file, so nothing mismatches: {:?}",
        result.errors
    );
}

fn code(err: &anyhow::Error) -> Option<String> {
    crate::error_code::code_from_error(err)
}

fn memory(files: &[(&str, &str)]) -> MemorySource {
    MemorySource::new(
        files
            .iter()
            .map(|(path, body)| (path.to_string(), body.as_bytes().to_vec()))
            .collect(),
    )
}

#[test]
fn toml_and_json5_configs_are_read_by_their_extension() {
    let source = memory(&[(
        "ferrflow.toml",
        "[[package]]\nname = \"from-toml\"\npath = \".\"\n",
    )]);
    let (config, file) = load_config_from_source(&source, None).unwrap();
    assert_eq!(file, "ferrflow.toml");
    assert_eq!(config.packages[0].name, "from-toml");

    let source = memory(&[(
        "ferrflow.json5",
        "{ package: [{ name: \"from-json5\", path: \".\", },], }",
    )]);
    let (config, file) = load_config_from_source(&source, None).unwrap();
    assert_eq!(file, "ferrflow.json5");
    assert_eq!(config.packages[0].name, "from-json5");
}

#[test]
fn json5_wins_over_toml_when_there_is_no_plain_json() {
    let source = memory(&[
        (
            "ferrflow.toml",
            "[[package]]\nname = \"toml\"\npath = \".\"\n",
        ),
        (
            "ferrflow.json5",
            "{ package: [{ name: \"json5\", path: \".\" }] }",
        ),
    ]);
    let (config, _) = load_config_from_source(&source, None).unwrap();
    assert_eq!(config.packages[0].name, "json5");
}

#[test]
fn a_broken_config_is_a_parse_error_not_a_missing_one() {
    let source = memory(&[("ferrflow.json", "{ not json")]);
    let err = load_config_from_source(&source, None).unwrap_err();
    assert_eq!(code(&err).as_deref(), Some("E1104"));
    assert!(format!("{err:#}").contains("ferrflow.json"), "{err:#}");
}

#[test]
fn a_config_that_is_not_utf8_is_rejected_with_its_own_code() {
    let mut files = BTreeMap::new();
    files.insert("ferrflow.json".to_string(), vec![0xff, 0xfe, b'{']);
    let err = load_config_from_source(&MemorySource::new(files), None).unwrap_err();
    assert_eq!(code(&err).as_deref(), Some("E1103"));
}

#[test]
fn missing_config_codes_differ_for_explicit_and_discovered_paths() {
    let empty = memory(&[]);
    let explicit = load_config_from_source(&empty, Some("custom.json")).unwrap_err();
    assert_eq!(code(&explicit).as_deref(), Some("E1105"));
    assert!(
        format!("{explicit:#}").contains("custom.json"),
        "{explicit:#}"
    );

    let discovered = load_config_from_source(&empty, None).unwrap_err();
    assert_eq!(code(&discovered).as_deref(), Some("E1106"));
    let message = format!("{discovered:#}");
    for name in [
        "ferrflow.json",
        "ferrflow.json5",
        "ferrflow.toml",
        ".ferrflow",
    ] {
        assert!(message.contains(name), "{message}");
    }
}

#[test]
fn an_explicit_path_is_used_even_when_a_default_config_exists() {
    let source = memory(&[
        (
            "ferrflow.json",
            r#"{"package": [{"name": "default", "path": "."}]}"#,
        ),
        (
            "ci/release.toml",
            "[[package]]\nname = \"ci\"\npath = \".\"\n",
        ),
    ]);
    let (config, file) = load_config_from_source(&source, Some("ci/release.toml")).unwrap();
    assert_eq!(file, "ci/release.toml");
    assert_eq!(config.packages[0].name, "ci");
}

#[test]
fn memory_source_treats_only_real_path_prefixes_as_directories() {
    let source = memory(&[("pkg/src/lib.rs", ""), ("pkg-other/file", "")]);
    assert!(source.path_exists("pkg").unwrap());
    assert!(source.path_exists("pkg/").unwrap());
    assert!(source.path_exists("pkg/src/lib.rs").unwrap());
    assert!(
        !source.path_exists("pk").unwrap(),
        "a name prefix is not a directory"
    );
    assert!(!source.path_exists("pkg/src/lib").unwrap());
    assert_eq!(
        source.read_file("pkg").unwrap(),
        None,
        "a directory has no content"
    );
}

#[test]
fn local_source_surfaces_read_errors_other_than_not_found() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("adir")).unwrap();
    let source = LocalSource {
        root: tmp.path().to_path_buf(),
    };
    assert!(
        source.read_file("adir").is_err(),
        "reading a directory must fail loudly, not look like a missing file"
    );
}

#[test]
fn local_entries_flag_a_package_path_missing_on_disk() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("present")).unwrap();
    let config = make_config(vec![
        make_package("present", "present"),
        make_package("gone", "gone"),
    ]);
    let entries = local_entries(&config, tmp.path());
    let errors: Vec<&ValidationEntry> = entries
        .iter()
        .filter(|e| e.level == ValidationLevel::Error)
        .collect();
    assert_eq!(errors.len(), 1, "{entries:?}");
    assert!(errors[0].message.contains("gone"), "{:?}", errors[0]);
}

#[test]
fn an_invalid_repo_spec_fails_before_any_request() {
    let err = run(None, true, Some("a/b/c/d"), None).unwrap_err();
    assert_eq!(code(&err).as_deref(), Some("E1100"));
}

#[test]
fn ref_without_repo_carries_its_error_code() {
    let err = run(None, false, None, Some("main")).unwrap_err();
    assert_eq!(code(&err).as_deref(), Some("E1107"));
}

#[test]
fn run_validates_the_local_repo_config_in_both_output_modes() {
    use crate::test_utils::{commit_file, init_repo, with_cwd};

    let (dir, _repo) = init_repo();
    let root = dir.path();
    fs::write(
        root.join("ferrflow.json"),
        r#"{"package": [{"name": "app", "path": ".",
            "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}]}]}"#,
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    commit_file(root, "seed.txt", "x", "chore: seed", 1_940_000_000);

    with_cwd(root, || run(None, true, None, None)).unwrap();
    with_cwd(root, || run(None, false, None, None)).unwrap();
}
