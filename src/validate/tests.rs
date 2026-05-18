use super::checks::*;
use super::*;
use crate::config::{Config, FileFormat, PackageConfig, VersionedFile, WorkspaceConfig};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn make_config(packages: Vec<PackageConfig>) -> Config {
    Config {
        workspace: WorkspaceConfig::default(),
        packages,
    }
}

fn make_package(name: &str, path: &str) -> PackageConfig {
    PackageConfig {
        name: name.to_string(),
        path: path.to_string(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        floating_tags: None,
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
