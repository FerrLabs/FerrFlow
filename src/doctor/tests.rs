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
