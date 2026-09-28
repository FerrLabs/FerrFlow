use super::report::{Check, Report, Section, Status};

#[test]
fn exit_code_is_zero_when_all_green() {
    let report = Report::build(vec![Section::new(
        "Repo",
        vec![Check::ok("git repository", None), Check::info("tags", None)],
    )]);
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.status, Status::Ok);
}

#[test]
fn a_warning_sets_exit_code_one() {
    let report = Report::build(vec![Section::new(
        "Repo",
        vec![Check::ok("a", None), Check::warn("b", Some("dirty".into()))],
    )]);
    assert_eq!(report.exit_code, 1);
    assert_eq!(report.status, Status::Warn);
}

#[test]
fn an_error_sets_exit_code_two_even_with_warnings() {
    let report = Report::build(vec![Section::new(
        "Repo",
        vec![
            Check::warn("a", None),
            Check::error("b", Some("no repo".into())),
        ],
    )]);
    assert_eq!(report.exit_code, 2);
    assert_eq!(report.status, Status::Error);
}

#[test]
fn json_shape_is_stable_for_fixtures() {
    let report = Report::build(vec![Section::new(
        "Repo",
        vec![Check::ok("git repository", Some("HEAD at abc1234".into()))],
    )]);
    let json: serde_json::Value = serde_json::from_str(&report.to_json().unwrap()).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["exit_code"], 0);
    assert_eq!(json["sections"][0]["title"], "Repo");
    assert_eq!(json["sections"][0]["checks"][0]["name"], "git repository");
    assert_eq!(json["sections"][0]["checks"][0]["status"], "ok");
    assert_eq!(
        json["sections"][0]["checks"][0]["detail"],
        "HEAD at abc1234"
    );
}

mod end_to_end {
    use crate::config::Config;
    use crate::doctor::checks;
    use crate::doctor::report::Status;
    use crate::git::{get_repo_root, open_repo};
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};
    use std::path::Path;

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn find<'a>(section: &'a super::Section, name: &str) -> &'a super::Check {
        section
            .checks
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("missing check '{name}'"))
    }

    #[test]
    fn fresh_repo_without_commits_flags_the_missing_pieces() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        with_cwd(root, || {
            let repo = open_repo(root).ok();
            let repo_ref = repo.as_ref();
            let section = checks::repo_section(repo_ref, None, root);
            assert_eq!(find(&section, "commit history").status, Status::Error);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn misconfigured_package_path_is_pinpointed() {
        let (dir, repo) = init_repo();
        let root = dir.path();
        write(
            root,
            ".ferrflow",
            r#"{"package":[{"name":"api","path":"packages/api","versionedFiles":[{"path":"packages/api/Cargo.toml","format":"toml"}]}]}"#,
        );
        commit_file(root, "seed.txt", "x", "chore: seed", 1_900_000_000);

        let config = Config::load(&get_repo_root(&repo).unwrap(), None).ok();
        let discovered = Config::discovered_config_paths(root);
        let section = checks::config_section(config.as_ref(), None, &discovered, root);

        assert!(
            section
                .checks
                .iter()
                .any(|c| c.status == Status::Error && c.name.contains("packages/api")),
            "expected an error naming the missing package path, got: {:?}",
            section.checks
        );
    }

    #[test]
    fn multiple_config_files_are_flagged_as_ambiguous() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        write(root, "ferrflow.json", r#"{"package":[]}"#);
        write(root, "ferrflow.toml", "");

        let discovered = Config::discovered_config_paths(root);
        let section = checks::config_section(None, None, &discovered, root);

        let config_file = find(&section, "config file");
        assert_eq!(config_file.status, Status::Error);
        assert!(config_file.detail.as_deref().unwrap().contains("ambiguous"));
        assert_eq!(section.checks.len(), 1);
    }

    #[test]
    fn ci_section_reports_the_pinned_ferrflow_action() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        write(
            root,
            ".github/workflows/release.yml",
            "jobs:\n  release:\n    steps:\n      - uses: FerrLabs/FerrFlow@v4\n",
        );
        let section = checks::ci_section(root);
        let action = find(&section, "FerrFlow action");
        assert_eq!(action.status, Status::Ok);
        assert_eq!(action.detail.as_deref(), Some("FerrLabs/FerrFlow@v4"));
    }

    #[test]
    fn clean_configured_repo_is_all_green() {
        let (dir, repo) = init_repo();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
        );
        write(
            root,
            ".ferrflow",
            r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#,
        );
        commit_file(root, "seed.txt", "x", "chore: seed", 1_900_000_000);
        git(root, &["tag", "v1.0.0"]);

        let config = Config::load(&get_repo_root(&repo).unwrap(), None).ok();
        let discovered = Config::discovered_config_paths(root);
        let section = checks::config_section(config.as_ref(), None, &discovered, root);

        assert!(
            section.checks.iter().all(|c| c.status != Status::Error),
            "a clean config should raise no errors: {:?}",
            section.checks
        );
        assert_eq!(find(&section, "config parses").status, Status::Ok);
    }
}

