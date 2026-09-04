use super::auth::{configure_git_command, extract_url_password, server_config_url, token_for_url};
use super::push::{local_tag_target_sha, parse_ls_remote_tags, remote_tag_target_shas};
use super::repo::Repository;
use super::retry::{is_transient_git_error, retry_transient};
use super::tags::{find_highest_semver_tag, find_last_tag, is_floating_tag, is_prerelease_tag};
use super::*;
use crate::config::OrphanedTagStrategy;
use crate::error_code;
use crate::test_utils::{git, git_with_env, init_repo_at};
use std::path::{Path, PathBuf};

#[test]
fn is_transient_classifies_network_errors_as_retryable() {
    let err = anyhow::anyhow!("Failed to push tags").context("connection reset by peer");
    assert!(is_transient_git_error(&err));

    let err = anyhow::anyhow!("502 Bad Gateway");
    assert!(is_transient_git_error(&err));

    let err = anyhow::anyhow!("ssl handshake failed");
    assert!(is_transient_git_error(&err));

    let err = anyhow::anyhow!("secondary rate limit exceeded");
    assert!(is_transient_git_error(&err));
}

#[test]
fn is_transient_classifies_libgit2_odb_staleness_as_retryable() {
    let err = anyhow::anyhow!("Failed to push tags")
        .context("object is no commit object; class=Invalid (3)");
    assert!(is_transient_git_error(&err));

    let err = anyhow::anyhow!("object not found - no match for id");
    assert!(is_transient_git_error(&err));

    let err = anyhow::anyhow!("odb read failed");
    assert!(is_transient_git_error(&err));
}

#[test]
fn is_transient_does_not_retry_terminal_errors() {
    let err = anyhow::anyhow!("non-fast-forward update rejected");
    assert!(!is_transient_git_error(&err));

    let err = anyhow::anyhow!("branch protection rule blocks this push");
    assert!(!is_transient_git_error(&err));

    let err = anyhow::anyhow!("authentication failed: bad token");
    assert!(!is_transient_git_error(&err));

    let err = anyhow::anyhow!("something completely unexpected happened");
    assert!(!is_transient_git_error(&err));
}

#[test]
fn retry_transient_succeeds_on_second_attempt() {
    use std::cell::Cell;
    let attempts = Cell::new(0);
    let result = retry_transient("test", || {
        attempts.set(attempts.get() + 1);
        if attempts.get() == 1 {
            Err(anyhow::anyhow!("connection timed out"))
        } else {
            Ok(())
        }
    });
    assert!(result.is_ok());
    assert_eq!(attempts.get(), 2);
}

#[test]
fn retry_transient_returns_immediately_on_terminal_error() {
    use std::cell::Cell;
    let attempts = Cell::new(0);
    let result = retry_transient("test", || {
        attempts.set(attempts.get() + 1);
        Err(anyhow::anyhow!("non-fast-forward"))
    });
    assert!(result.is_err());
    assert_eq!(attempts.get(), 1);
}

#[test]
fn parse_ls_remote_tags_returns_empty_for_empty_input() {
    let map = parse_ls_remote_tags("");
    assert!(map.is_empty());
}

#[test]
fn parse_ls_remote_tags_extracts_lightweight_tag_sha() {
    let input = "0123456789abcdef0123456789abcdef01234567\trefs/tags/site@v0.13.0\n";
    let map = parse_ls_remote_tags(input);
    assert_eq!(
        map.get("site@v0.13.0").map(String::as_str),
        Some("0123456789abcdef0123456789abcdef01234567")
    );
}

#[test]
fn parse_ls_remote_tags_prefers_dereferenced_commit_for_annotated_tag() {
    let input = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\trefs/tags/site@v0.13.0\n\
                 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\trefs/tags/site@v0.13.0^{}\n";
    let map = parse_ls_remote_tags(input);
    assert_eq!(
        map.get("site@v0.13.0").map(String::as_str),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
}

#[test]
fn parse_ls_remote_tags_handles_multiple_tags_mixed_types() {
    let input = "1111111111111111111111111111111111111111\trefs/tags/lightweight-tag\n\
                 2222222222222222222222222222222222222222\trefs/tags/annotated-tag\n\
                 3333333333333333333333333333333333333333\trefs/tags/annotated-tag^{}\n\
                 4444444444444444444444444444444444444444\trefs/heads/main\n";
    let map = parse_ls_remote_tags(input);
    assert_eq!(map.len(), 2);
    assert_eq!(
        map.get("lightweight-tag").map(String::as_str),
        Some("1111111111111111111111111111111111111111")
    );
    assert_eq!(
        map.get("annotated-tag").map(String::as_str),
        Some("3333333333333333333333333333333333333333"),
        "annotated tag should resolve to the dereferenced commit, not the tag object"
    );
}

fn init_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_repo_at(dir.path());
    (dir, repo)
}

static COMMIT_TIME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1_700_000_000);

fn next_commit_ts() -> i64 {
    COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

fn create_commit_in_repo(_repo: &Repository, dir: &Path, filename: &str, message: &str) {
    std::fs::write(dir.join(filename), format!("content of {filename}")).unwrap();
    git(dir, &["add", "--", filename]);
    let ts = next_commit_ts();
    let date = format!("{ts} +0000");
    git_with_env(
        dir,
        &["commit", "-m", message],
        &[("GIT_AUTHOR_DATE", &date), ("GIT_COMMITTER_DATE", &date)],
    );
}

fn create_lightweight_tag(repo: &Repository, tag_name: &str) {
    let workdir = repo.workdir().expect("workdir");
    git(workdir, &["tag", tag_name]);
}

fn create_annotated_tag(repo: &Repository, tag_name: &str, message: &str) {
    let workdir = repo.workdir().expect("workdir");
    git(workdir, &["tag", "-a", tag_name, "-m", message]);
}

#[test]
fn local_tag_target_sha_peels_annotated_tags_to_the_commit() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: a");
    create_annotated_tag(&repo, "v1.0.0", "Release 1.0.0");

    let head = git(dir.path(), &["rev-parse", "HEAD"]).trim().to_string();
    let tag_obj = git(dir.path(), &["rev-parse", "refs/tags/v1.0.0"])
        .trim()
        .to_string();
    assert_ne!(head, tag_obj, "annotated tag must be its own object");

    let sha = local_tag_target_sha(&repo, "v1.0.0").expect("resolves");
    assert_eq!(sha, head, "must peel to the commit, not the tag object");
}

#[test]
fn local_tag_target_sha_resolves_lightweight_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: a");
    create_lightweight_tag(&repo, "v1.0.0");

    let head = git(dir.path(), &["rev-parse", "HEAD"]).trim().to_string();
    let sha = local_tag_target_sha(&repo, "v1.0.0").expect("resolves");
    assert_eq!(sha, head);
}

#[test]
fn local_tag_target_sha_errors_on_a_missing_tag() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: a");
    drop(dir);
    assert!(local_tag_target_sha(&repo, "v9.9.9").is_err());
}

#[test]
fn open_repo_valid() {
    let (dir, _) = init_repo();
    let repo = open_repo(dir.path()).unwrap();
    assert!(repo.workdir().is_some());
}

#[test]
fn open_repo_not_a_repo() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("not_a_repo");
    std::fs::create_dir_all(&sub).unwrap();
    assert!(open_repo(&sub).is_err());
}

#[test]
fn get_repo_root_returns_workdir() {
    let (dir, repo) = init_repo();
    let root = get_repo_root(&repo).unwrap();
    assert_eq!(
        root.canonicalize().unwrap(),
        dir.path().canonicalize().unwrap()
    );
}

#[test]
fn tag_exists_false_when_no_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "file.txt", "initial");
    assert!(!tag_exists(&repo, "v1.0.0"));
}

#[test]
fn tag_exists_true_after_creation() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "file.txt", "initial");
    create_lightweight_tag(&repo, "v1.0.0");
    assert!(tag_exists(&repo, "v1.0.0"));
}

