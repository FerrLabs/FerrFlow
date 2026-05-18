use anyhow::{Context, Result};
use git2::{Cred, CredentialType, Repository};

use crate::error_code::{self, ErrorCodeExt};

pub(super) fn extract_url_password(url: &str) -> Option<(String, String)> {
    let after_scheme = url.split("://").nth(1)?;
    let userinfo = after_scheme.split('@').next()?;
    let (user, password) = userinfo.split_once(':')?;
    if password.is_empty() {
        return None;
    }
    Some((user.to_string(), password.to_string()))
}

pub(super) fn credentials_callback(
    url: &str,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> std::result::Result<Cred, git2::Error> {
    if allowed_types.contains(CredentialType::SSH_KEY) {
        return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
    }
    if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
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
        if let Some((user, password)) = extract_url_password(url) {
            return Cred::userpass_plaintext(&user, &password);
        }
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

pub(super) fn authenticated_remote_url(url: &str) -> Option<String> {
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

pub(super) fn get_authenticated_remote<'a>(
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

pub fn get_remote_url(repo: &Repository, remote_name: &str) -> Option<String> {
    let remote = repo.find_remote(remote_name).ok()?;
    Some(remote.url()?.to_string())
}
