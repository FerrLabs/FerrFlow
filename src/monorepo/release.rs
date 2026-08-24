use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::Config;
use crate::git::{get_repo_root, open_repo};
use crate::timing::Timing;

use super::run::checkpoint::Checkpoint;
use super::run::run_release_logic;

const MAX_RELEASE_REGENERATE_ATTEMPTS: usize = 3;

#[allow(clippy::too_many_arguments)]
pub fn release(
    config_path: Option<&Path>,
    dry_run: bool,
    verbose: bool,
    json: bool,
    force: bool,
    force_versions: &[String],
    excluded: &[String],
    channel: Option<&str>,
    draft: bool,
    force_unlock: bool,
    timing: &mut Timing,
) -> Result<()> {
    let repo = timing.stage("open_repo", || open_repo(&std::env::current_dir()?))?;
    let root = get_repo_root(&repo)?;
    let config = timing.stage("load config", || Config::load(&root, config_path))?;
    drop(repo);

    if !dry_run {
        crate::manifest::validate_in_sync(&config, &root)?;
    }

    if !json {
        if dry_run {
            tracing::info!("{}", "FerrFlow — Release (dry run)".bold().blue());
        } else {
            tracing::info!("{}", "FerrFlow — Release".bold().green());
        }
        tracing::info!("");
    }

    let single_shot = dry_run
        || matches!(
            config.workspace.release_commit_mode,
            crate::config::ReleaseCommitMode::Pr | crate::config::ReleaseCommitMode::None
        );

    if single_shot {
        let out = run_release_logic(
            &root,
            &config,
            dry_run,
            verbose,
            false,
            json,
            force,
            force_versions,
            excluded,
            channel,
            draft,
            force_unlock,
            timing,
        )?;
        if let Some(out) = out {
            out.print();
        }
        return Ok(());
    }

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=MAX_RELEASE_REGENERATE_ATTEMPTS {
        let pre_attempt_tags: std::collections::HashSet<String> = {
            let repo = open_repo(&root)?;
            crate::git::collect_all_tags(&repo).into_iter().collect()
        };

        let result = run_release_logic(
            &root,
            &config,
            dry_run,
            verbose,
            false,
            json,
            force,
            force_versions,
            excluded,
            channel,
            draft,
            force_unlock,
            timing,
        );

        match result {
            Ok(out) => {
                if let Some(out) = out {
                    out.print();
                }
                return Ok(());
            }
            Err(e)
                if attempt < MAX_RELEASE_REGENERATE_ATTEMPTS
                    && crate::git::is_push_rejected_error(&e) =>
            {
                tracing::warn!("");
                tracing::warn!(
                    "{}",
                    format!(
                        "Release attempt {attempt}/{MAX_RELEASE_REGENERATE_ATTEMPTS} \
                         pushed onto a stale '{}': {e:#}",
                        config.workspace.branch,
                    )
                    .yellow()
                );
                tracing::warn!(
                    "{}",
                    "Resetting working tree to remote tip and regenerating the release commit \
                     against the latest history…"
                        .dimmed()
                );

                cleanup_failed_release_attempt(&root, &config, &pre_attempt_tags)?;
                last_err = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!(
            "release failed after {MAX_RELEASE_REGENERATE_ATTEMPTS} regenerate attempts"
        )
    }))
}