#[test]
fn create_tag_works() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "file.txt", "initial");
    create_tag(&repo, "v1.0.0", "Release v1.0.0").unwrap();
    assert!(tag_exists(&repo, "v1.0.0"));
}

#[test]
fn create_tag_fails_if_exists() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "file.txt", "initial");
    create_tag(&repo, "v1.0.0", "Release v1.0.0").unwrap();
    assert!(create_tag(&repo, "v1.0.0", "Duplicate").is_err());
}

#[test]
fn find_last_tag_name_no_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "file.txt", "initial");
    assert_eq!(
        find_last_tag_name(&repo, "v", OrphanedTagStrategy::Warn).unwrap(),
        None
    );
}

#[test]
fn find_last_tag_name_with_prefix() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
    create_lightweight_tag(&repo, "v1.1.0");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "third");
    create_lightweight_tag(&repo, "other-tag");

    let result = find_last_tag_name(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert_eq!(result, Some("v1.1.0".to_string()));
}

#[test]
fn find_last_tag_name_annotated() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_annotated_tag(&repo, "v1.0.0", "Release 1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
    create_annotated_tag(&repo, "v2.0.0", "Release 2.0.0");

    let result = find_last_tag_name(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert_eq!(result, Some("v2.0.0".to_string()));
}

#[test]
fn find_last_tag_name_monorepo_prefix() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "api@v1.0.0");
    create_lightweight_tag(&repo, "site@v2.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
    create_lightweight_tag(&repo, "api@v1.1.0");

    assert_eq!(
        find_last_tag_name(&repo, "api@v", OrphanedTagStrategy::Warn).unwrap(),
        Some("api@v1.1.0".to_string())
    );
    assert_eq!(
        find_last_tag_name(&repo, "site@v", OrphanedTagStrategy::Warn).unwrap(),
        Some("site@v2.0.0".to_string())
    );
}

#[test]
fn find_highest_semver_returns_none_when_no_matching_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "other@v1.0.0");

    let result = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn).unwrap();
    assert_eq!(result, None);
}

#[test]
fn find_highest_semver_picks_highest_not_latest_in_time() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "api@v3.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
    create_lightweight_tag(&repo, "api@v2.1.0");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "third");
    create_lightweight_tag(&repo, "api@v2.2.0");

    let (tag_name, version) = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn)
        .unwrap()
        .expect("a tag should be found");
    assert_eq!(tag_name, "api@v3.0.0");
    assert_eq!(version, "3.0.0");
}

#[test]
fn find_highest_semver_strips_prefix_and_v() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "api@v1.2.3");

    let (_, version) = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(version, "1.2.3");
}

#[test]
fn find_highest_semver_ignores_prerelease_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "api@v2.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
    create_lightweight_tag(&repo, "api@v3.0.0-rc.1");

    let (tag_name, version) = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(tag_name, "api@v2.0.0");
    assert_eq!(version, "2.0.0");
}

#[test]
fn find_highest_semver_ignores_floating_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_lightweight_tag(&repo, "v1");
    create_lightweight_tag(&repo, "latest");

    let (tag_name, _) = find_highest_semver_tag(&repo, "v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(tag_name, "v1.0.0");
}

#[test]
fn find_highest_semver_skips_non_semver_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "api@vnightly");
    create_lightweight_tag(&repo, "api@v1.0.0");

    let (tag_name, _) = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(tag_name, "api@v1.0.0");
}

#[test]
fn find_highest_semver_respects_orphan_warn_strategy() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "api@v1.0.0");
    let initial_sha = git(dir.path(), &["rev-parse", "HEAD"]).trim().to_string();
    git(dir.path(), &["checkout", "-b", "orphan"]);
    create_commit_in_repo(&repo, dir.path(), "b.txt", "orphan-only");
    create_lightweight_tag(&repo, "api@v9.0.0");
    git(dir.path(), &["checkout", "main"]);
    git(dir.path(), &["update-ref", "refs/heads/main", &initial_sha]);
    git(dir.path(), &["checkout", "-f", "main"]);

    let repo = open_repo(dir.path()).unwrap();
    let result = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(result.0, "api@v1.0.0");
}

#[test]
fn get_commits_since_last_tag_no_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "fix: second");

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].message.trim(), "fix: second");
    assert_eq!(commits[1].message.trim(), "feat: first");
}

#[test]
fn get_commits_since_last_tag_with_tag() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "fix: second");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: third");

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 2);
    assert!(commits[0].message.contains("third"));
    assert!(commits[1].message.contains("second"));
}

#[test]
fn get_commits_skips_skip_ci() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "chore(release): bump [skip ci]");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: real change");

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 1);
    assert!(commits[0].message.contains("real change"));
}

#[test]
fn get_commits_skips_ci_skip_variant() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "chore(release): bump [ci skip]");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: real change");

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 1);
    assert!(commits[0].message.contains("real change"));
}

#[test]
fn get_commits_skip_marker_is_case_insensitive() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "chore(release): bump [SKIP CI]");

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 0);
}

#[test]
fn get_commits_skip_marker_ignored_in_body() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(
        &repo,
        dir.path(),
        "b.txt",
        "feat: document CI behavior\n\nThis commit explains how [skip ci] works.",
    );

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 1);
}

#[test]
fn get_commits_skip_markers_can_be_overridden() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: real [skip ci]");

    let custom = vec!["[no release]".to_string()];
    let commits =
        get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::Warn, &custom, None).unwrap();
    assert_eq!(
        commits.len(),
        1,
        "[skip ci] not in custom set, should not skip"
    );
}

#[test]
fn subject_has_skip_marker_subject_only() {
    use crate::git::subject_has_skip_marker;
    let markers = crate::config::default_commit_skip_markers();
    assert!(subject_has_skip_marker("chore: bump [skip ci]", &markers));
    assert!(subject_has_skip_marker("chore: bump [SKIP CI]", &markers));
    assert!(subject_has_skip_marker("chore: bump [ci skip]", &markers));
    assert!(subject_has_skip_marker("chore: bump [no ci]", &markers));
    assert!(subject_has_skip_marker(
        "chore: bump [skip actions]",
        &markers
    ));
    assert!(subject_has_skip_marker(
        "chore: bump [actions skip]",
        &markers
    ));
    assert!(!subject_has_skip_marker(
        "feat: documentation\n\nbody mentions [skip ci] only",
        &markers
    ));
    assert!(!subject_has_skip_marker("feat: normal commit", &markers));
    assert!(!subject_has_skip_marker("", &markers));
    assert!(!subject_has_skip_marker("anything", &[]));
}

#[test]
fn get_changed_files_initial_commit() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "hello.txt", "initial");

    let files = get_changed_files(&repo).unwrap();
    assert!(files.contains(&"hello.txt".to_string()));
}

#[test]
fn get_changed_files_subsequent_commit() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");

    let files = get_changed_files(&repo).unwrap();
    assert_eq!(files, vec!["b.txt".to_string()]);
}

#[test]
fn get_changed_files_since_tag_all_when_no_tag() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");

    let files = get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn, None).unwrap();
    assert!(files.contains(&"a.txt".to_string()));
    assert!(files.contains(&"b.txt".to_string()));
}

#[test]
fn get_changed_files_since_tag_only_new() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");

    let files = get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn, None).unwrap();
    assert!(!files.contains(&"a.txt".to_string()));
    assert!(files.contains(&"b.txt".to_string()));
}

#[test]
fn get_changed_files_since_tag_lists_deletions() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
    create_lightweight_tag(&repo, "v1.0.0");
    git(dir.path(), &["rm", "-q", "--", "a.txt"]);
    git(dir.path(), &["commit", "-m", "remove a"]);

    let files = get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn, None).unwrap();
    assert!(files.contains(&"a.txt".to_string()));
    assert!(!files.contains(&"b.txt".to_string()));
}

