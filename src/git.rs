use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use git2::{Cred, CredentialType, PushOptions, RemoteCallbacks, Repository, Sort};
use std::path::{Path, PathBuf};

pub use crate::changelog::GitLog;
use crate::config::OrphanedTagStrategy;
use crate::error_code::{self, ErrorCodeExt};

pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path)
        .with_context(|| format!("Not a git repository: {}", path.display()))
        .error_code(error_code::GIT_NOT_A_REPO)
}

pub fn get_repo_root(repo: &Repository) -> Result<PathBuf> {
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))
        .error_code(error_code::GIT_BARE_REPO)
}

/// Resolve the current branch name from HEAD, falling back to CI environment
/// variables when in detached HEAD state (common in CI runners).
pub fn resolve_current_branch(repo: &Repository, fallback: &str) -> String {
    // Try git2 first — works when HEAD points to a branch
    if let Ok(head) = repo.head()
        && head.is_branch()
        && let Some(name) = head.shorthand()
    {
        return name.to_string();
    }

    // Detached HEAD — try CI environment variables
    let ci_vars = [
        "GITHUB_REF_NAME",  // GitHub Actions
        "CI_COMMIT_BRANCH", // GitLab CI
        "BRANCH_NAME",      // Jenkins
        "CIRCLE_BRANCH",    // CircleCI
        "BITBUCKET_BRANCH", // Bitbucket Pipelines
        "BUILDKITE_BRANCH", // Buildkite
        "TRAVIS_BRANCH",    // Travis CI
    ];

    for var in ci_vars {
        if let Ok(val) = std::env::var(var)
            && !val.is_empty()
        {
            return val;
        }
    }

    fallback.to_string()
}

pub fn get_commits_since_last_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix, strategy)?;

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in walk {
        let oid = oid?;
        if let Some(stop) = last_tag_oid
            && oid == stop
        {
            break;
        }
        if let Ok(commit) = repo.find_commit(oid) {
            let message = commit.message().unwrap_or("").to_string();
            if message.contains("[skip ci]") {
                continue;
            }
            commits.push(GitLog {
                hash: oid.to_string()[..8].to_string(),
                message,
            });
        }
    }

    Ok(commits)
}

struct TagMatch {
    name: String,
    commit_oid: git2::Oid,
    time: i64,
}

fn find_matching_commit(
    repo: &Repository,
    orphaned_commit: &git2::Commit,
    strategy: &OrphanedTagStrategy,
) -> Option<git2::Oid> {
    let mut walk = repo.revwalk().ok()?;
    walk.push_head().ok()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME).ok()?;

    let limit = 1000;

    for (count, oid) in walk.enumerate() {
        if count >= limit {
            break;
        }
        let oid = match oid {
            Ok(o) => o,
            Err(_) => continue,
        };
        let candidate = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let matched = match strategy {
            OrphanedTagStrategy::TreeHash => candidate.tree_id() == orphaned_commit.tree_id(),
            OrphanedTagStrategy::Message => candidate.message() == orphaned_commit.message(),
            OrphanedTagStrategy::Warn => return None,
        };
        if matched {
            return Some(oid);
        }
    }
    None
}

/// Returns true if a tag looks like a floating tag (e.g. `v2` or `v2.3`) rather
/// than a full version tag (e.g. `v2.14.1`). Floating tags have at most one `.`
/// in the version part and contain only digits and dots.
fn is_floating_tag(tag_name: &str, prefix: &str) -> bool {
    let version_part = tag_name.strip_prefix(prefix).unwrap_or(tag_name);
    if version_part.is_empty() {
        return false;
    }
    // Floating tags are purely numeric with at most one dot: "2" or "2.3"
    let is_numeric = version_part.chars().all(|c| c.is_ascii_digit() || c == '.');
    let dot_count = version_part.chars().filter(|&c| c == '.').count();
    is_numeric && dot_count <= 1
}

