use crate::config::Config;
use crate::monorepo::run::run_release_logic;
use crate::test_utils::{commit_file, init_repo, with_cwd};
use crate::timing::Timing;
use std::path::Path;

fn write_fixture(dir: &Path, command: &str) {
    std::fs::create_dir_all(dir.join("api")).unwrap();
    std::fs::write(
        dir.join("api/package.json"),
        "{\n  \"name\": \"api\",\n  \"version\": \"1.2.3\",\n  \"private\": true\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".ferrflow"),
        format!(
            r#"{{"workspace": {{"releaseCommitMode": "none", "versionCommand": {command:?}}}, "package": [
                {{"name": "api", "path": "api", "versionedFiles": [{{"path": "api/package.json", "format": "json"}}]}}
            ]}}"#
        ),
    )
    .unwrap();
    commit_file(dir, "seed.txt", "x", "chore: seed", 1_972_000_000);
    commit_file(dir, "api/feature.txt", "y", "feat: change", 1_972_000_001);
}

fn run(dir: &Path, json: bool, run_version_hooks: bool) -> anyhow::Result<serde_json::Value> {
    let config = Config::load(dir, Some(&dir.join(".ferrflow"))).unwrap();
    let mut captured: Option<String> = None;
    with_cwd(dir, || {
        let out = run_release_logic(
            dir,
            &config,
            true,
            false,
            json,
            !json,
            false,
            &[],
            &[],
            None,
            false,
            false,
            run_version_hooks,
            &mut Timing::new(false),
        )?;
        captured = out.json;
        Ok(())
    })?;
    Ok(serde_json::from_str(&captured.expect("json set")).unwrap())
}

#[cfg(unix)]
#[test]
fn release_takes_the_version_and_tag_from_the_command() {
    let (dir, _repo) = init_repo();
    write_fixture(
        dir.path(),
        "echo \"$FERRFLOW_OLD_VERSION\" | awk -F. '{print $1+4\".0.0\"}'",
    );

    let v = run(dir.path(), false, true).unwrap();

    let released = &v["released"][0];
    assert_eq!(released["previous_version"], "1.2.3");
    assert_eq!(released["new_version"], "5.0.0");
    assert_eq!(released["tag"], "v5.0.0");
}

#[cfg(unix)]
#[test]
fn check_reports_the_version_as_undetermined_and_never_runs_the_command() {
    let (dir, _repo) = init_repo();
    let marker = dir.path().join("ran");
    write_fixture(
        dir.path(),
        &format!("touch {} && echo 9.9.9", marker.display()),
    );

    let v = run(dir.path(), true, false).unwrap();

    let package = &v["packages"][0];
    assert!(package["next_version"].is_null());
    assert!(package["tag"].is_null());
    assert_eq!(package["current_version"], "1.2.3");
    assert!(!marker.exists(), "check executed the versionCommand");
}

#[cfg(unix)]
#[test]
fn a_version_that_goes_backwards_fails_the_release() {
    let (dir, _repo) = init_repo();
    write_fixture(dir.path(), "echo 1.0.0");

    let err = run(dir.path(), false, true).unwrap_err();

    assert!(
        format!("{err:#}").contains("does not go past 1.2.3"),
        "{err:#}"
    );
}

#[cfg(unix)]
#[test]
fn a_failing_command_fails_the_release() {
    let (dir, _repo) = init_repo();
    write_fixture(dir.path(), "exit 3");

    let err = run(dir.path(), false, true).unwrap_err();

    assert!(format!("{err:#}").contains("exited with 3"), "{err:#}");
}

#[cfg(unix)]
#[test]
fn the_command_returning_the_current_version_releases_nothing() {
    let (dir, _repo) = init_repo();
    write_fixture(dir.path(), "echo 1.2.3");

    let v = run(dir.path(), false, true).unwrap();

    assert!(v["released"].as_array().unwrap().is_empty());
    assert_eq!(v["skipped"][0]["reason"], "version unchanged");
}