#[test]
fn get_changed_files_since_tag_lists_both_sides_of_a_rename() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "old.txt", "first");
    create_lightweight_tag(&repo, "v1.0.0");
    git(dir.path(), &["mv", "old.txt", "new.txt"]);
    git(dir.path(), &["commit", "-m", "rename"]);

    let files = get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn, None).unwrap();
    assert!(files.contains(&"old.txt".to_string()));
    assert!(files.contains(&"new.txt".to_string()));
}

#[test]
fn get_changed_files_since_tag_nested_paths_are_full_and_unquoted() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "v1.0.0");
    std::fs::create_dir_all(dir.path().join("packages/café")).unwrap();
    std::fs::write(dir.path().join("packages/café/index.js"), "x").unwrap();
    git(dir.path(), &["add", "--", "packages"]);
    git(dir.path(), &["commit", "-m", "add nested"]);

    let files = get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn, None).unwrap();
    assert_eq!(files, vec!["packages/café/index.js".to_string()]);
}

fn assert_walks_agree(repo: &Repository, cache: &CommitWalkCache, stop: Option<gix::ObjectId>) {
    let markers = crate::config::default_commit_skip_markers();
    let direct = get_commits_since_oid(repo, stop, &markers).unwrap();
    let cached = cache.commits_since(repo, stop).unwrap();
    let pairs = |logs: &[GitLog]| {
        logs.iter()
            .map(|l| (l.hash.clone(), l.message.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(pairs(&cached), pairs(&direct), "stop = {stop:?}");
}

#[test]
fn commit_walk_cache_matches_direct_walk_on_linear_history() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: a");
    let stop = repo.head_id().unwrap().detach();
    create_commit_in_repo(&repo, dir.path(), "b.txt", "fix: b");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: c");

    let cache = CommitWalkCache::new(crate::config::default_commit_skip_markers());
    assert_walks_agree(&repo, &cache, None);
    assert_walks_agree(&repo, &cache, Some(stop));
    assert_walks_agree(&repo, &cache, Some(repo.head_id().unwrap().detach()));
}

#[test]
fn commit_walk_cache_matches_direct_walk_across_merges() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: a");
    let base = repo.head_id().unwrap().detach();
    git(dir.path(), &["checkout", "-q", "-b", "side"]);
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: b");
    let side = repo.head_id().unwrap().detach();
    git(dir.path(), &["checkout", "-q", "-"]);
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: c");
    git(
        dir.path(),
        &["merge", "-q", "--no-ff", "side", "-m", "chore: merge side"],
    );
    create_commit_in_repo(&repo, dir.path(), "d.txt", "feat: d");

    let cache = CommitWalkCache::new(crate::config::default_commit_skip_markers());
    for stop in [None, Some(base), Some(side)] {
        assert_walks_agree(&repo, &cache, stop);
    }
}

#[test]
fn commit_walk_cache_matches_direct_walk_for_stop_outside_head_ancestry() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: a");
    git(dir.path(), &["checkout", "-q", "-b", "orphan"]);
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: b");
    let orphan = repo.head_id().unwrap().detach();
    git(dir.path(), &["checkout", "-q", "-"]);
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: c");

    let cache = CommitWalkCache::new(crate::config::default_commit_skip_markers());
    for _ in 0..3 {
        assert_walks_agree(&repo, &cache, Some(orphan));
    }
}

#[test]
fn commit_walk_cache_applies_skip_markers() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: a");
    let stop = repo.head_id().unwrap().detach();
    create_commit_in_repo(&repo, dir.path(), "b.txt", "chore: bump [skip ci]");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: c");

    let cache = CommitWalkCache::new(crate::config::default_commit_skip_markers());
    for _ in 0..3 {
        assert_walks_agree(&repo, &cache, Some(stop));
    }
    let cached = cache.commits_since(&repo, Some(stop)).unwrap();
    assert!(cached.iter().all(|l| !l.message.contains("[skip ci]")));
    assert!(cached.iter().any(|l| l.message.contains("feat: c")));
}

#[test]
fn get_changed_files_empty_for_merge_commits() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    git(dir.path(), &["checkout", "-q", "-b", "side"]);
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
    git(dir.path(), &["checkout", "-q", "-"]);
    create_commit_in_repo(&repo, dir.path(), "c.txt", "third");
    git(
        dir.path(),
        &["merge", "-q", "--no-ff", "side", "-m", "merge side"],
    );

    let files = get_changed_files(&repo).unwrap();
    assert!(files.is_empty());
}

#[test]
fn walks_stay_correct_with_commit_graph_present() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "fix: second");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: third");
    git(dir.path(), &["commit-graph", "write", "--reachable"]);

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 2);
    assert!(commits[0].message.contains("third"));
    assert!(commits[1].message.contains("second"));

    let idx = TagIndex::build(&repo).unwrap();
    assert!(
        idx.find_last_tag_commit("v", OrphanedTagStrategy::Warn)
            .is_some()
    );
}

#[test]
fn create_commit_adds_files() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");

    std::fs::write(dir.path().join("new.txt"), "new content").unwrap();
    create_commit(&repo, &["new.txt"], "feat: add new file").unwrap();

    let msg = git(dir.path(), &["log", "-1", "--format=%B"]);
    assert!(msg.contains("feat: add new file"));
}

#[test]
fn create_branch_and_commit_works() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");

    std::fs::write(dir.path().join("release.txt"), "bumped").unwrap();
    create_branch_and_commit(&repo, "release/v1.0.0", &["release.txt"], "chore: release").unwrap();

    let out = git(
        dir.path(),
        &["rev-parse", "--verify", "refs/heads/release/v1.0.0"],
    );
    assert!(!out.trim().is_empty());
}

#[test]
fn create_branch_and_commits_multiple() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");

    std::fs::write(dir.path().join("pkg1.txt"), "v1").unwrap();
    std::fs::write(dir.path().join("pkg2.txt"), "v2").unwrap();

    let commits: Vec<(&[&str], &str)> = vec![
        (&["pkg1.txt"], "chore(release): pkg1 v1.0.0"),
        (&["pkg2.txt"], "chore(release): pkg2 v2.0.0"),
    ];
    create_branch_and_commits(&repo, "release/multi", &commits).unwrap();

    let tip_msg = git(
        dir.path(),
        &["log", "-1", "--format=%B", "refs/heads/release/multi"],
    );
    assert!(tip_msg.contains("chore(release): pkg2 v2.0.0"));
    let parent_msg = git(
        dir.path(),
        &["log", "-1", "--format=%B", "refs/heads/release/multi^"],
    );
    assert!(parent_msg.contains("chore(release): pkg1 v1.0.0"));
}

#[test]
fn get_remote_url_https() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    git(
        dir.path(),
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/FerrLabs/FerrFlow.git",
        ],
    );
    let repo = open_repo(dir.path()).unwrap();
    let url = get_remote_url(&repo, "origin");
    assert_eq!(
        url,
        Some("https://github.com/FerrLabs/FerrFlow.git".to_string())
    );
    let _ = repo;
}

#[test]
fn get_remote_url_no_remote() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    let url = get_remote_url(&repo, "origin");
    assert_eq!(url, None);
}

#[test]
fn extract_url_password_https_with_token() {
    let result =
        extract_url_password("https://x-access-token:ghp_abc123@github.com/owner/repo.git");
    assert_eq!(
        result,
        Some(("x-access-token".to_string(), "ghp_abc123".to_string()))
    );
}

#[test]
fn extract_url_password_gitlab_ci() {
    let result =
        extract_url_password("https://gitlab-ci-token:secret@gitlab.com/group/project.git");
    assert_eq!(
        result,
        Some(("gitlab-ci-token".to_string(), "secret".to_string()))
    );
}

#[test]
fn extract_url_password_no_credentials() {
    assert_eq!(
        extract_url_password("https://github.com/owner/repo.git"),
        None
    );
}

#[test]
fn extract_url_password_username_only() {
    assert_eq!(
        extract_url_password("https://user@github.com/owner/repo.git"),
        None
    );
}

#[test]
fn extract_url_password_empty_password() {
    assert_eq!(
        extract_url_password("https://user:@github.com/owner/repo.git"),
        None
    );
}

