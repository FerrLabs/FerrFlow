use super::*;

fn rows() -> Vec<Row> {
    vec![
        Row {
            package: "api".into(),
            current: "1.0.0".into(),
            planned: Some("1.1.0".into()),
            reason: "minor".into(),
        },
        Row {
            package: "core".into(),
            current: "2.3.4".into(),
            planned: Some("2.3.5".into()),
            reason: "patch".into(),
        },
    ]
}

#[test]
fn the_emitted_command_carries_every_decision() {
    let mut forced = BTreeMap::new();
    forced.insert("api".to_string(), "2.0.0".to_string());
    forced.insert("core".to_string(), "3.0.0".to_string());
    let cmd = command_for(&forced, &["web".to_string(), "docs".to_string()]);
    assert_eq!(
        cmd,
        "ferrflow release --force-version api@2.0.0 --force-version core@3.0.0 \
         --exclude web --exclude docs"
    );
}

#[test]
fn no_decisions_emit_no_flags() {
    assert_eq!(command_for(&BTreeMap::new(), &[]), "ferrflow release");
}

#[test]
fn indices_are_one_based_and_reject_anything_outside_the_list() {
    let r = rows();
    assert_eq!(index(&r, "1").as_deref(), Some("api"));
    assert_eq!(index(&r, "2").as_deref(), Some("core"));
    assert_eq!(index(&r, "0"), None, "0 must not wrap to the last row");
    assert_eq!(index(&r, "3"), None);
    assert_eq!(index(&r, "x"), None);
}

#[test]
fn only_the_three_bump_words_parse() {
    assert_eq!(parse_bump("major"), Some(BumpType::Major));
    assert_eq!(parse_bump("minor"), Some(BumpType::Minor));
    assert_eq!(parse_bump("patch"), Some(BumpType::Patch));
    assert_eq!(parse_bump("Major"), None);
    assert_eq!(parse_bump("none"), None);
}

fn config() -> Config {
    let package = |name: &str| -> crate::config::PackageConfig {
        serde_json::from_value(serde_json::json!({ "name": name, "path": name })).unwrap()
    };
    Config {
        include: Vec::new(),
        workspace: crate::config::WorkspaceConfig::default(),
        packages: vec![package("api"), package("core")],
    }
}

fn session(input: &[&str]) -> Result<Option<BTreeMap<String, Override>>> {
    let lines = input.iter().map(|l| Ok(l.to_string()));
    collect_overrides(&rows(), &config(), lines)
}

