use anyhow::{Context, Result};
use git2::{Cred, CredentialType, PushOptions, RemoteCallbacks, Repository, Sort};
use std::path::{Path, PathBuf};

pub struct GitLog {
    pub hash: String,
    pub message: String,
}

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