#[test]
fn extract_url_password_ssh_url() {
    assert_eq!(extract_url_password("git@github.com:owner/repo.git"), None);
}

#[test]
fn token_for_url_uses_ferrflow_token_when_set() {
    let _guard = EnvGuard::new().set("FERRFLOW_TOKEN", "ff_secret");
    let (user, token) =
        token_for_url("https://github.com/owner/repo.git").expect("should find token");
    assert_eq!(user, "x-access-token");
    assert_eq!(token, "ff_secret");
}

#[test]
fn token_for_url_picks_gitlab_user_for_gitlab_urls() {
    let _guard = EnvGuard::new().set("FERRFLOW_TOKEN", "gl_secret");
    let (user, _) =
        token_for_url("https://gitlab.com/group/project.git").expect("should find token");
    assert_eq!(user, "oauth2");
}

#[test]
fn token_for_url_falls_back_to_provider_env() {
    let _guard = EnvGuard::new()
        .unset("FERRFLOW_TOKEN")
        .set("GITHUB_TOKEN", "gh_secret");
    let (user, token) =
        token_for_url("https://github.com/owner/repo.git").expect("should find token");
    assert_eq!(user, "x-access-token");
    assert_eq!(token, "gh_secret");
}

#[test]
fn token_for_url_returns_none_without_env() {
    let _guard = EnvGuard::new()
        .unset("FERRFLOW_TOKEN")
        .unset("GITHUB_TOKEN")
        .unset("GITLAB_TOKEN");
    assert_eq!(token_for_url("https://github.com/owner/repo.git"), None);
}

#[test]
fn configure_git_command_injects_credential_helper_inline() {
    let _guard = EnvGuard::new().set("FERRFLOW_TOKEN", "ff_secret");
    let mut cmd = std::process::Command::new("git");
    configure_git_command(&mut cmd, "https://github.com/owner/repo.git");
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(
        args.iter().any(|a| a == "-c"),
        "expected -c flag, got {args:?}"
    );
    let helper_arg = args
        .iter()
        .find(|a| a.starts_with("credential.helper="))
        .expect("expected credential.helper config");
    assert!(helper_arg.contains("username='x-access-token'"));
    assert!(helper_arg.contains("password='ff_secret'"));
    assert!(
        !args.iter().any(|a| a.contains("ff_secret@")),
        "token must NOT be embedded in URL"
    );
}

#[test]
fn configure_git_command_single_quote_escapes_dangerous_token_chars() {
    let _guard = EnvGuard::new().set("FERRFLOW_TOKEN", "evil';rm -rf /;#");
    let mut cmd = std::process::Command::new("git");
    configure_git_command(&mut cmd, "https://github.com/owner/repo.git");
    let helper_arg = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .find(|a| a.starts_with("credential.helper="))
        .expect("expected credential.helper config");

    assert!(
        helper_arg.contains(r"password='evil'\'';rm -rf /;#'"),
        "expected single-quote escape, got: {helper_arg}"
    );
}

#[test]
fn configure_git_command_strips_git_trace_env() {
    let _guard = EnvGuard::new().set("FERRFLOW_TOKEN", "ff_secret");
    let mut cmd = std::process::Command::new("git");
    cmd.env("GIT_TRACE", "1");
    cmd.env("GIT_CURL_VERBOSE", "1");
    cmd.env("GIT_TRACE_CURL", "1");
    configure_git_command(&mut cmd, "https://github.com/owner/repo.git");
    let removed: std::collections::HashSet<String> = cmd
        .get_envs()
        .filter(|(_, v)| v.is_none())
        .map(|(k, _)| k.to_string_lossy().into_owned())
        .collect();
    assert!(removed.contains("GIT_TRACE"));
    assert!(removed.contains("GIT_CURL_VERBOSE"));
    assert!(removed.contains("GIT_TRACE_CURL"));
}

#[test]
fn configure_git_command_resets_checkout_extraheader_when_authenticated() {
    let _guard = EnvGuard::new().set("FERRFLOW_TOKEN", "ff_secret");
    let mut cmd = std::process::Command::new("git");
    configure_git_command(&mut cmd, "https://github.com/owner/repo.git");
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(
        args.iter()
            .any(|a| a == "http.https://github.com/.extraheader="),
        "expected the persisted checkout extraheader to be reset to empty, got {args:?}"
    );
}

#[test]
fn configure_git_command_does_not_reset_extraheader_without_token() {
    let _guard = EnvGuard::new()
        .unset("FERRFLOW_TOKEN")
        .unset("GITHUB_TOKEN")
        .unset("GITLAB_TOKEN");
    let mut cmd = std::process::Command::new("git");
    configure_git_command(&mut cmd, "https://github.com/owner/repo.git");
    assert!(
        !cmd.get_args()
            .any(|a| a.to_string_lossy().contains("extraheader")),
        "must not touch extraheader when ferrflow has no token of its own"
    );
}

#[test]
fn server_config_url_derives_the_checkout_key() {
    assert_eq!(
        server_config_url("https://github.com/owner/repo.git").as_deref(),
        Some("https://github.com/")
    );
    assert_eq!(
        server_config_url("https://x-access-token:tok@ghe.acme.dev:8443/o/r.git").as_deref(),
        Some("https://ghe.acme.dev:8443/")
    );
    assert_eq!(server_config_url("git@github.com:owner/repo.git"), None);
    assert_eq!(server_config_url("not a url"), None);
}