fn find_last_tag(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<TagMatch>> {
    let head = repo.head()?.peel_to_commit()?.id();
    let latest: RefCell<Option<TagMatch>> = RefCell::new(None);
    let warnings: RefCell<Vec<String>> = RefCell::new(Vec::new());

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if !tag_name.starts_with(prefix) || is_floating_tag(tag_name, prefix) {
            return true;
        }

        let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
            tag_obj.target_id()
        } else {
            oid
        };

        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => {
                warnings.borrow_mut().push(format!(
                    "Warning: tag '{}' points to missing commit {} (likely garbage-collected). Skipping.\n  \
                     Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                     See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                    tag_name,
                    &commit_oid.to_string()[..7]
                ));
                return true;
            }
        };

        let reachable =
            head == commit_oid || repo.graph_descendant_of(head, commit_oid).unwrap_or(false);

        let (effective_oid, effective_time) = if reachable {
            (commit_oid, commit.time().seconds())
        } else {
            let short = &commit_oid.to_string()[..7];
            if strategy == OrphanedTagStrategy::Warn {
                warnings.borrow_mut().push(format!(
                    "Warning: tag '{}' points to orphaned commit {} (not reachable from HEAD).\n  \
                     Hint: set 'orphanedTagStrategy' to 'treeHash' or 'message' for automatic recovery.\n  \
                     See https://ferrflow.com/docs/configuration/config-file#orphaned-tag-strategy",
                    tag_name, short
                ));
                return true;
            }
            match find_matching_commit(repo, &commit, &strategy) {
                Some(matched_oid) => {
                    let strategy_name = match strategy {
                        OrphanedTagStrategy::TreeHash => "tree-hash",
                        OrphanedTagStrategy::Message => "message",
                        OrphanedTagStrategy::Warn => unreachable!(),
                    };
                    warnings.borrow_mut().push(format!(
                        "Info: tag '{}' was orphaned but matched commit {} on current branch via {}.",
                        tag_name,
                        &matched_oid.to_string()[..7],
                        strategy_name
                    ));
                    let matched_commit = match repo.find_commit(matched_oid) {
                        Ok(c) => c,
                        Err(_) => return true,
                    };
                    (matched_oid, matched_commit.time().seconds())
                }
                None => {
                    let strategy_name = match strategy {
                        OrphanedTagStrategy::TreeHash => "tree-hash",
                        OrphanedTagStrategy::Message => "message",
                        OrphanedTagStrategy::Warn => unreachable!(),
                    };
                    warnings.borrow_mut().push(format!(
                        "Warning: tag '{}' points to orphaned commit {}. No match found via {}. Skipping.\n  \
                         Hint: re-tag manually with 'git tag -f {} <correct-commit>'",
                        tag_name, short, strategy_name, tag_name
                    ));
                    return true;
                }
            }
        };

        let mut latest_ref = latest.borrow_mut();
        if latest_ref.is_none() || effective_time > latest_ref.as_ref().unwrap().time {
            *latest_ref = Some(TagMatch {
                name: tag_name.to_string(),
                commit_oid: effective_oid,
                time: effective_time,
            });
        }
        true
    })?;

    for w in warnings.borrow().iter() {
        eprintln!("{}", w);
    }

    Ok(latest.into_inner())
}