mod identity {
    use super::super::checks::identity_check;
    use super::Status;

    #[test]
    fn a_resolved_identity_is_shown() {
        let check = identity_check(Some("Jane Dev <jane@example.com>".into()), false);
        assert_eq!(check.status, Status::Ok);
        assert_eq!(check.detail.as_deref(), Some("Jane Dev <jane@example.com>"));
    }

    #[test]
    fn a_missing_identity_warns_off_actions() {
        let check = identity_check(None, false);
        assert_eq!(check.status, Status::Warn);
        assert!(check.detail.unwrap().contains("git config user.email"));
    }

    #[test]
    fn a_missing_identity_on_actions_is_covered_by_the_fallback() {
        let check = identity_check(None, true);
        assert_eq!(check.status, Status::Info);
        assert!(check.detail.unwrap().contains("github-actions[bot]"));
    }
}

mod lockfiles {
    use super::super::checks::versioning_section;
    use super::{Check, Status};
    use crate::config::Config;
    use std::path::Path;

    struct Fixture {
        dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new(workspace: &str, manifest_version: &str, lockfile: Option<&str>) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path();
            std::fs::create_dir_all(root.join("api")).unwrap();
            std::fs::write(
                root.join("api/Cargo.toml"),
                format!("[package]\nname = \"ferrflow-api\"\nversion = \"{manifest_version}\"\n"),
            )
            .unwrap();
            if let Some(recorded) = lockfile {
                std::fs::write(
                    root.join("api/Cargo.lock"),
                    format!("[[package]]\nname = \"ferrflow-api\"\nversion = \"{recorded}\"\n"),
                )
                .unwrap();
            }
            std::fs::write(
                root.join(".ferrflow"),
                format!(
                    r#"{{"workspace":{{{workspace}}},"package":[{{"name":"api","path":"api","versionedFiles":[{{"path":"api/Cargo.toml","format":"toml"}}]}}]}}"#
                ),
            )
            .unwrap();
            Self { dir }
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn lockfile_check(&self) -> Option<Check> {
            let config = Config::load(self.root(), Some(&self.root().join(".ferrflow"))).unwrap();
            versioning_section(Some(&config), self.root())
                .checks
                .into_iter()
                .find(|c| c.name.contains("lockfile"))
        }
    }

