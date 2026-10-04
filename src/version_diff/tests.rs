use super::*;

#[test]
fn parse_spec_finds_range_and_optional_package() {
    let two = vec!["api".to_string(), "v1.0.0..v2.0.0".to_string()];
    assert_eq!(parse_spec(&two).unwrap(), (Some("api"), "v1.0.0..v2.0.0"));

    let one = vec!["v1.0.0..v2.0.0".to_string()];
    assert_eq!(parse_spec(&one).unwrap(), (None, "v1.0.0..v2.0.0"));

    let rev = vec!["v1.0.0..v2.0.0".to_string(), "api".to_string()];
    assert_eq!(parse_spec(&rev).unwrap(), (Some("api"), "v1.0.0..v2.0.0"));
}

#[test]
fn parse_spec_requires_a_range() {
    let no_range = vec!["api".to_string(), "v1.0.0".to_string()];
    assert!(parse_spec(&no_range).is_err());
}

#[test]
fn split_range_rejects_empty_sides() {
    assert_eq!(split_range("v1.0.0..v2.0.0").unwrap(), ("v1.0.0", "v2.0.0"));
    assert!(split_range("..v2.0.0").is_err());
    assert!(split_range("v1.0.0..").is_err());
    assert!(split_range("v1.0.0").is_err());
}