pub fn find_last_tag_name(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<String>> {
    Ok(find_last_tag(repo, prefix, strategy)?.map(|t| t.name))
}

fn find_last_tag_commit(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<git2::Oid>> {
    Ok(find_last_tag(repo, prefix, strategy)?.map(|t| t.commit_oid))
}

/// Check if a tag name contains a pre-release suffix (has a `-` in the version part).
fn is_prerelease_tag(tag_name: &str, prefix: &str) -> bool {
    let version_part = tag_name.strip_prefix(prefix).unwrap_or(tag_name);
    version_part.contains('-')
}

/// Like `find_last_tag`, but skips pre-release tags (those with `-` in version part).
fn find_last_stable_tag(
    repo: &Repository,
    prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Option<TagMatch>> {
    let head = repo.head()?.peel_to_commit()?.id();
    let latest: RefCell<Option<TagMatch>> = RefCell::new(None);

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if !tag_name.starts_with(prefix)
            || is_prerelease_tag(tag_name, prefix)
            || is_floating_tag(tag_name, prefix)
        {
            return true;
        }

        let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
            tag_obj.target_id()
        } else {
            oid
        };

        let commit = match repo.find_commit(commit_oid) {
            Ok(c) => c,
            Err(_) => return true,
        };

        let reachable =
            head == commit_oid || repo.graph_descendant_of(head, commit_oid).unwrap_or(false);

        let (effective_oid, effective_time) = if reachable {
            (commit_oid, commit.time().seconds())
        } else {
            if strategy == OrphanedTagStrategy::Warn {
                return true;
            }
            match find_matching_commit(repo, &commit, &strategy) {
                Some(matched_oid) => {
                    let matched_commit = match repo.find_commit(matched_oid) {
                        Ok(c) => c,
                        Err(_) => return true,
                    };
                    (matched_oid, matched_commit.time().seconds())
                }
                None => return true,
            }
        };

        let mut latest_ref = latest.borrow_mut();
        if latest_ref.is_none() || effective_time > latest_ref.as_ref().unwrap().time {
            *latest_ref = Some(TagMatch {
                name: tag_name.to_string(),
                commit_oid: effective_oid,
                time: effective_time,
            });
        }
        true
    })?;

    Ok(latest.into_inner())
}

pub fn get_commits_since_last_stable_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_stable_tag(repo, tag_prefix, strategy)?.map(|t| t.commit_oid);

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in walk {
        let oid = oid?;
        if let Some(stop) = last_tag_oid
            && oid == stop
        {
            break;
        }
        if let Ok(commit) = repo.find_commit(oid) {
            let message = commit.message().unwrap_or("").to_string();
            if message.contains("[skip ci]") {
                continue;
            }
            commits.push(GitLog {
                hash: oid.to_string()[..8].to_string(),
                message,
            });
        }
    }

    Ok(commits)
}

/// Collect all tag names in the repository.
pub fn collect_all_tags(repo: &Repository) -> Vec<String> {
    let mut tags = Vec::new();
    let _ = repo.tag_foreach(|_oid, name| {
        let name = String::from_utf8_lossy(name);
        tags.push(name.trim_start_matches("refs/tags/").to_string());
        true
    });
    tags
}

pub fn get_changed_files(repo: &Repository) -> Result<Vec<String>> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => return Ok(vec![]),
    };
    let head_tree = head.tree()?;

    let files = if let Ok(parent) = head.parent(0) {
        let parent_tree = parent.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&head_tree), None)?;
        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files.push(path.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        )?;
        files
    } else {
        let mut files = Vec::new();
        head_tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
            if let Some(name) = entry.name() {
                files.push(name.to_string());
            }
            git2::TreeWalkResult::Ok
        })?;
        files
    };

    Ok(files)
}

