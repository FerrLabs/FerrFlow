use serde_json::Value;

use super::{check, empty_config, release_json, text};
use crate::config::Config;
use crate::forge::ReleaseResult;
use crate::monorepo::run::context::RunFlags;
use crate::monorepo::run::release_json::{ReleasedPackage, SkippedPackage};
use crate::monorepo::run::state::ReleaseState;
use crate::monorepo::run::summary::PlannedTag;
use crate::monorepo::types::CheckPackage;
use crate::test_utils::{commit_file, git, init_repo};

fn flags() -> RunFlags {
    RunFlags {
        dry_run: false,
        verbose: false,
        json: false,
        release_json: false,
        force: false,
        draft: false,
        shadow: false,
    }
}

fn parse(json: Option<String>) -> Value {
    serde_json::from_str(&json.expect("json output")).expect("valid json")
}

fn released(package: &str, tag: &str) -> ReleasedPackage {
    ReleasedPackage {
        package: package.to_string(),
        previous_version: "1.0.0".to_string(),
        new_version: "1.1.0".to_string(),
        bump_type: "minor".to_string(),
        tag: tag.to_string(),
        commit_count: 2,
        prerelease: false,
        version_source: None,
        forge_release_url: None,
        forge_release_id: None,
    }
}

fn planned(tag: &str, package: &str) -> PlannedTag {
    PlannedTag {
        tag: tag.to_string(),
        message: format!("Release {tag}"),
        body: String::new(),
        package: package.to_string(),
        version: "1.1.0".to_string(),
        commit_count: 2,
        is_prerelease: false,
    }
}

fn released_state() -> ReleaseState {
    ReleaseState {
        released: vec![released("api", "api@v1.1.0"), released("web", "web@v1.1.0")],
        skipped: vec![SkippedPackage {
            package: "cli".to_string(),
            reason: "not touched".to_string(),
        }],
        tags_to_create: vec![planned("api@v1.1.0", "api"), planned("web@v1.1.0", "web")],
        forge_results: vec![
            (
                "web@v1.1.0".to_string(),
                ReleaseResult {
                    id: Some(99),
                    url: Some("https://forge/web".to_string()),
                },
            ),
            (
                "stale@v0.1.0".to_string(),
                ReleaseResult {
                    id: Some(1),
                    url: Some("https://forge/stale".to_string()),
                },
            ),
        ],
        ..Default::default()
    }
}

#[test]
fn an_empty_config_with_release_json_reports_an_empty_release() {
    let out = empty_config(RunFlags {
        release_json: true,
        json: true,
        dry_run: true,
        ..flags()
    })
    .unwrap();

    let v = parse(out.json);
    assert_eq!(v["released"], serde_json::json!([]));
    assert_eq!(v["skipped"], serde_json::json!([]));
    assert_eq!(v["dry_run"], true);
    assert!(out.text_lines.is_empty());
}

#[test]
fn an_empty_config_with_json_reports_no_packages() {
    let out = empty_config(RunFlags {
        json: true,
        ..flags()
    })
    .unwrap();

    assert_eq!(parse(out.json), serde_json::json!({ "packages": [] }));
}

#[test]
fn an_empty_config_in_text_mode_points_at_init() {
    let out = empty_config(flags()).unwrap();

    assert!(out.json.is_none());
    assert_eq!(out.text_lines.len(), 1);
    assert!(out.text_lines[0].contains("ferrflow init"));
}

