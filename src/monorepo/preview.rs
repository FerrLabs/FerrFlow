use git2::Repository;
use std::path::Path;

use crate::config::Config;
use crate::forge::{self, ForgeKind};
use crate::git::get_remote_url;

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

pub(super) fn post_preview_comment(repo: &git2::Repository, config: &Config, root: &Path) {
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
        eprintln!("Warning: failed to post preview comment: {e}");
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
            pkg.name, pkg.current_version, pkg.next_version, pkg.bump_type
        ));
    }
    let commit_count: usize = packages.iter().map(|p| p.commits.len()).sum();
    body.push_str(&format!("\nBased on {} commit(s).", commit_count));
    body
}
