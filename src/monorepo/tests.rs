use super::util::*;
use crate::config::PackageConfig;

#[test]
fn pick_higher_semver_prefers_tag_when_tag_is_higher() {
    assert_eq!(pick_higher_semver("2.0.0", "3.0.0"), "3.0.0");
}

#[test]
fn pick_higher_semver_prefers_file_when_file_is_higher() {
    assert_eq!(pick_higher_semver("5.0.0", "2.0.0"), "5.0.0");
}

#[test]
fn pick_higher_semver_returns_tag_on_equality() {
    assert_eq!(pick_higher_semver("2.1.0", "2.1.0"), "2.1.0");
}

#[test]
fn pick_higher_semver_falls_back_to_tag_when_file_is_invalid() {
    assert_eq!(pick_higher_semver("garbage", "2.0.0"), "2.0.0");
}

#[test]
fn pick_higher_semver_falls_back_to_file_when_tag_is_invalid() {
    assert_eq!(pick_higher_semver("2.0.0", "garbage"), "2.0.0");
}

#[test]
fn pick_higher_semver_strips_leading_v() {
    assert_eq!(pick_higher_semver("v2.0.0", "v3.0.0"), "v3.0.0");
}

fn make_pkg(name: &str, path: &str, shared: &[&str]) -> PackageConfig {
    PackageConfig {
        name: name.into(),
        path: path.into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: shared.iter().map(|s| s.to_string()).collect(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
    }
}

#[test]
fn single_package_always_touched() {
    let pkg = make_pkg("app", ".", &[]);
    let files = vec!["README.md".to_string()];
    assert!(is_package_touched(&pkg, &files, false));
}

#[test]
fn monorepo_root_package_always_touched() {
    let pkg = make_pkg("root", ".", &[]);
    let files = vec!["something.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_package_touched_by_own_files() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["packages/api/src/main.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_package_not_touched_by_other_files() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["packages/site/index.ts".to_string()];
    assert!(!is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_package_touched_by_shared_path() {
    let pkg = make_pkg("api", "packages/api", &["packages/shared/"]);
    let files = vec!["packages/shared/types.ts".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_shared_path_trailing_slash_trimmed() {
    let pkg = make_pkg("api", "packages/api", &["lib/"]);
    let files = vec!["lib/utils.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_no_changed_files() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files: Vec<String> = vec![];
    assert!(!is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_path_with_dot_slash_prefix() {
    let pkg = make_pkg("api", "./packages/api", &[]);
    let files = vec!["packages/api/src/main.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn single_package_mode_always_touched() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["unrelated/file.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, false));
}

#[test]
fn monorepo_empty_path_is_root() {
    let pkg = make_pkg("root", "", &[]);
    let files = vec!["anything.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_exact_shared_path_file() {
    let pkg = make_pkg("api", "packages/api", &["shared-config.json"]);
    let files = vec!["shared-config.json".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_multiple_shared_paths() {
    let pkg = make_pkg("api", "packages/api", &["lib/", "proto/"]);
    let files = vec!["proto/schema.proto".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_similar_prefix_no_false_positive() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["packages/api-docs/README.md".to_string()];
    assert!(!is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_shared_path_with_trailing_slash() {
    let pkg = make_pkg("api", "packages/api", &["packages/shared/"]);
    let files = vec!["packages/shared/types.ts".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_empty_changed_files_single_package() {
    let pkg = make_pkg("app", "packages/app", &[]);
    let files: Vec<String> = vec![];
    assert!(is_package_touched(&pkg, &files, false));
}

#[test]
fn parse_force_version_single_repo() {
    let fv = "1.2.3";
    let result: Option<(Option<&str>, &str)> = if let Some(at_pos) = fv.find('@') {
        let name = &fv[..at_pos];
        let version = &fv[at_pos + 1..];
        Some((Some(name), version))
    } else {
        Some((None, fv))
    };
    assert_eq!(result, Some((None, "1.2.3")));
}

#[test]
fn parse_force_version_monorepo() {
    let fv = "api@2.0.0";
    let result: Option<(Option<&str>, &str)> = if let Some(at_pos) = fv.find('@') {
        let name = &fv[..at_pos];
        let version = &fv[at_pos + 1..];
        Some((Some(name), version))
    } else {
        Some((None, fv))
    };
    assert_eq!(result, Some((Some("api"), "2.0.0")));
}

#[test]
fn parse_force_version_with_v_prefix() {
    let fv = "v3.0.0";
    let clean = fv.strip_prefix('v').unwrap_or(fv);
    assert!(semver::Version::parse(clean).is_ok());
}

#[test]
fn parse_force_version_invalid_semver() {
    let fv = "not-a-version";
    let clean = fv.strip_prefix('v').unwrap_or(fv);
    assert!(semver::Version::parse(clean).is_err());
}
