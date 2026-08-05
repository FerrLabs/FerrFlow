use super::*;
use crate::test_utils::{commit_file, git, init_repo};
use std::path::PathBuf;

static COMMIT_TIME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1_960_000_000);

fn next_ts() -> i64 {
    COMMIT_TIME.fetch_add(10, std::sync::atomic::Ordering::SeqCst)
}

struct Fixture {
    _dir: tempfile::TempDir,
    repo: crate::git::Repository,
    config: Config,
    root: PathBuf,
}

impl Fixture {
    fn commit(&self, file: &str, message: &str) {
        if let Some(parent) = std::path::Path::new(file).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(self.root.join(parent)).unwrap();
        }
        commit_file(&self.root, file, "x", message, next_ts());
    }

    fn tag(&self, name: &str) {
        git(&self.root, &["tag", name]);
    }

    fn explain(&self, package: &str) -> Explanation {
        let pkg = resolve_package(&self.config, Some(package)).unwrap();
        super::explain(&self.repo, &self.root, &self.config, pkg, None).unwrap()
    }
}

/// Three packages: `core` standalone, `api` with a shared path and a default
/// (`same`) dependency on core, `web` with a `patch` policy on core.
fn monorepo() -> Fixture {
    let (dir, repo) = init_repo();
    let root = dir.path().to_path_buf();
    for name in ["core", "api", "web"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
        std::fs::write(
            root.join(name).join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\n"),
        )
        .unwrap();
    }
    std::fs::create_dir_all(root.join("shared")).unwrap();
    std::fs::write(
        root.join("ferrflow.json"),
        r#"{
  "package": [
    { "name": "core", "path": "core", "versionedFiles": [{ "path": "core/Cargo.toml", "format": "toml" }] },
    { "name": "api", "path": "api", "sharedPaths": ["shared/"], "dependsOn": ["core"],
      "versionedFiles": [{ "path": "api/Cargo.toml", "format": "toml" }] },
    { "name": "web", "path": "web", "dependsOn": [{ "name": "core", "propagate": "patch" }],
      "versionedFiles": [{ "path": "web/Cargo.toml", "format": "toml" }] }
  ]
}
"#,
    )
    .unwrap();
    git(&root, &["add", "-A"]);
    commit_file(&root, "seed.txt", "x", "chore: seed", next_ts());
    let config = Config::load(&root, Some(&root.join("ferrflow.json"))).unwrap();
    let fx = Fixture {
        _dir: dir,
        repo,
        config,
        root,
    };
    for name in ["core", "api", "web"] {
        fx.tag(&format!("{name}@v1.0.0"));
    }
    fx
}

fn bump_of(explanation: &Explanation) -> &Decision {
    &explanation.decision
}

#[test]
fn a_feat_commit_on_the_package_explains_the_minor_bump() {
    let fx = monorepo();
    fx.commit("core/src/lib.rs", "feat: add a thing");

    let x = fx.explain("core");

    assert!(x.touch.touched);
    assert_eq!(
        x.commits
            .iter()
            .map(|c| c.bump.as_str())
            .collect::<Vec<_>>(),
        ["minor"]
    );
    match bump_of(&x) {
        Decision::Bump {
            bump,
            from,
            to,
            triggered_by,
            ..
        } => {
            assert_eq!(
                (bump.as_str(), from.as_str(), to.as_str()),
                ("minor", "1.0.0", "1.1.0")
            );
            assert!(matches!(triggered_by, Trigger::Commits));
        }
        other => panic!("expected a bump, got {:?}", serde_json::to_string(other)),
    }
}

// The whole point of the command: name the files that were examined and say
// which rule each one failed, instead of a bare "not touched".
#[test]
fn an_untouched_package_lists_the_files_that_did_not_match() {
    let fx = monorepo();
    fx.commit("api/src/lib.rs", "feat: api change");

    let x = fx.explain("core");

    assert!(!x.touch.touched);
    assert_eq!(
        x.touch
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.matched.as_deref()))
            .collect::<Vec<_>>(),
        [("api/src/lib.rs", None)]
    );
    assert!(
        x.commits.is_empty(),
        "an untouched package classifies nothing"
    );
    match bump_of(&x) {
        Decision::Skipped { reason } => assert_eq!(reason, "not touched"),
        other => panic!("expected a skip, got {:?}", serde_json::to_string(other)),
    }
}

#[test]
fn a_shared_path_hit_names_the_rule_that_matched() {
    let fx = monorepo();
    fx.commit("shared/util.rs", "feat: shared change");

    let x = fx.explain("api");

    assert!(x.touch.touched);
    assert_eq!(
        x.touch.files.first().and_then(|f| f.matched.as_deref()),
        Some("shared/")
    );
}