pub fn get_changed_files_since_tag(
    repo: &Repository,
    tag_prefix: &str,
    strategy: OrphanedTagStrategy,
) -> Result<Vec<String>> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => return Ok(vec![]),
    };
    let head_tree = head.tree()?;

    let old_tree = if let Some(tag_oid) = find_last_tag_commit(repo, tag_prefix, strategy)? {
        let tag_commit = repo.find_commit(tag_oid)?;
        Some(tag_commit.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(old_tree.as_ref(), Some(&head_tree), None)?;
    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                files.push(path.to_string_lossy().to_string());
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(files)
}

/// Extract password from a URL like `https://user:password@host/path`.
fn extract_url_password(url: &str) -> Option<(String, String)> {
    let after_scheme = url.split("://").nth(1)?;
    let userinfo = after_scheme.split('@').next()?;
    let (user, password) = userinfo.split_once(':')?;
    if password.is_empty() {
        return None;
    }
    Some((user.to_string(), password.to_string()))
}

fn credentials_callback(
    url: &str,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> std::result::Result<Cred, git2::Error> {
    if allowed_types.contains(CredentialType::SSH_KEY) {
        return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
    }
    if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
        // 1. Try FERRFLOW_TOKEN or GITHUB_TOKEN/GITLAB_TOKEN env vars
        if let Ok(token) = std::env::var("FERRFLOW_TOKEN").or_else(|_| {
            if url.contains("gitlab") {
                std::env::var("GITLAB_TOKEN")
            } else {
                std::env::var("GITHUB_TOKEN")
            }
        }) {
            let user = username_from_url.unwrap_or_else(|| {
                if url.contains("gitlab") {
                    "oauth2"
                } else {
                    "x-access-token"
                }
            });
            return Cred::userpass_plaintext(user, &token);
        }
        // 2. Try credentials embedded in the remote URL
        if let Some((user, password)) = extract_url_password(url) {
            return Cred::userpass_plaintext(&user, &password);
        }
        // 3. Try git credential helper (local dev)
        if let Ok(cfg) = git2::Config::open_default()
            && let Ok(cred) = Cred::credential_helper(&cfg, url, username_from_url)
        {
            return Ok(cred);
        }
        eprintln!(
            "Warning: No git credentials found. Set FERRFLOW_TOKEN (or GITHUB_TOKEN/GITLAB_TOKEN) or embed credentials in the remote URL."
        );
    }
    Cred::default()
}

fn make_fetch_options() -> git2::FetchOptions<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(callbacks);
    opts
}

pub fn fetch_tags(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;
    let mut opts = make_fetch_options();
    remote.fetch(&["refs/tags/*:refs/tags/*"], Some(&mut opts), None)?;
    Ok(())
}

pub fn tag_exists(repo: &Repository, tag_name: &str) -> bool {
    repo.refname_to_id(&format!("refs/tags/{tag_name}")).is_ok()
}

pub fn create_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<()> {
    if tag_exists(repo, tag_name) {
        Err(anyhow::anyhow!("tag {tag_name} already exists"))
            .error_code(error_code::GIT_TAG_EXISTS)?;
    }
    let head = repo.head()?.peel_to_commit()?;
    let sig = signature(repo)?;
    repo.tag(tag_name, head.as_object(), &sig, message, false)?;
    Ok(())
}

/// Create a tag, or move it if it already exists. Returns true if the tag was moved.
pub fn create_or_move_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<bool> {
    let existed = tag_exists(repo, tag_name);
    if existed {
        repo.tag_delete(tag_name)?;
    }
    let head = repo.head()?.peel_to_commit()?;
    let sig = signature(repo)?;
    repo.tag(tag_name, head.as_object(), &sig, message, false)?;
    Ok(existed)
}

pub fn force_push_tags(repo: &Repository, remote_name: &str, tags: &[&str]) -> Result<()> {
    if tags.is_empty() {
        return Ok(());
    }
    let mut remote = get_authenticated_remote(repo, remote_name)?;

    let push_errors = Rc::new(RefCell::new(Vec::new()));
    let mut push_options = make_push_options(push_errors.clone());

    let refspecs: Vec<String> = tags
        .iter()
        .map(|tag| format!("+refs/tags/{tag}:refs/tags/{tag}"))
        .collect();
    let refspec_refs: Vec<&str> = refspecs.iter().map(String::as_str).collect();
    remote
        .push(&refspec_refs, Some(&mut push_options))
        .with_context(|| "Failed to force-push floating tags")
        .error_code(error_code::GIT_FLOATING_TAGS)?;
    check_push_errors(&push_errors)
        .with_context(|| "Floating tag push rejected")
        .error_code(error_code::GIT_FLOATING_TAGS)?;
    Ok(())
}

/// If a tag exists, return its message.
pub fn get_tag_message(repo: &Repository, tag_name: &str) -> Option<String> {
    let oid = repo.refname_to_id(&format!("refs/tags/{tag_name}")).ok()?;
    let obj = repo.find_object(oid, None).ok()?;
    let tag = obj.as_tag()?;
    tag.message().map(String::from)
}

fn signature(repo: &Repository) -> Result<git2::Signature<'static>> {
    if let Ok(sig) = repo.signature() {
        return Ok(sig);
    }
    Ok(git2::Signature::now("FerrFlow", "contact@ferrflow.com")?)
}