fn cleanup_failed_release_attempt(
    root: &Path,
    config: &Config,
    pre_attempt_tags: &std::collections::HashSet<String>,
) -> Result<()> {
    let repo = open_repo(root)?;
    let after: std::collections::HashSet<String> =
        crate::git::collect_all_tags(&repo).into_iter().collect();
    if let Some(workdir) = repo.workdir() {
        for tag in after.difference(pre_attempt_tags) {
            let _ = std::process::Command::new("git")
                .current_dir(workdir)
                .args(["tag", "-d", tag])
                .output();
        }
    }

    let target_branch = crate::git::resolve_current_branch(&repo, &config.workspace.branch);
    crate::git::reset_branch_to_remote(&repo, &config.workspace.remote, &target_branch)?;

    Checkpoint::delete(root)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monorepo::run::checkpoint::Phase;
    use std::collections::HashSet;

    fn git(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t.t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t.t")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    }

    fn commit(dir: &Path, file: &str) {
        std::fs::write(dir.join(file), file).unwrap();
        git(dir, &["add", "--", file]);
        git(dir, &["commit", "-m", &format!("add {file}")]);
    }

    fn config() -> Config {
        serde_json::from_str(r#"{"workspace":{"branch":"main","remote":"origin"}}"#).unwrap()
    }

    fn repo_with_failed_attempt() -> (tempfile::TempDir, std::path::PathBuf, HashSet<String>) {
        let base = tempfile::tempdir().unwrap();

        let remote = base.path().join("remote.git");
        std::fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "--bare", "-b", "main"]);

        let local = base.path().join("local");
        std::fs::create_dir_all(&local).unwrap();
        git(&local, &["init", "-b", "main"]);
        git(
            &local,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        commit(&local, "base.txt");
        git(&local, &["push", "origin", "main:main"]);

        let pre_attempt_tags: HashSet<String> =
            crate::git::collect_all_tags(&open_repo(&local).unwrap())
                .into_iter()
                .collect();

        commit(&local, "release.txt");
        git(&local, &["tag", "-a", "v1.1.1", "-m", "Release 1.1.1"]);
        git(&local, &["tag", "-a", "v1", "-m", "Release 1.1.1"]);

        let head = git(&local, &["rev-parse", "HEAD~1"]).trim().to_string();
        let mut cp = Checkpoint::new(head, vec!["v1.1.1".to_string()]);
        cp.advance(Phase::TagsCreated);
        cp.save(&local).unwrap();

        (base, local, pre_attempt_tags)
    }

    #[test]
    fn cleanup_drops_the_checkpoint_that_recorded_the_deleted_tags() {
        let (_base, local, pre) = repo_with_failed_attempt();
        assert!(Checkpoint::load(&local).unwrap().is_some());

        cleanup_failed_release_attempt(&local, &config(), &pre).unwrap();

        assert!(
            Checkpoint::load(&local).unwrap().is_none(),
            "a checkpoint surviving the cleanup makes the next attempt skip tag creation"
        );
    }

    #[test]
    fn cleanup_deletes_the_tags_the_failed_attempt_created() {
        let (_base, local, pre) = repo_with_failed_attempt();

        cleanup_failed_release_attempt(&local, &config(), &pre).unwrap();

        let after: HashSet<String> = crate::git::collect_all_tags(&open_repo(&local).unwrap())
            .into_iter()
            .collect();
        assert!(!after.contains("v1.1.1"), "versioned tag must be gone");
        assert!(!after.contains("v1"), "floating tag must be gone");
    }

    #[test]
    fn cleanup_keeps_tags_that_existed_before_the_attempt() {
        let base = tempfile::tempdir().unwrap();
        let remote = base.path().join("remote.git");
        std::fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "--bare", "-b", "main"]);

        let local = base.path().join("local");
        std::fs::create_dir_all(&local).unwrap();
        git(&local, &["init", "-b", "main"]);
        git(
            &local,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        commit(&local, "base.txt");
        git(&local, &["tag", "-a", "v1.1.0", "-m", "Release 1.1.0"]);
        git(&local, &["push", "origin", "main:main"]);

        let pre: HashSet<String> = crate::git::collect_all_tags(&open_repo(&local).unwrap())
            .into_iter()
            .collect();

        commit(&local, "release.txt");
        git(&local, &["tag", "-a", "v1.1.1", "-m", "Release 1.1.1"]);

        cleanup_failed_release_attempt(&local, &config(), &pre).unwrap();

        let after: HashSet<String> = crate::git::collect_all_tags(&open_repo(&local).unwrap())
            .into_iter()
            .collect();
        assert!(after.contains("v1.1.0"), "pre-existing tag must survive");
        assert!(!after.contains("v1.1.1"), "attempt tag must be gone");
    }
}