// A package skipped on its own commits still releases when an upstream moves.
// Reporting "not touched" and stopping there would contradict the next release.
#[test]
fn a_cascaded_package_reports_the_dependency_as_the_trigger() {
    let fx = monorepo();
    fx.commit("core/src/lib.rs", "feat!: rewrite core");

    let web = fx.explain("web");
    assert!(!web.touch.touched);
    assert_eq!(
        web.dependencies
            .iter()
            .map(|d| (
                d.name.as_str(),
                d.propagate.as_str(),
                d.upstream_bump.as_deref(),
                d.resulting_bump.as_str()
            ))
            .collect::<Vec<_>>(),
        [("core", "patch", Some("major"), "patch")]
    );
    match bump_of(&web) {
        Decision::Bump {
            bump,
            to,
            triggered_by,
            ..
        } => {
            assert_eq!((bump.as_str(), to.as_str()), ("patch", "1.0.1"));
            assert!(matches!(triggered_by, Trigger::Dependency));
        }
        other => panic!(
            "expected a cascade bump, got {:?}",
            serde_json::to_string(other)
        ),
    }

    // Same upstream, default policy: `api` takes the major.
    let api = fx.explain("api");
    match bump_of(&api) {
        Decision::Bump { bump, to, .. } => {
            assert_eq!((bump.as_str(), to.as_str()), ("major", "2.0.0"))
        }
        other => panic!(
            "expected a cascade bump, got {:?}",
            serde_json::to_string(other)
        ),
    }
}

// "I committed five times and nothing released" — the commits have to be listed
// with their classification, or the skip reason is unactionable.
#[test]
fn a_chore_only_history_still_lists_the_commits_it_rejected() {
    let fx = monorepo();
    fx.commit("core/src/a.rs", "chore: tidy");
    fx.commit("core/src/b.rs", "docs: notes");

    let x = fx.explain("core");

    assert!(x.touch.touched);
    assert_eq!(
        x.commits
            .iter()
            .map(|c| (c.subject.as_str(), c.bump.as_str()))
            .collect::<Vec<_>>(),
        [("docs: notes", "none"), ("chore: tidy", "none")]
    );
    match bump_of(&x) {
        Decision::Skipped { reason } => assert_eq!(reason, "no releasable commits"),
        other => panic!("expected a skip, got {:?}", serde_json::to_string(other)),
    }
}

#[test]
fn a_never_released_package_reports_no_tag() {
    let (dir, repo) = init_repo();
    let root = dir.path().to_path_buf();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("ferrflow.json"),
        r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#,
    )
    .unwrap();
    git(&root, &["add", "-A"]);
    commit_file(&root, "seed.txt", "x", "feat: first", next_ts());
    let config = Config::load(&root, Some(&root.join("ferrflow.json"))).unwrap();
    let fx = Fixture {
        _dir: dir,
        repo,
        config,
        root,
    };

    let x = fx.explain("app");
    assert!(x.last_tag.is_none());
    assert!(!x.monorepo);
    assert_eq!(
        x.touch.files.first().and_then(|f| f.matched.as_deref()),
        Some("single-package repo")
    );
}

// `matching_rule` restates `is_touched_by` one file at a time so the report can
// name the rule; if the two ever disagree the explanation is a lie.
#[test]
fn the_per_file_rule_agrees_with_the_touch_check() {
    let fx = monorepo();
    let api = fx.config.packages.iter().find(|p| p.name == "api").unwrap();

    for file in [
        "api/src/lib.rs",
        "api/Cargo.toml",
        "shared/util.rs",
        "shared",
        "apiary/src/lib.rs",
        "core/src/lib.rs",
        "README.md",
        "",
    ] {
        let files = [file.to_string()];
        assert_eq!(
            matching_rule(api, file, true).is_some(),
            api.is_touched_by(&files, true),
            "disagreement on {file:?}"
        );
    }
}

#[test]
fn naming_no_package_in_a_monorepo_lists_the_candidates() {
    let fx = monorepo();
    let err = resolve_package(&fx.config, None).unwrap_err().to_string();
    assert!(
        err.contains("core") && err.contains("api") && err.contains("web"),
        "{err}"
    );
}

#[test]
fn an_unknown_package_name_is_rejected_with_the_known_ones() {
    let fx = monorepo();
    let err = resolve_package(&fx.config, Some("nope"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("nope") && err.contains("core"), "{err}");
}
