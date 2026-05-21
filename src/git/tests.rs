use super::auth::{
    configure_git_command, credentials_callback, extract_url_password, token_for_url,
};
use super::push::{fetch_and_rebase, parse_ls_remote_tags};
use super::retry::{is_transient_git_error, retry_transient};
use super::tags::{find_highest_semver_tag, find_last_tag, is_floating_tag, is_prerelease_tag};
use super::*;
use crate::config::OrphanedTagStrategy;
use crate::error_code;
use git2::{CredentialType, Repository, Signature};
use std::fs;
use std::path::Path;

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
    // E2006 firing immediately after a successful branch push, when
    // libgit2's ODB cache hasn't caught up with objects we just wrote.
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

    // Unknown error: default to not retrying so we don't mask logic
    // bugs with infinite retry-attempts.
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
    // Lightweight tag — only the bare line, no ^{} deref entry.
    let input = "0123456789abcdef0123456789abcdef01234567\trefs/tags/site@v0.13.0\n";
    let map = parse_ls_remote_tags(input);
    assert_eq!(
        map.get("site@v0.13.0").map(String::as_str),
        Some("0123456789abcdef0123456789abcdef01234567")
    );
}

#[test]
fn parse_ls_remote_tags_prefers_dereferenced_commit_for_annotated_tag() {
    // Annotated tags: ls-remote emits two lines — the tag object SHA, and
    // the commit it points to with `^{}`. We must prefer the commit so it
    // can be compared with the local commit SHA from peel_to_commit().
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
    let repo = Repository::init(dir.path()).unwrap();

    // Configure user for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();

    (dir, repo)
}

/// Counter to give each commit a distinct timestamp in tests.
static COMMIT_TIME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1_700_000_000);

fn create_commit_in_repo(repo: &Repository, dir: &Path, filename: &str, message: &str) {
    let file_path = dir.join(filename);
    fs::write(&file_path, format!("content of {filename}")).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(filename)).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    // Use an incrementing timestamp so commits have deterministic ordering
    let ts = COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let sig = Signature::new("Test", "test@test.com", &git2::Time::new(ts, 0)).unwrap();

    let parents: Vec<git2::Commit> = match repo.head() {
        Ok(head) => vec![head.peel_to_commit().unwrap()],
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .unwrap();
}

fn create_lightweight_tag(repo: &Repository, tag_name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag_lightweight(tag_name, head.as_object(), false)
        .unwrap();
}

fn create_annotated_tag(repo: &Repository, tag_name: &str, message: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let sig = Signature::now("Test", "test@test.com").unwrap();
    repo.tag(tag_name, head.as_object(), &sig, message, false)
        .unwrap();
}

// -----------------------------------------------------------------------
// open_repo / get_repo_root
// -----------------------------------------------------------------------

#[test]
fn open_repo_valid() {
    let (dir, _) = init_repo();
    let repo = open_repo(dir.path()).unwrap();
    assert!(repo.workdir().is_some());
}

#[test]
fn open_repo_not_a_repo() {
    let dir = tempfile::tempdir().unwrap();
    // Empty dir, no .git
    let sub = dir.path().join("not_a_repo");
    fs::create_dir_all(&sub).unwrap();
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

// -----------------------------------------------------------------------
// tag_exists / create_tag
// -----------------------------------------------------------------------

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

// -----------------------------------------------------------------------
// find_last_tag_name
// -----------------------------------------------------------------------

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

// -----------------------------------------------------------------------
// find_highest_semver_tag
// -----------------------------------------------------------------------

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
    // Reproduces the real-world drift scenario: an older-in-time but
    // higher-semver tag (v3.0.0) exists alongside a later-in-time but
    // lower-semver tag (v2.2.0). `find_last_tag` would return v2.2.0;
    // `find_highest_semver_tag` must return v3.0.0.
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
    // "api@vnightly" matches the prefix but isn't a valid semver.
    create_lightweight_tag(&repo, "api@vnightly");
    create_lightweight_tag(&repo, "api@v1.0.0");

    let (tag_name, _) = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(tag_name, "api@v1.0.0");
}

