use std::process::Command;

use super::repo::Repository;

#[cfg(test)]
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

pub(super) fn configure_git_command(cmd: &mut Command, url: &str) {
    scrub_trace_env(cmd);
    if let Some((user, token)) = token_for_url(url) {
        let escaped_user = single_quote_escape(&user);
        let escaped_token = single_quote_escape(&token);
        let helper = format!(
            "!f() {{ echo username='{}'; echo password='{}'; }}; f",
            escaped_user, escaped_token
        );
        cmd.arg("-c").arg(format!("credential.helper={}", helper));
    }
}

pub(super) fn scrub_trace_env(cmd: &mut Command) {
    for var in [
        "GIT_TRACE",
        "GIT_TRACE_PACK_ACCESS",
        "GIT_TRACE_PACKET",
        "GIT_TRACE_PACKFILE",
        "GIT_TRACE_PERFORMANCE",
        "GIT_TRACE_SETUP",
        "GIT_TRACE_SHALLOW",
        "GIT_TRACE_CURL",
        "GIT_TRACE_CURL_NO_DATA",
        "GIT_CURL_VERBOSE",
        "GCM_TRACE",
    ] {
        cmd.env_remove(var);
    }
}

fn single_quote_escape(value: &str) -> String {
    value.replace('\'', "'\\''")
}

pub fn get_remote_url(repo: &Repository, remote_name: &str) -> Option<String> {
    let remote = repo.find_remote(remote_name).ok()?;
    let url = remote
        .url(gix::remote::Direction::Push)
        .or_else(|| remote.url(gix::remote::Direction::Fetch))?;
    Some(url.to_bstring().to_string())
}