fn scoped_pkg() -> PackageConfig {
    serde_json::from_str(r#"{"name":"api","path":"packages/api","sharedPaths":["proto"]}"#)
        .expect("valid package json")
}

#[test]
fn file_list_is_scoped_to_the_package_in_a_monorepo() {
    let files = vec![
        "packages/api/src/main.rs".to_string(),
        "packages/web/app.ts".to_string(),
        "proto/schema.proto".to_string(),
    ];
    let scoped = scope_files_to_package(&scoped_pkg(), true, &[], files);
    assert_eq!(
        scoped,
        vec![
            "packages/api/src/main.rs".to_string(),
            "proto/schema.proto".to_string()
        ],
        "the sibling package's file must be dropped, the shared path kept"
    );
}

#[test]
fn file_list_is_untouched_in_a_single_package_repo() {
    let files = vec!["anything/at/all.rs".to_string()];
    let scoped = scope_files_to_package(&scoped_pkg(), false, &[], files.clone());
    assert_eq!(scoped, files);
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

fn log(hash: &str, message: &str) -> crate::git::GitLog {
    crate::git::GitLog {
        id: hash.to_string(),
        hash: hash.to_string(),
        message: message.to_string(),
    }
}

fn package(name: &str, path: &str) -> PackageConfig {
    serde_json::from_value(serde_json::json!({ "name": name, "path": path }))
        .expect("valid package json")
}

fn config_with(packages: Vec<PackageConfig>) -> Config {
    Config {
        include: Vec::new(),
        workspace: WorkspaceConfig::default(),
        packages,
    }
}

struct Inputs {
    pkg: PackageConfig,
    commits: Vec<crate::git::GitLog>,
    files: Vec<String>,
    formats: CommitFormats,
}

impl Inputs {
    fn new(commits: Vec<crate::git::GitLog>, files: Vec<String>) -> Self {
        Self {
            pkg: package("api", "packages/api"),
            commits,
            files,
            formats: CommitFormats::default(),
        }
    }

    fn report(&self) -> DiffReport<'_> {
        DiffReport {
            pkg: &self.pkg,
            from: "v1.0.0",
            to: "v2.0.0",
            overall: self
                .commits
                .iter()
                .map(|c| determine_bump(&c.message, &self.formats))
                .max()
                .unwrap_or(BumpType::None),
            commits: &self.commits,
            files: &self.files,
            changelog: "## 2.0.0\n- drop v1\n\n",
            formats: &self.formats,
        }
    }

    fn human(&self) -> String {
        let mut buf = Vec::new();
        write_human(&mut buf, &self.report()).unwrap();
        strip_ansi(&String::from_utf8(buf).unwrap())
    }

    fn json(&self) -> serde_json::Value {
        serde_json::to_value(json_report(&self.report())).unwrap()
    }
}

fn mixed_history() -> Inputs {
    Inputs::new(
        vec![
            log("aaa1111", "feat(api)!: drop the v1 endpoints"),
            log("bbb2222", "fix: handle an empty body"),
            log("ccc3333", "Merge branch topic"),
        ],
        vec!["packages/api/src/lib.rs".to_string()],
    )
}

#[test]
fn json_classifies_each_commit_and_lists_only_breaking_subjects() {
    let json = mixed_history().json();

    assert_eq!(json["package"], "api");
    assert_eq!(json["bump"], "major");
    assert_eq!(
        json["breaking"],
        serde_json::json!(["feat(api)!: drop the v1 endpoints"])
    );

    let commits = json["commits"].as_array().unwrap();
    assert_eq!(commits.len(), 3, "every commit in range is reported");

    assert_eq!(commits[0]["type"], "feat");
    assert_eq!(commits[0]["scope"], "api");
    assert_eq!(commits[0]["breaking"], true);
    assert_eq!(commits[0]["bump"], "major");

    assert_eq!(commits[1]["type"], "fix");
    assert_eq!(commits[1]["breaking"], false);
    assert_eq!(commits[1]["bump"], "patch");
    assert!(
        commits[1].get("scope").is_none(),
        "an unscoped commit must not carry a null scope: {}",
        commits[1]
    );
}

#[test]
fn json_omits_type_and_scope_for_a_non_conventional_commit() {
    let json = mixed_history().json();
    let merge = &json["commits"][2];
    assert_eq!(merge["hash"], "ccc3333");
    assert_eq!(merge["bump"], "none");
    assert!(merge.get("type").is_none(), "{merge}");
    assert!(merge.get("scope").is_none(), "{merge}");
}

#[test]
fn json_reports_no_bump_for_an_empty_range() {
    let json = Inputs::new(Vec::new(), Vec::new()).json();
    assert_eq!(json["bump"], "none");
    assert_eq!(json["commits"], serde_json::json!([]));
    assert_eq!(json["breaking"], serde_json::json!([]));
    assert_eq!(json["files_changed"], serde_json::json!([]));
}

#[test]
fn human_output_heads_with_the_range_and_overall_bump() {
    let out = mixed_history().human();
    assert_eq!(out.lines().next(), Some("api  v1.0.0 → v2.0.0  (major)"));
    assert!(out.contains("Commits (3)"), "{out}");
    assert!(
        out.lines().any(|l| l.contains("patch")
            && l.contains("bbb2222")
            && l.ends_with("  fix: handle an empty body")),
        "each commit line pairs its bump, hash and subject:\n{out}"
    );
}

#[test]
fn human_output_lists_breaking_changes_only_when_there_are_some() {
    let out = mixed_history().human();
    assert!(out.contains("Breaking changes (1)"), "{out}");
    assert!(
        out.contains("  ! feat(api)!: drop the v1 endpoints"),
        "{out}"
    );

    let calm = Inputs::new(
        vec![log("bbb2222", "fix: handle an empty body")],
        Vec::new(),
    )
    .human();
    assert!(!calm.contains("Breaking changes"), "{calm}");
    assert!(calm.starts_with("api  v1.0.0 → v2.0.0  (patch)"), "{calm}");
}

#[test]
fn human_output_marks_an_empty_range_explicitly() {
    let out = Inputs::new(Vec::new(), Vec::new()).human();
    assert!(out.contains("Commits (0)\n  (none)"), "{out}");
    assert!(out.contains("Files changed (0)"), "{out}");
}

#[test]
fn human_output_caps_the_file_list_and_counts_the_rest() {
    let files: Vec<String> = (0..MAX_FILES_SHOWN + 5)
        .map(|i| format!("packages/api/f{i:03}.rs"))
        .collect();
    let out = Inputs::new(Vec::new(), files).human();

    assert!(out.contains(&format!("Files changed ({})", MAX_FILES_SHOWN + 5)));
    assert!(out.contains(&format!("f{:03}.rs", MAX_FILES_SHOWN - 1)));
    assert!(
        !out.contains(&format!("f{:03}.rs", MAX_FILES_SHOWN)),
        "files past the cap must not be printed:\n{out}"
    );
    assert!(out.contains("… and 5 more"), "{out}");
}

#[test]
fn human_output_indents_the_changelog_under_its_heading() {
    let out = mixed_history().human();
    let heading = out.lines().position(|l| l == "Changelog").unwrap();
    let rest: Vec<&str> = out.lines().skip(heading + 1).collect();
    assert_eq!(rest, vec!["  ## 2.0.0", "  - drop v1"]);
}

#[test]
fn a_single_package_resolves_without_being_named() {
    let config = config_with(vec![package("app", ".")]);
    assert_eq!(resolve_package(&config, None).unwrap().name, "app");
}

#[test]
fn package_resolution_errors_carry_their_codes() {
    let code = |err: anyhow::Error| error_code::code_from_error(&err).unwrap();

    let empty = config_with(Vec::new());
    assert_eq!(code(resolve_package(&empty, None).unwrap_err()), "E7001");

    let mono = config_with(vec![package("api", "api"), package("web", "web")]);
    assert_eq!(code(resolve_package(&mono, None).unwrap_err()), "E7004");
    assert_eq!(
        code(resolve_package(&mono, Some("cli")).unwrap_err()),
        "E7002"
    );
    assert_eq!(resolve_package(&mono, Some("web")).unwrap().name, "web");
}

#[test]
fn bad_ranges_carry_the_range_error_code() {
    let err = split_range("v1.0.0..").unwrap_err();
    assert_eq!(error_code::code_from_error(&err).as_deref(), Some("E7003"));
    let err = parse_spec(&["api".to_string()]).unwrap_err();
    assert_eq!(error_code::code_from_error(&err).as_deref(), Some("E7003"));
}

mod git_backed {
    use super::*;
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};

    fn head(root: &Path) -> ObjectId {
        git(root, &["rev-parse", "HEAD"]).trim().parse().unwrap()
    }

    #[test]
    fn an_endpoint_resolves_through_the_package_tag_template() {
        let (dir, repo) = init_repo();
        let root = dir.path();
        commit_file(root, "a.txt", "1", "feat: one", 1_900_000_000);
        git(root, &["tag", "v1.0.0"]);
        let expected = head(root);

        let pkg = package("app", ".");
        let ws = WorkspaceConfig::default();
        for endpoint in ["v1.0.0", "1.0.0"] {
            let (oid, tag) = resolve_endpoint(&repo, &pkg, &ws, false, endpoint).unwrap();
            assert_eq!(tag, "v1.0.0", "endpoint {endpoint}");
            assert_eq!(oid, expected, "endpoint {endpoint}");
        }
    }

    #[test]
    fn a_monorepo_endpoint_finds_the_package_scoped_tag() {
        let (dir, repo) = init_repo();
        let root = dir.path();
        commit_file(root, "a.txt", "1", "feat: one", 1_900_000_100);
        git(root, &["tag", "api@v1.2.0"]);

        let pkg = package("api", "api");
        let ws = WorkspaceConfig::default();
        let (_, tag) = resolve_endpoint(&repo, &pkg, &ws, true, "v1.2.0").unwrap();
        assert_eq!(tag, "api@v1.2.0");
    }

    #[test]
    fn an_unknown_endpoint_names_every_tag_it_tried() {
        let (dir, repo) = init_repo();
        commit_file(dir.path(), "a.txt", "1", "feat: one", 1_900_000_200);

        let pkg = package("api", "api");
        let err =
            resolve_endpoint(&repo, &pkg, &WorkspaceConfig::default(), true, "v9.0.0").unwrap_err();
        assert_eq!(error_code::code_from_error(&err).as_deref(), Some("E7005"));
        let message = format!("{err:#}");
        assert!(message.contains("v9.0.0, api@v9.0.0"), "{message}");
    }

    #[test]
    fn only_commits_touching_the_package_count_in_a_monorepo() {
        let (dir, repo) = init_repo();
        let root = dir.path();
        std::fs::create_dir_all(root.join("api")).unwrap();
        std::fs::create_dir_all(root.join("web")).unwrap();
        commit_file(root, "api/lib.rs", "1", "feat: api", 1_900_000_300);
        let api_commit = head(root);
        commit_file(root, "web/app.ts", "1", "feat: web", 1_900_000_310);
        let web_commit = head(root);

        let pkg = package("api", "api");
        assert!(commit_touches_package(&repo, &pkg, true, &[], api_commit));
        assert!(!commit_touches_package(&repo, &pkg, true, &[], web_commit));
        assert!(
            commit_touches_package(&repo, &pkg, false, &[], web_commit),
            "a single-package repo keeps every commit"
        );
    }

    #[test]
    fn run_diffs_two_tags_and_fails_on_an_unknown_one() {
        let (dir, _repo) = init_repo();
        let root = dir.path().to_path_buf();
        std::fs::write(
            root.join("ferrflow.json"),
            r#"{ "package": [{ "name": "app", "path": "." }] }"#,
        )
        .unwrap();
        commit_file(&root, "a.txt", "1", "chore: seed", 1_900_000_400);
        git(&root, &["tag", "v1.0.0"]);
        commit_file(&root, "b.txt", "1", "feat: more", 1_900_000_410);
        git(&root, &["tag", "v1.1.0"]);

        let spec = |range: &str| vec![range.to_string()];
        with_cwd(&root, || run(&spec("v1.0.0..v1.1.0"), true, None)).unwrap();

        let err = with_cwd(&root, || run(&spec("v1.0.0..v7.0.0"), false, None)).unwrap_err();
        assert_eq!(error_code::code_from_error(&err).as_deref(), Some("E7005"));
    }
}
