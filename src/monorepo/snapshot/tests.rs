use super::*;
use crate::test_utils::{commit_file, git, init_repo};

fn suffix(s: &str) -> Prerelease {
    Prerelease::new(s).unwrap()
}

fn opts(tag: &str) -> SnapshotOptions<'_> {
    SnapshotOptions {
        tag,
        from_zero: false,
        no_git: false,
        run_hooks: false,
        dry_run: false,
        verbose: false,
    }
}

#[test]
fn version_appends_suffix_and_commit_to_next_version() {
    assert_eq!(
        snapshot_version("1.2.3", &suffix("pr-123"), "abc1234", false).unwrap(),
        "1.2.3-pr-123.abc1234"
    );
}

#[test]
fn version_replaces_existing_prerelease_and_build_metadata() {
    assert_eq!(
        snapshot_version("2.0.0-beta.4+build.7", &suffix("pr-9"), "abc1234", false).unwrap(),
        "2.0.0-pr-9.abc1234"
    );
}

#[test]
fn from_zero_ignores_the_computed_version() {
    assert_eq!(
        snapshot_version("3.4.5", &suffix("pr-1"), "abc1234", true).unwrap(),
        "0.0.0-pr-1.abc1234"
    );
}

#[test]
fn numeric_commit_with_leading_zero_stays_valid_semver() {
    let v = snapshot_version("1.0.0", &suffix("pr-1"), "0123456", false).unwrap();
    assert_eq!(v, "1.0.0-pr-1.g0123456");
    Version::parse(&v).unwrap();
}

#[test]
fn non_semver_next_version_is_an_error() {
    assert!(snapshot_version("not-a-version", &suffix("pr-1"), "abc1234", false).is_err());
}

#[test]
fn suffix_must_be_a_valid_prerelease() {
    assert!(parse_suffix("pr-123").is_ok());
    assert!(parse_suffix("pr.123").is_ok());
    assert!(parse_suffix("").is_err());
    assert!(parse_suffix("pr_123").is_err());
    assert!(parse_suffix("pr..1").is_err());
    assert!(parse_suffix("pr/1").is_err());
}

fn package_json(version: &str) -> String {
    format!("{{\n  \"name\": \"x\",\n  \"version\": \"{version}\"\n}}\n")
}

fn monorepo(extra_ws: &str) -> (tempfile::TempDir, Config) {
    let (dir, _repo) = init_repo();
    let root = dir.path();
    std::fs::write(
        root.join(".ferrflow"),
        format!(
            r#"{{"workspace": {{"releaseCommitMode": "none"{extra_ws}}}, "package": [
                {{"name": "api", "path": "api", "versionedFiles": [{{"path": "api/package.json", "format": "json"}}]}},
                {{"name": "web", "path": "web", "versionedFiles": [{{"path": "web/package.json", "format": "json"}}]}}
            ]}}"#
        ),
    )
    .unwrap();
    for pkg in ["api", "web"] {
        std::fs::create_dir_all(root.join(pkg)).unwrap();
        std::fs::write(root.join(pkg).join("package.json"), package_json("1.0.0")).unwrap();
    }
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "chore: init"]);
    git(root, &["tag", "api@v1.0.0"]);
    git(root, &["tag", "web@v1.0.0"]);
    commit_file(root, "api/index.js", "x", "feat(api): add x", 1_900_000_000);

    let config = Config::load(root, Some(&root.join(".ferrflow"))).unwrap();
    (dir, config)
}

fn head(root: &Path) -> String {
    git(root, &["rev-parse", "HEAD"]).trim().to_string()
}