    #[test]
    fn a_lockfile_that_disagrees_with_its_manifest_is_reported() {
        let fx = Fixture::new(r#""updateLockfiles":true"#, "2026.8.1", Some("6.1.0"));

        let check = fx.lockfile_check().expect("a lockfile check");

        assert_eq!(check.status, Status::Warn);
        let detail = check.detail.unwrap_or_default();
        assert!(
            detail.contains("6.1.0") && detail.contains("2026.8.1"),
            "{detail}"
        );
        assert!(
            detail.contains("--locked"),
            "the detail should say what actually breaks: {detail}"
        );
    }

    #[test]
    fn a_lockfile_left_out_of_releases_is_flagged_before_it_drifts() {
        let fx = Fixture::new("", "1.0.0", Some("1.0.0"));

        let check = fx.lockfile_check().expect("a lockfile check");

        assert_eq!(
            check.status,
            Status::Warn,
            "versions agree today, but nothing keeps them agreeing on the next release"
        );
        assert!(
            check.detail.unwrap_or_default().contains("updateLockfiles"),
            "the warning has to name the setting that fixes it"
        );
    }

    #[test]
    fn a_synced_lockfile_that_agrees_is_not_a_warning() {
        let fx = Fixture::new(r#""updateLockfiles":true"#, "1.0.0", Some("1.0.0"));

        let check = fx.lockfile_check().expect("a lockfile check");

        assert_eq!(check.status, Status::Ok);
    }

    #[test]
    fn a_package_without_a_lockfile_is_not_mentioned() {
        let fx = Fixture::new(r#""updateLockfiles":true"#, "1.0.0", None);

        assert!(
            fx.lockfile_check().is_none(),
            "nothing to say about a lockfile that does not exist"
        );
    }
}

fn only(vars: &'static [&'static str]) -> impl Fn(&str) -> bool {
    move |var| vars.contains(&var)
}

fn token_status(
    forge: Option<crate::config::ForgeKind>,
    vars: &'static [&'static str],
) -> (Status, String) {
    let check = super::checks::token_check(forge, only(vars));
    (check.status, check.detail.unwrap_or_default())
}

#[test]
fn an_unrecognised_forge_does_not_count_the_github_token() {
    let (status, detail) = token_status(None, &["GITHUB_TOKEN"]);
    assert_eq!(status, Status::Warn);
    assert!(
        detail.contains("only sent to a remote recognised as GitHub"),
        "{detail}"
    );
}

#[test]
fn ferrflow_token_is_enough_whatever_the_forge() {
    assert_eq!(token_status(None, &["FERRFLOW_TOKEN"]).0, Status::Ok);
    assert_eq!(
        token_status(Some(crate::config::ForgeKind::Gitea), &["FERRFLOW_TOKEN"]).0,
        Status::Ok
    );
}

#[test]
fn forgejo_token_counts_for_gitea() {
    let (status, detail) = token_status(Some(crate::config::ForgeKind::Gitea), &["FORGEJO_TOKEN"]);
    assert_eq!(status, Status::Ok);
    assert_eq!(detail, "FORGEJO_TOKEN is set");
}

#[test]
fn gitea_with_only_the_github_token_warns_that_releases_need_their_own() {
    let (status, detail) = token_status(Some(crate::config::ForgeKind::Gitea), &["GITHUB_TOKEN"]);
    assert_eq!(status, Status::Warn);
    assert!(
        detail.contains("releases need GITEA_TOKEN or FORGEJO_TOKEN"),
        "{detail}"
    );
}

#[test]
fn a_known_forge_without_its_token_warns() {
    let (status, detail) = token_status(Some(crate::config::ForgeKind::Gitlab), &["GITHUB_TOKEN"]);
    assert_eq!(status, Status::Warn);
    assert!(
        detail.contains("GITLAB_TOKEN or FERRFLOW_TOKEN"),
        "{detail}"
    );
    assert_eq!(
        token_status(Some(crate::config::ForgeKind::Github), &["GITHUB_TOKEN"]).0,
        Status::Ok
    );
}

mod sections {
    use super::{Check, Section, Status};
    use crate::config::Config;
    use crate::doctor::checks;
    use crate::git::open_repo;
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};
    use std::path::Path;

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn find<'a>(section: &'a Section, name: &str) -> &'a Check {
        section
            .checks
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("missing check '{name}' in {:?}", section.checks))
    }

    fn detail(check: &Check) -> &str {
        check.detail.as_deref().unwrap_or_default()
    }

    fn config_from(root: &Path, json: &str) -> Config {
        write(root, ".ferrflow", json);
        Config::load(root, Some(&root.join(".ferrflow"))).unwrap()
    }

    fn repo_section(root: &Path, config: Option<&Config>) -> Section {
        let repo = open_repo(root).unwrap();
        checks::repo_section(Some(&repo), config, root)
    }

    #[test]
    fn a_missing_repository_is_the_only_repo_check() {
        let dir = tempfile::tempdir().unwrap();
        let section = checks::repo_section(None, None, dir.path());
        assert_eq!(section.checks.len(), 1);
        assert_eq!(section.checks[0].status, Status::Error);
        assert!(detail(&section.checks[0]).contains("git init"));
    }

    #[test]
    fn uncommitted_changes_are_counted_and_warned_about() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        commit_file(root, "a.txt", "x", "chore: seed", 1_900_000_000);

        assert_eq!(
            find(&repo_section(root, None), "working tree clean").status,
            Status::Ok
        );

        write(root, "b.txt", "new");
        let one = repo_section(root, None);
        let check = find(&one, "working tree clean");
        assert_eq!(check.status, Status::Warn);
        assert!(
            detail(check).starts_with("1 uncommitted change;"),
            "{check:?}"
        );

        write(root, "a.txt", "changed");
        let two = repo_section(root, None);
        assert!(
            detail(find(&two, "working tree clean")).starts_with("2 uncommitted changes;"),
            "{:?}",
            two.checks
        );
    }

    #[test]
    fn a_committed_head_is_shown_abbreviated() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        commit_file(root, "a.txt", "x", "chore: seed", 1_900_000_000);
        let head = git(root, &["rev-parse", "HEAD"]);

        let section = repo_section(root, None);
        let check = find(&section, "commit history");
        assert_eq!(check.status, Status::Ok);
        assert_eq!(detail(check), format!("HEAD at {}", &head[..7]));
    }

    #[test]
    fn credentials_in_the_remote_url_are_never_printed() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        git(
            root,
            &[
                "remote",
                "add",
                "origin",
                "https://bot:s3cr3t@github.com/acme/app.git",
            ],
        );

        let section = repo_section(root, None);
        let check = find(&section, "remote configured");
        assert_eq!(check.status, Status::Ok);
        assert_eq!(detail(check), "origin → https://github.com/acme/app.git");
    }

    #[test]
    fn the_configured_remote_name_is_the_one_checked() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        git(
            root,
            &["remote", "add", "origin", "https://github.com/acme/app.git"],
        );
        let config = config_from(
            root,
            r#"{"workspace":{"remote":"upstream"},"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#,
        );

        let section = repo_section(root, Some(&config));
        let check = find(&section, "remote configured");
        assert_eq!(check.status, Status::Warn);
        assert!(detail(check).contains("no 'upstream' remote"), "{check:?}");
    }

    #[test]
    fn tags_are_counted_and_their_absence_is_only_informational() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        commit_file(root, "a.txt", "x", "chore: seed", 1_900_000_000);

        assert_eq!(
            find(&repo_section(root, None), "tags present").status,
            Status::Info
        );

        git(root, &["tag", "v1.0.0"]);
        assert_eq!(
            detail(find(&repo_section(root, None), "tags present")),
            "1 local tag"
        );

        git(root, &["tag", "v1.1.0"]);
        assert_eq!(
            detail(find(&repo_section(root, None), "tags present")),
            "2 local tags"
        );
    }

    #[test]
    fn no_config_file_warns_that_versions_are_auto_detected() {
        let dir = tempfile::tempdir().unwrap();
        let section = checks::config_section(None, None, &[], dir.path());
        let check = find(&section, "config file");
        assert_eq!(check.status, Status::Warn);
        assert!(detail(check).contains("ferrflow init"));
    }

    #[test]
    fn a_config_that_fails_to_load_is_an_error_carrying_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        let discovered = vec![dir.path().join("ferrflow.json")];
        let section = checks::config_section(
            None,
            Some("expected value at line 1"),
            &discovered,
            dir.path(),
        );

        assert_eq!(find(&section, "config file").status, Status::Ok);
        let parses = find(&section, "config parses");
        assert_eq!(parses.status, Status::Error);
        assert_eq!(detail(parses), "expected value at line 1");
        assert_eq!(section.checks.len(), 2);
    }

    #[test]
    fn versioning_is_skipped_without_a_parsed_config() {
        let dir = tempfile::tempdir().unwrap();
        let section = checks::versioning_section(None, dir.path());
        assert_eq!(section.checks.len(), 1);
        assert!(detail(&section.checks[0]).contains("skipped"));
    }

    #[test]
    fn versioning_reports_each_package_current_version_and_the_declared_strategy() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "api/Cargo.toml",
            "[package]\nname = \"api\"\nversion = \"2.3.4\"\n",
        );
        let config = config_from(
            root,
            r#"{"workspace":{"versioning":"calver"},"package":[
                {"name":"api","path":"api","versionedFiles":[{"path":"api/Cargo.toml","format":"toml"}]},
                {"name":"web","path":"web","versionedFiles":[{"path":"web/package.json","format":"json"}]}
            ]}"#,
        );

        let section = checks::versioning_section(Some(&config), root);

        assert_eq!(detail(find(&section, "strategy")), "declared: calver");
        assert_eq!(detail(find(&section, "api")), "v2.3.4");
        assert_eq!(
            detail(find(&section, "web")),
            "vunknown",
            "a missing version file must not abort the report"
        );
    }

    #[test]
    fn an_undeclared_strategy_is_reported_as_auto_detected() {
        let dir = tempfile::tempdir().unwrap();
        let config = config_from(
            dir.path(),
            r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#,
        );
        let section = checks::versioning_section(Some(&config), dir.path());
        assert!(detail(find(&section, "strategy")).starts_with("auto-detected"));
    }

    #[test]
    fn the_forge_is_read_from_the_remote_url() {
        let (dir, _repo) = init_repo();
        git(
            dir.path(),
            &["remote", "add", "origin", "https://github.com/acme/app.git"],
        );
        let repo = open_repo(dir.path()).unwrap();
        let section = checks::forge_section(Some(&repo), None, false);
        let check = find(&section, "forge");
        assert_eq!(check.status, Status::Ok);
        assert_eq!(detail(check), "GitHub (from remote URL)");
    }

    #[test]
    fn a_configured_forge_counts_even_without_a_remote() {
        let (dir, repo) = init_repo();
        let config = config_from(
            dir.path(),
            r#"{"workspace":{"forge":"gitlab"},"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#,
        );
        let section = checks::forge_section(Some(&repo), Some(&config), true);

        assert_eq!(detail(find(&section, "forge")), "GitLab (from config)");
        let reachable = find(&section, "forge reachable");
        assert_eq!(reachable.status, Status::Info);
        assert!(detail(reachable).contains("not implemented for GitLab"));
    }

    #[test]
    fn an_unknown_forge_warns_and_has_nothing_to_reach() {
        let (_dir, repo) = init_repo();
        let section = checks::forge_section(Some(&repo), None, true);
        assert_eq!(find(&section, "forge").status, Status::Warn);
        assert_eq!(
            detail(find(&section, "forge reachable")),
            "no forge to reach"
        );
    }

    #[test]
    fn the_online_check_only_runs_when_asked() {
        let (_dir, repo) = init_repo();
        let section = checks::forge_section(Some(&repo), None, false);
        assert!(section.checks.iter().all(|c| c.name != "forge reachable"));
    }

    #[test]
    fn gitlab_and_forgejo_pipelines_are_recognised() {
        let gitlab = tempfile::tempdir().unwrap();
        write(gitlab.path(), ".gitlab-ci.yml", "stages: []\n");
        assert_eq!(
            detail(find(&checks::ci_section(gitlab.path()), "workflows")),
            ".gitlab-ci.yml"
        );

        let forgejo = tempfile::tempdir().unwrap();
        write(forgejo.path(), ".forgejo/workflows/ci.yml", "on: push\n");
        assert_eq!(
            detail(find(&checks::ci_section(forgejo.path()), "workflows")),
            ".forgejo/workflows/"
        );

        let none = tempfile::tempdir().unwrap();
        assert_eq!(
            find(&checks::ci_section(none.path()), "workflows").status,
            Status::Info
        );
    }

    #[test]
    fn a_quoted_legacy_action_reference_is_found() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".github/workflows/release.yaml",
            "steps:\n  - uses: \"FerrFlow-Org/ferrflow@v3\"\n",
        );
        let section = checks::ci_section(dir.path());
        let action = find(&section, "FerrFlow action");
        assert_eq!(action.status, Status::Ok);
        assert_eq!(detail(action), "FerrFlow-Org/ferrflow@v3");
    }

    #[test]
    fn only_yaml_files_are_scanned_for_the_action() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".github/workflows/README.md",
            "uses: FerrLabs/FerrFlow@v4\n",
        );
        write(
            dir.path(),
            ".github/workflows/ci.yml",
            "steps:\n  - uses: actions/checkout@v4\n",
        );
        let section = checks::ci_section(dir.path());
        assert_eq!(find(&section, "FerrFlow action").status, Status::Info);
    }

    #[test]
    fn doctor_run_from_a_subdirectory_inspects_the_repository_root() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
        );
        write(
            root,
            ".ferrflow",
            r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#,
        );
        write(root, ".github/workflows/ci.yml", "on: push\n");
        std::fs::create_dir_all(root.join("src/deep")).unwrap();

        let mut report = None;
        with_cwd(&root.join("src/deep"), || {
            report = Some(crate::doctor::build_report(None, false));
            Ok(())
        })
        .unwrap();
        let report = report.unwrap().unwrap();

        let section = |title: &str| {
            report
                .sections
                .iter()
                .find(|s| s.title == title)
                .unwrap_or_else(|| panic!("missing section {title}"))
        };
        assert_eq!(detail(find(section("Config"), "config file")), ".ferrflow");
        assert_eq!(detail(find(section("Versioning"), "app")), "v1.0.0");
        assert_eq!(find(section("CI"), "workflows").status, Status::Ok);
    }

    #[test]
    fn doctor_outside_a_repository_fails_with_exit_code_two() {
        let dir = tempfile::tempdir().unwrap();

        let mut report = None;
        with_cwd(dir.path(), || {
            report = Some(crate::doctor::build_report(None, false));
            Ok(())
        })
        .unwrap();
        let report = report.unwrap().unwrap();

        assert_eq!(report.exit_code, 2);
        assert_eq!(report.status, Status::Error);
        assert_eq!(report.sections[0].checks[0].name, "git repository");
        assert_eq!(report.sections[0].checks[0].status, Status::Error);
    }

    #[test]
    fn an_explicit_config_path_that_does_not_exist_is_reported_not_fatal() {
        let (dir, _repo) = init_repo();

        let mut report = None;
        with_cwd(dir.path(), || {
            report = Some(crate::doctor::build_report(
                Some(Path::new("missing.json")),
                false,
            ));
            Ok(())
        })
        .unwrap();
        let report = report.unwrap().unwrap();

        let config = report
            .sections
            .iter()
            .find(|s| s.title == "Config")
            .unwrap();
        assert_eq!(find(config, "config parses").status, Status::Error);
        assert_eq!(report.exit_code, 2);
    }
}