#[test]
fn find_highest_semver_respects_orphan_warn_strategy() {
    // An orphaned higher tag is ignored under Warn — we don't want to
    // use a tag that points at a branch no longer reachable from HEAD.
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "api@v1.0.0");
    // Create a second branch, tag v9.0.0 on it, then abandon it by moving
    // HEAD back to the main branch.
    let main_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("orphan", &main_commit, false).unwrap();
    repo.set_head("refs/heads/orphan").unwrap();
    create_commit_in_repo(&repo, dir.path(), "b.txt", "orphan-only");
    create_lightweight_tag(&repo, "api@v9.0.0");
    // Back to the original branch (HEAD does not include v9.0.0 anymore).
    repo.set_head(
        repo.head()
            .unwrap()
            .shorthand()
            .map(|_| "refs/heads/master")
            .unwrap_or("refs/heads/master"),
    )
    .unwrap();
    // reset HEAD to the initial commit
    let initial_oid = main_commit.id();
    repo.reference(
        "refs/heads/master",
        initial_oid,
        true,
        "reset after orphan test",
    )
    .unwrap();
    repo.set_head("refs/heads/master").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();

    let result = find_highest_semver_tag(&repo, "api@v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(result.0, "api@v1.0.0");
}

// -----------------------------------------------------------------------
// get_commits_since_last_tag
// -----------------------------------------------------------------------

#[test]
fn get_commits_since_last_tag_no_tags() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: first");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "fix: second");

    let commits = get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
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

    let commits = get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert_eq!(commits.len(), 2);
    // Most recent first (topological order)
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

    let commits = get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert_eq!(commits.len(), 1);
    assert!(commits[0].message.contains("real change"));
}

// -----------------------------------------------------------------------
// get_changed_files
// -----------------------------------------------------------------------

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

// -----------------------------------------------------------------------
// get_changed_files_since_tag
// -----------------------------------------------------------------------

#[test]
fn get_changed_files_since_tag_all_when_no_tag() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");

    let files = get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert!(files.contains(&"a.txt".to_string()));
    assert!(files.contains(&"b.txt".to_string()));
}

#[test]
fn get_changed_files_since_tag_only_new() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
    create_lightweight_tag(&repo, "v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "second");

    let files = get_changed_files_since_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert!(!files.contains(&"a.txt".to_string()));
    assert!(files.contains(&"b.txt".to_string()));
}

// -----------------------------------------------------------------------
// create_commit
// -----------------------------------------------------------------------

#[test]
fn create_commit_adds_files() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");

    fs::write(dir.path().join("new.txt"), "new content").unwrap();
    create_commit(&repo, &["new.txt"], "feat: add new file").unwrap();

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert!(head.message().unwrap().contains("feat: add new file"));
}

// -----------------------------------------------------------------------
// create_branch_and_commit
// -----------------------------------------------------------------------

#[test]
fn create_branch_and_commit_works() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");

    fs::write(dir.path().join("release.txt"), "bumped").unwrap();
    create_branch_and_commit(&repo, "release/v1.0.0", &["release.txt"], "chore: release").unwrap();

    // Branch should exist
    assert!(
        repo.find_branch("release/v1.0.0", git2::BranchType::Local)
            .is_ok()
    );
}

#[test]
fn create_branch_and_commits_multiple() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");

    fs::write(dir.path().join("pkg1.txt"), "v1").unwrap();
    fs::write(dir.path().join("pkg2.txt"), "v2").unwrap();

    let commits: Vec<(&[&str], &str)> = vec![
        (&["pkg1.txt"], "chore(release): pkg1 v1.0.0"),
        (&["pkg2.txt"], "chore(release): pkg2 v2.0.0"),
    ];
    create_branch_and_commits(&repo, "release/multi", &commits).unwrap();

    let branch = repo
        .find_branch("release/multi", git2::BranchType::Local)
        .unwrap();
    let tip = branch.get().peel_to_commit().unwrap();
    assert_eq!(tip.message().unwrap(), "chore(release): pkg2 v2.0.0");
    let parent = tip.parent(0).unwrap();
    assert_eq!(parent.message().unwrap(), "chore(release): pkg1 v1.0.0");
}

// -----------------------------------------------------------------------
// get_remote_url
// -----------------------------------------------------------------------

#[test]
fn get_remote_url_https() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    repo.remote("origin", "https://github.com/FerrLabs/FerrFlow.git")
        .unwrap();
    let url = get_remote_url(&repo, "origin");
    assert_eq!(
        url,
        Some("https://github.com/FerrLabs/FerrFlow.git".to_string())
    );
}

