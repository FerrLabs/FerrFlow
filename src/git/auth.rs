use anyhow::{Context, Result};
use git2::{Cred, CredentialType, Repository};
use std::process::Command;

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

pub(super) fn token_for_url(url: &str) -> Option<(String, String)> {
    if let Ok(token) = std::env::var("FERRFLOW_TOKEN") {
        let user = if url.contains("gitlab") {
            "oauth2"
        } else {
            "x-access-token"
        };
        return Some((user.to_string(), token));
    }
    if url.contains("gitlab") {
        if let Ok(token) = std::env::var("GITLAB_TOKEN") {
            return Some(("oauth2".to_string(), token));
        }
    } else if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        return Some(("x-access-token".to_string(), token));
    }
    None
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
        if let Some((user, token)) = token_for_url(url) {
            return Cred::userpass_plaintext(&user, &token);
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
            "Warning: No git credentials found. Set FERRFLOW_TOKEN (or GITHUB_TOKEN/GITLAB_TOKEN), \
             configure a git credential helper, or embed credentials in the remote URL."
        );
    }
    Cred::default()
}

pub(super) fn configure_git_command(cmd: &mut Command, url: &str) {
    if let Some((user, token)) = token_for_url(url) {
        let escaped_user = shell_escape(&user);
        let escaped_token = shell_escape(&token);
        let helper = format!(
            "!f() {{ echo username={}; echo password={}; }}; f",
            escaped_user, escaped_token
        );
        cmd.arg("-c").arg(format!("credential.helper={}", helper));
    }
}

fn shell_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn get_remote<'a>(repo: &'a Repository, remote_name: &str) -> Result<git2::Remote<'a>> {
    repo.find_remote(remote_name)
        .with_context(|| format!("Remote '{}' not found", remote_name))
        .error_code(error_code::GIT_REMOTE_NOT_FOUND)
}

pub fn get_remote_url(repo: &Repository, remote_name: &str) -> Option<String> {
    let remote = repo.find_remote(remote_name).ok()?;
    Some(remote.url()?.to_string())
}
