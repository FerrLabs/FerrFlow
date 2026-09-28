use super::*;

#[test]
fn breaking_only_disagrees_when_the_commits_asked_for_less_than_major() {
    assert!(disagrees(BumpType::Patch, &ApiVerdict::Breaking));
    assert!(disagrees(BumpType::Minor, &ApiVerdict::Breaking));
    assert!(disagrees(BumpType::None, &ApiVerdict::Breaking));
    assert!(!disagrees(BumpType::Major, &ApiVerdict::Breaking));
}

#[test]
fn a_verdict_other_than_breaking_never_disagrees() {
    for verdict in [
        ApiVerdict::Compatible,
        ApiVerdict::Inconclusive("build failed".into()),
        ApiVerdict::NotChecked("not installed".into()),
    ] {
        assert!(
            !disagrees(BumpType::Patch, &verdict),
            "{verdict:?} should not disagree"
        );
    }
}

#[test]
fn an_absent_analyser_is_not_checked_rather_than_compatible() {
    let dir = tempfile::tempdir().unwrap();
    let verdict = check_rust_api("x", dir.path(), &PathBuf::from("."), "v1.0.0");
    assert!(
        matches!(verdict, ApiVerdict::NotChecked(_)),
        "a package with no Cargo.toml must not read as compatible, got {verdict:?}"
    );
}

#[test]
fn an_unknown_cargo_subcommand_reads_as_not_installed_not_inconclusive() {
    assert!(is_missing_subcommand(
        b"error: no such command: `semver-checks`"
    ));
    assert!(!is_missing_subcommand(
        b"error: failed to build rustdoc JSON"
    ));
}

#[test]
fn the_last_meaningful_line_skips_trailing_blanks() {
    assert_eq!(
        last_meaningful_line(b"first\nerror: boom\n\n  \n"),
        "error: boom"
    );
    assert_eq!(last_meaningful_line(b""), "(no output)");
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_escape = false;
    for ch in text.chars() {
        match (in_escape, ch) {
            (false, '\u{1b}') => in_escape = true,
            (true, 'm') => in_escape = false,
            (true, _) => {}
            (false, c) => out.push(c),
        }
    }
    out
}

fn entry(package: &str, commit_bump: BumpType, api: ApiVerdict) -> PackageReport {
    PackageReport {
        package: package.to_string(),
        baseline: Some(format!("{package}@v1.0.0")),
        commit_bump: format!("{commit_bump:?}").to_lowercase(),
        disagrees: disagrees(commit_bump, &api),
        api,
    }
}

fn text(packages: Vec<PackageReport>) -> String {
    let disagreements = packages.iter().filter(|p| p.disagrees).count();
    let report = Report {
        packages,
        disagreements,
    };
    let mut buf = Vec::new();
    write_text(&mut buf, &report).unwrap();
    strip_ansi(&String::from_utf8(buf).unwrap())
}

#[test]
fn the_verdict_is_flattened_into_the_package_entry_as_verdict_and_detail() {
    let checked =
        serde_json::to_value(entry("api", BumpType::Minor, ApiVerdict::Breaking)).unwrap();
    assert_eq!(checked["verdict"], "breaking");
    assert!(checked.get("detail").is_none(), "{checked}");
    assert_eq!(checked["commit_bump"], "minor");
    assert_eq!(checked["disagrees"], true);

    let skipped = serde_json::to_value(entry(
        "web",
        BumpType::Patch,
        ApiVerdict::NotChecked("no analyser".into()),
    ))
    .unwrap();
    assert_eq!(skipped["verdict"], "not-checked");
    assert_eq!(skipped["detail"], "no analyser");
    assert_eq!(skipped["disagrees"], false);
}

#[test]
fn text_flags_a_package_whose_commits_understate_a_break() {
    let out = text(vec![
        entry("api", BumpType::Minor, ApiVerdict::Breaking),
        entry("core", BumpType::Major, ApiVerdict::Breaking),
    ]);
    assert!(
        out.contains("✗ the API broke but the commits ask for minor, not major"),
        "{out}"
    );
    assert_eq!(
        out.matches("the API broke").count(),
        1,
        "a major commit already covers the break:\n{out}"
    );
    assert!(
        out.contains("1 package(s) where the commits understate the change"),
        "{out}"
    );
}