#[test]
fn get_remote_url_no_remote() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    let url = get_remote_url(&repo, "origin");
    assert_eq!(url, None);
}

// -----------------------------------------------------------------------
// extract_url_password
// -----------------------------------------------------------------------

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
    assert!(helper_arg.contains("username=x-access-token"));
    assert!(helper_arg.contains("password=ff_secret"));
    assert!(
        !args.iter().any(|a| a.contains("ff_secret@")),
        "token must NOT be embedded in URL"
    );
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

// -----------------------------------------------------------------------
// orphaned tag handling
// -----------------------------------------------------------------------

/// Creates an orphaned tag scenario: tag points to a commit not reachable
/// from HEAD, but whose tree hash and message match HEAD's commit.
fn create_orphaned_tag_scenario(tag_name: &str) -> (Repository, tempfile::TempDir) {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: original");
    create_lightweight_tag(&repo, tag_name);

    // Create a new root commit with the same tree and message (simulates rebase).
    // We write the commit without updating HEAD, then force-move HEAD to it.
    {
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = head.tree().unwrap();
        let ts = COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let sig = Signature::new("Test", "test@test.com", &git2::Time::new(ts, 0)).unwrap();
        let old_id = head.id();
        let new_oid = repo
            .commit(None, &sig, &sig, "feat: original", &tree, &[])
            .unwrap();
        assert_ne!(old_id, new_oid);
        // Force-move the current branch to the new orphan commit
        let head_ref = repo.head().unwrap();
        let branch_name = head_ref.name().unwrap();
        repo.reference(branch_name, new_oid, true, "force-move for test")
            .unwrap();
    }

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

    // Create a completely different root commit (different tree and message)
    {
        let ts = COMMIT_TIME.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let sig = Signature::new("Test", "test@test.com", &git2::Time::new(ts, 0)).unwrap();
        fs::write(dir.path().join("b.txt"), "different").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("b.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let new_oid = repo
            .commit(None, &sig, &sig, "feat: totally different", &tree, &[])
            .unwrap();
        let head_ref = repo.head().unwrap();
        let branch_name = head_ref.name().unwrap();
        repo.reference(branch_name, new_oid, true, "force-move for test")
            .unwrap();
    }

    let result = find_last_tag_name(&repo, "v", OrphanedTagStrategy::TreeHash).unwrap();
    assert_eq!(result, None);
}

#[test]
fn get_commits_since_orphaned_tag_with_recovery() {
    let (repo, dir) = create_orphaned_tag_scenario("v1.0.0");
    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: new feature");

    let commits = get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::TreeHash).unwrap();
    assert_eq!(commits.len(), 1);
    assert!(commits[0].message.contains("new feature"));
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

    // Stable commits should include everything since v1.0.0
    let commits = get_commits_since_last_stable_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
    assert_eq!(commits.len(), 3);

    // Regular commits should include only since v2.0.0-beta.2
    let commits = get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
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
fn credentials_callback_uses_oauth2_for_gitlab() {
    unsafe { std::env::set_var("FERRFLOW_TOKEN", "test-token") };
    let result = credentials_callback(
        "https://gitlab.com/group/project.git",
        None,
        CredentialType::USER_PASS_PLAINTEXT,
    );
    unsafe { std::env::remove_var("FERRFLOW_TOKEN") };
    assert!(result.is_ok());
}

#[test]
fn credentials_callback_uses_x_access_token_for_github() {
    unsafe { std::env::set_var("FERRFLOW_TOKEN", "test-token") };
    let result = credentials_callback(
        "https://github.com/owner/repo.git",
        None,
        CredentialType::USER_PASS_PLAINTEXT,
    );
    unsafe { std::env::remove_var("FERRFLOW_TOKEN") };
    assert!(result.is_ok());
}

#[test]
fn credentials_callback_falls_back_to_github_token() {
    unsafe { std::env::remove_var("FERRFLOW_TOKEN") };
    unsafe { std::env::set_var("GITHUB_TOKEN", "gh-fallback-token") };
    let result = credentials_callback(
        "https://github.com/owner/repo.git",
        None,
        CredentialType::USER_PASS_PLAINTEXT,
    );
    unsafe { std::env::remove_var("GITHUB_TOKEN") };
    assert!(result.is_ok());
}

#[test]
fn credentials_callback_falls_back_to_gitlab_token() {
    unsafe { std::env::remove_var("FERRFLOW_TOKEN") };
    unsafe { std::env::set_var("GITLAB_TOKEN", "gl-fallback-token") };
    let result = credentials_callback(
        "https://gitlab.com/group/project.git",
        None,
        CredentialType::USER_PASS_PLAINTEXT,
    );
    unsafe { std::env::remove_var("GITLAB_TOKEN") };
    assert!(result.is_ok());
}

#[test]
fn credentials_callback_uses_oauth2_for_self_hosted_gitlab() {
    unsafe { std::env::set_var("FERRFLOW_TOKEN", "test-token") };
    let result = credentials_callback(
        "https://git.example.gitlab.com/group/project.git",
        None,
        CredentialType::USER_PASS_PLAINTEXT,
    );
    unsafe { std::env::remove_var("FERRFLOW_TOKEN") };
    assert!(result.is_ok());
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
    // Floating tags: major-only or major.minor
    assert!(is_floating_tag("v2", "v"));
    assert!(is_floating_tag("v2.3", "v"));
    assert!(is_floating_tag("v10", "v"));
    assert!(is_floating_tag("v0", "v"));

    // Full version tags are NOT floating
    assert!(!is_floating_tag("v2.14.1", "v"));
    assert!(!is_floating_tag("v0.1.0", "v"));
    assert!(!is_floating_tag("v1.0.0", "v"));
    assert!(!is_floating_tag("v10.20.30", "v"));

    // Monorepo prefixes
    assert!(is_floating_tag("api@v1", "api@v"));
    assert!(is_floating_tag("api@v1.2", "api@v"));
    assert!(!is_floating_tag("api@v1.2.3", "api@v"));

    // Pre-release tags are NOT floating (contain non-digit chars)
    assert!(!is_floating_tag("v2.0.0-beta.1", "v"));
    assert!(!is_floating_tag("v1.0.0-rc.1", "v"));

    // Edge case: prefix matches exactly (empty version part)
    assert!(!is_floating_tag("v", "v"));
}

#[test]
fn find_last_tag_skips_floating_tags() {
    let (dir, repo) = init_repo();

    create_commit_in_repo(&repo, dir.path(), "a.txt", "feat: initial");
    repo.tag_lightweight(
        "v1.0.0",
        &repo.head().unwrap().peel_to_commit().unwrap().into_object(),
        false,
    )
    .unwrap();

    create_commit_in_repo(&repo, dir.path(), "b.txt", "feat: second");
    // Create a floating tag pointing to a newer commit
    repo.tag_lightweight(
        "v1",
        &repo.head().unwrap().peel_to_commit().unwrap().into_object(),
        false,
    )
    .unwrap();

    let result = find_last_tag(&repo, "v", OrphanedTagStrategy::Warn)
        .unwrap()
        .unwrap();
    assert_eq!(result.name, "v1.0.0");
}

// -----------------------------------------------------------------------
// resolve_current_branch
// -----------------------------------------------------------------------

#[test]
fn resolve_branch_from_head() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    let branch = resolve_current_branch(&repo, "fallback");
    // HEAD points to the default branch, not "fallback"
    assert_ne!(branch, "fallback");
    assert!(!branch.is_empty());
}