pub fn create_commit(repo: &Repository, files: &[&str], message: &str) -> Result<()> {
    let mut index = repo.index()?;
    for file in files {
        index.add_path(Path::new(file))?;
    }
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = signature(repo)?;
    let parent = repo.head()?.peel_to_commit()?;

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;
    Ok(())
}

pub fn get_remote_url(repo: &Repository, remote_name: &str) -> Option<String> {
    let remote = repo.find_remote(remote_name).ok()?;
    Some(remote.url()?.to_string())
}

pub fn create_branch_and_commit(
    repo: &Repository,
    branch_name: &str,
    files: &[&str],
    message: &str,
) -> Result<()> {
    create_branch_and_commits(repo, branch_name, &[(files, message)])
}

pub fn create_branch_and_commits(
    repo: &Repository,
    branch_name: &str,
    commits: &[(&[&str], &str)],
) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(branch_name, &head, false)?;

    let refname = format!("refs/heads/{branch_name}");
    let sig = signature(repo)?;
    let mut parent = head;

    for (files, message) in commits {
        let mut index = repo.index()?;
        for file in *files {
            index.add_path(Path::new(file))?;
        }
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let oid = repo.commit(Some(&refname), &sig, &sig, message, &tree, &[&parent])?;
        parent = repo.find_commit(oid)?;
    }
    Ok(())
}

/// Build an authenticated remote URL when `FERRFLOW_TOKEN` is set.
/// This ensures the token takes priority over any credentials embedded in the
/// original URL (e.g. `gitlab-ci-token:xxx` injected by GitLab CI).
fn authenticated_remote_url(url: &str) -> Option<String> {
    let token = std::env::var("FERRFLOW_TOKEN").ok()?;
    let user = if url.contains("gitlab") {
        "oauth2"
    } else {
        "x-access-token"
    };
    if let Some(scheme_end) = url.find("://") {
        let scheme = &url[..scheme_end];
        let rest = &url[scheme_end + 3..];
        let host_and_path = if let Some(at) = rest.find('@') {
            &rest[at + 1..]
        } else {
            rest
        };
        Some(format!("{scheme}://{user}:{token}@{host_and_path}"))
    } else {
        None
    }
}

/// Get a remote, overriding its URL with `FERRFLOW_TOKEN` credentials when available.
fn get_authenticated_remote<'a>(
    repo: &'a Repository,
    remote_name: &str,
) -> Result<git2::Remote<'a>> {
    let remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("Remote '{}' not found", remote_name))
        .error_code(error_code::GIT_REMOTE_NOT_FOUND)?;
    if let Some(url) = remote.url()
        && let Some(authed_url) = authenticated_remote_url(url)
    {
        drop(remote);
        return Ok(repo.remote_anonymous(&authed_url)?);
    }
    Ok(remote)
}

fn make_push_options(push_errors: Rc<RefCell<Vec<String>>>) -> PushOptions<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);
    let errors = push_errors.clone();
    callbacks.push_update_reference(move |refname, status| {
        if let Some(msg) = status {
            errors.borrow_mut().push(format!("{refname}: {msg}"));
        }
        Ok(())
    });
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);
    push_options
}

fn check_push_errors(errors: &RefCell<Vec<String>>) -> Result<()> {
    let errs = errors.borrow();
    if errs.is_empty() {
        return Ok(());
    }
    let joined = errs.join("; ");
    Err(anyhow::anyhow!("Push rejected by remote: {joined}"))
        .error_code(error_code::GIT_PUSH_REJECTED)?;
    Ok(())
}

pub fn verify_remote_branch(
    repo: &Repository,
    remote_name: &str,
    branch: &str,
    expected_oid: git2::Oid,
) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);

    let connection = remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

    let expected_ref = format!("refs/heads/{branch}");
    for head in connection.list()? {
        if head.name() == expected_ref {
            if head.oid() == expected_oid {
                return Ok(());
            }
            Err(anyhow::anyhow!(
                "Remote branch '{}' points to {} but expected {}",
                branch,
                head.oid(),
                expected_oid,
            ))
            .error_code(error_code::GIT_PUSH_VERIFY_FAILED)?;
        }
    }
    Err(anyhow::anyhow!(
        "Remote branch '{}' not found after push",
        branch
    ))
    .error_code(error_code::GIT_REMOTE_BRANCH_NOT_FOUND)?;
    Ok(())
}

