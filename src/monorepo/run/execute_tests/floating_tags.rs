use crate::config::{Config, FloatingTagLevel, ReleaseCommitMode};

use super::PlannedTag;
use super::{Harness, PlanRun, RecordingForge, commit_file, git, tag_to_create};

fn floating_harness(packages: &[&str], levels: &[FloatingTagLevel]) -> Harness {
    let mut harness = Harness::new();
    let entries: Vec<String> = packages
        .iter()
        .map(|name| {
            let path = if packages.len() == 1 { "." } else { name };
            format!(r#"{{"name":"{name}","path":"{path}"}}"#)
        })
        .collect();
    let config: Config =
        serde_json::from_str(&format!(r#"{{"package":[{}]}}"#, entries.join(","))).unwrap();
    harness.config.packages = config.packages;
    harness.config.workspace.release_commit_mode = ReleaseCommitMode::None;
    harness.config.workspace.floating_tags = levels.to_vec();
    harness
}

fn prerelease(tag: &str, pkg: &str, version: &str) -> PlannedTag {
    PlannedTag {
        is_prerelease: true,
        ..tag_to_create(tag, pkg, version)
    }
}

fn commit_of(harness: &Harness, rev: &str) -> String {
    harness
        .git(&["rev-parse", &format!("{rev}^{{commit}}")])
        .trim()
        .to_string()
}

fn remote_commit_of(harness: &Harness, rev: &str) -> String {
    git(
        &harness.remote,
        &["rev-parse", &format!("{rev}^{{commit}}")],
    )
    .trim()
    .to_string()
}

fn tag_message(harness: &Harness, tag: &str) -> String {
    harness
        .git(&["tag", "-l", "--format=%(contents:subject)", tag])
        .trim()
        .to_string()
}

fn local_tags(harness: &Harness) -> Vec<String> {
    harness
        .git(&["tag", "-l"])
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn a_stable_release_creates_major_and_minor_aliases_and_pushes_them() {
    let harness = floating_harness(
        &["app"],
        &[FloatingTagLevel::Major, FloatingTagLevel::Minor],
    );
    let forge = RecordingForge::default();
    let mut run = PlanRun {
        pkg_outputs: vec![("app".to_string(), Vec::new())],
        ..Default::default()
    };

    run.execute(&harness, &[tag_to_create("v1.2.0", "app", "1.2.0")], &forge)
        .expect("release with floating tags");

    let head = commit_of(&harness, "HEAD");
    for alias in ["v1", "v1.2"] {
        assert_eq!(commit_of(&harness, alias), head, "{alias} locally");
        assert_eq!(remote_commit_of(&harness, alias), head, "{alias} on remote");
        assert_eq!(tag_message(&harness, alias), "Release 1.2.0");
    }
    let lines = run.package_lines("app").join("\n");
    assert!(lines.contains("Created floating tag"), "{lines}");
    assert!(
        run.shared_outputs
            .iter()
            .any(|l| l == "✓ Pushed floating tags"),
        "{:?}",
        run.shared_outputs
    );
}

#[test]
fn an_existing_alias_moves_forward_to_the_new_release() {
    let harness = floating_harness(&["app"], &[FloatingTagLevel::Major]);
    harness.git(&["tag", "-a", "v1", "-m", "Release 1.2.0"]);
    let old = commit_of(&harness, "v1");
    commit_file(&harness.root, "b.txt", "b", "feat: more");
    let forge = RecordingForge::default();
    let mut run = PlanRun {
        pkg_outputs: vec![("app".to_string(), Vec::new())],
        ..Default::default()
    };

    run.execute(&harness, &[tag_to_create("v1.3.0", "app", "1.3.0")], &forge)
        .expect("forward move is allowed");

    let head = commit_of(&harness, "HEAD");
    assert_ne!(old, head);
    assert_eq!(commit_of(&harness, "v1"), head);
    assert_eq!(remote_commit_of(&harness, "v1"), head);
    assert_eq!(tag_message(&harness, "v1"), "Release 1.3.0");
    let lines = run.package_lines("app").join("\n");
    assert!(lines.contains("Moved floating tag"), "{lines}");
}

#[test]
fn an_alias_that_would_move_backward_is_refused_and_left_alone() {
    let harness = floating_harness(&["app"], &[FloatingTagLevel::Major]);
    harness.git(&["tag", "-a", "v1", "-m", "Release 1.5.0"]);
    let pinned = commit_of(&harness, "v1");
    commit_file(&harness.root, "b.txt", "b", "fix: hotfix");
    let forge = RecordingForge::default();

    let err = PlanRun::default()
        .execute(&harness, &[tag_to_create("v1.2.4", "app", "1.2.4")], &forge)
        .expect_err("a backward move without --force must fail");

    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("would move backward (1.5.0 → 1.2.4)"),
        "{rendered}"
    );
    assert_eq!(commit_of(&harness, "v1"), pinned);
    assert_eq!(tag_message(&harness, "v1"), "Release 1.5.0");
    assert!(
        harness.remote_tags().is_empty(),
        "nothing may be pushed once the alias check failed"
    );
    assert!(forge.create_calls.lock().unwrap().is_empty());
}

#[test]
fn force_moves_an_alias_backward() {
    let harness = floating_harness(&["app"], &[FloatingTagLevel::Major]);
    harness.git(&["tag", "-a", "v1", "-m", "Release 1.5.0"]);
    commit_file(&harness.root, "b.txt", "b", "fix: hotfix");
    let forge = RecordingForge::default();
    let mut run = PlanRun {
        force: true,
        ..Default::default()
    };

    run.execute(&harness, &[tag_to_create("v1.2.4", "app", "1.2.4")], &forge)
        .expect("--force overrides the backward check");

    let head = commit_of(&harness, "HEAD");
    assert_eq!(commit_of(&harness, "v1"), head);
    assert_eq!(remote_commit_of(&harness, "v1"), head);
    assert_eq!(tag_message(&harness, "v1"), "Release 1.2.4");
}

#[test]
fn a_prerelease_never_moves_a_stable_alias() {
    let mut harness = floating_harness(&["app"], &[FloatingTagLevel::Major]);
    harness.config.workspace.latest_tag = Some("latest".to_string());
    harness.git(&["tag", "-a", "v2", "-m", "Release 2.0.0"]);
    harness.git(&["tag", "-a", "latest", "-m", "Release 2.0.0"]);
    let pinned = commit_of(&harness, "v2");
    commit_file(&harness.root, "b.txt", "b", "feat!: next");
    let forge = RecordingForge::default();

    PlanRun::default()
        .execute(
            &harness,
            &[prerelease("v2.1.0-rc.1", "app", "2.1.0-rc.1")],
            &forge,
        )
        .expect("prerelease release");

    assert_eq!(commit_of(&harness, "v2"), pinned);
    assert_eq!(commit_of(&harness, "latest"), pinned);
    assert_eq!(harness.remote_tags(), vec!["v2.1.0-rc.1".to_string()]);
}

#[test]
fn monorepo_aliases_carry_each_package_prefix() {
    let harness = floating_harness(&["api", "web"], &[FloatingTagLevel::Major]);
    let forge = RecordingForge::default();

    PlanRun::default()
        .execute(
            &harness,
            &[
                tag_to_create("api@v1.4.0", "api", "1.4.0"),
                tag_to_create("web@v2.1.0", "web", "2.1.0"),
            ],
            &forge,
        )
        .expect("monorepo release");

    let tags = local_tags(&harness);
    assert!(tags.contains(&"api@v1".to_string()), "{tags:?}");
    assert!(tags.contains(&"web@v2".to_string()), "{tags:?}");
    assert!(!tags.contains(&"v1".to_string()), "{tags:?}");
    assert!(!tags.contains(&"v2".to_string()), "{tags:?}");
    assert_eq!(tag_message(&harness, "api@v1"), "Release 1.4.0");
    assert_eq!(tag_message(&harness, "web@v2"), "Release 2.1.0");
}

#[test]
fn a_package_override_opts_out_of_the_workspace_aliases() {
    let mut harness = floating_harness(&["api", "web"], &[FloatingTagLevel::Major]);
    harness.config.packages[1].floating_tags = Some(Vec::new());
    let forge = RecordingForge::default();

    PlanRun::default()
        .execute(
            &harness,
            &[
                tag_to_create("api@v1.4.0", "api", "1.4.0"),
                tag_to_create("web@v2.1.0", "web", "2.1.0"),
            ],
            &forge,
        )
        .expect("monorepo release");

    let tags = local_tags(&harness);
    assert!(tags.contains(&"api@v1".to_string()), "{tags:?}");
    assert!(!tags.contains(&"web@v2".to_string()), "{tags:?}");
}

#[test]
fn the_latest_alias_follows_a_stable_release() {
    let mut harness = floating_harness(&["api", "web"], &[]);
    harness.config.workspace.latest_tag = Some("{name}-latest".to_string());
    let forge = RecordingForge::default();

    PlanRun::default()
        .execute(
            &harness,
            &[tag_to_create("api@v1.4.0", "api", "1.4.0")],
            &forge,
        )
        .expect("release with a latest alias");

    assert_eq!(tag_message(&harness, "api-latest"), "Release 1.4.0");
    assert_eq!(
        remote_commit_of(&harness, "api-latest"),
        commit_of(&harness, "HEAD")
    );
    assert!(!local_tags(&harness).contains(&"web-latest".to_string()));
}

#[test]
fn a_tag_for_an_unknown_package_is_an_error() {
    let harness = floating_harness(&["app"], &[FloatingTagLevel::Major]);
    let forge = RecordingForge::default();

    let err = PlanRun::default()
        .execute(
            &harness,
            &[tag_to_create("v1.0.0", "ghost", "1.0.0")],
            &forge,
        )
        .expect_err("an alias for a package outside the config cannot be resolved");

    assert!(
        format!("{err:#}").contains("package 'ghost' not found in config"),
        "{err:#}"
    );
    assert!(harness.remote_tags().is_empty());
}