fn words(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

#[test]
fn control_words_parse_to_their_commands() {
    assert!(matches!(parse_command(&words("")), Command::Skip));
    assert!(matches!(parse_command(&words("   ")), Command::Skip));
    assert!(matches!(parse_command(&words("done")), Command::Done));
    assert!(matches!(parse_command(&words("quit")), Command::Quit));
    assert!(matches!(parse_command(&words("q")), Command::Quit));
}

#[test]
fn set_commands_carry_the_index_and_the_choice() {
    assert!(matches!(
        parse_command(&words("2 major")),
        Command::Set {
            n: "2",
            choice: Some(Override::Bump(BumpType::Major))
        }
    ));
    assert!(matches!(
        parse_command(&words("exclude 1")),
        Command::Set {
            n: "1",
            choice: Some(Override::Excluded)
        }
    ));
    assert!(matches!(
        parse_command(&words("include 1")),
        Command::Set {
            n: "1",
            choice: None
        }
    ));
}

#[test]
fn malformed_commands_are_unknown_rather_than_guessed() {
    for line in [
        "1 huge",
        "api major",
        "-1 major",
        "1 major now",
        "exclude",
        "exclude 1 2",
        "Done",
        "major 1",
    ] {
        assert!(
            matches!(parse_command(&words(line)), Command::Unknown),
            "{line:?} should not parse"
        );
    }
}

#[test]
fn applying_to_an_index_outside_the_list_changes_nothing() {
    let r = rows();
    let mut overrides = BTreeMap::new();
    apply_choice(&r, &mut overrides, "0", Some(Override::Excluded));
    apply_choice(&r, &mut overrides, "3", Some(Override::Excluded));
    assert!(overrides.is_empty());
}

#[test]
fn include_drops_an_earlier_choice_and_a_later_choice_replaces_it() {
    let r = rows();
    let mut overrides = BTreeMap::new();
    apply_choice(&r, &mut overrides, "1", Some(Override::Excluded));
    apply_choice(
        &r,
        &mut overrides,
        "1",
        Some(Override::Bump(BumpType::Major)),
    );
    assert_eq!(overrides.get("api"), Some(&Override::Bump(BumpType::Major)));
    apply_choice(&r, &mut overrides, "1", None);
    assert!(overrides.is_empty());
}

#[test]
fn a_session_collects_choices_until_done_and_ignores_noise() {
    let overrides = session(&[
        "1 major",
        "",
        "bogus",
        "exclude 2",
        "9 patch",
        "done",
        "1 patch",
    ])
    .unwrap()
    .expect("done keeps the choices");
    assert_eq!(
        overrides,
        BTreeMap::from([
            ("api".to_string(), Override::Bump(BumpType::Major)),
            ("core".to_string(), Override::Excluded),
        ]),
        "input after done must not be read"
    );
}

#[test]
fn quitting_discards_every_choice() {
    assert_eq!(session(&["1 major", "quit"]).unwrap(), None);
}

#[test]
fn end_of_input_keeps_the_choices_made_so_far() {
    let overrides = session(&["2 minor"]).unwrap().unwrap();
    assert_eq!(
        overrides,
        BTreeMap::from([("core".to_string(), Override::Bump(BumpType::Minor))])
    );
}

#[test]
fn a_read_error_ends_the_session_with_an_error() {
    let lines = vec![
        Ok("1 major".to_string()),
        Err(std::io::Error::other("tty closed")),
    ];
    let err = collect_overrides(&rows(), &config(), lines.into_iter()).unwrap_err();
    assert!(err.to_string().contains("tty closed"), "{err}");
}

#[test]
fn a_chosen_bump_resolves_from_the_current_version() {
    let r = rows();
    let c = config();
    assert_eq!(
        resolved_version(&c, &r, "api", BumpType::Major).as_deref(),
        Some("2.0.0")
    );
    assert_eq!(
        resolved_version(&c, &r, "core", BumpType::Patch).as_deref(),
        Some("2.3.5")
    );
    assert_eq!(resolved_version(&c, &r, "web", BumpType::Patch), None);
}

#[test]
fn a_package_with_no_version_yet_bumps_from_zero() {
    let mut r = rows();
    r[0].current = String::new();
    assert_eq!(
        resolved_version(&config(), &r, "api", BumpType::Minor).as_deref(),
        Some("0.1.0")
    );
}

#[test]
fn no_override_leaves_the_plan_unchanged() {
    assert_eq!(changed_command(&config(), &rows(), &BTreeMap::new()), None);
}

#[test]
fn overrides_become_forced_versions_and_exclusions() {
    let overrides = BTreeMap::from([
        ("api".to_string(), Override::Bump(BumpType::Major)),
        ("core".to_string(), Override::Excluded),
    ]);
    assert_eq!(
        changed_command(&config(), &rows(), &overrides).as_deref(),
        Some("ferrflow release --force-version api@2.0.0 --exclude core")
    );
}

#[test]
fn the_release_plan_json_still_matches_the_shape_the_session_reads() {
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};

    let (dir, _repo) = init_repo();
    let root = dir.path();
    std::fs::write(
        root.join("ferrflow.json"),
        r#"{ "package": [{ "name": "app", "path": ".",
            "versionedFiles": [{ "path": "Cargo.toml", "format": "toml" }] }] }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    git(root, &["add", "-A"]);
    commit_file(root, "seed.txt", "x", "chore: seed", 1_930_000_000);
    git(root, &["tag", "v1.0.0"]);
    commit_file(root, "feature.txt", "x", "feat: add a thing", 1_930_000_010);

    let mut plan = None;
    with_cwd(root, || {
        plan = Some(super::super::plan_json(None, None)?);
        Ok(())
    })
    .unwrap();
    let plan: PlanJson = serde_json::from_str(&plan.unwrap()).expect("session contract");

    assert_eq!(plan.packages.len(), 1);
    let entry = &plan.packages[0];
    assert_eq!(entry.name, "app");
    assert_eq!(entry.current_version, "1.0.0");
    assert_eq!(entry.next_version, "1.1.0");
    assert_eq!(entry.bump_type, "minor");
}
