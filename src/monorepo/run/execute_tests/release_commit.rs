use std::collections::HashMap;

use crate::config::{Config, ReleaseCommitBody, ReleaseCommitMode, ReleaseCommitScope};

use super::{Harness, PlanRun, RecordingForge, commit_file, git, init_workdir, tag_to_create};

const RELEASE_BRANCH: &str = "ferrflow/release-main";

fn two_package_harness(mode: ReleaseCommitMode) -> Harness {
    let mut harness = Harness::new();
    let config: Config = serde_json::from_str(
        r#"{"package":[{"name":"api","path":"api"},{"name":"web","path":"web"}]}"#,
    )
    .unwrap();
    harness.config.packages = config.packages;
    harness.config.workspace.release_commit_mode = mode;
    harness
}

fn bump_files(harness: &Harness) {
    std::fs::write(harness.root.join("api.txt"), "api 1.1.0").unwrap();
    std::fs::write(harness.root.join("web.txt"), "web 2.0.0").unwrap();
    std::fs::write(harness.root.join("stray.txt"), "not part of the release").unwrap();
}

fn tags() -> Vec<super::PlannedTag> {
    vec![
        tag_to_create("api@v1.1.0", "api", "1.1.0"),
        tag_to_create("web@v2.0.0", "web", "2.0.0"),
    ]
}

fn split_run() -> PlanRun {
    PlanRun {
        files_to_commit: vec!["api.txt".to_string(), "web.txt".to_string()],
        files_per_package: HashMap::from([
            ("api".to_string(), vec!["api.txt".to_string()]),
            ("web".to_string(), vec!["web.txt".to_string()]),
        ]),
        ..Default::default()
    }
}

fn subject(harness: &Harness, rev: &str) -> String {
    harness
        .git(&["log", "-1", "--format=%s", rev])
        .trim()
        .to_string()
}

fn files_in(dir: &std::path::Path, rev: &str) -> Vec<String> {
    let mut files: Vec<String> = git(dir, &["show", "--name-only", "--format=", rev])
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    files.sort();
    files
}

fn release_commit_count(harness: &Harness) -> usize {
    harness
        .git(&["rev-list", "--count", "HEAD"])
        .trim()
        .parse::<usize>()
        .unwrap()
        - 1
}

#[test]
fn a_grouped_commit_names_every_package_and_stages_only_release_files() {
    let harness = two_package_harness(ReleaseCommitMode::Commit);
    bump_files(&harness);
    let forge = RecordingForge::default();
    let mut run = split_run();

    run.execute(&harness, &tags(), &forge)
        .expect("grouped release");

    assert_eq!(release_commit_count(&harness), 1);
    assert_eq!(
        subject(&harness, "HEAD"),
        "chore(release): api v1.1.0, web v2.0.0 [skip ci]"
    );
    assert_eq!(files_in(&harness.root, "HEAD"), vec!["api.txt", "web.txt"]);
    assert!(
        harness
            .git(&["status", "--porcelain"])
            .contains("?? stray.txt"),
        "a dirty file outside the release must stay out of the commit"
    );
    assert_eq!(
        git(&harness.remote, &["rev-parse", "main"]).trim(),
        harness.git(&["rev-parse", "HEAD"]).trim(),
        "the release commit reaches the remote branch"
    );
    assert!(
        run.shared_outputs
            .contains(&"✓ Committed release changes".to_string()),
        "{:?}",
        run.shared_outputs
    );
}

#[test]
fn per_package_scope_makes_one_commit_per_package_with_its_own_files() {
    let mut harness = two_package_harness(ReleaseCommitMode::Commit);
    harness.config.workspace.release_commit_scope = ReleaseCommitScope::PerPackage;
    bump_files(&harness);
    let forge = RecordingForge::default();
    let mut run = split_run();

    run.execute(&harness, &tags(), &forge)
        .expect("per-package release");

    assert_eq!(release_commit_count(&harness), 2);
    assert_eq!(
        subject(&harness, "HEAD~1"),
        "chore(release): api v1.1.0 [skip ci]"
    );
    assert_eq!(files_in(&harness.root, "HEAD~1"), vec!["api.txt"]);
    assert_eq!(
        subject(&harness, "HEAD"),
        "chore(release): web v2.0.0 [skip ci]"
    );
    assert_eq!(files_in(&harness.root, "HEAD"), vec!["web.txt"]);
    assert!(
        run.shared_outputs
            .contains(&"✓ Committed release changes (per-package)".to_string())
    );
}