#[test]
fn configure_git_command_skips_helper_without_token() {
    let _guard = EnvGuard::new()
        .unset("FERRFLOW_TOKEN")
        .unset("GITHUB_TOKEN")
        .unset("GITLAB_TOKEN");
    let mut cmd = std::process::Command::new("git");
    configure_git_command(&mut cmd, "https://github.com/owner/repo.git");
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(args.is_empty(), "no flags should be added, got {args:?}");
}

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    keys: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn new() -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        Self {
            _lock: lock,
            keys: Vec::new(),
        }
    }

    fn set(mut self, key: &str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        self.keys.push((key.to_string(), previous));
        self
    }

    fn unset(mut self, key: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::remove_var(key);
        }
        self.keys.push((key.to_string(), previous));
        self
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.keys.iter().rev() {
            unsafe {
                match previous {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

fn temp_repo_with_commit() -> (Repository, tempfile::TempDir) {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "init.txt", "initial commit");
    (repo, dir)
}

#[test]
fn create_or_move_tag_new() {
    let (repo, _dir) = temp_repo_with_commit();
    let moved = super::create_or_move_tag(&repo, "v1", "Floating tag").unwrap();
    assert!(!moved);
    assert!(super::tag_exists(&repo, "v1"));
}

#[test]
fn create_or_move_tag_moves_existing() {
    let (repo, _dir) = temp_repo_with_commit();
    super::create_tag(&repo, "v1", "First").unwrap();

    let path = _dir.path().join("second.txt");
    std::fs::write(&path, "second").unwrap();
    super::create_commit(&repo, &["second.txt"], "second commit").unwrap();

    let moved = super::create_or_move_tag(&repo, "v1", "Floating tag").unwrap();
    assert!(moved);
    assert!(super::tag_exists(&repo, "v1"));
}

fn create_orphaned_tag_scenario(tag_name: &str) -> (Repository, tempfile::TempDir) {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: original");
    create_lightweight_tag(&repo, tag_name);

    let head_branch = git(dir.path(), &["symbolic-ref", "HEAD"])
        .trim()
        .to_string();
    let tree_sha = git(dir.path(), &["rev-parse", "HEAD^{tree}"])
        .trim()
        .to_string();
    let ts = COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let date = format!("{ts} +0000");
    let new_sha = git_with_env(
        dir.path(),
        &["commit-tree", &tree_sha, "-m", "feat: original"],
        &[("GIT_AUTHOR_DATE", &date), ("GIT_COMMITTER_DATE", &date)],
    )
    .trim()
    .to_string();
    git(dir.path(), &["update-ref", &head_branch, &new_sha]);
    let repo = open_repo(dir.path()).unwrap();
    (repo, dir)
}

#[test]
fn orphaned_tag_warn_skips() {
    let (repo, _dir) = create_orphaned_tag_scenario("v1.0.0");
    let result = find_last_tag_name(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert_eq!(result, None);
}

#[test]
fn orphaned_tag_tree_hash_recovers() {
    let (repo, _dir) = create_orphaned_tag_scenario("v1.0.0");
    let result = find_last_tag_name(&repo, "v", OrphanedTagStrategy::TreeHash).unwrap();
    assert_eq!(result, Some("v1.0.0".to_string()));
}

#[test]
fn orphaned_tag_message_recovers() {
    let (repo, _dir) = create_orphaned_tag_scenario("v1.0.0");
    let result = find_last_tag_name(&repo, "v", OrphanedTagStrategy::Message).unwrap();
    assert_eq!(result, Some("v1.0.0".to_string()));
}

#[test]
fn orphaned_tag_no_match() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: original");
    create_lightweight_tag(&repo, "v1.0.0");

    let head_branch = git(dir.path(), &["symbolic-ref", "HEAD"])
        .trim()
        .to_string();
    std::fs::write(dir.path().join("b.txt"), "different").unwrap();
    git(dir.path(), &["add", "--", "b.txt"]);
    let tree_sha = git(dir.path(), &["write-tree"]).trim().to_string();
    let ts = COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let date = format!("{ts} +0000");
    let new_sha = git_with_env(
        dir.path(),
        &["commit-tree", &tree_sha, "-m", "feat: totally different"],
        &[("GIT_AUTHOR_DATE", &date), ("GIT_COMMITTER_DATE", &date)],
    )
    .trim()
    .to_string();
    git(dir.path(), &["update-ref", &head_branch, &new_sha]);
    let repo = open_repo(dir.path()).unwrap();

    let result = find_last_tag_name(&repo, "v", OrphanedTagStrategy::TreeHash).unwrap();
    assert_eq!(result, None);
}

#[test]
fn get_commits_since_orphaned_tag_with_recovery() {
    let (repo, dir) = create_orphaned_tag_scenario("v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: new feature");

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::TreeHash,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 1);
    assert!(commits[0].message.contains("new feature"));
}

#[test]
fn ancestor_cache_matches_uncached_on_orphan_recovery() {
    let (repo, dir) = create_orphaned_tag_scenario("v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: new feature");

    let ancestors = build_head_ancestors(&repo).unwrap();
    let markers = crate::config::default_commit_skip_markers();
    let msgs =
        |c: &[crate::changelog::GitLog]| c.iter().map(|g| g.message.clone()).collect::<Vec<_>>();

    let uncached =
        get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::TreeHash, &markers, None)
            .unwrap();
    let cached = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::TreeHash,
        &markers,
        Some(&ancestors),
    )
    .unwrap();

    assert_eq!(msgs(&cached), msgs(&uncached));
    assert_eq!(cached.len(), 1);

    let files_uncached =
        get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::TreeHash, None).unwrap();
    let files_cached =
        get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::TreeHash, Some(&ancestors))
            .unwrap();
    assert_eq!(files_cached, files_uncached);
}

#[test]
fn get_commits_since_last_stable_tag_skips_prereleases() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: initial");
    create_annotated_tag(&repo, "v1.0.0", "Release v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: beta feature");
    create_annotated_tag(&repo, "v2.0.0-beta.1", "Release v2.0.0-beta.1");
    create_commit_in_repo(&repo, dir.path(), "c.txt", "feat: another beta feature");
    create_annotated_tag(&repo, "v2.0.0-beta.2", "Release v2.0.0-beta.2");
    create_commit_in_repo(&repo, dir.path(), "d.txt", "fix: last fix");

    let commits = get_commits_since_last_stable_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 3);

    let commits = get_commits_since_last_tag(
        &repo,
        "v",
        OrphanedTagStrategy::Warn,
        &crate::config::default_commit_skip_markers(),
        None,
    )
    .unwrap();
    assert_eq!(commits.len(), 1);
}

#[test]
fn collect_all_tags_returns_tag_names() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: initial");
    create_annotated_tag(&repo, "v1.0.0", "Release v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: second");
    create_annotated_tag(&repo, "v1.1.0-beta.1", "Release v1.1.0-beta.1");

    let tags = collect_all_tags(&repo);
    assert!(tags.contains(&"v1.0.0".to_string()));
    assert!(tags.contains(&"v1.1.0-beta.1".to_string()));
}

#[test]
fn is_prerelease_tag_detection() {
    assert!(!is_prerelease_tag("v1.0.0", "v"));
    assert!(is_prerelease_tag("v1.0.0-beta.1", "v"));
    assert!(is_prerelease_tag("v2.0.0-rc.3", "v"));
    assert!(!is_prerelease_tag("v2.0.0", "v"));
    assert!(is_prerelease_tag("sdk@v1.0.0-dev.1", "sdk@v"));
    assert!(!is_prerelease_tag("sdk@v1.0.0", "sdk@v"));
}

#[test]
fn is_floating_tag_detection() {
    assert!(is_floating_tag("v2", "v"));
    assert!(is_floating_tag("v2.3", "v"));
    assert!(is_floating_tag("v10", "v"));
    assert!(is_floating_tag("v0", "v"));

    assert!(!is_floating_tag("v2.14.1", "v"));
    assert!(!is_floating_tag("v0.1.0", "v"));
    assert!(!is_floating_tag("v1.0.0", "v"));
    assert!(!is_floating_tag("v10.20.30", "v"));

    assert!(is_floating_tag("api@v1", "api@v"));
    assert!(is_floating_tag("api@v1.2", "api@v"));
    assert!(!is_floating_tag("api@v1.2.3", "api@v"));

    assert!(!is_floating_tag("v2.0.0-beta.1", "v"));
    assert!(!is_floating_tag("v1.0.0-rc.1", "v"));

    assert!(!is_floating_tag("v", "v"));
}

#[test]
fn find_last_tag_skips_floating_tags() {
    let (dir, repo) = init_repo();

    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: initial");
    git(dir.path(), &["tag", "v1.0.0"]);

    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: second");
    git(dir.path(), &["tag", "v1"]);

    let result = find_last_tag(&repo, "v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(result.name, "v1.0.0");
}

#[test]
fn resolve_branch_from_head() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    let branch = resolve_current_branch(&repo, "fallback");
    assert_ne!(branch, "fallback");
    assert!(!branch.is_empty());
}

#[test]
fn resolve_branch_detached_returns_non_empty() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    let head_oid = git(dir.path(), &["rev-parse", "HEAD"]).trim().to_string();
    git(dir.path(), &["checkout", "--detach", &head_oid]);
    let repo = open_repo(dir.path()).unwrap();

    let branch = resolve_current_branch(&repo, "my-fallback");
    assert!(!branch.is_empty());
}

