use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::error_code::{self, ErrorCodeExt};

pub type Repository = gix::Repository;

pub fn open_repo(path: &Path) -> Result<Repository> {
    gix::discover(path)
        .with_context(|| format!("Not a git repository: {}", path.display()))
        .error_code(error_code::GIT_NOT_A_REPO)
}

pub fn get_repo_root(repo: &Repository) -> Result<PathBuf> {
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))
        .error_code(error_code::GIT_BARE_REPO)
}

pub fn resolve_current_branch(repo: &Repository, fallback: &str) -> String {
    if let Ok(head) = repo.head()
        && let Some(name) = head.referent_name()
    {
        let full = name.as_bstr().to_string();
        if let Some(short) = full.strip_prefix("refs/heads/") {
            return short.to_string();
        }
        return full;
    }

    let ci_vars = [
        "GITHUB_REF_NAME",
        "CI_COMMIT_BRANCH",
        "BRANCH_NAME",
        "CIRCLE_BRANCH",
        "BITBUCKET_BRANCH",
        "BUILDKITE_BRANCH",
        "TRAVIS_BRANCH",
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