/// Resolve the local refspec source for a branch push.
/// In CI environments with detached HEAD, `refs/heads/{branch}` may not exist,
/// so we fall back to pushing HEAD directly.
fn resolve_push_source(repo: &Repository, branch: &str) -> String {
    let local_ref = format!("refs/heads/{branch}");
    if repo.find_reference(&local_ref).is_ok() {
        local_ref
    } else {
        "HEAD".to_string()
    }
}

pub fn push_branch(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    try_push_branch(repo, remote_name, branch)
}

pub fn push_tags(repo: &Repository, remote_name: &str, tags: &[&str]) -> Result<()> {
    if tags.is_empty() {
        return Ok(());
    }
    let mut remote = get_authenticated_remote(repo, remote_name)?;

    let push_errors = Rc::new(RefCell::new(Vec::new()));
    let mut opts = make_push_options(push_errors.clone());

    let tag_refspecs: Vec<String> = tags
        .iter()
        .map(|tag| format!("refs/tags/{tag}:refs/tags/{tag}"))
        .collect();
    let tag_refs: Vec<&str> = tag_refspecs.iter().map(String::as_str).collect();
    remote
        .push(&tag_refs, Some(&mut opts))
        .with_context(|| "Failed to push tags")
        .error_code(error_code::GIT_PUSH_TAGS)?;
    check_push_errors(&push_errors)
        .with_context(|| "Tag push rejected")
        .error_code(error_code::GIT_PUSH_TAGS)?;
    Ok(())
}

fn try_push_branch(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;
    let push_errors = Rc::new(RefCell::new(Vec::new()));
    let mut opts = make_push_options(push_errors.clone());
    let source = resolve_push_source(repo, branch);
    let branch_refspec = format!("{source}:refs/heads/{branch}");
    remote
        .push(&[&branch_refspec], Some(&mut opts))
        .with_context(|| format!("Failed to push branch '{branch}'"))
        .error_code(error_code::GIT_PUSH_BRANCH)?;
    check_push_errors(&push_errors)
        .with_context(|| format!("Branch push rejected for '{branch}'"))
        .error_code(error_code::GIT_PUSH_REJECTED)?;
    Ok(())
}

