use std::path::Path;

use crate::config::Config;
use crate::forge::{self, ForgeKind};
use crate::git::{Repository, get_remote_url};

use super::types::{CheckPackage, CheckResult};

#[derive(Debug, PartialEq)]
pub(crate) enum ForgeUnavailable {
    NoRemote(String),
    UnknownForge(String),
    NoToken(ForgeKind),
}

impl std::fmt::Display for ForgeUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRemote(remote) => write!(f, "remote `{remote}` not found"),
            Self::UnknownForge(url) => write!(
                f,
                "could not tell which forge hosts {url}, set `forge` in the config"
            ),
            Self::NoToken(kind) => {
                let vars: Vec<_> = std::iter::once(crate::config::GENERIC_TOKEN_ENV_VAR)
                    .chain(kind.token_env_vars().iter().copied())
                    .collect();
                write!(f, "no {kind:?} token found in {}", vars.join(" or "))
            }
        }
    }
}

struct ForgeTarget {
    kind: ForgeKind,
    slug: String,
    host: String,
}

fn forge_target(remote_url: &str, configured: ForgeKind) -> Result<ForgeTarget, ForgeUnavailable> {
    let unknown = || ForgeUnavailable::UnknownForge(remote_url.to_string());
    let slug = forge::extract_repo_slug(remote_url).ok_or_else(unknown)?;
    let host = forge::extract_host(remote_url).ok_or_else(unknown)?;
    let kind = match configured {
        ForgeKind::Auto => forge::detect_forge_with_probe(remote_url).ok_or_else(unknown)?,
        explicit => explicit,
    };
    Ok(ForgeTarget { kind, slug, host })
}

pub(crate) fn try_build_forge_instance(
    repo: &Repository,
    config: &Config,
) -> Result<Box<dyn forge::Forge>, ForgeUnavailable> {
    let remote = &config.workspace.remote;
    let remote_url =
        get_remote_url(repo, remote).ok_or_else(|| ForgeUnavailable::NoRemote(remote.clone()))?;
    let ForgeTarget { kind, slug, host } = forge_target(&remote_url, config.workspace.forge)?;
    let token = forge::resolve_token(kind).ok_or(ForgeUnavailable::NoToken(kind))?;
    Ok(forge::build_forge(kind, token, slug, host))
}

pub(crate) fn build_forge_instance(
    repo: &Repository,
    config: &Config,
) -> Option<Box<dyn forge::Forge>> {
    try_build_forge_instance(repo, config).ok()
}

pub(super) fn post_preview_comment(repo: &Repository, config: &Config, root: &Path) {
    let pr_id = match forge::detect_pr_number() {
        Some(id) => id,
        None => return, // Not in a PR context, skip silently
    };

    let forge_instance = match try_build_forge_instance(repo, config) {
        Ok(f) => f,
        Err(reason) => {
            tracing::warn!("Warning: preview comment not posted: {reason}");
            return;
        }
    };

    let json_result = capture_check_json(root);
    let body = format_preview_comment(&json_result);
    let marker = "<!-- ferrflow-preview -->";

    let result = (|| -> anyhow::Result<()> {
        match forge_instance.find_comment(pr_id, marker)? {
            Some(comment_id) => forge_instance.update_comment(pr_id, comment_id, &body)?,
            None => forge_instance.create_comment(pr_id, &body)?,
        }
        Ok(())
    })();

    if let Err(e) = result {
        tracing::warn!("Warning: failed to post preview comment: {e}");
    }
}

fn capture_check_json(root: &Path) -> Vec<CheckPackage> {
    let exe = std::env::current_exe().unwrap_or_else(|_| "ferrflow".into());
    let output = std::process::Command::new(exe)
        .args(["check", "--json"])
        .current_dir(root)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            serde_json::from_str::<CheckResult>(&stdout)
                .map(|r| r.packages)
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn format_preview_comment(packages: &[CheckPackage]) -> String {
    let mut body = String::from("<!-- ferrflow-preview -->\n**FerrFlow Release Preview**\n\n");
    if packages.is_empty() {
        body.push_str("No releasable changes detected.");
        return body;
    }
    body.push_str("| Package | Current | Next | Bump |\n");
    body.push_str("|---------|---------|------|------|\n");
    for pkg in packages {
        body.push_str(&format!(
            "| {} | `{}` | `{}` | {} |\n",
            escape_md_cell(&pkg.name),
            escape_md_cell(&pkg.current_version),
            escape_md_cell(&pkg.next_version),
            escape_md_cell(&pkg.bump_type),
        ));
    }
    let commit_count: usize = packages.iter().map(|p| p.commits.len()).sum();
    body.push_str(&format!("\nBased on {} commit(s).", commit_count));
    body
}

pub(super) fn escape_md_cell(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '\n' | '\r' => out.push(' '),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '`' => out.push_str("\\`"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_md_cell_passthrough_for_normal_text() {
        assert_eq!(escape_md_cell("api"), "api");
        assert_eq!(escape_md_cell("1.2.3"), "1.2.3");
        assert_eq!(escape_md_cell("minor"), "minor");
    }

    #[test]
    fn escape_md_cell_escapes_pipe_breaking_table() {
        assert_eq!(escape_md_cell("a|b"), r"a\|b");
    }

    #[test]
    fn escape_md_cell_squashes_newlines_to_keep_row() {
        assert_eq!(escape_md_cell("line1\nline2"), "line1 line2");
        assert_eq!(escape_md_cell("a\rb"), "a b");
    }

    #[test]
    fn escape_md_cell_neutralizes_html_tags() {
        assert_eq!(
            escape_md_cell("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
    }

    #[test]
    fn escape_md_cell_escapes_backticks_and_backslash() {
        assert_eq!(escape_md_cell(r"foo`code`bar"), r"foo\`code\`bar");
        assert_eq!(escape_md_cell(r"path\to\thing"), r"path\\to\\thing");
    }

    #[test]
    fn escape_md_cell_blocks_link_injection() {
        assert_eq!(
            escape_md_cell("foo](javascript:alert(1))"),
            "foo](javascript:alert(1))"
        );
        assert_eq!(escape_md_cell("|<img src=x>"), r"\|&lt;img src=x&gt;");
    }

    #[test]
    fn missing_token_names_every_variable_that_was_read() {
        assert_eq!(
            ForgeUnavailable::NoToken(ForgeKind::Gitlab).to_string(),
            "no Gitlab token found in FERRFLOW_TOKEN or GITLAB_TOKEN"
        );
        assert_eq!(
            ForgeUnavailable::NoToken(ForgeKind::Gitea).to_string(),
            "no Gitea token found in FERRFLOW_TOKEN or GITEA_TOKEN or FORGEJO_TOKEN"
        );
    }

    #[test]
    fn forge_target_detects_known_hosts_without_a_token() {
        let target = forge_target("git@gitlab.com:group/app.git", ForgeKind::Auto).unwrap();
        assert_eq!(target.kind, ForgeKind::Gitlab);
        assert_eq!(target.slug, "group/app");
        assert_eq!(target.host, "gitlab.com");
    }

    #[test]
    fn forge_target_honours_an_explicit_forge() {
        let target =
            forge_target("https://git.example.com/team/app.git", ForgeKind::Gitea).unwrap();
        assert_eq!(target.kind, ForgeKind::Gitea);
        assert_eq!(target.host, "git.example.com");
    }

    #[test]
    fn forge_target_reports_an_unparseable_remote() {
        assert_eq!(
            forge_target("not a remote", ForgeKind::Github).err(),
            Some(ForgeUnavailable::UnknownForge("not a remote".into()))
        );
    }
}
