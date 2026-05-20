//! Parity guarantee for the gitoxide tag enumeration path.
//!
//! `collect_all_tags` now routes through gix by default — this suite
//! asserts that the gix output matches the libgit2 implementation
//! (still exposed as `collect_all_tags_libgit2` for verification) on
//! the shapes that occur in real repos: empty, lightweight only,
//! annotated only, mixed, and 200-package monorepo scale.
//!
//! Run with `cargo test --test gix_parity -- --nocapture` to see the
//! perf delta line at the end.

use ferrflow::git::{collect_all_tags_gix, collect_all_tags_libgit2};
use git2::{Repository, Signature};
use std::collections::HashSet;
use std::time::Instant;

fn init_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test").unwrap();
    cfg.set_str("user.email", "test@test.com").unwrap();
    let sig = Signature::now("Test", "test@test.com").unwrap();
    let tree_id = {
        let mut idx = repo.index().unwrap();
        idx.write_tree().unwrap()
    };
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }
    (dir, repo)
}

fn add_lightweight_tag(repo: &Repository, name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag_lightweight(name, head.as_object(), false).unwrap();
}

fn add_annotated_tag(repo: &Repository, name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let sig = Signature::now("Test", "test@test.com").unwrap();
    repo.tag(name, head.as_object(), &sig, &format!("msg {name}"), false)
        .unwrap();
}

#[test]
fn parity_empty_repo() {
    let (dir, _repo) = init_repo();
    let g2 = collect_all_tags_libgit2(&Repository::open(dir.path()).unwrap());
    let gx = collect_all_tags_gix(dir.path()).unwrap();
    assert!(g2.is_empty());
    assert!(gx.is_empty());
}

#[test]
fn parity_lightweight_tags() {
    let (dir, repo) = init_repo();
    add_lightweight_tag(&repo, "v1.0.0");
    add_lightweight_tag(&repo, "v1.0.1");
    add_lightweight_tag(&repo, "v2.0.0");
    let g2: HashSet<String> = collect_all_tags_libgit2(&repo).into_iter().collect();
    let gx: HashSet<String> = collect_all_tags_gix(dir.path())
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(g2, gx);
    assert_eq!(g2.len(), 3);
}

#[test]
fn parity_annotated_tags() {
    let (dir, repo) = init_repo();
    add_annotated_tag(&repo, "annot-1");
    add_annotated_tag(&repo, "annot-2");
    let g2: HashSet<String> = collect_all_tags_libgit2(&repo).into_iter().collect();
    let gx: HashSet<String> = collect_all_tags_gix(dir.path())
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(g2, gx);
}

#[test]
fn parity_mixed_tags() {
    let (dir, repo) = init_repo();
    add_lightweight_tag(&repo, "v0.1.0");
    add_annotated_tag(&repo, "rc-2");
    add_lightweight_tag(&repo, "site@v3.0.0");
    add_annotated_tag(&repo, "api@v1.4.2");
    let g2: HashSet<String> = collect_all_tags_libgit2(&repo).into_iter().collect();
    let gx: HashSet<String> = collect_all_tags_gix(dir.path())
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(g2, gx);
    assert!(g2.contains("v0.1.0"));
    assert!(g2.contains("api@v1.4.2"));
}

#[test]
fn parity_at_scale_200_tags() {
    let (dir, repo) = init_repo();
    for i in 0..200 {
        add_lightweight_tag(&repo, &format!("pkg-{i:03}@v0.1.0"));
    }
    let g2: HashSet<String> = collect_all_tags_libgit2(&repo).into_iter().collect();
    let gx: HashSet<String> = collect_all_tags_gix(dir.path())
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(g2, gx);
    assert_eq!(g2.len(), 200);
}

/// Cheap wall-clock comparison printed under `--nocapture`. Not a
/// regression gate — just the bench number this PR is justified by.
#[test]
fn smoke_perf_200_tags() {
    let (dir, repo) = init_repo();
    for i in 0..200 {
        add_lightweight_tag(&repo, &format!("pkg-{i:03}@v0.1.0"));
    }

    let _ = collect_all_tags_libgit2(&repo);
    let _ = collect_all_tags_gix(dir.path()).unwrap();

    let runs = 50;
    let t = Instant::now();
    for _ in 0..runs {
        let _ = collect_all_tags_libgit2(&repo);
    }
    let g2_each = t.elapsed() / runs;

    let t = Instant::now();
    for _ in 0..runs {
        let _ = collect_all_tags_gix(dir.path()).unwrap();
    }
    let gx_each = t.elapsed() / runs;

    println!("collect_all_tags(libgit2, 200 tags) median {g2_each:?}");
    println!("collect_all_tags(gix,    200 tags) median {gx_each:?}");
}