#[test]
fn push_rejects_when_remote_advanced_instead_of_rebasing() {
    let base_dir = tempfile::tempdir().unwrap();

    let remote_path = base_dir.path().join("remote.git");
    std::fs::create_dir_all(&remote_path).unwrap();
    git(&remote_path, &["init", "--bare", "-b", "main"]);

    let local_path = base_dir.path().join("local");
    std::fs::create_dir_all(&local_path).unwrap();
    let repo = init_repo_at(&local_path);
    git(
        &local_path,
        &["remote", "add", "origin", remote_path.to_str().unwrap()],
    );

    create_commit_in_repo(&repo, &local_path, "base.txt", "commit A");
    git(&local_path, &["push", "origin", "main:main"]);

    let helper_path = base_dir.path().join("helper");
    std::fs::create_dir_all(&helper_path).unwrap();
    let helper_repo = init_repo_at(&helper_path);
    git(
        &helper_path,
        &["remote", "add", "origin", remote_path.to_str().unwrap()],
    );
    git(
        &helper_path,
        &["fetch", "origin", "main:refs/remotes/origin/main"],
    );
    git(
        &helper_path,
        &["reset", "--hard", "refs/remotes/origin/main"],
    );
    create_commit_in_repo(
        &helper_repo,
        &helper_path,
        "from_concurrent_pr.txt",
        "commit B",
    );
    git(&helper_path, &["push", "origin", "main:main"]);

    create_commit_in_repo(&repo, &local_path, "release_commit.txt", "commit X");

    let local_head_before = git(&local_path, &["rev-parse", "HEAD"]);

    let repo = open_repo(&local_path).unwrap();
    let err = super::push::push(&repo, "origin", "main")
        .expect_err("push onto an advanced remote must be rejected, not rebased");

    assert!(
        is_push_rejected_error(&err),
        "the rejection must be classified as retryable so the release regenerate \
         loop replans against the new base; got: {err:#}"
    );

    let local_head_after = git(&local_path, &["rev-parse", "HEAD"]);
    assert_eq!(
        local_head_before, local_head_after,
        "push must not rewrite HEAD: release tags already point at it"
    );

    let head_tree = git(&local_path, &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(
        !head_tree.contains("from_concurrent_pr.txt"),
        "no rebase should have happened"
    );
}

#[test]
fn reset_branch_to_remote_drops_local_commit_and_dirty_tree() {
    let base_dir = tempfile::tempdir().unwrap();

    let remote_path = base_dir.path().join("remote.git");
    std::fs::create_dir_all(&remote_path).unwrap();
    git(&remote_path, &["init", "--bare", "-b", "main"]);

    let local_path = base_dir.path().join("local");
    std::fs::create_dir_all(&local_path).unwrap();
    let repo = init_repo_at(&local_path);
    git(
        &local_path,
        &["remote", "add", "origin", remote_path.to_str().unwrap()],
    );

    create_commit_in_repo(&repo, &local_path, "base.txt", "commit A");
    git(&local_path, &["push", "origin", "main:main"]);

    let helper_path = base_dir.path().join("helper");
    std::fs::create_dir_all(&helper_path).unwrap();
    let helper_repo = init_repo_at(&helper_path);
    git(
        &helper_path,
        &["remote", "add", "origin", remote_path.to_str().unwrap()],
    );
    git(
        &helper_path,
        &["fetch", "origin", "main:refs/remotes/origin/main"],
    );
    git(
        &helper_path,
        &["reset", "--hard", "refs/remotes/origin/main"],
    );
    create_commit_in_repo(&helper_repo, &helper_path, "remote_only.txt", "commit B");
    let remote_b_oid = git(&helper_path, &["rev-parse", "HEAD"]).trim().to_string();
    git(&helper_path, &["push", "origin", "main:main"]);

    create_commit_in_repo(&repo, &local_path, "release.txt", "commit X");
    let x_oid = git(&local_path, &["rev-parse", "HEAD"]).trim().to_string();
    std::fs::write(local_path.join("dirty.txt"), "stale hook output").unwrap();

    let repo = open_repo(&local_path).unwrap();
    reset_branch_to_remote(&repo, "origin", "main").expect("reset must succeed");

    let new_head = git(&local_path, &["rev-parse", "HEAD"]).trim().to_string();
    assert_eq!(new_head, remote_b_oid, "HEAD must be at remote B");
    assert_ne!(new_head, x_oid, "HEAD must not still be at X");

    assert!(!local_path.join("release.txt").exists());
    assert!(local_path.join("base.txt").exists());
    assert!(local_path.join("remote_only.txt").exists());
    assert!(!local_path.join("dirty.txt").exists());

    let main_ref = git(&local_path, &["rev-parse", "refs/heads/main"])
        .trim()
        .to_string();
    assert_eq!(main_ref, remote_b_oid);
    let symref = git(&local_path, &["symbolic-ref", "HEAD"])
        .trim()
        .to_string();
    assert_eq!(symref, "refs/heads/main");
}

#[test]
fn is_push_rejected_error_recognises_known_signatures() {
    let e = anyhow::anyhow!("upstream rejected the push").context(error_code::GIT_PUSH_REJECTED);
    assert!(is_push_rejected_error(&e));

    let e =
        anyhow::anyhow!("Rebase conflict: cannot rebase release commits on top of remote 'main'.");
    assert!(is_push_rejected_error(&e));

    let e = anyhow::anyhow!("refs/heads/main: push declined due to repository rule violations");
    assert!(is_push_rejected_error(&e));

    let e = anyhow::anyhow!(
        "Updates were rejected because the tip of your current branch is non-fast-forward"
    );
    assert!(is_push_rejected_error(&e));

    let e = anyhow::anyhow!(
        "Tag(s) already exist on remote pointing to a different commit: \
         api@v3.13.3 (local 1e3ed96 != remote c3ae651)."
    )
    .context(error_code::GIT_PUSH_TAGS);
    assert!(is_push_rejected_error(&e));

    let e = anyhow::anyhow!("hook failed: prettier exited with status 1");
    assert!(!is_push_rejected_error(&e));
}

fn push_branch_failure(detail: &str) -> anyhow::Error {
    anyhow::anyhow!("Failed to push branch 'main': {detail}").context(error_code::GIT_PUSH_BRANCH)
}

#[test]
fn a_github_side_push_failure_is_transient_not_a_stale_branch() {
    let err = push_branch_failure(
        "remote: fatal error in commit_refs\n\
         To https://github.com/FerrLabs/FerrFlow\n \
         ! [remote rejected] main -> main (failure)\n\
         error: failed to push some refs",
    );

    assert!(
        is_transient_git_error(&err),
        "a server-side failure must be retried by the push layer"
    );
    assert!(
        !is_push_rejected_error(&err),
        "it is not a race, so it must not be reported as a stale branch \
         nor burn the regenerate attempts"
    );
}

#[test]
fn a_remote_that_advanced_before_we_fetched_is_a_race() {
    // This is the wording git actually produces in CI, where the release job
    // pushes without having fetched the branch first.
    let err = push_branch_failure(
        " ! [rejected]        HEAD -> main (fetch first)\n\
         error: failed to push some refs",
    );

    assert!(is_push_rejected_error(&err));
    assert!(!is_transient_git_error(&err));
}

#[test]
fn a_createcommitonbranch_expectation_failure_is_a_race() {
    let err = anyhow::anyhow!(
        "createCommitOnBranch failed: Expected branch to point to \"ca3a40a\" but it did not.  Pull and try again."
    )
    .context(error_code::GITHUB_GRAPHQL_ERROR);

    assert!(
        is_push_rejected_error(&err),
        "the authored-commit path hits the same race the git path retries, so it has to reach the same regenerate loop"
    );
    assert!(!is_transient_git_error(&err));
}

#[test]
fn a_non_fast_forward_is_a_race() {
    let err = push_branch_failure(" ! [rejected]        HEAD -> main (non-fast-forward)");

    assert!(is_push_rejected_error(&err));
}

#[test]
fn a_stale_force_with_lease_expectation_is_a_race() {
    let err = push_branch_failure(" ! [rejected]        main -> main (stale info)");

    assert!(is_push_rejected_error(&err));
}

#[test]
fn a_permanent_push_failure_is_neither_retried_nor_regenerated() {
    for detail in [
        "remote: Permission to FerrLabs/FerrFlow.git denied to ferrflow[bot].\n\
         fatal: unable to access ...: The requested URL returned error: 403",
        "remote: Repository not found.\nfatal: repository not found",
        "fatal: Authentication failed for 'https://github.com/FerrLabs/FerrFlow'",
    ] {
        let err = push_branch_failure(detail);
        assert!(
            !is_push_rejected_error(&err),
            "must fail fast with the real cause instead of three misleading \
             stale-branch attempts: {detail}"
        );
        assert!(!is_transient_git_error(&err), "{detail}");
    }
}

#[test]
fn a_divergent_remote_tag_still_routes_to_the_regenerate_path() {
    let err = anyhow::anyhow!(
        "Tag(s) already exist on remote pointing to a different commit: \
         api@v3.13.3 (local 1e3ed96 != remote c3ae651)."
    )
    .context(error_code::GIT_PUSH_TAGS);

    assert!(
        is_push_rejected_error(&err),
        "matched by its own wording now that the generic code no longer counts"
    );
}

fn commit_file_at(dir: &Path, rel: &str, message: &str) -> String {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, format!("content of {rel}")).unwrap();
    git(dir, &["add", "--", rel]);
    let ts = next_commit_ts();
    let date = format!("{ts} +0000");
    git_with_env(
        dir,
        &["commit", "-m", message],
        &[("GIT_AUTHOR_DATE", &date), ("GIT_COMMITTER_DATE", &date)],
    );
    git(dir, &["rev-parse", "HEAD"]).trim().to_string()
}

