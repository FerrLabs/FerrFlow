use std::collections::HashSet;

use super::{auto_stage_new_files, collect_dirty_files, tags_for_package};
use crate::test_utils::{commit_file, git, init_repo};

fn set(paths: &[&str]) -> HashSet<String> {
    paths.iter().map(|p| p.to_string()).collect()
}

#[test]
fn tags_are_matched_by_the_whole_package_prefix() {
    let tags: Vec<String> = [
        "api@v1.0.0",
        "api-client@v2.0.0",
        "web@v1.0.0",
        "api@v1.1.0",
    ]
    .iter()
    .map(|t| t.to_string())
    .collect();

    assert_eq!(
        tags_for_package(&tags, "api@v"),
        ["api@v1.0.0", "api@v1.1.0"]
    );
    assert_eq!(
        tags_for_package(&tags, "api-client@v"),
        ["api-client@v2.0.0"]
    );
    assert!(tags_for_package(&tags, "docs@v").is_empty());
}

#[test]
fn a_clean_tree_has_no_dirty_files() {
    let (dir, repo) = init_repo();
    commit_file(dir.path(), "a.txt", "x", "chore: seed", 1_900_000_000);
    assert!(collect_dirty_files(&repo).is_empty());
}

#[test]
fn untracked_modified_and_staged_files_are_all_dirty() {
    let (dir, repo) = init_repo();
    let root = dir.path();
    commit_file(root, "tracked.txt", "x", "chore: seed", 1_900_000_000);
    commit_file(root, "staged.txt", "x", "chore: seed", 1_900_000_001);
    std::fs::create_dir_all(root.join("sub dir")).unwrap();
    commit_file(root, "sub dir/kept.txt", "x", "chore: seed", 1_900_000_002);

    std::fs::write(root.join("tracked.txt"), "changed").unwrap();
    std::fs::write(root.join("staged.txt"), "changed").unwrap();
    git(root, &["add", "staged.txt"]);
    std::fs::write(root.join("added.txt"), "new").unwrap();
    git(root, &["add", "added.txt"]);
    std::fs::write(root.join("sub dir/new file.txt"), "new").unwrap();

    assert_eq!(
        collect_dirty_files(&repo),
        set(&[
            "tracked.txt",
            "staged.txt",
            "added.txt",
            "sub dir/new file.txt"
        ])
    );
}

#[test]
fn only_files_a_step_dirtied_are_added_to_the_commit_and_never_twice() {
    let (dir, repo) = init_repo();
    let root = dir.path();
    commit_file(root, "a.txt", "x", "chore: seed", 1_900_000_000);
    std::fs::write(root.join("preexisting.txt"), "dirty before").unwrap();
    let before = collect_dirty_files(&repo);

    std::fs::write(root.join("a.txt"), "bumped").unwrap();
    std::fs::write(root.join("generated.lock"), "new").unwrap();
    let mut files_to_commit = vec!["a.txt".to_string()];

    auto_stage_new_files(&repo, &before, &mut files_to_commit);

    assert_eq!(files_to_commit.len(), 2, "{files_to_commit:?}");
    assert!(files_to_commit.contains(&"a.txt".to_string()));
    assert!(files_to_commit.contains(&"generated.lock".to_string()));
    assert!(
        !files_to_commit.contains(&"preexisting.txt".to_string()),
        "a file that was already dirty is the user's, not the release's"
    );
}
