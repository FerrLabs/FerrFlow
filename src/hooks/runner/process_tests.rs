use super::*;
use crate::hooks::HookCommit;
use std::path::Path;

fn ctx() -> HookContext {
    HookContext {
        package: "api".into(),
        old_version: "1.0.0".into(),
        new_version: "1.1.0-beta.1".into(),
        bump_type: "minor".into(),
        tag: "api@v1.1.0-beta.1".into(),
        dry_run: false,
        package_path: "packages/api".into(),
        channel: Some("beta".into()),
        error_code: None,
        monorepo: true,
        is_prerelease: true,
        changelog: String::new(),
        commits: vec![HookCommit::from_commit(
            "abc1234",
            "feat(api): add search",
            &Default::default(),
        )],
        bumped_files: Vec::new(),
        all_packages: Vec::new(),
        release_url: None,
    }
}

fn write_var(var: &str, file: &str) -> String {
    if cfg!(windows) {
        format!("echo %{var}%> {file}")
    } else {
        format!("printf '%s' \"${var}\" > {file}")
    }
}

fn touch(file: &str) -> String {
    if cfg!(windows) {
        format!("echo ran> {file}")
    } else {
        format!("echo ran > {file}")
    }
}

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file))
        .unwrap_or_else(|e| panic!("{file} was not written: {e}"))
        .trim()
        .to_string()
}

fn policy(on_failure: OnFailure) -> HookPolicy {
    HookPolicy {
        on_failure,
        timeout: crate::hooks::resolve::DEFAULT_HOOK_TIMEOUT,
    }
}

fn run(command: &str, dir: &Path, on_failure: OnFailure, dry_run: bool) -> Result<()> {
    run_hook(
        HookPoint::PreBump,
        command,
        &ctx(),
        policy(on_failure),
        dry_run,
        false,
        dir,
    )
}

#[test]
fn a_dry_run_does_not_execute_the_hook() {
    let dir = tempfile::tempdir().unwrap();
    run(&touch("ran.txt"), dir.path(), OnFailure::Abort, true).unwrap();
    assert!(!dir.path().join("ran.txt").exists());
}

#[test]
fn the_hook_runs_in_the_given_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    run(&touch("ran.txt"), dir.path(), OnFailure::Abort, false).unwrap();
    assert_eq!(read(dir.path(), "ran.txt"), "ran");
}

#[test]
fn release_context_reaches_the_hook_environment() {
    let dir = tempfile::tempdir().unwrap();
    for (var, expected) in [
        ("FERRFLOW_PACKAGE", "api"),
        ("FERRFLOW_OLD_VERSION", "1.0.0"),
        ("FERRFLOW_NEW_VERSION", "1.1.0-beta.1"),
        ("FERRFLOW_BUMP_TYPE", "minor"),
        ("FERRFLOW_TAG", "api@v1.1.0-beta.1"),
        ("FERRFLOW_CHANNEL", "beta"),
        ("FERRFLOW_IS_PRERELEASE", "true"),
        ("FERRFLOW_MONOREPO", "true"),
        ("FERRFLOW_DRY_RUN", "false"),
        ("FERRFLOW_PACKAGE_PATH", "packages/api"),
    ] {
        run(
            &write_var(var, "env.txt"),
            dir.path(),
            OnFailure::Abort,
            false,
        )
        .unwrap();
        assert_eq!(read(dir.path(), "env.txt"), expected, "{var}");
    }
}

#[test]
fn a_release_summary_hook_sees_every_tag_of_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let summary = HookContext::release_summary(
        dir.path(),
        &["api@v1.1.0".to_string(), "web@v2.0.0".to_string()],
        false,
        true,
    );
    run_hook(
        HookPoint::OnSuccess,
        &write_var("FERRFLOW_TAG", "tags.txt"),
        &summary,
        policy(OnFailure::Abort),
        false,
        false,
        dir.path(),
    )
    .unwrap();
    assert_eq!(read(dir.path(), "tags.txt"), "api@v1.1.0,web@v2.0.0");
}

#[test]
fn commits_reach_the_hook_as_parseable_json() {
    let dir = tempfile::tempdir().unwrap();
    run(
        &write_var("FERRFLOW_COMMITS_JSON", "commits.json"),
        dir.path(),
        OnFailure::Abort,
        false,
    )
    .unwrap();

    let commits: serde_json::Value = serde_json::from_str(&read(dir.path(), "commits.json"))
        .expect("FERRFLOW_COMMITS_JSON must be valid JSON");
    assert_eq!(commits[0]["hash"], "abc1234");
    assert_eq!(commits[0]["type"], "feat");
    assert_eq!(commits[0]["scope"], "api");
}