fn oid(sha: &str) -> gix::ObjectId {
    gix::ObjectId::from_hex(sha.as_bytes()).expect("valid sha")
}

#[test]
fn changed_files_for_commit_lists_only_that_commit_s_paths() {
    let (dir, _repo) = init_repo();
    commit_file_at(dir.path(), "packages/api/main.rs", "feat: api");
    let web = commit_file_at(dir.path(), "packages/web/app.ts", "feat: web");

    let repo = open_repo(dir.path()).unwrap();
    let files = get_changed_files_for_commit(&repo, oid(&web)).expect("diffs against the parent");

    assert_eq!(files, vec!["packages/web/app.ts".to_string()]);
}

#[test]
fn changed_files_for_the_root_commit_is_its_whole_tree() {
    let (dir, _repo) = init_repo();
    let root = commit_file_at(dir.path(), "packages/api/main.rs", "feat: api");

    let repo = open_repo(dir.path()).unwrap();
    let files = get_changed_files_for_commit(&repo, oid(&root)).expect("root commit is diffable");

    assert_eq!(files, vec!["packages/api/main.rs".to_string()]);
}

#[test]
fn signing_is_off_when_the_repo_does_not_ask_for_it() {
    let dir = tempfile::tempdir().unwrap();
    init_repo_at(dir.path());

    assert!(
        !crate::git::commits::commit_signing_enabled(dir.path()),
        "a repo with commit.gpgsign=false must not be treated as signing"
    );
}

#[test]
fn signing_follows_commit_gpgsign_so_both_commit_paths_agree() {
    let dir = tempfile::tempdir().unwrap();
    init_repo_at(dir.path());

    git(dir.path(), &["config", "commit.gpgsign", "true"]);
    assert!(
        crate::git::commits::commit_signing_enabled(dir.path()),
        "commit.gpgsign=true must reach the commit-tree path, which ignores it on its own"
    );

    git(dir.path(), &["config", "commit.gpgsign", "false"]);
    assert!(!crate::git::commits::commit_signing_enabled(dir.path()));
}

#[test]
fn signing_follows_every_spelling_git_accepts_for_a_boolean() {
    let dir = tempfile::tempdir().unwrap();
    init_repo_at(dir.path());

    for truthy in ["true", "yes", "on", "1"] {
        git(dir.path(), &["config", "commit.gpgsign", truthy]);
        assert!(
            crate::git::commits::commit_signing_enabled(dir.path()),
            "commit.gpgsign={truthy} means signing to git, so it must here too"
        );
    }

    for falsy in ["false", "no", "off", "0"] {
        git(dir.path(), &["config", "commit.gpgsign", falsy]);
        assert!(
            !crate::git::commits::commit_signing_enabled(dir.path()),
            "commit.gpgsign={falsy} must not enable signing"
        );
    }
}

/// Local clone plus a bare remote it can push to, which is the only shape that
/// can tell a local-ref check from a remote one.
fn repo_with_remote(dir: &Path) -> (PathBuf, PathBuf) {
    let remote = dir.join("remote.git");
    std::fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "--bare", "-b", "main"]);

    let local = dir.join("local");
    std::fs::create_dir_all(&local).unwrap();
    init_repo_at(&local);
    git(
        &local,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    commit_file_at(&local, "a.txt", "feat: first");
    git(&local, &["push", "origin", "main:main"]);
    (local, remote)
}

#[test]
fn a_tag_the_remote_has_moved_is_left_alone_even_when_the_local_ref_still_matches() {
    let dir = tempfile::tempdir().unwrap();
    let (local, remote) = repo_with_remote(dir.path());

    git(&local, &["tag", "v1.0.0"]);
    git(&local, &["push", "origin", "v1.0.0"]);
    let repo = open_repo(&local).unwrap();
    let recorded = local_tag_target_sha(&repo, "v1.0.0").unwrap();

    // Someone else recreates the tag on a different commit. Our local ref is
    // untouched and still equals `recorded`, so a local-only check would pass.
    let other = dir.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    init_repo_at(&other);
    git(
        &other,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    commit_file_at(&other, "b.txt", "feat: theirs");
    git(&other, &["push", "--force", "origin", "HEAD:main"]);
    git(&other, &["tag", "-f", "v1.0.0"]);
    git(&other, &["push", "--force", "origin", "v1.0.0"]);

    assert_eq!(
        local_tag_target_sha(&repo, "v1.0.0").unwrap(),
        recorded,
        "precondition: the local ref must still look unchanged"
    );

    delete_tag_if_unchanged(&repo, "origin", "v1.0.0", &recorded).unwrap();

    let remote_tags = git(&remote, &["tag", "-l"]);
    assert!(
        remote_tags.contains("v1.0.0"),
        "the remote tag someone else recreated must survive, got {remote_tags:?}"
    );
}

#[test]
fn a_tag_absent_from_this_checkout_is_not_deleted_from_the_remote() {
    let dir = tempfile::tempdir().unwrap();
    let (local, remote) = repo_with_remote(dir.path());

    // Tag pushed by someone else, never fetched here.
    let other = dir.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    init_repo_at(&other);
    git(
        &other,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&other, &["fetch", "origin", "main"]);
    git(&other, &["checkout", "-f", "FETCH_HEAD"]);
    git(&other, &["tag", "theirs-v9.9.9"]);
    git(&other, &["push", "origin", "theirs-v9.9.9"]);

    let repo = open_repo(&local).unwrap();
    assert!(!tag_exists(&repo, "theirs-v9.9.9"), "precondition");

    delete_tag_if_unchanged(
        &repo,
        "origin",
        "theirs-v9.9.9",
        "deadbeefdeadbeefdeadbeefdeadbeef",
    )
    .unwrap();

    let remote_tags = git(&remote, &["tag", "-l"]);
    assert!(
        remote_tags.contains("theirs-v9.9.9"),
        "a tag this checkout never saw must not be deleted, got {remote_tags:?}"
    );
}

#[test]
fn an_annotated_tag_still_at_its_recorded_commit_is_deleted() {
    // FerrFlow creates annotated tags, so `refs/tags/x` is the tag object while
    // the checkpoint records the commit. Getting this wrong makes every real
    // rollback refuse itself.
    let dir = tempfile::tempdir().unwrap();
    let (local, remote) = repo_with_remote(dir.path());

    git(&local, &["tag", "-a", "v1.0.0", "-m", "Release v1.0.0"]);
    git(&local, &["push", "origin", "v1.0.0"]);
    let repo = open_repo(&local).unwrap();
    let commit = local_tag_target_sha(&repo, "v1.0.0").unwrap();

    delete_tag_if_unchanged(&repo, "origin", "v1.0.0", &commit).unwrap();

    let remote_tags = git(&remote, &["tag", "-l"]);
    assert!(
        !remote_tags.contains("v1.0.0"),
        "an annotated tag at its recorded commit must be deleted, got {remote_tags:?}"
    );
}

#[test]
fn a_tag_still_at_the_recorded_sha_on_the_remote_is_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let (local, remote) = repo_with_remote(dir.path());

    git(&local, &["tag", "v1.0.0"]);
    git(&local, &["push", "origin", "v1.0.0"]);
    let repo = open_repo(&local).unwrap();
    let sha = local_tag_target_sha(&repo, "v1.0.0").unwrap();

    delete_tag_if_unchanged(&repo, "origin", "v1.0.0", &sha).unwrap();

    assert!(!tag_exists(&repo, "v1.0.0"), "the local tag should go");
    let remote_tags = git(&remote, &["tag", "-l"]);
    assert!(
        !remote_tags.contains("v1.0.0"),
        "the remote tag should go too, got {remote_tags:?}"
    );
}

