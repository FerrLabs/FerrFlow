use super::ReleaseState;
use crate::monorepo::run::context::RunFlags;
use crate::monorepo::run::plan::SkipReason;
use crate::monorepo::run::summary::PlannedTag;

fn flags(json: bool, release_json: bool) -> RunFlags {
    RunFlags {
        dry_run: false,
        verbose: false,
        json,
        release_json,
        force: false,
        draft: false,
        shadow: false,
    }
}

fn text_flags() -> RunFlags {
    flags(false, false)
}

#[test]
fn only_untouched_skips_count_towards_the_untouched_hint() {
    let mut state = ReleaseState::default();

    state.record_skip("api", SkipReason::NotTouched, text_flags());
    state.record_skip("web", SkipReason::NotTouched, text_flags());
    state.record_skip("cli", SkipReason::NoNewCommits, text_flags());
    state.record_skip("docs", SkipReason::Excluded, text_flags());

    assert_eq!(state.untouched_skipped, 2);
}

#[test]
fn skips_worth_reading_are_printed_and_routine_ones_are_not() {
    let mut state = ReleaseState::default();

    state.record_skip("api", SkipReason::NotTouched, text_flags());
    state.record_skip("web", SkipReason::NoNewCommits, text_flags());
    state.record_skip("docs", SkipReason::Excluded, text_flags());
    state.record_skip("cli", SkipReason::NoReleasableCommits, text_flags());
    state.record_skip(
        "sdk",
        SkipReason::VersionUnchanged {
            version: "2.3.4".to_string(),
        },
        text_flags(),
    );

    assert_eq!(state.shared_outputs.len(), 2, "{:?}", state.shared_outputs);
    assert!(state.shared_outputs[0].contains("cli"));
    assert!(state.shared_outputs[0].contains("no releasable commits"));
    assert!(state.shared_outputs[1].contains("sdk"));
    assert!(state.shared_outputs[1].contains("stays at 2.3.4"));
}

#[test]
fn machine_output_suppresses_skip_lines() {
    for f in [flags(true, false), flags(false, true)] {
        let mut state = ReleaseState::default();
        state.record_skip("cli", SkipReason::NoReleasableCommits, f);
        assert!(
            state.shared_outputs.is_empty(),
            "{:?}",
            state.shared_outputs
        );
    }
}

#[test]
fn release_json_records_every_skip_with_its_label() {
    let mut state = ReleaseState::default();

    state.record_skip("api", SkipReason::NotTouched, flags(false, true));
    state.record_skip(
        "sdk",
        SkipReason::VersionUnchanged {
            version: "2.3.4".to_string(),
        },
        flags(false, true),
    );

    let recorded: Vec<(&str, &str)> = state
        .skipped
        .iter()
        .map(|s| (s.package.as_str(), s.reason.as_str()))
        .collect();
    assert_eq!(
        recorded,
        vec![("api", "not touched"), ("sdk", "version unchanged")]
    );
}

#[test]
fn skips_are_not_recorded_for_json_when_release_json_is_off() {
    let mut state = ReleaseState::default();

    state.record_skip("api", SkipReason::NotTouched, flags(true, false));

    assert!(state.skipped.is_empty());
}

#[test]
fn a_package_line_lands_under_that_package_only() {
    let mut state = ReleaseState::default();
    state
        .pkg_outputs
        .push(("api".to_string(), vec!["api header".to_string()]));
    state
        .pkg_outputs
        .push(("web".to_string(), vec!["web header".to_string()]));

    state.push_package_line("api", "  ✓ Updated api/package.json".to_string());
    state.push_package_line("ghost", "  ✓ Updated nothing".to_string());

    assert_eq!(
        state.pkg_outputs,
        vec![
            (
                "api".to_string(),
                vec![
                    "api header".to_string(),
                    "  ✓ Updated api/package.json".to_string()
                ]
            ),
            ("web".to_string(), vec!["web header".to_string()]),
        ]
    );
}

#[test]
fn tracked_files_feed_both_the_grouped_and_the_per_package_commit() {
    let mut state = ReleaseState::default();

    state.track_file("api", "api/package.json");
    state.track_file("web", "web/package.json");
    state.track_file("api", "api/CHANGELOG.md");

    assert_eq!(
        state.files_to_commit,
        vec!["api/package.json", "web/package.json", "api/CHANGELOG.md"]
    );
    assert_eq!(
        state.files_per_package["api"],
        vec!["api/package.json", "api/CHANGELOG.md"]
    );
    assert_eq!(state.files_per_package["web"], vec!["web/package.json"]);
}

#[test]
fn finalize_tags_become_the_release_and_are_reported_as_finalize_bumps() {
    let mut state = ReleaseState::default();
    let tag = PlannedTag {
        tag: "api@v2.0.0-rc.1".to_string(),
        message: "Release api@v2.0.0-rc.1".to_string(),
        body: String::new(),
        package: "api".to_string(),
        version: "2.0.0-rc.1".to_string(),
        commit_count: 4,
        is_prerelease: true,
    };

    state.record_finalize_tags(vec![tag]);

    assert!(state.any_bumped);
    assert_eq!(state.tags_to_create.len(), 1);
    assert_eq!(state.tags_to_create[0].tag, "api@v2.0.0-rc.1");
    let released = &state.released[0];
    assert_eq!(released.package, "api");
    assert_eq!(released.bump_type, "finalize");
    assert_eq!(released.new_version, "2.0.0-rc.1");
    assert_eq!(released.commit_count, 4);
    assert!(released.prerelease);
    assert_eq!(state.pkg_outputs[0].0, "api");
    assert!(state.pkg_outputs[0].1[0].contains("release merged, tagging now"));
}
