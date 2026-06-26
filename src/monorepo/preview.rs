use std::path::Path;

use crate::config::Config;
use crate::forge::{self, ForgeKind};
use crate::git::{Repository, get_remote_url};

use super::types::{CheckPackage, CheckResult};

pub(super) fn build_forge_instance(
    repo: &Repository,
    config: &Config,
) -> Option<Box<dyn forge::Forge>> {
    let remote_url = get_remote_url(repo, &config.workspace.remote)?;
    let slug = forge::extract_repo_slug(&remote_url)?;
    let host = forge::extract_host(&remote_url)?;

    let kind = match config.workspace.forge {
        ForgeKind::Auto => forge::detect_forge_from_url(&remote_url)?,
        explicit => explicit,
    };

    let token = forge::resolve_token(kind)?;
    Some(forge::build_forge(kind, token, slug, host))
}

pub(super) fn post_preview_comment(repo: &Repository, config: &Config, root: &Path) {
    let pr_id = match forge::detect_pr_number() {
        Some(id) => id,
        None => return, // Not in a PR context, skip silently
    };

    let forge_instance = match build_forge_instance(repo, config) {
        Some(f) => f,
        None => return, // No forge detected or no token, skip silently
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

/// Escape a string for safe insertion into a GitHub-flavored Markdown
/// table cell. Closes a markdown-injection vector where a package name
/// like `foo](https://evil)` or one containing `|` / newline / `<script>`
/// could break out of the cell or inject arbitrary content. github.com
/// renders the preview comment as untrusted user content but custom forge
/// installs (Gitea, Forgejo) may not — escape defensively at the source.
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
        // Without escaping this would render as a link.
        // With escaping the `]` is fine but `<` (from a malicious payload)
        // and embedded angle brackets are HTML-encoded.
        assert_eq!(
            escape_md_cell("foo](javascript:alert(1))"),
            "foo](javascript:alert(1))"
        );
        // Combined attack: package name containing both pipe and HTML.
        assert_eq!(escape_md_cell("|<img src=x>"), r"\|&lt;img src=x&gt;");
    }
}