#[test]
fn per_package_scope_with_a_single_package_falls_back_to_one_grouped_commit() {
    let mut harness = two_package_harness(ReleaseCommitMode::Commit);
    harness.config.workspace.release_commit_scope = ReleaseCommitScope::PerPackage;
    bump_files(&harness);
    let forge = RecordingForge::default();
    let mut run = PlanRun {
        files_to_commit: vec!["api.txt".to_string()],
        ..Default::default()
    };

    run.execute(
        &harness,
        &[tag_to_create("api@v1.1.0", "api", "1.1.0")],
        &forge,
    )
    .expect("single package release");

    assert_eq!(release_commit_count(&harness), 1);
    assert_eq!(files_in(&harness.root, "HEAD"), vec!["api.txt"]);
}

#[test]
fn skip_ci_off_drops_the_marker_and_a_summary_body_lists_each_package() {
    let mut harness = two_package_harness(ReleaseCommitMode::Commit);
    harness.config.workspace.skip_ci = Some(false);
    harness.config.workspace.release_commit_body = ReleaseCommitBody::Summary;
    bump_files(&harness);
    let forge = RecordingForge::default();

    split_run()
        .execute(&harness, &tags(), &forge)
        .expect("release");

    let message = harness.git(&["log", "-1", "--format=%B", "HEAD"]);
    assert_eq!(
        message.trim(),
        "chore(release): api v1.1.0, web v2.0.0\n\n- api 1.1.0 (1 commit)\n- web 2.0.0 (1 commit)"
    );
}

#[test]
fn pr_mode_commits_on_the_release_branch_and_leaves_the_target_alone() {
    let harness = two_package_harness(ReleaseCommitMode::Pr);
    bump_files(&harness);
    let before = harness.git(&["rev-parse", "HEAD"]).trim().to_string();
    let forge = RecordingForge::default();
    let mut run = split_run();

    run.execute(&harness, &tags(), &forge)
        .expect("release proposed");

    assert_eq!(harness.git(&["rev-parse", "HEAD"]).trim(), before);
    assert_eq!(
        git(&harness.remote, &["rev-parse", "main"]).trim(),
        before,
        "the target branch only moves when the PR merges"
    );
    assert_eq!(
        git(
            &harness.remote,
            &["log", "-1", "--format=%s", RELEASE_BRANCH]
        )
        .trim(),
        "chore(release): api v1.1.0, web v2.0.0"
    );
    assert_eq!(
        files_in(&harness.remote, RELEASE_BRANCH),
        vec!["api.txt", "web.txt"]
    );

    let requests = forge.mr_requests.lock().unwrap();
    let (head, base, body) = requests.first().expect("a release PR was opened");
    assert_eq!(head, RELEASE_BRANCH);
    assert_eq!(base, "main");
    assert_eq!(
        body,
        "Automated release commit.\n\n- `api@v1.1.0`\n- `web@v2.0.0`"
    );
    assert!(
        run.shared_outputs
            .iter()
            .any(|l| l.starts_with("✓ Created pull request #")),
        "{:?}",
        run.shared_outputs
    );
}

#[test]
fn pr_mode_per_package_scope_stacks_one_commit_per_package_on_the_branch() {
    let mut harness = two_package_harness(ReleaseCommitMode::Pr);
    harness.config.workspace.release_commit_scope = ReleaseCommitScope::PerPackage;
    bump_files(&harness);
    let forge = RecordingForge::default();

    split_run()
        .execute(&harness, &tags(), &forge)
        .expect("release proposed");

    let log = git(
        &harness.remote,
        &["log", "--format=%s", &format!("main..{RELEASE_BRANCH}")],
    );
    let subjects: Vec<&str> = log.lines().collect();
    assert_eq!(
        subjects,
        vec!["chore(release): web v2.0.0", "chore(release): api v1.1.0"]
    );
}