#[test]
fn text_reports_agreement_and_explains_why_a_package_was_not_checked() {
    let out = text(vec![entry(
        "web",
        BumpType::Patch,
        ApiVerdict::NotChecked("no analyser for this package type".into()),
    )]);
    assert!(out.contains("    baseline     web@v1.0.0"), "{out}");
    assert!(out.contains("    commits say  patch"), "{out}");
    assert!(
        out.contains("    api says     not checked (no analyser for this package type)"),
        "{out}"
    );
    assert!(
        out.contains("no disagreement between commits and API"),
        "{out}"
    );
}

#[test]
fn an_inconclusive_run_shows_the_analysers_last_words() {
    let out = text(vec![entry(
        "api",
        BumpType::Patch,
        ApiVerdict::Inconclusive("error: failed to build rustdoc JSON".into()),
    )]);
    assert!(
        out.contains("inconclusive (error: failed to build rustdoc JSON)"),
        "{out}"
    );
}

mod git_backed {
    use super::*;
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};

    const TWO_PACKAGES: &str = r#"{ "package": [
        { "name": "api", "path": "api" },
        { "name": "web", "path": "web" }
    ] }"#;

    fn workspace(config: &str) -> (tempfile::TempDir, crate::git::Repository, Config) {
        let (dir, repo) = init_repo();
        let root = dir.path();
        std::fs::create_dir_all(root.join("api")).unwrap();
        std::fs::create_dir_all(root.join("web")).unwrap();
        std::fs::write(root.join("ferrflow.json"), config).unwrap();
        commit_file(root, "api/a.txt", "1", "chore: seed", 1_910_000_000);
        let config = Config::load(root, Some(&root.join("ferrflow.json"))).unwrap();
        (dir, repo, config)
    }

    #[test]
    fn a_package_without_a_baseline_is_not_checked_and_asks_for_nothing() {
        let (dir, repo, config) = workspace(TWO_PACKAGES);
        let report = build_report(&repo, dir.path(), &config, None).unwrap();

        assert_eq!(report.packages.len(), 2);
        let api = &report.packages[0];
        assert_eq!(api.baseline, None);
        assert_eq!(api.commit_bump, "none");
        assert_eq!(
            api.api,
            ApiVerdict::NotChecked("no baseline tag to compare against".into())
        );
        assert_eq!(report.disagreements, 0);
    }

    #[test]
    fn the_commit_bump_is_the_highest_since_the_package_baseline() {
        let (dir, repo, config) = workspace(TWO_PACKAGES);
        let root = dir.path();
        git(root, &["tag", "api@v1.0.0"]);
        commit_file(root, "api/b.txt", "1", "fix: api patch", 1_910_000_010);
        commit_file(root, "api/c.txt", "1", "feat: api feature", 1_910_000_020);

        let report = build_report(&repo, root, &config, Some("api")).unwrap();

        assert_eq!(
            report.packages.len(),
            1,
            "only the named package is reported"
        );
        let api = &report.packages[0];
        assert_eq!(api.baseline.as_deref(), Some("api@v1.0.0"));
        assert_eq!(api.commit_bump, "minor");
        assert!(
            matches!(api.api, ApiVerdict::NotChecked(_)),
            "a package with no Cargo.toml must not be compared: {:?}",
            api.api
        );
        assert!(!api.disagrees);
    }

    #[test]
    fn an_unknown_package_is_rejected_with_its_error_code() {
        let (dir, repo, config) = workspace(TWO_PACKAGES);
        let err = build_report(&repo, dir.path(), &config, Some("cli"))
            .err()
            .expect("unknown package");
        assert_eq!(
            crate::error_code::code_from_error(&err).as_deref(),
            Some("E7002")
        );
    }

    #[test]
    fn run_succeeds_when_nothing_disagrees_and_fails_on_an_unknown_package() {
        let (dir, _repo, _config) = workspace(TWO_PACKAGES);
        let root = dir.path();
        with_cwd(root, || run(None, None, true)).unwrap();
        with_cwd(root, || run(None, Some("web"), false)).unwrap();
        let err = with_cwd(root, || run(None, Some("cli"), false)).unwrap_err();
        assert_eq!(
            crate::error_code::code_from_error(&err).as_deref(),
            Some("E7002")
        );
    }
}
