use std::path::PathBuf;

use super::{PublishedRelease, ReleaseOutputFormat, ReleaseOutputTarget, render};

fn release(name: &str, version: &str) -> PublishedRelease {
    PublishedRelease {
        name: name.to_string(),
        version: version.to_string(),
        tag: format!("{name}@v{version}"),
    }
}

#[test]
fn github_format_lists_releases() {
    let out = render(ReleaseOutputFormat::Github, &[release("mcp", "3.0.0")]).unwrap();
    assert_eq!(
        out,
        "released=true\nreleases=[{\"name\":\"mcp\",\"version\":\"3.0.0\",\"tag\":\"mcp@v3.0.0\"}]\n"
    );
}

#[test]
fn dotenv_format_prefixes_keys() {
    let out = render(ReleaseOutputFormat::Dotenv, &[release("api", "1.2.0")]).unwrap();
    assert_eq!(
        out,
        "FERRFLOW_RELEASED=true\nFERRFLOW_RELEASES=[{\"name\":\"api\",\"version\":\"1.2.0\",\"tag\":\"api@v1.2.0\"}]\n"
    );
}

#[test]
fn json_format_is_one_object() {
    let out = render(
        ReleaseOutputFormat::Json,
        &[release("api", "1.2.0"), release("web", "0.4.1")],
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["released"], true);
    assert_eq!(value["releases"][1]["name"], "web");
    assert_eq!(value["releases"][1]["tag"], "web@v0.4.1");
}

#[test]
fn nothing_released_reports_false_and_empty_list() {
    assert_eq!(
        render(ReleaseOutputFormat::Github, &[]).unwrap(),
        "released=false\nreleases=[]\n"
    );
    assert_eq!(
        render(ReleaseOutputFormat::Dotenv, &[]).unwrap(),
        "FERRFLOW_RELEASED=false\nFERRFLOW_RELEASES=[]\n"
    );
    assert_eq!(
        render(ReleaseOutputFormat::Json, &[]).unwrap(),
        "{\"released\":false,\"releases\":[]}\n"
    );
}

#[test]
fn github_falls_back_to_github_output() {
    let target =
        ReleaseOutputTarget::resolve(ReleaseOutputFormat::Github, None, Some("/gh/out".into()))
            .unwrap();
    assert_eq!(target.path, PathBuf::from("/gh/out"));
}

#[test]
fn explicit_file_wins_over_github_output() {
    let target = ReleaseOutputTarget::resolve(
        ReleaseOutputFormat::Github,
        Some(PathBuf::from("out.txt")),
        Some("/gh/out".into()),
    )
    .unwrap();
    assert_eq!(target.path, PathBuf::from("out.txt"));
}

#[test]
fn github_without_destination_is_an_error() {
    assert!(ReleaseOutputTarget::resolve(ReleaseOutputFormat::Github, None, None).is_err());
    assert!(
        ReleaseOutputTarget::resolve(ReleaseOutputFormat::Github, None, Some("".into())).is_err()
    );
}

#[test]
fn dotenv_and_json_require_a_file() {
    let gh = Some("/gh/out".into());
    assert!(ReleaseOutputTarget::resolve(ReleaseOutputFormat::Dotenv, None, gh.clone()).is_err());
    assert!(ReleaseOutputTarget::resolve(ReleaseOutputFormat::Json, None, gh).is_err());
}

#[test]
fn github_appends_to_existing_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("github_output");
    std::fs::write(&path, "other=1\n").unwrap();
    let target =
        ReleaseOutputTarget::resolve(ReleaseOutputFormat::Github, Some(path.clone()), None)
            .unwrap();
    target.write(&[]).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "other=1\nreleased=false\nreleases=[]\n"
    );
}

#[test]
fn dotenv_overwrites_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferrflow.env");
    std::fs::write(&path, "FERRFLOW_RELEASED=true\nSTALE=1\n").unwrap();
    let target =
        ReleaseOutputTarget::resolve(ReleaseOutputFormat::Dotenv, Some(path.clone()), None)
            .unwrap();
    target.write(&[]).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "FERRFLOW_RELEASED=false\nFERRFLOW_RELEASES=[]\n"
    );
}
