use super::super::tag_info::TagReport;
use super::super::{
    CommitReport, Decision, DependencyReport, Explanation, FileMatch, TouchReport, Trigger,
};
use super::{lines, truncate};
use crate::monorepo::version_source::VersionSource;

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

fn rendered(x: &Explanation) -> Vec<String> {
    lines(x).iter().map(|l| strip_ansi(l)).collect()
}

fn has(out: &[String], expected: &str) -> bool {
    out.iter().any(|l| l == expected)
}

fn position(out: &[String], prefix: &str) -> usize {
    out.iter()
        .position(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starting with {prefix:?} in {out:#?}"))
}

fn file(path: &str, matched: Option<&str>) -> FileMatch {
    FileMatch {
        path: path.to_string(),
        matched: matched.map(str::to_string),
    }
}

fn commit(hash: &str, subject: &str, bump: &str) -> CommitReport {
    CommitReport {
        hash: hash.to_string(),
        subject: subject.to_string(),
        bump: bump.to_string(),
    }
}

fn explanation() -> Explanation {
    Explanation {
        package: "api".to_string(),
        path: "packages/api".to_string(),
        shared_paths: Vec::new(),
        strategy: "semver".to_string(),
        current_version: "1.2.0".to_string(),
        version_source: None,
        monorepo: true,
        channel: None,
        touch: TouchReport {
            touched: true,
            recovered: false,
            files: vec![
                file("packages/api/src/lib.rs", Some("packages/api/")),
                file("docs/readme.md", None),
            ],
        },
        last_tag: Some(TagReport {
            name: "api@v1.2.0".to_string(),
            commit: Some("abc1234".to_string()),
            age: Some("3 days ago".to_string()),
            reachable_from_head: true,
        }),
        commits: vec![commit("def5678", "feat: add pagination", "minor")],
        dependencies: Vec::new(),
        decision: Decision::Bump {
            bump: "minor".to_string(),
            from: "1.2.0".to_string(),
            to: "1.3.0".to_string(),
            tag: "api@v1.3.0".to_string(),
            prerelease: false,
            triggered_by: Trigger::Commits,
        },
    }
}

#[test]
fn truncate_keeps_short_subjects_intact() {
    assert_eq!(truncate("feat: short", 52), "feat: short");
}

#[test]
fn truncate_cuts_on_characters_not_bytes() {
    let subject = "féat: ".repeat(20);
    let cut = truncate(&subject, 10);
    assert_eq!(cut.chars().count(), 10);
    assert!(cut.ends_with('…'));
}

#[test]
fn the_sections_follow_the_order_of_the_decision_chain() {
    let out = rendered(&explanation());
    let tag = position(&out, "Last tag:");
    let touch = position(&out, "Touch check");
    let commits = position(&out, "Commits considered");
    let decision = position(&out, "Decision:");
    assert!(
        tag < touch && touch < commits && commits < decision,
        "{out:#?}"
    );
    assert_eq!(
        decision,
        out.len() - 1,
        "the decision closes the explanation"
    );
}

#[test]
fn the_header_only_shows_optional_fields_when_they_are_set() {
    let plain = rendered(&explanation());
    assert_eq!(plain[0], "Package: api");
    assert!(has(&plain, "  Version:       1.2.0"), "{plain:#?}");
    assert!(!plain.iter().any(|l| l.contains("Shared paths:")));
    assert!(!plain.iter().any(|l| l.contains("Channel:")));

    let mut x = explanation();
    x.shared_paths = vec!["proto/".to_string(), "schemas/".to_string()];
    x.channel = Some("beta".to_string());
    x.version_source = Some(VersionSource::Tag {
        tag: "api@v1.2.0".to_string(),
    });
    let full = rendered(&x);
    assert!(has(&full, "  Shared paths:  proto/, schemas/"), "{full:#?}");
    assert!(has(&full, "  Channel:       beta"), "{full:#?}");
    assert!(
        has(&full, "  Version:       1.2.0 (from tag api@v1.2.0)"),
        "{full:#?}"
    );
}

#[test]
fn the_last_tag_line_says_whether_head_can_reach_it() {
    let out = rendered(&explanation());
    assert!(
        has(
            &out,
            "Last tag: api@v1.2.0 (abc1234, 3 days ago, reachable from HEAD)"
        ),
        "{out:#?}"
    );

    let mut x = explanation();
    x.last_tag = Some(TagReport {
        name: "api@v1.2.0".to_string(),
        commit: None,
        age: None,
        reachable_from_head: false,
    });
    let orphan = rendered(&x);
    assert!(
        has(&orphan, "Last tag: api@v1.2.0 (NOT reachable from HEAD)"),
        "{orphan:#?}"
    );
}

#[test]
fn a_package_without_tags_is_reported_as_never_released() {
    let mut x = explanation();
    x.last_tag = None;
    let out = rendered(&x);
    assert!(
        out.iter()
            .any(|l| l.starts_with("Last tag: none") && l.contains("never been released")),
        "{out:#?}"
    );
}

#[test]
fn each_changed_file_shows_the_rule_that_matched_it_or_none() {
    let out = rendered(&explanation());
    let matched = out
        .iter()
        .find(|l| l.contains("packages/api/src/lib.rs"))
        .unwrap();
    assert!(matched.trim_start().starts_with('✓'), "{matched}");
    assert!(matched.ends_with("matches packages/api/"), "{matched}");

    let unmatched = out.iter().find(|l| l.contains("docs/readme.md")).unwrap();
    assert!(unmatched.trim_start().starts_with('✗'), "{unmatched}");
    assert!(unmatched.ends_with("no match"), "{unmatched}");

    assert!(has(&out, "Touch check (changed files at HEAD):"));
    assert!(has(&out, "→ touched"));
}

#[test]
fn a_recovered_touch_names_the_setting_that_widened_it() {
    let mut x = explanation();
    x.touch.recovered = true;
    let out = rendered(&x);
    assert!(
        has(&out, "Touch check (changed files since the last tag):"),
        "{out:#?}"
    );
    assert!(
        has(&out, "→ touched (recovered by recoverMissedReleases)"),
        "{out:#?}"
    );
}

#[test]
fn an_untouched_package_skips_the_commit_list() {
    let mut x = explanation();
    x.touch.touched = false;
    x.touch.files.clear();
    x.decision = Decision::Skipped {
        reason: "not-touched".to_string(),
    };
    let out = rendered(&x);
    assert!(has(&out, "  (none)"), "{out:#?}");
    assert!(has(&out, "→ not touched"), "{out:#?}");
    assert!(
        !out.iter().any(|l| l.starts_with("Commits considered")),
        "{out:#?}"
    );
    assert_eq!(
        out.last().map(String::as_str),
        Some("Decision: no release — not-touched")
    );
}

#[test]
fn commits_show_their_bump_and_a_dash_when_they_do_not_bump() {
    let mut x = explanation();
    let long = format!("feat: {}", "x".repeat(80));
    x.commits = vec![
        commit("aaa1111", "chore: tidy", "none"),
        commit("bbb2222", &long, "minor"),
    ];
    let out = rendered(&x);
    assert!(has(&out, "Commits considered (2):"));

    let chore = out.iter().find(|l| l.contains("aaa1111")).unwrap();
    assert!(chore.ends_with('—'), "{chore}");

    let feat = out.iter().find(|l| l.contains("bbb2222")).unwrap();
    assert!(feat.ends_with("minor"), "{feat}");
    assert!(feat.contains(&truncate(&long, 52)), "{feat}");
    assert!(!feat.contains(&long), "long subjects are cut: {feat}");
}

#[test]
fn a_touched_package_with_no_commits_says_so() {
    let mut x = explanation();
    x.commits.clear();
    let out = rendered(&x);
    let heading = position(&out, "Commits considered (0):");
    assert_eq!(out[heading + 1], "  (none)");
}

#[test]
fn dependencies_show_the_upstream_bump_and_what_propagates() {
    let mut x = explanation();
    x.dependencies = vec![
        DependencyReport {
            name: "core".to_string(),
            propagate: "patch".to_string(),
            upstream_bump: Some("major".to_string()),
            resulting_bump: "patch".to_string(),
        },
        DependencyReport {
            name: "proto".to_string(),
            propagate: "same".to_string(),
            upstream_bump: None,
            resulting_bump: "none".to_string(),
        },
    ];
    let out = rendered(&x);
    let heading = position(&out, "Dependencies:");
    assert!(heading < position(&out, "Decision:"));

    let core = out.iter().find(|l| l.contains("core")).unwrap();
    assert!(core.contains("bumping (major)"), "{core}");
    assert!(core.ends_with("propagate: patch → patch"), "{core}");

    let proto = out.iter().find(|l| l.contains("proto")).unwrap();
    assert!(proto.contains("not bumping"), "{proto}");
    assert!(proto.ends_with("propagate: same → none"), "{proto}");
}

#[test]
fn no_dependency_section_without_dependencies() {
    let out = rendered(&explanation());
    assert!(!out.iter().any(|l| l == "Dependencies:"), "{out:#?}");
}

#[test]
fn the_decision_names_what_triggered_the_bump() {
    let own = rendered(&explanation());
    assert_eq!(
        own.last().unwrap(),
        "Decision: minor bump from its own commits — 1.2.0 → 1.3.0, tag api@v1.3.0"
    );

    let mut cascade = explanation();
    cascade.decision = Decision::Bump {
        bump: "patch".to_string(),
        from: "1.2.0".to_string(),
        to: "1.2.1-beta.1".to_string(),
        tag: "api@v1.2.1-beta.1".to_string(),
        prerelease: true,
        triggered_by: Trigger::Dependency,
    };
    assert_eq!(
        rendered(&cascade).last().unwrap(),
        "Decision: patch bump from the dependency cascade (prerelease) — 1.2.0 → 1.2.1-beta.1, tag api@v1.2.1-beta.1"
    );

    let mut forced = explanation();
    forced.decision = Decision::Bump {
        bump: "forced".to_string(),
        from: "1.2.0".to_string(),
        to: "5.0.0".to_string(),
        tag: "api@v5.0.0".to_string(),
        prerelease: false,
        triggered_by: Trigger::Forced,
    };
    assert!(
        rendered(&forced)
            .last()
            .unwrap()
            .starts_with("Decision: forced bump set with --force-version — ")
    );
}