/// Fetch the remote branch and rebase local commits on top of it.
/// Returns Ok(()) if the rebase succeeded, or an error if it failed.
fn fetch_and_rebase(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    // Fetch the remote branch
    let mut remote = get_authenticated_remote(repo, remote_name)?;
    let mut opts = make_fetch_options();
    remote.fetch(
        &[&format!(
            "refs/heads/{branch}:refs/remotes/{remote_name}/{branch}"
        )],
        Some(&mut opts),
        None,
    )?;
    drop(remote);

    let remote_ref = format!("refs/remotes/{remote_name}/{branch}");
    let remote_oid = repo
        .refname_to_id(&remote_ref)
        .with_context(|| format!("Could not find remote ref {remote_ref} after fetch"))?;

    let local_commit = repo.head()?.peel_to_commit()?;
    let local_oid = local_commit.id();

    // If already up-to-date or remote is behind, nothing to rebase
    if remote_oid == local_oid || repo.graph_descendant_of(local_oid, remote_oid)? {
        return Ok(());
    }

    // Count how many local commits are ahead of the merge base
    let merge_base = repo
        .merge_base(local_oid, remote_oid)
        .with_context(|| "No common ancestor between local and remote branch")?;

    // Collect local commits from HEAD back to merge_base (exclusive)
    let mut local_commits = Vec::new();
    let mut walk = repo.revwalk()?;
    walk.push(local_oid)?;
    walk.hide(merge_base)?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    for oid in walk {
        local_commits.push(oid?);
    }

    if local_commits.is_empty() {
        return Ok(());
    }

    // Replay each local commit on top of remote HEAD
    let mut current_parent = repo.find_commit(remote_oid)?;
    for commit_oid in &local_commits {
        let commit = repo.find_commit(*commit_oid)?;
        let tree = commit.tree()?;
        let parent_tree = current_parent.tree()?;

        let mut merge_index =
            repo.merge_trees(&parent_tree, &tree, &commit.parent(0)?.tree()?, None)?;
        if merge_index.has_conflicts() {
            anyhow::bail!(
                "Rebase conflict: cannot rebase release commits on top of remote '{branch}'. \
                 Run manually or use releaseCommitMode = \"pr\"."
            );
        }

        let new_tree_oid = merge_index.write_tree_to(repo)?;
        let new_tree = repo.find_tree(new_tree_oid)?;

        let new_oid = repo.commit(
            None,
            &commit.author(),
            &commit.committer(),
            commit.message().unwrap_or(""),
            &new_tree,
            &[&current_parent],
        )?;
        current_parent = repo.find_commit(new_oid)?;
    }

    // Move HEAD (and local branch if it exists) to the new tip
    let local_ref = format!("refs/heads/{branch}");
    if repo.find_reference(&local_ref).is_ok() {
        repo.reference(
            &local_ref,
            current_parent.id(),
            true,
            "ferrflow: rebase on push",
        )?;
    }
    repo.set_head_detached(current_parent.id())?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;

    Ok(())
}

const MAX_PUSH_RETRIES: usize = 3;

pub fn push(repo: &Repository, remote_name: &str, branch: &str, tags: &[&str]) -> Result<()> {
    // Push branch with retry on non-fast-forward
    for attempt in 1..=MAX_PUSH_RETRIES {
        match try_push_branch(repo, remote_name, branch) {
            Ok(()) => break,
            Err(e) => {
                let is_non_ff = e.chain().any(|cause| {
                    let msg = cause.to_string().to_lowercase();
                    msg.contains("non-fastforward")
                        || msg.contains("not fast forward")
                        || msg.contains("non-fast-forward")
                        || msg.contains("push rejected")
                });

                if !is_non_ff || attempt == MAX_PUSH_RETRIES {
                    return Err(e)
                        .with_context(|| {
                            format!("Failed to push branch '{branch}' after {attempt} attempt(s)")
                        })
                        .error_code(error_code::GIT_PUSH_BRANCH);
                }

                eprintln!(
                    "Push rejected (non-fast-forward), rebasing on remote and retrying ({attempt}/{MAX_PUSH_RETRIES})..."
                );
                fetch_and_rebase(repo, remote_name, branch)?;
            }
        }
    }

    // Verify branch landed on remote
    let head_oid = repo.head()?.peel_to_commit()?.id();
    verify_remote_branch(repo, remote_name, branch, head_oid)
        .with_context(|| "Post-push verification failed: release commit not on remote branch")
        .error_code(error_code::GIT_PUSH_VERIFY_FAILED)?;

    // Push tags separately
    push_tags(repo, remote_name, tags)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OrphanedTagStrategy;
    use git2::{Repository, Signature};
    use std::fs;

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
    static COMMIT_TIME: std::sync::atomic::AtomicI64 =
        std::sync::atomic::AtomicI64::new(1_700_000_000);

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
        create_branch_and_commit(&repo, "release/v1.0.0", &["release.txt"], "chore: release")
            .unwrap();

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
        repo.remote("origin", "https://github.com/FerrFlow-Org/FerrFlow.git")
            .unwrap();
        let url = get_remote_url(&repo, "origin");
        assert_eq!(
            url,
            Some("https://github.com/FerrFlow-Org/FerrFlow.git".to_string())
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

        let commits =
            get_commits_since_last_tag(&repo, "v", OrphanedTagStrategy::TreeHash).unwrap();
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
        let commits =
            get_commits_since_last_stable_tag(&repo, "v", OrphanedTagStrategy::Warn).unwrap();
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
}