fn version_in(root: &Path, pkg: &str) -> String {
    let raw = std::fs::read_to_string(root.join(pkg).join("package.json")).unwrap();
    serde_json::from_str::<serde_json::Value>(&raw).unwrap()["version"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn snapshots_only_packages_with_pending_changes() {
    let (dir, config) = monorepo("");
    let root = dir.path();
    let sha = head(root)[..7].to_string();

    let report = run(root, config, &opts("pr-123")).unwrap();

    assert_eq!(report.commit, sha);
    assert_eq!(report.packages.len(), 1);
    assert_eq!(report.packages[0].name, "api");
    assert_eq!(report.packages[0].previous_version, "1.0.0");
    assert_eq!(report.packages[0].version, format!("1.1.0-pr-123.{sha}"));
    assert_eq!(version_in(root, "api"), format!("1.1.0-pr-123.{sha}"));
    assert_eq!(version_in(root, "web"), "1.0.0");
}

#[test]
fn counts_every_commit_since_the_last_release_not_just_head() {
    let (dir, config) = monorepo("");
    let root = dir.path();
    commit_file(root, "README.md", "docs", "docs: readme", 1_900_000_100);

    let report = run(root, config, &opts("pr-2")).unwrap();

    assert_eq!(report.packages.len(), 1);
    assert_eq!(report.packages[0].name, "api");
}

#[test]
fn leaves_history_and_tags_untouched() {
    let (dir, config) = monorepo("");
    let root = dir.path();
    let before_head = head(root);
    let before_tags = git(root, &["tag", "--list"]);

    run(root, config, &opts("pr-1")).unwrap();

    assert_eq!(head(root), before_head);
    assert_eq!(git(root, &["tag", "--list"]), before_tags);
}

#[test]
fn dry_run_writes_nothing() {
    let (dir, config) = monorepo("");
    let root = dir.path();

    let report = run(
        root,
        config,
        &SnapshotOptions {
            dry_run: true,
            ..opts("pr-1")
        },
    )
    .unwrap();

    assert!(report.dry_run);
    assert!(report.packages[0].version.starts_with("1.1.0-pr-1."));
    assert_eq!(version_in(root, "api"), "1.0.0");
}

#[test]
fn dirty_tree_is_refused_without_no_git() {
    let (dir, config) = monorepo("");
    let root = dir.path();
    std::fs::write(root.join("api/index.js"), "edited").unwrap();

    let err = run(root, config, &opts("pr-1")).unwrap_err();

    assert!(format!("{err:#}").contains("--no-git"), "{err:#}");
    assert_eq!(version_in(root, "api"), "1.0.0");
}

#[test]
fn no_git_runs_on_a_dirty_tree() {
    let (dir, config) = monorepo("");
    let root = dir.path();
    std::fs::write(root.join("api/index.js"), "edited").unwrap();

    let report = run(
        root,
        config,
        &SnapshotOptions {
            no_git: true,
            ..opts("pr-1")
        },
    )
    .unwrap();

    assert_eq!(report.packages.len(), 1);
    assert!(version_in(root, "api").starts_with("1.1.0-pr-1."));
}

#[test]
fn invalid_tag_fails_before_touching_files() {
    let (dir, config) = monorepo("");
    let root = dir.path();

    assert!(run(root, config, &opts("pr_1")).is_err());
    assert_eq!(version_in(root, "api"), "1.0.0");
}

#[test]
fn json_report_shape_is_stable() {
    let (dir, config) = monorepo("");
    let report = run(dir.path(), config, &opts("pr-7")).unwrap();

    let json = serde_json::to_value(&report).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(keys, ["commit", "dry_run", "packages", "tag"]);
    assert_eq!(json["tag"], "pr-7");

    let mut pkg_keys: Vec<&str> = json["packages"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    pkg_keys.sort();
    assert_eq!(pkg_keys, ["name", "previous_version", "version"]);
}

#[test]
fn publish_hooks_see_the_snapshot_version() {
    let (dir, config) = monorepo(
        r#", "hooks": {"prePublish": "echo pre $FERRFLOW_PACKAGE $FERRFLOW_NEW_VERSION $FERRFLOW_CHANNEL >> hooks.log", "postPublish": "echo post $FERRFLOW_PACKAGE >> hooks.log"}"#,
    );
    let root = dir.path();
    let sha = head(root)[..7].to_string();

    run(
        root,
        config,
        &SnapshotOptions {
            run_hooks: true,
            no_git: true,
            ..opts("pr-5")
        },
    )
    .unwrap();
    let log = std::fs::read_to_string(root.join("hooks.log")).unwrap();
    assert_eq!(
        log,
        format!("pre api 1.1.0-pr-5.{sha} pr-5\npost api\n"),
        "pre_publish must fire for every package before any post_publish"
    );
}

#[test]
fn hooks_do_not_run_unless_asked() {
    let (dir, config) = monorepo(r#", "hooks": {"prePublish": "touch hooks.log"}"#);
    let root = dir.path();

    run(
        root,
        config,
        &SnapshotOptions {
            no_git: true,
            ..opts("pr-5")
        },
    )
    .unwrap();

    assert!(!root.join("hooks.log").exists());
}

#[test]
fn lockfile_follows_the_snapshot_version_when_update_lockfiles_is_on() {
    if std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (dir, _repo) = init_repo();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "").unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"1.0.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".ferrflow"),
        r#"{"workspace": {"releaseCommitMode": "none", "updateLockfiles": true}, "package": [
            {"name": "alpha", "path": ".", "versionedFiles": [{"path": "Cargo.toml", "format": "toml"}]}
        ]}"#,
    )
    .unwrap();
    let gen_lock = std::process::Command::new("cargo")
        .current_dir(root)
        .args(["generate-lockfile", "--offline"])
        .output()
        .unwrap();
    assert!(gen_lock.status.success());
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "chore: init"]);
    git(root, &["tag", "v1.0.0"]);
    commit_file(
        root,
        "src/lib.rs",
        "pub fn x() {}",
        "feat: x",
        1_900_000_000,
    );
    let config = Config::load(root, Some(&root.join(".ferrflow"))).unwrap();

    let report = run(root, config, &opts("pr-3")).unwrap();

    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    assert!(
        lock.contains(&format!("version = \"{}\"", report.packages[0].version)),
        "Cargo.lock must carry the snapshot version:\n{lock}"
    );
}
