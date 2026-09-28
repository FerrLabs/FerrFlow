use std::path::Path;

use super::merged_release_tags;
use crate::config::Config;
use crate::git::open_repo;
use crate::monorepo::run::summary::PlannedTag;
use crate::test_utils::{commit_file, git, init_repo};

fn single_config() -> Config {
    serde_json::from_str(
        r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"version.json","format":"json"}]}]}"#,
    )
    .unwrap()
}

fn version_json(version: &str) -> String {
    format!(r#"{{"name":"app","version":"{version}"}}"#)
}

fn pending(root: &Path, config: &Config, all_tags: &[&str]) -> Vec<PlannedTag> {
    let repo = open_repo(root).unwrap();
    let all_tags: Vec<String> = all_tags.iter().map(|t| t.to_string()).collect();
    merged_release_tags(&repo, config, root, &all_tags, None)
}

fn released_at(root: &Path, version: &str) {
    commit_file(
        root,
        "version.json",
        &version_json("1.0.0"),
        "chore: seed",
        1_990_000_000,
    );
    git(root, &["tag", "-a", "v1.0.0", "-m", "Release v1.0.0"]);
    commit_file(root, "a.txt", "a", "feat: a feature", 1_990_000_100);
    commit_file(
        root,
        "version.json",
        &version_json(version),
        &format!("chore(release): app v{version}"),
        1_990_000_200,
    );
}

#[test]
fn a_merged_release_commit_yields_the_tag_to_create() {
    let (dir, _repo) = init_repo();
    released_at(dir.path(), "1.1.0");

    let tags = pending(dir.path(), &single_config(), &["v1.0.0"]);

    assert_eq!(tags.len(), 1);
    let t = &tags[0];
    assert_eq!(t.tag, "v1.1.0");
    assert_eq!(t.message, "Release v1.1.0");
    assert_eq!(t.package, "app");
    assert_eq!(t.version, "1.1.0");
    assert_eq!(t.commit_count, 2);
    assert!(!t.is_prerelease);
    assert!(t.body.contains("a feature"), "{}", t.body);
}

#[test]
fn a_version_already_tagged_is_not_tagged_again() {
    let (dir, _repo) = init_repo();
    released_at(dir.path(), "1.1.0");

    let tags = pending(dir.path(), &single_config(), &["v1.0.0", "v1.1.0"]);

    assert!(tags.is_empty());
}

#[test]
fn a_bumped_file_without_a_release_commit_is_not_finalised() {
    let (dir, _repo) = init_repo();
    commit_file(
        dir.path(),
        "version.json",
        &version_json("1.0.0"),
        "chore: seed",
        1_991_000_000,
    );
    git(dir.path(), &["tag", "-a", "v1.0.0", "-m", "Release v1.0.0"]);
    commit_file(
        dir.path(),
        "version.json",
        &version_json("1.1.0"),
        "chore: hand-edited version",
        1_991_000_100,
    );

    let tags = pending(dir.path(), &single_config(), &["v1.0.0"]);

    assert!(tags.is_empty());
}

#[test]
fn a_merged_prerelease_is_flagged_as_one() {
    let (dir, _repo) = init_repo();
    released_at(dir.path(), "2.0.0-rc.1");

    let tags = pending(dir.path(), &single_config(), &["v1.0.0"]);

    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].tag, "v2.0.0-rc.1");
    assert!(tags[0].is_prerelease);
}

#[test]
fn a_package_without_a_readable_version_is_skipped() {
    let (dir, _repo) = init_repo();
    released_at(dir.path(), "1.1.0");
    let config: Config = serde_json::from_str(
        r#"{"package":[
            {"name":"app","path":".","versionedFiles":[{"path":"version.json","format":"json"}]},
            {"name":"ghost","path":"ghost","versionedFiles":[{"path":"ghost/version.json","format":"json"}]},
            {"name":"bare","path":"bare"}
        ]}"#,
    )
    .unwrap();

    let tags = pending(dir.path(), &config, &["v1.0.0"]);

    let names: Vec<&str> = tags.iter().map(|t| t.package.as_str()).collect();
    assert_eq!(names, vec!["app"]);
    assert_eq!(tags[0].tag, "app@v1.1.0");
}
