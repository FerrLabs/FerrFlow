use std::process::Command;

use super::repo::Repository;
use crate::config::ForgeKind;
use crate::forge::extract_host;
use crate::forge::gitlab::GitLabToken;

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

fn is_gitlab(url: &str) -> bool {
    extract_host(url).is_some_and(|host| host.contains("gitlab"))
}

fn forge_of(url: &str, configured: ForgeKind) -> Option<ForgeKind> {
    match configured {
        ForgeKind::Auto if is_gitlab(url) => Some(ForgeKind::Gitlab),
        ForgeKind::Auto => crate::forge::detect_forge_with_probe(url),
        explicit => Some(explicit),
    }
}

fn non_empty_env(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|value| !value.is_empty())
}

pub(super) fn token_for_url(url: &str, forge: ForgeKind) -> Option<(String, String)> {
    let ferrflow_token = non_empty_env("FERRFLOW_TOKEN");
    let gitlab_token = non_empty_env("GITLAB_TOKEN");
    let github_token = non_empty_env("GITHUB_TOKEN");
    if ferrflow_token.is_none() && gitlab_token.is_none() && github_token.is_none() {
        return None;
    }
    let is_gitlab = forge_of(url, forge) == Some(ForgeKind::Gitlab);
    if let Some(token) = ferrflow_token {
        let user = if is_gitlab {
            GitLabToken::of(&token).git_username()
        } else {
            "x-access-token"
        };
        return Some((user.to_string(), token));
    }
    if is_gitlab {
        let token = gitlab_token?;
        let user = GitLabToken::of(&token).git_username();
        return Some((user.to_string(), token));
    }
    github_token.map(|token| ("x-access-token".to_string(), token))
}

/// Where the credential reaches git. `/proc/<pid>/cmdline` is world-readable,
/// so anything in argv is visible to every other process on the host, which on
/// a shared runner means every other repository's job. `/proc/<pid>/environ` is
/// mode 0600 and readable only by the owner.
///
/// Set on the git command alone, never on our own process, so nothing else we
/// spawn inherits them. Hooks in particular run as separate commands.
pub(super) const GIT_USER_VAR: &str = "FERRFLOW_GIT_USER";
pub(super) const GIT_PASSWORD_VAR: &str = "FERRFLOW_GIT_PASSWORD";

/// Reads the credential from the environment instead of carrying it. Both
/// values expand inside double quotes, so a token containing quotes, spaces or
/// shell metacharacters needs no escaping and cannot break out of the helper.
const CREDENTIAL_HELPER: &str =
    r#"!f() { echo "username=$FERRFLOW_GIT_USER"; echo "password=$FERRFLOW_GIT_PASSWORD"; }; f"#;

pub(super) fn configure_git_command(cmd: &mut Command, url: &str, forge: ForgeKind) {
    scrub_trace_env(cmd);
    if let Some((user, token)) = token_for_url(url, forge) {
        cmd.env(GIT_USER_VAR, user);
        cmd.env(GIT_PASSWORD_VAR, token);
        cmd.arg("-c")
            .arg(format!("credential.helper={CREDENTIAL_HELPER}"));
        if let Some(server) = server_config_url(url) {
            cmd.arg("-c").arg(format!("http.{server}.extraheader="));
        }
    }
}

pub(super) fn server_config_url(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next().unwrap_or(authority);
    if host.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{host}/"))
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

pub fn get_remote_url(repo: &Repository, remote_name: &str) -> Option<String> {
    let remote = repo.find_remote(remote_name).ok()?;
    let url = remote
        .url(gix::remote::Direction::Push)
        .or_else(|| remote.url(gix::remote::Direction::Fetch))?;
    Some(url.to_bstring().to_string())
}