#[test]
fn resolve_branch_detached_returns_non_empty() {
    let (dir, repo) = init_repo();
    create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
    let head_oid = repo.head().unwrap().target().unwrap();
    repo.set_head_detached(head_oid).unwrap();

    // In detached state, the function should return either a CI env var
    // or the fallback — never an empty string.
    let branch = resolve_current_branch(&repo, "my-fallback");
    assert!(!branch.is_empty());
}

// -----------------------------------------------------------------------
// fetch_and_rebase — regression test for #367
// -----------------------------------------------------------------------

/// Simulates what the release bot does when a concurrent push advances
/// main between the action's checkout and its push:
///
///   A  ← common base
///   ├── B   (fast-forward on the remote while we were working —
///   │       analog of a feature PR merging just before we push)
///   └── X   (our local release commit, parent = A)
///
/// After fetch_and_rebase, the replayed X' must contain BOTH B's changes
/// and X's changes. Issue #367: the previous merge_trees arg order quietly
/// reverted B so the rebased commit ended up as "A + X — B" — losing
/// every file B had touched.
#[test]
fn fetch_and_rebase_preserves_concurrent_remote_changes() {
    use std::path::Path as StdPath;
    let base_dir = tempfile::tempdir().unwrap();

    // --- Set up a bare "remote" repo ---
    // init_bare picks the default branch name from git's init.defaultBranch
    // config, which differs by platform (master on some Linux distros,
    // main on newer ones, etc.). Avoid the whole problem by not using the
    // bare's default at all: we create commits locally, set our *own*
    // HEAD to a fixed branch name, and point the bare's HEAD at it
    // symbolically.
    let remote_path = base_dir.path().join("remote.git");
    let bare = Repository::init_bare(&remote_path).unwrap();
    // Make the bare's HEAD symbolic ref target "main" so clones pick that.
    bare.set_head("refs/heads/main").unwrap();
    drop(bare);

    // --- Set up a local working repo (non-clone to avoid default-branch
    //     pitfalls) and wire origin to the bare remote manually. ---
    let local_path = base_dir.path().join("local");
    std::fs::create_dir_all(&local_path).unwrap();
    let repo = Repository::init(&local_path).unwrap();
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@test.com").unwrap();
    }
    repo.remote("origin", remote_path.to_str().unwrap())
        .unwrap();
    // Pin local HEAD to refs/heads/main so create_commit_in_repo commits
    // onto the branch we expect.
    repo.set_head("refs/heads/main").unwrap();

    create_commit_in_repo(&repo, &local_path, "base.txt", "commit A");
    // Push A to the remote so it becomes the shared base.
    repo.find_remote("origin")
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();
    let base_oid = repo.head().unwrap().target().unwrap();

    // --- Advance the remote with commit B (simulated concurrent merge) ---
    // Same init-then-add-remote dance to keep the branch naming under our
    // control.
    let helper_path = base_dir.path().join("helper");
    std::fs::create_dir_all(&helper_path).unwrap();
    let helper = Repository::init(&helper_path).unwrap();
    {
        let mut cfg = helper.config().unwrap();
        cfg.set_str("user.name", "Helper").unwrap();
        cfg.set_str("user.email", "helper@test.com").unwrap();
    }
    helper
        .remote("origin", remote_path.to_str().unwrap())
        .unwrap();
    // Fetch + check out main from the remote so we commit on top of A,
    // not on an unrelated root.
    helper
        .find_remote("origin")
        .unwrap()
        .fetch(&["refs/heads/main:refs/heads/main"], None, None)
        .unwrap();
    helper.set_head("refs/heads/main").unwrap();
    helper
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    create_commit_in_repo(&helper, &helper_path, "from_concurrent_pr.txt", "commit B");
    helper
        .find_remote("origin")
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();

    // --- Back in local, create commit X on top of A (the release commit) ---
    // Local HEAD is still at A at this point (we haven't fetched).
    assert_eq!(repo.head().unwrap().target().unwrap(), base_oid);
    create_commit_in_repo(&repo, &local_path, "release_commit.txt", "commit X");

    // --- Call fetch_and_rebase on local ---
    fetch_and_rebase(&repo, "origin", "main").expect("rebase should succeed");

    // --- Verify: the resulting HEAD tree contains BOTH B's file and X's file ---
    let tip = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = tip.tree().unwrap();
    assert!(
        tree.get_path(StdPath::new("base.txt")).is_ok(),
        "base.txt (from A) must be in the rebased tree"
    );
    assert!(
        tree.get_path(StdPath::new("from_concurrent_pr.txt"))
            .is_ok(),
        "from_concurrent_pr.txt (from B — the concurrent remote change) must be in the rebased tree; \
             this is the regression from #367"
    );
    assert!(
        tree.get_path(StdPath::new("release_commit.txt")).is_ok(),
        "release_commit.txt (from X — our local commit being rebased) must be in the rebased tree"
    );

    // The new tip's first parent should be B's oid (the remote HEAD we fetched).
    let parent = tip.parent(0).unwrap();
    let parent_tree = parent.tree().unwrap();
    assert!(
        parent_tree
            .get_path(StdPath::new("from_concurrent_pr.txt"))
            .is_ok(),
        "rebased commit's parent should be B (the fetched remote HEAD)"
    );
}

