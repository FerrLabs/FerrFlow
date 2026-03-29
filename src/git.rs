use anyhow::{Context, Result};
use git2::{Cred, CredentialType, PushOptions, RemoteCallbacks, Repository, Sort};
use std::path::{Path, PathBuf};

pub use crate::changelog::GitLog;

pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path).with_context(|| format!("Not a git repository: {}", path.display()))
}

pub fn get_repo_root(repo: &Repository) -> Result<PathBuf> {
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))
}

pub fn get_commits_since_last_tag(repo: &Repository, tag_prefix: &str) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix)?;

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

pub fn find_last_tag_name(repo: &Repository, prefix: &str) -> Result<Option<String>> {
    let head = repo.head()?.peel_to_commit()?.id();
    let mut latest: Option<(i64, String)> = None;

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if tag_name.starts_with(prefix) {
            let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
                tag_obj.target_id()
            } else {
                oid
            };
            if let Ok(commit) = repo.find_commit(commit_oid) {
                let reachable = head == commit_oid
                    || repo.graph_descendant_of(head, commit_oid).unwrap_or(false);
                if !reachable {
                    return true;
                }
                let time = commit.time().seconds();
                if latest.is_none() || time > latest.as_ref().unwrap().0 {
                    latest = Some((time, tag_name.to_string()));
                }
            }
        }
        true
    })?;

    Ok(latest.map(|(_, name)| name))
}

fn find_last_tag_commit(repo: &Repository, prefix: &str) -> Result<Option<git2::Oid>> {
    let head = repo.head()?.peel_to_commit()?.id();
    let mut latest: Option<(i64, git2::Oid)> = None;

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if tag_name.starts_with(prefix) {
            let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
                tag_obj.target_id()
            } else {
                oid
            };
            if let Ok(commit) = repo.find_commit(commit_oid) {
                let reachable = head == commit_oid
                    || repo.graph_descendant_of(head, commit_oid).unwrap_or(false);
                if !reachable {
                    return true;
                }
                let time = commit.time().seconds();
                if latest.is_none() || time > latest.unwrap().0 {
                    latest = Some((time, commit_oid));
                }
            }
        }
        true
    })?;

    Ok(latest.map(|(_, oid)| oid))
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

pub fn get_changed_files_since_tag(repo: &Repository, tag_prefix: &str) -> Result<Vec<String>> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => return Ok(vec![]),
    };
    let head_tree = head.tree()?;

    let old_tree = if let Some(tag_oid) = find_last_tag_commit(repo, tag_prefix)? {
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

pub fn fetch_tags(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("Remote '{}' not found", remote_name))?;
    remote.fetch(&["refs/tags/*:refs/tags/*"], None, None)?;
    Ok(())
}

pub fn tag_exists(repo: &Repository, tag_name: &str) -> bool {
    repo.refname_to_id(&format!("refs/tags/{tag_name}")).is_ok()
}

pub fn create_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<()> {
    if tag_exists(repo, tag_name) {
        anyhow::bail!("tag {tag_name} already exists");
    }
    let head = repo.head()?.peel_to_commit()?;
    let sig = signature(repo)?;
    repo.tag(tag_name, head.as_object(), &sig, message, false)?;
    Ok(())
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

pub fn get_repo_slug(repo: &Repository, remote_name: &str) -> Option<String> {
    let remote = repo.find_remote(remote_name).ok()?;
    let url = remote.url()?.to_string();

    let after = if url.contains("github.com/") {
        url.split("github.com/").nth(1)?
    } else if url.contains("github.com:") {
        url.split("github.com:").nth(1)?
    } else {
        return None;
    };

    Some(after.trim_end_matches(".git").to_string())
}

pub fn create_branch_and_commit(
    repo: &Repository,
    branch_name: &str,
    files: &[&str],
    message: &str,
) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(branch_name, &head, false)?;

    let refname = format!("refs/heads/{branch_name}");
    let mut index = repo.index()?;
    for file in files {
        index.add_path(Path::new(file))?;
    }
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = signature(repo)?;

    repo.commit(Some(&refname), &sig, &sig, message, &tree, &[&head])?;
    Ok(())
}