#[test]
fn an_open_release_pr_is_updated_instead_of_duplicated() {
    let harness = two_package_harness(ReleaseCommitMode::Pr);
    bump_files(&harness);
    let forge = RecordingForge {
        open_pr: Some(41),
        ..Default::default()
    };
    let mut run = split_run();

    run.execute(&harness, &tags(), &forge)
        .expect("release proposed");

    assert_eq!(*forge.updated_mrs.lock().unwrap(), vec![41]);
    assert!(forge.mr_requests.lock().unwrap().is_empty());
    assert!(
        run.shared_outputs
            .iter()
            .any(|l| l.starts_with("✓ Updated pull request #")),
        "{:?}",
        run.shared_outputs
    );
}

#[test]
fn auto_merge_is_requested_only_when_configured() {
    let mut harness = two_package_harness(ReleaseCommitMode::Pr);
    harness.config.workspace.auto_merge_releases = false;
    bump_files(&harness);
    let forge = RecordingForge::default();

    split_run()
        .execute(&harness, &tags(), &forge)
        .expect("release proposed");

    assert_eq!(*forge.auto_merge_calls.lock().unwrap(), 0);
}

#[test]
fn a_failed_auto_merge_request_does_not_fail_the_release() {
    let mut harness = two_package_harness(ReleaseCommitMode::Pr);
    harness.config.workspace.auto_merge_releases = true;
    bump_files(&harness);
    let forge = RecordingForge {
        auto_merge_failure: Some("auto-merge is disabled for this repository".to_string()),
        ..Default::default()
    };
    let mut run = split_run();

    run.execute(&harness, &tags(), &forge)
        .expect("auto-merge is best effort");

    assert_eq!(*forge.auto_merge_calls.lock().unwrap(), 1);
    assert!(!run.shared_outputs.iter().any(|l| l.contains("Auto-merge")));
}

#[test]
fn a_forge_without_merge_requests_only_warns_when_the_mr_fails() {
    let harness = two_package_harness(ReleaseCommitMode::Pr);
    bump_files(&harness);
    let forge = RecordingForge {
        mr_failure: Some("merge requests are not supported".to_string()),
        no_merge_requests: true,
        ..Default::default()
    };

    split_run()
        .execute(&harness, &tags(), &forge)
        .expect("a forge that cannot open PRs does not fail the pushed branch");

    assert_eq!(forge.mr_requests.lock().unwrap().len(), 1);
}

#[test]
fn a_release_branch_carrying_a_foreign_commit_is_left_untouched() {
    let harness = two_package_harness(ReleaseCommitMode::Pr);
    let helper = harness._dir.path().join("helper");
    init_workdir(&helper);
    git(
        &helper,
        &["remote", "add", "origin", harness.remote.to_str().unwrap()],
    );
    git(&helper, &["fetch", "origin", "main"]);
    git(&helper, &["checkout", "-B", "work", "FETCH_HEAD"]);
    commit_file(
        &helper,
        "manual.txt",
        "fix",
        "fix: hand edit on the release PR",
    );
    git(
        &helper,
        &["push", "origin", &format!("HEAD:{RELEASE_BRANCH}")],
    );
    let foreign_tip = git(&harness.remote, &["rev-parse", RELEASE_BRANCH])
        .trim()
        .to_string();

    bump_files(&harness);
    let forge = RecordingForge::default();

    split_run()
        .execute(&harness, &tags(), &forge)
        .expect("a foreign release branch is a warning, not a failure");

    assert_eq!(
        git(&harness.remote, &["rev-parse", RELEASE_BRANCH]).trim(),
        foreign_tip,
        "the hand-edited branch must not be force-pushed over"
    );
    assert!(forge.mr_requests.lock().unwrap().is_empty());
    assert!(forge.updated_mrs.lock().unwrap().is_empty());
}