// ── reset_branch_to_remote — used by the release retry path ─────────
//
// Topology (same skeleton as the rebase test):
//   A   ← shared base (pushed)
//   ├── B   (advances on the remote)
//   └── X   (our local in-progress release commit, plus dirty files)
//
// After reset_branch_to_remote, local HEAD must be at B (remote tip),
// X must be gone, and dirty working-tree files introduced after X
// must have been wiped — that's the contract the release retry loop
// depends on.
#[test]
fn reset_branch_to_remote_drops_local_commit_and_dirty_tree() {
    use std::path::Path as StdPath;
    let base_dir = tempfile::tempdir().unwrap();

    let remote_path = base_dir.path().join("remote.git");
    let bare = Repository::init_bare(&remote_path).unwrap();
    bare.set_head("refs/heads/main").unwrap();
    drop(bare);

    let local_path = base_dir.path().join("local");
    std::fs::create_dir_all(&local_path).unwrap();
    let repo = Repository::init(&local_path).unwrap();
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@test.com").unwrap();
    }
    repo.remote("origin", remote_path.to_str().unwrap())
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();

    create_commit_in_repo(&repo, &local_path, "base.txt", "commit A");
    repo.find_remote("origin")
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();

    // Advance the remote with B.
    let helper_path = base_dir.path().join("helper");
    std::fs::create_dir_all(&helper_path).unwrap();
    let helper = Repository::init(&helper_path).unwrap();
    {
        let mut cfg = helper.config().unwrap();
        cfg.set_str("user.name", "Helper").unwrap();
        cfg.set_str("user.email", "helper@test.com").unwrap();
    }
    helper
        .remote("origin", remote_path.to_str().unwrap())
        .unwrap();
    helper
        .find_remote("origin")
        .unwrap()
        .fetch(&["refs/heads/main:refs/heads/main"], None, None)
        .unwrap();
    helper.set_head("refs/heads/main").unwrap();
    helper
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    create_commit_in_repo(&helper, &helper_path, "remote_only.txt", "commit B");
    let remote_b_oid = helper.head().unwrap().target().unwrap();
    helper
        .find_remote("origin")
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();

    // Local: build the release commit X on top of A and add a dirty,
    // unstaged file that the cleanup must also wipe.
    create_commit_in_repo(&repo, &local_path, "release.txt", "commit X");
    let x_oid = repo.head().unwrap().target().unwrap();
    std::fs::write(local_path.join("dirty.txt"), "stale hook output").unwrap();

    reset_branch_to_remote(&repo, "origin", "main").expect("reset must succeed");

    // HEAD is at B, not X.
    let new_head = repo.head().unwrap().target().unwrap();
    assert_eq!(new_head, remote_b_oid, "HEAD must be at remote B");
    assert_ne!(new_head, x_oid, "HEAD must not still be at X");

    // Working tree was reset: release.txt is gone, base.txt and
    // remote_only.txt exist, dirty.txt was wiped.
    assert!(!local_path.join("release.txt").exists());
    assert!(local_path.join("base.txt").exists());
    assert!(local_path.join("remote_only.txt").exists());
    assert!(!local_path.join("dirty.txt").exists());

    // The branch ref must also point at B, and HEAD must be reattached
    // to refs/heads/main (so subsequent commits land on the branch).
    let main_ref = repo.find_reference("refs/heads/main").unwrap();
    assert_eq!(main_ref.target().unwrap(), remote_b_oid);
    assert!(repo.head().unwrap().is_branch());
    let _ = StdPath::new("dummy"); // silence unused import on some configs
}

// is_push_rejected_error — used by the retry trigger.
#[test]
fn is_push_rejected_error_recognises_known_signatures() {
    // GIT_PUSH_REJECTED via attached ErrorCode.
    let e = anyhow::anyhow!("upstream rejected the push").context(error_code::GIT_PUSH_REJECTED);
    assert!(is_push_rejected_error(&e));

    // Rebase conflict bail message.
    let e =
        anyhow::anyhow!("Rebase conflict: cannot rebase release commits on top of remote 'main'.");
    assert!(is_push_rejected_error(&e));

    // Server-side rule violation phrasing as it comes back from GitHub.
    let e = anyhow::anyhow!("refs/heads/main: push declined due to repository rule violations");
    assert!(is_push_rejected_error(&e));

    // Plain non-fast-forward from libgit2.
    let e = anyhow::anyhow!(
        "Updates were rejected because the tip of your current branch is non-fast-forward"
    );
    assert!(is_push_rejected_error(&e));

    // Unrelated error must not match.
    let e = anyhow::anyhow!("hook failed: prettier exited with status 1");
    assert!(!is_push_rejected_error(&e));
}
