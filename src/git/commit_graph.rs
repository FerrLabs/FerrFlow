use std::path::Path;

use super::repo::Repository;
use super::shell::run_git;

const MIN_COMMITS: usize = 1000;

/// Best-effort: write a commit-graph when the repo has none and its history is
/// large enough to pay for it. gix reads `.git/objects/info/commit-graph` on
/// every revwalk (`use_commit_graph(true)`), but never writes one — so a fresh
/// CI clone (a pack with no graph) never benefits. Writing it once makes later
/// runs' tag scans and commit walks materially faster (#690).
///
/// Never fails the command. Callers must skip this on read-only `--dry-run`
/// paths — it shells out to `git commit-graph write`, which touches `.git`.
pub fn write_commit_graph_if_absent(repo: &Repository) {
    maybe_write(repo, MIN_COMMITS);
}

fn maybe_write(repo: &Repository, min_commits: usize) {
    let Some(workdir) = repo.workdir() else {
        return;
    };
    if commit_graph_present(repo) {
        return;
    }
    if reachable_commit_count(workdir).is_none_or(|count| count <= min_commits) {
        return;
    }
    if run_git(workdir, &["commit-graph", "write", "--reachable"]).is_ok() {
        tracing::debug!("wrote commit-graph for faster future revision walks");
    }
}

fn commit_graph_present(repo: &Repository) -> bool {
    let info = repo.git_dir().join("objects").join("info");
    info.join("commit-graph").is_file()
        || info
            .join("commit-graphs")
            .join("commit-graph-chain")
            .is_file()
}

fn reachable_commit_count(workdir: &Path) -> Option<usize> {
    run_git(workdir, &["rev-list", "--count", "HEAD"])
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{commit_file, init_repo};

    #[test]
    fn counts_reachable_history() {
        let (dir, _repo) = init_repo();
        for i in 0..3 {
            commit_file(
                dir.path(),
                &format!("f{i}.txt"),
                "x",
                "chore: c",
                1_900_000_000 + i,
            );
        }
        assert_eq!(reachable_commit_count(dir.path()), Some(3));
    }

    #[test]
    fn detects_and_writes_the_graph() {
        let (dir, repo) = init_repo();
        commit_file(dir.path(), "a.txt", "x", "chore: a", 1_900_000_000);

        assert!(!commit_graph_present(&repo), "a fresh repo has no graph");

        maybe_write(&repo, 0);
        assert!(
            commit_graph_present(&repo),
            "the graph should exist after a write"
        );
    }

    #[test]
    fn skips_small_histories() {
        let (dir, repo) = init_repo();
        commit_file(dir.path(), "a.txt", "x", "chore: a", 1_900_000_000);

        maybe_write(&repo, MIN_COMMITS);
        assert!(
            !commit_graph_present(&repo),
            "a tiny history must not trigger a write"
        );
    }
}