#[test]
fn a_failing_hook_aborts_with_its_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let err = run("exit 3", dir.path(), OnFailure::Abort, false).unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("exit 3"), "{msg}");
    assert!(msg.contains("pre_bump"), "{msg}");
    assert_eq!(
        crate::error_code::code_from_error(&err),
        Some(error_code::HOOK_FAILED.to_string())
    );
}

#[test]
fn a_failing_hook_with_continue_does_not_stop_the_release() {
    let dir = tempfile::tempdir().unwrap();
    run("exit 3", dir.path(), OnFailure::Continue, false).unwrap();
}

#[test]
fn a_failing_hook_aborts_in_verbose_mode_too() {
    let dir = tempfile::tempdir().unwrap();
    let err = run_hook(
        HookPoint::PostTag,
        "exit 5",
        &ctx(),
        policy(OnFailure::Abort),
        false,
        true,
        dir.path(),
    )
    .unwrap_err();
    assert!(format!("{err:#}").contains("exit 5"), "{err:#}");
}

#[test]
fn build_metadata_is_the_trimmed_stdout_of_the_command() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        capture_build_metadata("echo build-7.abc", dir.path()).unwrap(),
        "build-7.abc"
    );
}

#[test]
fn build_metadata_runs_in_the_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("meta.txt"), "sha-42\n").unwrap();
    let command = if cfg!(windows) {
        "type meta.txt"
    } else {
        "cat meta.txt"
    };
    assert_eq!(
        capture_build_metadata(command, dir.path()).unwrap(),
        "sha-42"
    );
}

#[test]
fn build_metadata_rejects_a_failing_command() {
    let dir = tempfile::tempdir().unwrap();
    let err = capture_build_metadata("exit 2", dir.path()).unwrap_err();
    assert!(format!("{err:#}").contains("exited with 2"), "{err:#}");
}

#[test]
fn build_metadata_rejects_empty_output() {
    let dir = tempfile::tempdir().unwrap();
    let err = capture_build_metadata("exit 0", dir.path()).unwrap_err();
    assert!(format!("{err:#}").contains("printed nothing"), "{err:#}");
}

#[test]
fn build_metadata_rejects_output_semver_would_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let err = capture_build_metadata("echo not valid", dir.path()).unwrap_err();
    assert!(
        format!("{err:#}").contains("dot-separated alphanumerics"),
        "{err:#}"
    );
}

fn sleep_for(seconds: u32) -> String {
    if cfg!(windows) {
        format!("ping -n {} 127.0.0.1 >NUL", seconds + 1)
    } else {
        format!("sleep {seconds}")
    }
}

fn bounded(on_failure: OnFailure, seconds: u64) -> HookPolicy {
    HookPolicy {
        on_failure,
        timeout: std::time::Duration::from_secs(seconds),
    }
}

fn run_with(command: &str, dir: &Path, policy: HookPolicy) -> Result<()> {
    run_hook(
        HookPoint::PreBump,
        command,
        &ctx(),
        policy,
        false,
        false,
        dir,
    )
}

#[test]
fn a_hook_past_its_timeout_is_killed_and_reported() {
    let dir = tempfile::tempdir().unwrap();
    let started = std::time::Instant::now();

    let err = run_with(&sleep_for(30), dir.path(), bounded(OnFailure::Abort, 1)).unwrap_err();

    assert!(started.elapsed() < std::time::Duration::from_secs(15));
    let msg = format!("{err:#}");
    assert!(msg.contains("timed out after 1s"), "{msg}");
    assert_eq!(
        crate::error_code::code_from_error(&err),
        Some(error_code::HOOK_FAILED.to_string())
    );
}

#[test]
fn a_timed_out_hook_with_continue_does_not_stop_the_release() {
    let dir = tempfile::tempdir().unwrap();
    run_with(&sleep_for(30), dir.path(), bounded(OnFailure::Continue, 1)).unwrap();
}

#[test]
fn a_hook_reading_stdin_gets_end_of_file_instead_of_blocking() {
    let dir = tempfile::tempdir().unwrap();
    let command = if cfg!(windows) {
        "sort > stdin.txt"
    } else {
        "cat > stdin.txt"
    };

    run_with(command, dir.path(), bounded(OnFailure::Abort, 10)).unwrap();

    assert_eq!(read(dir.path(), "stdin.txt"), "");
}

#[cfg(unix)]
#[test]
fn a_timeout_also_kills_what_the_hook_started_in_the_background() {
    let dir = tempfile::tempdir().unwrap();

    run_with(
        "sleep 30 & echo $! > child.pid; sleep 30",
        dir.path(),
        bounded(OnFailure::Continue, 1),
    )
    .unwrap();

    let pid = read(dir.path(), "child.pid");
    std::thread::sleep(std::time::Duration::from_millis(200));
    let alive = std::process::Command::new("kill")
        .args(["-0", &pid])
        .status()
        .unwrap()
        .success();
    assert!(!alive, "background child {pid} outlived the hook");
}
