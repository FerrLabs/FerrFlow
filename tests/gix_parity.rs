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

use ferrflow::git::{
    TagIndex, collect_all_tags_gix, collect_all_tags_libgit2, tag_index_build_gix,
    tag_index_build_gix_with,
};
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

/// Compare TagIndex outputs between the gix path (default) and the
/// libgit2 fallback. The struct's `entries` field is private, so we
/// probe via the public query methods on a few prefixes.
fn extract_index_view(idx: &TagIndex) -> (Vec<String>, usize) {
    use ferrflow::config::OrphanedTagStrategy;
    let mut names = Vec::new();
    for prefix in &["", "v", "pkg-001@v", "pkg-100@v", "site@v", "api@v"] {
        if let Some(n) = idx.find_last_tag_name(prefix, OrphanedTagStrategy::Warn) {
            names.push(format!("{prefix}|{n}"));
        }
    }
    (names, idx.ancestors.len())
}

/// Parity between the libgit2 and gix implementations of TagIndex.
/// `TagIndex::build` currently routes through libgit2 (gix was slower
/// on per-tag commit lookups — see [`TagIndex::build`] comment), but
/// both implementations exist and must agree on the query results.
#[test]
fn tag_index_parity_mixed() {
    let (dir, repo) = init_repo();
    add_lightweight_tag(&repo, "v0.1.0");
    add_annotated_tag(&repo, "v0.2.0");
    add_lightweight_tag(&repo, "api@v1.0.0");
    add_annotated_tag(&repo, "site@v2.3.0");
    add_lightweight_tag(&repo, "pkg-001@v0.5.0");

    let g2 = TagIndex::build_libgit2(&Repository::open(dir.path()).unwrap()).unwrap();
    let gx = tag_index_build_gix(dir.path()).unwrap();

    let (g2_names, g2_anc) = extract_index_view(&g2);
    let (gx_names, gx_anc) = extract_index_view(&gx);
    let g2_set: HashSet<_> = g2_names.iter().collect();
    let gx_set: HashSet<_> = gx_names.iter().collect();
    assert_eq!(g2_set, gx_set, "tag query results diverge");
    assert_eq!(g2_anc, gx_anc, "ancestor set sizes diverge");
}

#[test]
fn tag_index_parity_at_scale_200_tags() {
    let (dir, repo) = init_repo();
    for i in 0..200 {
        add_lightweight_tag(&repo, &format!("pkg-{i:03}@v0.1.0"));
    }
    let g2 = TagIndex::build_libgit2(&Repository::open(dir.path()).unwrap()).unwrap();
    let gx = tag_index_build_gix(dir.path()).unwrap();
    let (g2_names, g2_anc) = extract_index_view(&g2);
    let (gx_names, gx_anc) = extract_index_view(&gx);
    assert_eq!(
        g2_names.iter().collect::<HashSet<_>>(),
        gx_names.iter().collect::<HashSet<_>>()
    );
    assert_eq!(g2_anc, gx_anc);
}

/// Architectural unlock probe: does sharing the gix::Repository
/// handle across calls flip the perf back to gix-winning?
///
/// Compared to `tag_index_smoke_perf_200_tags` below which opens gix
/// fresh per call, this variant opens once. The delta tells us how
/// much of the gix slowdown was just the per-call open cost vs how
/// much is the per-tag object lookup overhead.
#[test]
fn tag_index_smoke_perf_200_tags_shared_handle() {
    let (dir, repo) = init_repo();
    for i in 0..200 {
        add_lightweight_tag(&repo, &format!("pkg-{i:03}@v0.1.0"));
    }
    let gix_repo = gix::open(dir.path()).unwrap();

    let _ = TagIndex::build_libgit2(&Repository::open(dir.path()).unwrap()).unwrap();
    let _ = tag_index_build_gix_with(&gix_repo).unwrap();

    let runs = 20;
    let t = Instant::now();
    for _ in 0..runs {
        let _ = TagIndex::build_libgit2(&Repository::open(dir.path()).unwrap()).unwrap();
    }
    let g2_each = t.elapsed() / runs;

    let t = Instant::now();
    for _ in 0..runs {
        let _ = tag_index_build_gix_with(&gix_repo).unwrap();
    }
    let gx_each = t.elapsed() / runs;

    println!("TagIndex::build_libgit2          (200 tags) median {g2_each:?}");
    println!("tag_index_build_gix_with (shared, 200 tags) median {gx_each:?}");
}

/// Why TagIndex::build stays on libgit2: this bench documents the
/// regression that would have shipped if we'd flipped it to gix.
#[test]
fn tag_index_smoke_perf_200_tags() {
    let (dir, repo) = init_repo();
    for i in 0..200 {
        add_lightweight_tag(&repo, &format!("pkg-{i:03}@v0.1.0"));
    }
    let _ = TagIndex::build_libgit2(&Repository::open(dir.path()).unwrap()).unwrap();
    let _ = tag_index_build_gix(dir.path()).unwrap();

    let runs = 20;
    let t = Instant::now();
    for _ in 0..runs {
        let _ = TagIndex::build_libgit2(&Repository::open(dir.path()).unwrap()).unwrap();
    }
    let g2_each = t.elapsed() / runs;

    let t = Instant::now();
    for _ in 0..runs {
        let _ = tag_index_build_gix(dir.path()).unwrap();
    }
    let gx_each = t.elapsed() / runs;

    println!("TagIndex::build_libgit2(200 tags) median {g2_each:?}");
    println!("tag_index_build_gix   (200 tags) median {gx_each:?}");
    // Not asserted: this is documentation, not a regression gate.
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
