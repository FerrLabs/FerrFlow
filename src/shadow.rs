use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::error_code::{self, ErrorCodeExt};
use crate::git::{collect_all_tags, get_repo_root, open_repo};

pub fn run(config_path: Option<&Path>, verbose: bool, keep: bool) -> Result<()> {
    let root = get_repo_root(&open_repo(&std::env::current_dir()?)?)?;
    let remote = Config::load(&root, config_path)?.workspace.remote;

    let workspace = tempfile::Builder::new()
        .prefix("ferrflow-shadow-")
        .tempdir()?;
    let clone = workspace.path().join("repo");
    clone_repo(&root, &clone, &remote).error_code(error_code::GIT_SHADOW_CLONE)?;

    tracing::info!("{}", "FerrFlow shadow release".bold().blue());
    tracing::info!("{}", format!("Cloned into {}", clone.display()).dimmed());
    tracing::info!("");

    let config_path = config_path.map(|p| relocate(p, &root, &clone));
    let result = release_in(&clone, config_path.as_deref(), verbose);

    if keep {
        let kept = workspace.keep();
        tracing::info!("");
        tracing::info!("Kept the clone at {}", kept.join("repo").display());
    }
    let summary = result?;
    summary.print();
    Ok(())
}

fn release_in(clone: &Path, config_path: Option<&Path>, verbose: bool) -> Result<ShadowSummary> {
    let config = Config::load(clone, config_path)?;
    crate::manifest::validate_in_sync(&config, clone)?;
    crate::git::ensure_commit_identity(clone)?;

    let head_before = git_stdout(clone, &["rev-parse", "HEAD"])?;
    let tags_before = tag_set(clone)?;

    crate::monorepo::run::run_shadow_release(clone, &config, verbose)?;

    let range = format!("{}..HEAD", head_before.trim());
    Ok(ShadowSummary {
        commits: lines(&git_stdout(clone, &["log", "--format=%s", &range])?),
        files: lines(&git_stdout(
            clone,
            &["diff", "--name-only", head_before.trim()],
        )?),
        tags: tag_set(clone)?.difference(&tags_before).cloned().collect(),
    })
}

fn clone_repo(root: &Path, clone: &Path, remote: &str) -> Result<()> {
    let workspace = clone.parent().context("clone path has no parent")?;
    let sink = workspace.join("push-sink.git");
    let root = root.to_string_lossy();
    let clone_dir = clone.to_string_lossy();
    let sink_dir = sink.to_string_lossy();
    git_stdout(
        workspace,
        &[
            "clone",
            "--quiet",
            "--no-hardlinks",
            "--origin",
            remote,
            &root,
            &clone_dir,
        ],
    )?;
    git_stdout(workspace, &["init", "--quiet", "--bare", &sink_dir])?;
    git_stdout(clone, &["remote", "set-url", "--push", remote, &sink_dir])?;
    copy_commit_settings(Path::new(root.as_ref()), clone)
}

const COMMIT_SETTINGS: &[&str] = &[
    "user.name",
    "user.email",
    "user.signingkey",
    "gpg.format",
    "commit.gpgsign",
    "tag.gpgsign",
];

fn copy_commit_settings(root: &Path, clone: &Path) -> Result<()> {
    for key in COMMIT_SETTINGS {
        if let Ok(value) = git_stdout(root, &["config", "--get", key]) {
            git_stdout(clone, &["config", key, value.trim()])?;
        }
    }
    Ok(())
}

fn relocate(path: &Path, root: &Path, clone: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), |rel| clone.join(rel))
}

fn tag_set(dir: &Path) -> Result<BTreeSet<String>> {
    Ok(collect_all_tags(&open_repo(dir)?).into_iter().collect())
}

fn git_stdout(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !out.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[derive(Debug, PartialEq)]
struct ShadowSummary {
    commits: Vec<String>,
    files: Vec<String>,
    tags: Vec<String>,
}

impl ShadowSummary {
    fn print(&self) {
        tracing::info!("");
        if self.commits.is_empty() && self.tags.is_empty() {
            tracing::info!("{}", "Nothing would be released.".dimmed());
            return;
        }
        Self::section("Commits", &self.commits);
        Self::section("Files written", &self.files);
        Self::section("Tags created", &self.tags);
        tracing::info!("");
        tracing::info!(
            "{}",
            "Nothing was pushed or published. The original repository is untouched.".dimmed()
        );
    }

    fn section(title: &str, items: &[String]) {
        tracing::info!("{}", title.bold());
        if items.is_empty() {
            tracing::info!("  {}", "none".dimmed());
        }
        for item in items {
            tracing::info!("  {item}");
        }
    }
}

#[cfg(test)]
#[path = "shadow/tests.rs"]
mod tests;
