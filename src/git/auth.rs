use std::process::Command;
use std::sync::Once;

use super::repo::Repository;
use crate::config::{ForgeKind, GENERIC_TOKEN_ENV_VAR, all_token_env_vars};
use crate::forge::gitlab::GitLabToken;
use crate::forge::{AUTHORITY_END, extract_host};

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

const GITHUB_TOKEN_VAR: &str = "GITHUB_TOKEN";

static GITEA_FALLBACK_WARNING: Once = Once::new();
static GITHUB_TOKEN_WITHHELD_WARNING: Once = Once::new();

pub(super) fn token_for_url(url: &str, forge: ForgeKind) -> Option<(String, String)> {
    if !all_token_env_vars().any(|var| non_empty_env(var).is_some()) {
        return None;
    }
    select_credential(url, forge_of(url, forge))
}

pub(super) fn select_credential(url: &str, forge: Option<ForgeKind>) -> Option<(String, String)> {
    let token = non_empty_env(GENERIC_TOKEN_ENV_VAR)
        .or_else(|| forge.and_then(forge_token))
        .or_else(|| github_token_fallback(url, forge))?;
    Some((git_username(forge, &token).to_string(), token))
}

fn forge_token(forge: ForgeKind) -> Option<String> {
    forge
        .token_env_vars()
        .iter()
        .find_map(|var| non_empty_env(var))
}

fn github_token_fallback(url: &str, forge: Option<ForgeKind>) -> Option<String> {
    let token = non_empty_env(GITHUB_TOKEN_VAR)?;
    let host = || extract_host(url).unwrap_or_else(|| "this remote".to_string());
    match forge {
        Some(ForgeKind::Gitea) => {
            GITEA_FALLBACK_WARNING.call_once(|| {
                tracing::warn!(
                    "Warning: pushing to {} with GITHUB_TOKEN because neither GITEA_TOKEN nor FORGEJO_TOKEN is set. \
                     This fallback goes away in the next major version: pass the token as GITEA_TOKEN \
                     (on Gitea or Forgejo Actions, `GITEA_TOKEN: ${{{{ secrets.GITHUB_TOKEN }}}}`).",
                    host()
                );
            });
            Some(token)
        }
        None if is_http(url) => {
            GITHUB_TOKEN_WITHHELD_WARNING.call_once(|| {
                tracing::warn!(
                    "Warning: not sending GITHUB_TOKEN to {}, which is not recognised as GitHub. \
                     Set `forge` in the config if it is a GitHub Enterprise instance, or pass the token as FERRFLOW_TOKEN.",
                    host()
                );
            });
            None
        }
        _ => None,
    }
}

fn is_http(url: &str) -> bool {
    url.split_once("://").is_some_and(|(scheme, _)| {
        scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http")
    })
}

fn git_username(forge: Option<ForgeKind>, token: &str) -> &'static str {
    match forge {
        Some(ForgeKind::Gitlab) => GitLabToken::of(token).git_username(),
        Some(ForgeKind::Bitbucket) => "x-token-auth",
        _ => "x-access-token",
    }
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
    let authority = rest.split(AUTHORITY_END).next()?;
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