#[test]
fn check_serialises_the_planned_packages() {
    let state = ReleaseState {
        json_packages: vec![CheckPackage {
            name: "api".to_string(),
            current_version: "1.0.0".to_string(),
            next_version: "1.1.0".to_string(),
            bump_type: "minor".to_string(),
            tag: "api@v1.1.0".to_string(),
            channel: None,
            prerelease: false,
            version_source: None,
            commits: Vec::new(),
        }],
        ..Default::default()
    };

    let v = parse(check(state).unwrap().json);

    let packages = v["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["name"], "api");
    assert_eq!(packages[0]["next_version"], "1.1.0");
    assert!(
        packages[0].get("channel").is_none(),
        "a stable plan omits the channel"
    );
}

#[test]
fn release_json_attaches_each_forge_release_to_its_own_tag() {
    let (dir, repo) = init_repo();
    commit_file(dir.path(), "a.txt", "a", "feat: a", 1_992_000_000);
    let head = git(dir.path(), &["rev-parse", "HEAD"]).trim().to_string();

    let v = parse(
        release_json(&repo, released_state(), "main", false)
            .unwrap()
            .json,
    );

    let released = v["released"].as_array().unwrap();
    assert!(released[0]["forge_release_url"].is_null());
    assert!(released[0]["forge_release_id"].is_null());
    assert_eq!(released[1]["forge_release_url"], "https://forge/web");
    assert_eq!(released[1]["forge_release_id"], 99);
    assert_eq!(
        released.len(),
        2,
        "a result for an unknown tag adds nothing"
    );
    assert_eq!(v["skipped"][0]["package"], "cli");
    assert_eq!(v["skipped"][0]["reason"], "not touched");
    assert_eq!(
        v["git"]["tags_pushed"],
        serde_json::json!(["api@v1.1.0", "web@v1.1.0"])
    );
    assert_eq!(v["git"]["branch"], "main");
    assert_eq!(v["git"]["commit"], head[..7]);
    assert_eq!(v["dry_run"], false);
}

#[test]
fn a_dry_run_release_json_claims_no_pushed_tags() {
    let (dir, repo) = init_repo();
    commit_file(dir.path(), "a.txt", "a", "feat: a", 1_992_100_000);

    let v = parse(
        release_json(&repo, released_state(), "main", true)
            .unwrap()
            .json,
    );

    assert_eq!(v["git"]["tags_pushed"], serde_json::json!([]));
    assert_eq!(v["released"].as_array().unwrap().len(), 2);
}

#[test]
fn text_says_nothing_to_release_unless_verbose() {
    let config = Config::default();

    let quiet = text(ReleaseState::default(), &config, flags());
    let verbose = text(
        ReleaseState::default(),
        &config,
        RunFlags {
            verbose: true,
            ..flags()
        },
    );

    assert!(
        quiet
            .text_lines
            .last()
            .is_some_and(|l| l.contains("Nothing to release.")),
        "{:?}",
        quiet.text_lines
    );
    assert!(
        !verbose
            .text_lines
            .iter()
            .any(|l| l.contains("Nothing to release.")),
        "{:?}",
        verbose.text_lines
    );
}

#[test]
fn text_stays_silent_about_nothing_to_release_once_something_bumped() {
    let state = ReleaseState {
        any_bumped: true,
        ..Default::default()
    };

    let out = text(state, &Config::default(), flags());

    assert!(
        !out.text_lines
            .iter()
            .any(|l| l.contains("Nothing to release."))
    );
}

fn untouched_state() -> ReleaseState {
    ReleaseState {
        untouched_skipped: 2,
        ..Default::default()
    }
}

fn mentions_untouched(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|l| l.contains("2 packages not touched by the last commit"))
}

#[test]
fn the_untouched_hint_only_shows_on_a_dry_run() {
    let config = Config::default();
    let dry = RunFlags {
        dry_run: true,
        ..flags()
    };

    assert!(mentions_untouched(
        &text(untouched_state(), &config, dry).text_lines
    ));
    assert!(!mentions_untouched(
        &text(untouched_state(), &config, flags()).text_lines
    ));
}

#[test]
fn the_untouched_hint_is_dropped_when_missed_releases_are_recovered() {
    let mut config = Config::default();
    config.workspace.recover_missed_releases = true;
    let dry = RunFlags {
        dry_run: true,
        ..flags()
    };

    assert!(!mentions_untouched(
        &text(untouched_state(), &config, dry).text_lines
    ));
}
