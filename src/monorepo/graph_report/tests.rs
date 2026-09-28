use super::*;

fn pkg(name: &str, deps: &[&str]) -> PackageConfig {
    PackageConfig {
        build_metadata: None,
        version_source: None,
        name: name.to_string(),
        path: name.to_string(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: vec![],
        depends_on: deps
            .iter()
            .map(|s| crate::config::Dependency::Name(s.to_string()))
            .collect(),
        update_lockfiles: None,
        versioning: None,
        tag_template: None,
        version_template: None,
        floating_tags: None,
        latest_tag: None,
        hooks: None,
        publishers: vec![],
    }
}

fn find<'a>(report: &'a GraphReport, name: &str) -> &'a GraphPackage {
    report
        .packages
        .iter()
        .find(|p| p.name == name)
        .expect("package in report")
}

#[test]
fn dependents_are_the_reverse_of_depends_on() {
    let report = build_report(&[
        pkg("core", &[]),
        pkg("api", &["core"]),
        pkg("cli", &["api", "core"]),
    ]);

    assert_eq!(find(&report, "core").depends_on, Vec::<String>::new());
    assert_eq!(find(&report, "core").dependents, vec!["api", "cli"]);
    assert_eq!(find(&report, "cli").depends_on, vec!["api", "core"]);
    assert_eq!(find(&report, "cli").dependents, Vec::<String>::new());
}

#[test]
fn release_order_puts_dependencies_before_dependents() {
    let report = build_report(&[
        pkg("cli", &["api"]),
        pkg("api", &["core"]),
        pkg("core", &[]),
    ]);

    assert_eq!(report.cycle, None);
    let order = &report.release_order;
    let at = |name: &str| order.iter().position(|n| n == name).expect("in order");
    assert!(at("core") < at("api"), "{order:?}");
    assert!(at("api") < at("cli"), "{order:?}");
}

#[test]
fn a_cycle_is_reported_with_a_closed_path_and_no_order() {
    let report = build_report(&[pkg("a", &["b"]), pkg("b", &["a"])]);

    let cycle = report.cycle.expect("cycle detected");
    assert_eq!(cycle.first(), cycle.last(), "path should close: {cycle:?}");
    assert!(cycle.contains(&"a".to_string()) && cycle.contains(&"b".to_string()));
    assert!(report.release_order.is_empty());
}

#[test]
fn a_dependency_outside_the_workspace_is_left_out_rather_than_invented() {
    let report = build_report(&[pkg("api", &["serde", "core"]), pkg("core", &[])]);

    assert_eq!(find(&report, "api").depends_on, vec!["core"]);
    assert_eq!(report.cycle, None);
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

fn text(report: &GraphReport) -> String {
    let mut buf = Vec::new();
    write_text(&mut buf, report).unwrap();
    strip_ansi(&String::from_utf8(buf).unwrap())
}

#[test]
fn text_lists_both_directions_and_a_dash_for_none() {
    let out = text(&build_report(&[
        pkg("core", &[]),
        pkg("api", &["core"]),
        pkg("cli", &["api", "core"]),
    ]));
    assert!(
        out.contains("  core\n    depends on  —\n    required by api, cli\n"),
        "{out}"
    );
    assert!(
        out.contains("  cli\n    depends on  api, core\n    required by —\n"),
        "{out}"
    );
    assert!(out.contains("release order core → api → cli"), "{out}");
    assert!(!out.contains("cycle"), "{out}");
}

#[test]
fn text_shows_the_closed_cycle_instead_of_an_order() {
    let out = text(&build_report(&[pkg("a", &["b"]), pkg("b", &["a"])]));
    let line = out
        .lines()
        .find(|l| l.contains("cycle detected:"))
        .unwrap_or_else(|| panic!("{out}"));
    let path: Vec<&str> = line
        .split("cycle detected: ")
        .nth(1)
        .unwrap()
        .split(" → ")
        .collect();
    assert_eq!(path.len(), 3, "{line}");
    assert_eq!(path.first(), path.last(), "{line}");
    assert!(out.contains("no release order exists while this cycle stands"));
    assert!(!out.contains("release order a"), "{out}");
}

#[test]
fn the_impact_bump_accepts_only_real_bump_levels() {
    assert_eq!(parse_bump("major").unwrap(), BumpType::Major);
    assert_eq!(parse_bump("minor").unwrap(), BumpType::Minor);
    assert_eq!(parse_bump("patch").unwrap(), BumpType::Patch);
    let err = parse_bump("none").unwrap_err().to_string();
    assert!(err.contains("\"none\""), "{err}");
    assert!(parse_bump("Major").is_err());
}

#[test]
fn json_carries_packages_order_and_a_null_cycle() {
    let json =
        serde_json::to_value(build_report(&[pkg("core", &[]), pkg("api", &["core"])])).unwrap();
    assert_eq!(json["release_order"], serde_json::json!(["core", "api"]));
    assert!(json["cycle"].is_null());
    assert_eq!(
        json["packages"][0]["dependents"],
        serde_json::json!(["api"])
    );
}

mod run_in_repo {
    use super::*;
    use crate::test_utils::{commit_file, init_repo, with_cwd};

    fn repo_with(config: &str) -> tempfile::TempDir {
        let (dir, _repo) = init_repo();
        std::fs::write(dir.path().join("ferrflow.json"), config).unwrap();
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_920_000_000);
        dir
    }

    #[test]
    fn run_fails_only_when_the_graph_has_a_cycle() {
        let acyclic = repo_with(
            r#"{ "package": [
                { "name": "core", "path": "core" },
                { "name": "api", "path": "api", "dependsOn": ["core"] }
            ] }"#,
        );
        with_cwd(acyclic.path(), || run(None, false, None, "patch")).unwrap();
        with_cwd(acyclic.path(), || run(None, true, None, "patch")).unwrap();

        let cyclic = repo_with(
            r#"{ "package": [
                { "name": "a", "path": "a", "dependsOn": ["b"] },
                { "name": "b", "path": "b", "dependsOn": ["a"] }
            ] }"#,
        );
        let err = with_cwd(cyclic.path(), || run(None, true, None, "patch")).unwrap_err();
        assert!(err.to_string().contains("dependency cycle"), "{err}");
    }

    #[test]
    fn an_impact_query_runs_for_a_real_bump_and_rejects_an_unknown_one() {
        let dir = repo_with(r#"{ "package": [{ "name": "core", "path": "core" }] }"#);
        with_cwd(dir.path(), || run(None, true, Some("core"), "minor")).unwrap();
        let err = with_cwd(dir.path(), || run(None, false, Some("core"), "huge")).unwrap_err();
        assert!(err.to_string().contains("unknown bump"), "{err}");
    }
}
