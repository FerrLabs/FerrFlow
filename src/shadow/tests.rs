use super::*;
use crate::test_utils::{commit_file, git, init_repo_at};

struct Fixture {
    _base: tempfile::TempDir,
    source: PathBuf,
    remote: PathBuf,
    clone: PathBuf,
}

const PLAIN_CONFIG: &str = r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#;

fn fixture(release_message: &str) -> Fixture {
    fixture_with(release_message, PLAIN_CONFIG)
}

fn fixture_with(release_message: &str, config: &str) -> Fixture {
    let base = tempfile::tempdir().unwrap();
    let remote = base.path().join("remote.git");
    std::fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "--quiet", "--bare", "-b", "main"]);

    let source = base.path().join("source");
    std::fs::create_dir_all(&source).unwrap();
    init_repo_at(&source);
    git(
        &source,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    std::fs::write(
        source.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(source.join(".ferrflow"), config).unwrap();
    git(&source, &["add", "."]);
    git(&source, &["commit", "--quiet", "-m", "chore: init"]);
    git(&source, &["tag", "-a", "v1.0.0", "-m", "v1.0.0"]);
    commit_file(
        &source,
        "lib.rs",
        "fn a() {}",
        release_message,
        1_990_000_000,
    );
    git(
        &source,
        &["push", "--quiet", "--follow-tags", "origin", "main"],
    );

    let work = base.path().join("shadow");
    std::fs::create_dir_all(&work).unwrap();
    let clone = work.join("repo");
    clone_repo(&source, &clone, "origin").unwrap();

    Fixture {
        _base: base,
        source,
        remote,
        clone,
    }
}

fn refs(dir: &Path) -> String {
    git(dir, &["for-each-ref", "--format=%(refname) %(objectname)"])
}

#[test]
fn shadow_release_commits_and_tags_inside_the_clone() {
    let f = fixture("feat: add a");

    let summary = release_in(&f.clone, None, false).unwrap();

    assert_eq!(summary.tags, vec!["v1.1.0".to_string()]);
    assert!(summary.files.contains(&"Cargo.toml".to_string()));
    assert_eq!(summary.commits.len(), 1);
    assert!(summary.commits[0].starts_with("chore(release):"));
    let manifest = std::fs::read_to_string(f.clone.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("version = \"1.1.0\""));
}

#[test]
fn shadow_release_leaves_the_source_and_its_remote_untouched() {
    let f = fixture("feat: add a");
    let source_before = refs(&f.source);
    let remote_before = refs(&f.remote);

    release_in(&f.clone, None, false).unwrap();

    assert_eq!(refs(&f.source), source_before);
    assert_eq!(refs(&f.remote), remote_before);
    let manifest = std::fs::read_to_string(f.source.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("version = \"1.0.0\""));
}

#[test]
fn shadow_release_pushes_nothing_even_to_the_sink() {
    let f = fixture("feat: add a");

    release_in(&f.clone, None, false).unwrap();

    let sink = f.clone.parent().unwrap().join("push-sink.git");
    assert_eq!(refs(&sink), "");
}

#[test]
fn a_push_from_the_clone_lands_in_the_sink_not_the_source() {
    let f = fixture("feat: add a");
    let source_before = refs(&f.source);

    git(&f.clone, &["tag", "probe"]);
    git(&f.clone, &["push", "--quiet", "origin", "probe"]);

    assert_eq!(refs(&f.source), source_before);
    let sink = f.clone.parent().unwrap().join("push-sink.git");
    assert!(refs(&sink).contains("refs/tags/probe"));
}

#[test]
fn nothing_to_release_writes_nothing() {
    let f = fixture("chore: tidy");

    let summary = release_in(&f.clone, None, false).unwrap();

    assert_eq!(
        summary,
        ShadowSummary {
            commits: vec![],
            files: vec![],
            tags: vec![],
        }
    );
}

#[test]
fn clone_inherits_the_source_commit_identity() {
    let f = fixture("chore: tidy");

    assert_eq!(
        git(&f.clone, &["config", "user.email"]).trim(),
        "test@test.com"
    );
}

#[test]
fn relocate_moves_an_absolute_config_path_into_the_clone() {
    let root = Path::new("/work/repo");
    let clone = Path::new("/tmp/shadow/repo");

    assert_eq!(
        relocate(&root.join("conf/ferrflow.json"), root, clone),
        clone.join("conf/ferrflow.json")
    );
    assert_eq!(
        relocate(Path::new("ferrflow.json"), root, clone),
        PathBuf::from("ferrflow.json")
    );
    assert_eq!(
        relocate(Path::new("/elsewhere/ferrflow.json"), root, clone),
        PathBuf::from("/elsewhere/ferrflow.json")
    );
}

#[test]
fn shadow_release_runs_pre_publish_hooks_but_not_the_success_hook() {
    let sep = std::path::MAIN_SEPARATOR;
    let config = format!(
        r#"{{"workspace":{{"hooks":{{"prePublish":"echo ran > ..{sep}{sep}pre-publish","onSuccess":"echo ran > ..{sep}{sep}on-success"}}}},"package":[{{"name":"app","path":".","versionedFiles":[{{"path":"Cargo.toml","format":"toml"}}]}}]}}"#
    );
    let f = fixture_with("feat: add a", &config);
    let outside = f.clone.parent().unwrap();

    release_in(&f.clone, None, false).unwrap();

    assert!(outside.join("pre-publish").exists());
    assert!(!outside.join("on-success").exists());
}