#[test]
fn a_tag_that_never_reached_the_remote_is_cleaned_up_locally() {
    let dir = tempfile::tempdir().unwrap();
    let (local, _remote) = repo_with_remote(dir.path());

    git(&local, &["tag", "v1.0.0"]);
    let repo = open_repo(&local).unwrap();
    let sha = local_tag_target_sha(&repo, "v1.0.0").unwrap();

    delete_tag_if_unchanged(&repo, "origin", "v1.0.0", &sha).unwrap();

    assert!(
        !tag_exists(&repo, "v1.0.0"),
        "a tag the failed run created but never pushed should still be cleaned up"
    );
}

#[test]
fn reverting_the_release_commit_undoes_its_changes() {
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("local");
    std::fs::create_dir_all(&local).unwrap();
    init_repo_at(&local);
    commit_file_at(&local, "Cargo.toml", "chore: initial");
    std::fs::write(local.join("Cargo.toml"), "version = \"1.1.0\"").unwrap();
    git(&local, &["add", "-A"]);
    git(&local, &["commit", "-m", "chore(release): v1.1.0"]);
    let release_sha = git(&local, &["rev-parse", "HEAD"]).trim().to_string();

    let repo = open_repo(&local).unwrap();
    revert_commit(&repo, &release_sha).unwrap();

    let content = std::fs::read_to_string(local.join("Cargo.toml")).unwrap();
    assert!(
        !content.contains("1.1.0"),
        "the bump must be undone, got {content:?}"
    );
    let head_subject = git(&local, &["log", "-1", "--format=%s"]);
    assert!(head_subject.starts_with("Revert"), "{head_subject}");
}

/// Local clone plus a bare remote, the only shape where a peeled sha can differ
/// from an unpeeled one.
fn repo_and_remote(dir: &Path) -> (PathBuf, PathBuf) {
    let remote = dir.join("remote.git");
    std::fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "--bare", "-b", "main"]);

    let local = dir.join("local");
    std::fs::create_dir_all(&local).unwrap();
    init_repo_at(&local);
    git(
        &local,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    commit_file_at(&local, "a.txt", "feat: first");
    git(&local, &["push", "origin", "main:main"]);
    (local, remote)
}

#[test]
fn remote_tag_shas_peel_an_annotated_tag_to_its_commit() {
    let dir = tempfile::tempdir().unwrap();
    let (local, remote) = repo_and_remote(dir.path());
    git(&local, &["tag", "-a", "v1.0.0", "-m", "Release v1.0.0"]);
    git(&local, &["push", "origin", "v1.0.0"]);

    let repo = open_repo(&local).unwrap();
    let commit = local_tag_target_sha(&repo, "v1.0.0").unwrap();
    let tag_object = git(&local, &["rev-parse", "refs/tags/v1.0.0"])
        .trim()
        .to_string();
    assert_ne!(
        commit, tag_object,
        "precondition: an annotated tag's object is not its commit"
    );

    let shas = remote_tag_target_shas(&local, remote.to_str().unwrap(), &["v1.0.0"]).unwrap();

    assert_eq!(
        shas.get("v1.0.0").map(String::as_str),
        Some(commit.as_str()),
        "the remote sha must be the commit, not the tag object"
    );
}

#[test]
fn pushing_an_annotated_tag_already_on_the_remote_is_a_no_op() {
    // FerrFlow creates annotated tags, so this is the ordinary case. Comparing
    // a peeled local sha against an unpeeled remote one made it look diverged
    // and failed the push with a message about a commit difference that was not
    // there. See #949.
    let dir = tempfile::tempdir().unwrap();
    let (local, _remote) = repo_and_remote(dir.path());
    git(&local, &["tag", "-a", "v1.0.0", "-m", "Release v1.0.0"]);
    git(&local, &["push", "origin", "v1.0.0"]);

    let repo = open_repo(&local).unwrap();

    push_tags(&repo, "origin", &["v1.0.0"])
        .expect("re-pushing the same annotated tag must succeed");
}

#[test]
fn a_lightweight_tag_already_on_the_remote_is_still_a_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let (local, _remote) = repo_and_remote(dir.path());
    git(&local, &["tag", "v1.0.0"]);
    git(&local, &["push", "origin", "v1.0.0"]);

    let repo = open_repo(&local).unwrap();

    push_tags(&repo, "origin", &["v1.0.0"])
        .expect("the case that already worked must keep working");
}

#[test]
fn an_annotated_tag_the_remote_holds_at_another_commit_is_still_rejected() {
    // The fix must not turn the divergence check off: a tag the remote really
    // does hold elsewhere has to keep failing the push.
    let dir = tempfile::tempdir().unwrap();
    let (local, remote) = repo_and_remote(dir.path());

    let other = dir.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    init_repo_at(&other);
    git(
        &other,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    commit_file_at(&other, "b.txt", "feat: theirs");
    git(&other, &["tag", "-a", "v1.0.0", "-m", "Theirs"]);
    git(&other, &["push", "origin", "v1.0.0"]);

    git(&local, &["tag", "-a", "v1.0.0", "-m", "Ours"]);
    let repo = open_repo(&local).unwrap();

    let err = push_tags(&repo, "origin", &["v1.0.0"])
        .expect_err("a genuinely divergent tag must still be refused");
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("already exist on remote"),
        "expected the divergence error, got {rendered}"
    );
}

#[test]
fn unreachable_tags_are_named_in_one_line() {
    let tags: Vec<String> = ["v2.0.0", "v2.1.0", "v2.2.0", "v2.3.0"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let msg = super::tags::unreachable_tags_message(&tags);

    assert!(msg.contains("4 tags are not reachable"), "{msg}");
    for tag in &tags {
        assert!(msg.contains(tag.as_str()), "{tag} missing from {msg}");
    }
    assert_eq!(
        msg.matches("orphanedTagStrategy").count(),
        1,
        "the hint belongs once, not once per tag: {msg}"
    );
}

#[test]
fn a_single_unreachable_tag_reads_as_singular() {
    let tags = vec!["v2.0.0".to_string()];
    let msg = super::tags::unreachable_tags_message(&tags);
    assert!(msg.contains("1 tag is not reachable"), "{msg}");
}

#[test]
fn a_long_list_of_unreachable_tags_is_truncated() {
    let tags: Vec<String> = (0..12).map(|n| format!("v2.{n}.0")).collect();
    let msg = super::tags::unreachable_tags_message(&tags);

    assert!(msg.contains("12 tags are not reachable"), "{msg}");
    assert!(msg.contains("and 7 more"), "{msg}");
    assert!(
        !msg.contains("v2.11.0"),
        "the tail must be summarised, not listed: {msg}"
    );
}

#[test]
fn a_tag_already_reported_is_not_reported_again() {
    let mut seen = std::collections::HashSet::new();
    let first =
        super::tags::take_unreported(vec!["v2.0.0".to_string(), "v2.1.0".to_string()], &mut seen);
    assert_eq!(first.len(), 2, "the first lookup names every tag");

    let second =
        super::tags::take_unreported(vec!["v2.0.0".to_string(), "v2.1.0".to_string()], &mut seen);
    assert!(
        second.is_empty(),
        "a command that resolves the last tag twice must not warn twice, got {second:?}"
    );

    let third = super::tags::take_unreported(vec!["v2.2.0".to_string()], &mut seen);
    assert_eq!(third, vec!["v2.2.0".to_string()], "a new tag still reports");
}