pub fn push_branch(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("Remote '{}' not found", remote_name))?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        if allowed_types.contains(CredentialType::SSH_KEY) {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        } else if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            git2::Config::open_default()
                .and_then(|cfg| Cred::credential_helper(&cfg, url, username_from_url))
        } else {
            Cred::default()
        }
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    remote.push(&[&refspec], Some(&mut push_options))?;

    Ok(())
}

pub fn push(repo: &Repository, remote_name: &str, branch: &str, tags: &[&str]) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("Remote '{}' not found", remote_name))?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        if allowed_types.contains(CredentialType::SSH_KEY) {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        } else if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            git2::Config::open_default()
                .and_then(|cfg| Cred::credential_helper(&cfg, url, username_from_url))
        } else {
            Cred::default()
        }
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let mut refspecs: Vec<String> = vec![format!("refs/heads/{branch}:refs/heads/{branch}")];
    for tag in tags {
        refspecs.push(format!("refs/tags/{tag}:refs/tags/{tag}"));
    }
    let refspec_refs: Vec<&str> = refspecs.iter().map(String::as_str).collect();
    remote.push(&refspec_refs, Some(&mut push_options))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(find_last_tag_name(&repo, "v").unwrap(), None);
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

        let result = find_last_tag_name(&repo, "v").unwrap();
        assert_eq!(result, Some("v1.1.0".to_string()));
    }

    #[test]
    fn find_last_tag_name_annotated() {
        let (dir, repo) = init_repo();
        create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
        create_annotated_tag(&repo, "v1.0.0", "Release 1.0.0");
        create_commit_in_repo(&repo, dir.path(), "b.txt", "second");
        create_annotated_tag(&repo, "v2.0.0", "Release 2.0.0");

        let result = find_last_tag_name(&repo, "v").unwrap();
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
            find_last_tag_name(&repo, "api@v").unwrap(),
            Some("api@v1.1.0".to_string())
        );
        assert_eq!(
            find_last_tag_name(&repo, "site@v").unwrap(),
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

        let commits = get_commits_since_last_tag(&repo, "v").unwrap();
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

        let commits = get_commits_since_last_tag(&repo, "v").unwrap();
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

        let commits = get_commits_since_last_tag(&repo, "v").unwrap();
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

        let files = get_changed_files_since_tag(&repo, "v").unwrap();
        assert!(files.contains(&"a.txt".to_string()));
        assert!(files.contains(&"b.txt".to_string()));
    }

    #[test]
    fn get_changed_files_since_tag_only_new() {
        let (dir, repo) = init_repo();
        create_commit_in_repo(&repo, dir.path(), "a.txt", "first");
        create_lightweight_tag(&repo, "v1.0.0");
        create_commit_in_repo(&repo, dir.path(), "b.txt", "second");

        let files = get_changed_files_since_tag(&repo, "v").unwrap();
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

    // -----------------------------------------------------------------------
    // get_repo_slug
    // -----------------------------------------------------------------------

    #[test]
    fn get_repo_slug_https() {
        let (dir, repo) = init_repo();
        create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
        repo.remote("origin", "https://github.com/FerrFlow-Org/FerrFlow.git")
            .unwrap();
        let slug = get_repo_slug(&repo, "origin");
        assert_eq!(slug, Some("FerrFlow-Org/FerrFlow".to_string()));
    }

    #[test]
    fn get_repo_slug_ssh() {
        let (dir, repo) = init_repo();
        create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
        repo.remote("origin", "git@github.com:FerrFlow-Org/FerrFlow.git")
            .unwrap();
        let slug = get_repo_slug(&repo, "origin");
        assert_eq!(slug, Some("FerrFlow-Org/FerrFlow".to_string()));
    }

    #[test]
    fn get_repo_slug_no_remote() {
        let (dir, repo) = init_repo();
        create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
        let slug = get_repo_slug(&repo, "origin");
        assert_eq!(slug, None);
    }

    #[test]
    fn get_repo_slug_non_github() {
        let (dir, repo) = init_repo();
        create_commit_in_repo(&repo, dir.path(), "a.txt", "initial");
        repo.remote("origin", "https://gitlab.com/foo/bar.git")
            .unwrap();
        let slug = get_repo_slug(&repo, "origin");
        assert_eq!(slug, None);
    }
}
