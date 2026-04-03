use std::fs;
use std::path::Path;
use std::process::Command;

fn ferrflow_bin() -> String {
    env!("CARGO_BIN_EXE_ferrflow").to_string()
}

fn init_repo(dir: &Path) {
    run_git(dir, &["init", "-b", "main"]);
    run_git(dir, &["config", "user.name", "Test"]);
    run_git(dir, &["config", "user.email", "test@test.com"]);
}

fn run_git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_ferrflow_config(dir: &Path, config: &str) {
    fs::write(dir.join("ferrflow.json"), config).unwrap();
}

/// Strip ANSI escape codes for easier assertion.
fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

fn run_ferrflow_check(dir: &Path) -> String {
    let output = Command::new(ferrflow_bin())
        .args(["check"])
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        panic!(
            "ferrflow check failed (exit {}):\nstdout: {stdout}\nstderr: {stderr}",
            output.status
        );
    }
    strip_ansi(&stdout)
}

/// Set up a monorepo with two packages (alpha and beta), tagged at given versions.
fn setup_monorepo(dir: &Path, alpha_ver: &str, beta_ver: &str) {
    init_repo(dir);

    write_ferrflow_config(
        dir,
        &format!(
            r#"{{
            "package": [
                {{
                    "name": "alpha",
                    "path": "alpha",
                    "versioned_files": [{{"path": "alpha/version.toml", "format": "toml"}}]
                }},
                {{
                    "name": "beta",
                    "path": "beta",
                    "versioned_files": [{{"path": "beta/version.toml", "format": "toml"}}]
                }}
            ]
        }}"#
        ),
    );

    fs::create_dir_all(dir.join("alpha/src")).unwrap();
    fs::create_dir_all(dir.join("beta/src")).unwrap();
    fs::write(
        dir.join("alpha/version.toml"),
        format!("[package]\nname = \"alpha\"\nversion = \"{alpha_ver}\"\n"),
    )
    .unwrap();
    fs::write(
        dir.join("beta/version.toml"),
        format!("[package]\nname = \"beta\"\nversion = \"{beta_ver}\"\n"),
    )
    .unwrap();

    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "chore: initial setup"]);

    run_git(dir, &["tag", &format!("alpha@v{alpha_ver}")]);
    run_git(dir, &["tag", &format!("beta@v{beta_ver}")]);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn monorepo_output_groups_packages_with_blank_line() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    setup_monorepo(root, "0.1.0", "0.1.0");

    // Touch both packages in a single commit (get_changed_files checks HEAD vs HEAD~1)
    fs::write(root.join("alpha/src/lib.rs"), "pub fn a() {}").unwrap();
    fs::write(root.join("beta/src/lib.rs"), "pub fn b() {}").unwrap();
    run_git(root, &["add", "."]);
    run_git(
        root,
        &["commit", "-m", "feat: add features to both packages"],
    );

    let output = run_ferrflow_check(root);

    // Both packages should appear in the output
    assert!(
        output.contains("alpha") && output.contains("0.2.0"),
        "output should contain alpha with bump:\n{output}"
    );
    assert!(
        output.contains("beta") && output.contains("0.2.0"),
        "output should contain beta with bump:\n{output}"
    );

    // Packages should be separated by a blank line (grouped output)
    let alpha_pos = output.find("alpha").unwrap();
    let beta_pos = output.find("beta").unwrap();
    let between = &output[alpha_pos..beta_pos];
    assert!(
        between.contains("\n\n"),
        "packages should be separated by a blank line:\n{output}"
    );
}

#[test]
fn single_package_output_no_double_blank_lines() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    init_repo(root);

    write_ferrflow_config(
        root,
        r#"{
            "package": [
                {
                    "name": "myapp",
                    "path": ".",
                    "versioned_files": [{ "path": "version.toml", "format": "toml" }]
                }
            ]
        }"#,
    );

    fs::write(
        root.join("version.toml"),
        "[package]\nname = \"myapp\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-m", "chore: initial setup"]);
    run_git(root, &["tag", "v0.1.0"]);

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn hello() {}").unwrap();
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-m", "feat: add new feature"]);

    let output = run_ferrflow_check(root);

    assert!(
        output.contains("myapp"),
        "output should contain myapp package:\n{output}"
    );

    // No consecutive empty lines (no separator needed for single package)
    assert!(
        !output.contains("\n\n\n"),
        "single-package output should not have triple newlines:\n{output}"
    );
}

#[test]
fn nothing_to_release_output_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    init_repo(root);

    write_ferrflow_config(
        root,
        r#"{
            "package": [
                {
                    "name": "myapp",
                    "path": ".",
                    "versioned_files": [{ "path": "version.toml", "format": "toml" }]
                }
            ]
        }"#,
    );

    fs::write(
        root.join("version.toml"),
        "[package]\nname = \"myapp\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-m", "chore: initial setup"]);
    run_git(root, &["tag", "v0.1.0"]);

    let output = run_ferrflow_check(root);

    assert!(
        output.contains("Nothing to release"),
        "should show 'Nothing to release' when no new commits:\n{output}"
    );
}

#[test]
fn monorepo_package_order_follows_config() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // alpha first in config, beta second
    setup_monorepo(root, "1.0.0", "1.0.0");

    // Touch both in one commit
    fs::write(root.join("alpha/src/lib.rs"), "pub fn a() {}").unwrap();
    fs::write(root.join("beta/src/lib.rs"), "pub fn b() {}").unwrap();
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-m", "feat: update both"]);

    let output = run_ferrflow_check(root);

    let alpha_pos = output
        .find("alpha")
        .expect("output should mention alpha package");
    let beta_pos = output
        .find("beta")
        .expect("output should mention beta package");

    assert!(
        alpha_pos < beta_pos,
        "alpha should appear before beta (config order):\n{output}"
    );
}

#[test]
fn monorepo_only_touched_package_in_output() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    setup_monorepo(root, "0.1.0", "0.1.0");

    // Only touch alpha, not beta
    fs::write(root.join("alpha/src/lib.rs"), "pub fn a() {}").unwrap();
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-m", "feat(alpha): add feature"]);

    let output = run_ferrflow_check(root);

    // alpha should have a version bump line
    assert!(
        output.contains("alpha") && output.contains("0.2.0"),
        "alpha should show version bump:\n{output}"
    );

    // beta should NOT have a version bump line
    let has_beta_bump = output
        .lines()
        .any(|l| l.contains("beta") && l.contains("0.2.0"));
    assert!(
        !has_beta_bump,
        "beta should not show a version bump:\n{output}"
    );
}
