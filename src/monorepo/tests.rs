use super::util::*;
use crate::config::PackageConfig;

mod cache_flow {
    use crate::cache;
    use crate::git::open_repo;
    use crate::test_utils::{commit_file, init_repo, with_cwd};
    use crate::timing::Timing;

    fn setup(dir: &std::path::Path) {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join(".ferrflow"),
            r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"Cargo.toml","format":"toml"}]}]}"#,
        )
        .unwrap();
    }

    fn run_check(dir: &std::path::Path) {
        let config = dir.join(".ferrflow");
        with_cwd(dir, || {
            super::super::check(
                Some(&config),
                false,
                false,
                None,
                false,
                &mut Timing::new(false),
            )
        })
        .unwrap();
    }

    #[test]
    fn first_check_writes_cache_second_hits_with_same_key() {
        let (dir, _repo) = init_repo();
        setup(dir.path());
        commit_file(dir.path(), "a.txt", "x", "feat: a", 1_950_000_000);

        run_check(dir.path());

        let repo = open_repo(dir.path()).unwrap();
        let cache_dir = cache::cache_dir(&repo);
        let key = cache::compute_key(
            &repo,
            dir.path(),
            Some(&dir.path().join(".ferrflow")),
            "text",
        )
        .expect("key");
        let first = cache::read(&cache_dir, &key).expect("cache written on miss");

        run_check(dir.path());
        let second = cache::read(&cache_dir, &key).expect("still a hit");
        assert_eq!(first.text_lines, second.text_lines);
        assert!(!first.text_lines.is_empty());
    }

    #[test]
    fn cache_busts_after_a_new_commit() {
        let (dir, _repo) = init_repo();
        setup(dir.path());
        commit_file(dir.path(), "a.txt", "x", "feat: a", 1_950_000_100);
        run_check(dir.path());

        let repo = open_repo(dir.path()).unwrap();
        let cache_dir = cache::cache_dir(&repo);
        let old_key = cache::compute_key(
            &repo,
            dir.path(),
            Some(&dir.path().join(".ferrflow")),
            "text",
        )
        .unwrap();

        commit_file(dir.path(), "b.txt", "y", "feat: b", 1_950_000_200);
        let repo2 = open_repo(dir.path()).unwrap();
        let new_key = cache::compute_key(
            &repo2,
            dir.path(),
            Some(&dir.path().join(".ferrflow")),
            "text",
        )
        .unwrap();
        assert!(cache::read(&cache_dir, &new_key).is_none());
        assert!(cache::read(&cache_dir, &old_key).is_some());
    }
}

mod manifest_flow {
    use crate::config::Config;
    use crate::manifest::{self, Manifest};
    use crate::status::{self, OutputFormat};
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};
    use crate::timing::Timing;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn add_bare_remote(dir: &Path) -> tempfile::TempDir {
        let bare = tempfile::tempdir().unwrap();
        git(dir, &["init", "--bare", &bare.path().to_string_lossy()]);
        git(
            dir,
            &["remote", "add", "origin", &bare.path().to_string_lossy()],
        );
        bare
    }

    fn write_config(dir: &Path, manifest_mode: bool, commit_mode: &str) {
        let manifest_line = if manifest_mode {
            r#""manifestFile": ".ferrflow.manifest.json", "#
        } else {
            ""
        };
        std::fs::write(
            dir.join(".ferrflow"),
            format!(
                r#"{{"workspace": {{{manifest_line}"releaseCommitMode": "{commit_mode}"}}, "package": [
                    {{"name": "api", "path": "api", "versionedFiles": [{{"path": "api/version.json", "format": "json"}}], "changelog": "api/CHANGELOG.md"}},
                    {{"name": "web", "path": "web", "versionedFiles": [{{"path": "web/version.json", "format": "json"}}], "changelog": "web/CHANGELOG.md"}}
                ]}}"#
            ),
        )
        .unwrap();
    }

    fn write_pkg(dir: &Path, pkg: &str, version: &str) {
        std::fs::create_dir_all(dir.join(pkg)).unwrap();
        std::fs::write(
            dir.join(pkg).join("version.json"),
            format!("{{\"name\": \"{pkg}\", \"version\": \"{version}\"}}"),
        )
        .unwrap();
    }

    fn run_release(dir: &Path) -> anyhow::Result<()> {
        let config = dir.join(".ferrflow");
        with_cwd(dir, || {
            crate::monorepo::release(
                Some(&config),
                false,
                false,
                false,
                false,
                None,
                None,
                false,
                false,
                &mut Timing::new(false),
            )
        })
    }

    #[test]
    fn release_none_mode_writes_full_sorted_snapshot() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.0.0");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path(), true, "none");
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_960_000_000);
        commit_file(
            dir.path(),
            "api/feature.txt",
            "y",
            "feat: api change",
            1_960_000_001,
        );
        let _remote = add_bare_remote(dir.path());

        run_release(dir.path()).unwrap();

        let manifest_path = dir.path().join(".ferrflow.manifest.json");
        let manifest = manifest::read(&manifest_path).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.version_of("api"), Some("1.1.0"));
        assert_eq!(manifest.version_of("web"), Some("2.0.0"));

        let raw = std::fs::read_to_string(&manifest_path).unwrap();
        let api_at = raw.find("\"api\"").unwrap();
        let web_at = raw.find("\"web\"").unwrap();
        assert!(api_at < web_at, "packages must be sorted");
        serde_json::from_str::<serde_json::Value>(&raw).expect("valid JSON");
    }

    #[test]
    fn manifest_off_writes_no_manifest() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.0.0");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path(), false, "none");
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_961_000_000);
        commit_file(
            dir.path(),
            "api/feature.txt",
            "y",
            "feat: api change",
            1_961_000_001,
        );
        let _remote = add_bare_remote(dir.path());

        run_release(dir.path()).unwrap();

        assert!(!dir.path().join(".ferrflow.manifest.json").exists());
    }

    #[test]
    fn release_commit_mode_includes_manifest_in_release_commit() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.0.0");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path(), true, "commit");
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_966_000_000);
        commit_file(
            dir.path(),
            "api/feature.txt",
            "y",
            "feat: api change",
            1_966_000_001,
        );
        let _remote = add_bare_remote(dir.path());

        run_release(dir.path()).unwrap();

        let committed = git(dir.path(), &["show", "--name-only", "--format=", "HEAD"]);
        assert!(
            committed.contains(".ferrflow.manifest.json"),
            "manifest must be part of the release commit, got: {committed}"
        );
        let manifest = manifest::read(&dir.path().join(".ferrflow.manifest.json")).unwrap();
        assert_eq!(manifest.version_of("api"), Some("1.1.0"));
        assert_eq!(manifest.version_of("web"), Some("2.0.0"));
    }

    #[test]
    fn diverged_manifest_blocks_release_start() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.1.0");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path(), true, "commit");
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_962_000_000);
        commit_file(
            dir.path(),
            "api/feature.txt",
            "y",
            "feat: api change",
            1_962_000_001,
        );

        let mut packages = BTreeMap::new();
        packages.insert("api".to_string(), "1.0.0".to_string());
        packages.insert("web".to_string(), "2.0.0".to_string());
        manifest::write_atomic(
            &dir.path().join(".ferrflow.manifest.json"),
            &Manifest::new(packages, "t".into(), "sha".into()),
        )
        .unwrap();

        let err = run_release(dir.path()).unwrap_err();
        assert!(format!("{err:?}").contains("sync-manifest"));
        let api = std::fs::read_to_string(dir.path().join("api/version.json")).unwrap();
        assert!(api.contains("1.1.0"));
    }

    #[test]
    fn sync_manifest_regenerates_from_files() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "3.4.5");
        write_pkg(dir.path(), "web", "6.7.8");
        write_config(dir.path(), true, "none");
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_963_000_000);
        let config = dir.path().join(".ferrflow");

        with_cwd(dir.path(), || manifest::sync_cwd(Some(&config))).unwrap();

        let manifest = manifest::read(&dir.path().join(".ferrflow.manifest.json")).unwrap();
        assert_eq!(manifest.version_of("api"), Some("3.4.5"));
        assert_eq!(manifest.version_of("web"), Some("6.7.8"));
    }

    #[test]
    fn sync_manifest_errors_when_disabled() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.0.0");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path(), false, "none");
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_964_000_000);
        let config = dir.path().join(".ferrflow");

        let err = with_cwd(dir.path(), || manifest::sync_cwd(Some(&config))).unwrap_err();
        assert!(format!("{err:?}").contains("manifest_file"));
    }

    #[test]
    fn status_reads_from_manifest_when_present() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.0.0");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path(), true, "none");
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_965_000_000);

        let mut packages = BTreeMap::new();
        packages.insert("api".to_string(), "1.1.0".to_string());
        packages.insert("web".to_string(), "2.0.0".to_string());
        manifest::write_atomic(
            &dir.path().join(".ferrflow.manifest.json"),
            &Manifest::new(packages, "t".into(), "sha".into()),
        )
        .unwrap();

        let config = dir.path().join(".ferrflow");
        let loaded = Config::load(dir.path(), Some(&config)).unwrap();
        let manifest = manifest::read(&dir.path().join(".ferrflow.manifest.json")).unwrap();
        let api = loaded.packages.iter().find(|p| p.name == "api").unwrap();
        assert_eq!(manifest.version_of(&api.name), Some("1.1.0"));

        with_cwd(dir.path(), || {
            status::run(Some(&config), &OutputFormat::Json, &mut Timing::new(false))
        })
        .unwrap();
    }
}

mod release_json_diff {
    use crate::changelog::{ChangelogRender, GitLog};
    use crate::config::Config;
    use crate::conventional_commits::BumpType;
    use crate::monorepo::run::{collect_dry_run_diffs, run_release_logic};
    use crate::test_utils::{commit_file, init_repo, with_cwd};
    use crate::timing::Timing;
    use std::path::Path;

    fn write_config(dir: &Path) {
        std::fs::write(
            dir.join(".ferrflow"),
            r#"{"workspace": {"releaseCommitMode": "none"}, "package": [
                {"name": "api", "path": "api", "versionedFiles": [{"path": "api/package.json", "format": "json"}], "changelog": "api/CHANGELOG.md"},
                {"name": "web", "path": "web", "versionedFiles": [{"path": "web/package.json", "format": "json"}], "changelog": "web/CHANGELOG.md"}
            ]}"#,
        )
        .unwrap();
    }

    fn write_pkg(dir: &Path, pkg: &str, version: &str) {
        std::fs::create_dir_all(dir.join(pkg)).unwrap();
        std::fs::write(
            dir.join(pkg).join("package.json"),
            format!("{{\n  \"name\": \"{pkg}\",\n  \"version\": \"{version}\",\n  \"private\": true\n}}\n"),
        )
        .unwrap();
    }

    fn dry_run_json(dir: &Path) -> serde_json::Value {
        let config_path = dir.join(".ferrflow");
        let config = Config::load(dir, Some(&config_path)).unwrap();
        let mut captured: Option<String> = None;
        with_cwd(dir, || {
            let out = run_release_logic(
                dir,
                &config,
                true,
                false,
                false,
                true,
                false,
                None,
                None,
                false,
                false,
                &mut Timing::new(false),
            )?;
            captured = out.expect("dry-run returns output").json;
            Ok(())
        })
        .unwrap();
        let raw = captured.expect("json set");
        serde_json::from_str(&raw).expect("valid JSON")
    }

    #[test]
    fn dry_run_json_has_expected_shape() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.2.3");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path());
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_970_000_000);
        commit_file(
            dir.path(),
            "api/feature.txt",
            "y",
            "feat: api change",
            1_970_000_001,
        );

        let v = dry_run_json(dir.path());

        assert_eq!(v["dry_run"], true);
        assert_eq!(v["git"]["branch"], "main");
        assert_eq!(
            v["git"]["tags_pushed"].as_array().unwrap().len(),
            0,
            "dry-run pushes nothing"
        );

        let released = v["released"].as_array().unwrap();
        assert_eq!(released.len(), 1);
        let api = &released[0];
        assert_eq!(api["package"], "api");
        assert_eq!(api["previous_version"], "1.2.3");
        assert_eq!(api["new_version"], "1.3.0");
        assert_eq!(api["bump_type"], "minor");
        assert_eq!(api["tag"], "api@v1.3.0");
        assert_eq!(api["commit_count"], 2);
        assert_eq!(api["prerelease"], false);
        assert!(api["forge_release_url"].is_null());
        assert!(api["forge_release_id"].is_null());

        let skipped = v["skipped"].as_array().unwrap();
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0]["package"], "web");
        assert!(skipped[0]["reason"].is_string());
    }

    #[test]
    fn empty_release_yields_empty_released() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.2.3");
        write_pkg(dir.path(), "web", "2.0.0");
        write_config(dir.path());
        commit_file(dir.path(), "seed.txt", "x", "chore: seed", 1_971_000_000);

        let v = dry_run_json(dir.path());

        assert_eq!(v["released"].as_array().unwrap().len(), 0);
        let skipped = v["skipped"].as_array().unwrap();
        assert_eq!(skipped.len(), 2);
        assert_eq!(v["dry_run"], true);
    }

    #[test]
    fn dry_run_diff_covers_versioned_file_and_changelog() {
        let (dir, _repo) = init_repo();
        write_pkg(dir.path(), "api", "1.2.3");
        std::fs::write(
            dir.path().join("api/CHANGELOG.md"),
            "# Changelog\n\n## [1.2.3]\n\n- old\n",
        )
        .unwrap();
        write_config(dir.path());

        let config_path = dir.path().join(".ferrflow");
        let config = Config::load(dir.path(), Some(&config_path)).unwrap();
        let pkg = config.packages.iter().find(|p| p.name == "api").unwrap();
        let commits = vec![GitLog {
            hash: "abc1234".to_string(),
            message: "feat: api change".to_string(),
        }];
        let render = ChangelogRender::default();

        let diffs =
            collect_dry_run_diffs(pkg, dir.path(), "1.3.0", &commits, BumpType::Minor, &render);

        let paths: Vec<&str> = diffs.iter().map(|(p, _)| p.as_str()).collect();
        assert!(
            paths.contains(&"api/package.json"),
            "versioned file diff missing: {paths:?}"
        );
        assert!(
            paths.contains(&"api/CHANGELOG.md"),
            "changelog diff missing: {paths:?}"
        );

        let pkg_diff = &diffs
            .iter()
            .find(|(p, _)| p == "api/package.json")
            .unwrap()
            .1;
        assert!(pkg_diff.contains("@@ -"));
        assert!(pkg_diff.contains("-    \"version\": \"1.2.3\","));
        assert!(pkg_diff.contains("+    \"version\": \"1.3.0\","));
    }
}

#[test]
fn pick_higher_semver_prefers_tag_when_tag_is_higher() {
    assert_eq!(pick_higher_semver("2.0.0", "3.0.0"), "3.0.0");
}

#[test]
fn pick_higher_semver_prefers_file_when_file_is_higher() {
    assert_eq!(pick_higher_semver("5.0.0", "2.0.0"), "5.0.0");
}

#[test]
fn pick_higher_semver_returns_tag_on_equality() {
    assert_eq!(pick_higher_semver("2.1.0", "2.1.0"), "2.1.0");
}

#[test]
fn pick_higher_semver_falls_back_to_tag_when_file_is_invalid() {
    assert_eq!(pick_higher_semver("garbage", "2.0.0"), "2.0.0");
}

#[test]
fn pick_higher_semver_falls_back_to_file_when_tag_is_invalid() {
    assert_eq!(pick_higher_semver("2.0.0", "garbage"), "2.0.0");
}

#[test]
fn pick_higher_semver_strips_leading_v() {
    assert_eq!(pick_higher_semver("v2.0.0", "v3.0.0"), "v3.0.0");
}

fn make_pkg(name: &str, path: &str, shared: &[&str]) -> PackageConfig {
    PackageConfig {
        name: name.into(),
        path: path.into(),
        versioned_files: vec![],
        changelog: None,
        shared_paths: shared.iter().map(|s| s.to_string()).collect(),
        depends_on: vec![],
        versioning: None,
        tag_template: None,
        hooks: None,
        floating_tags: None,
        publishers: vec![],
        update_lockfiles: None,
    }
}

#[test]
fn single_package_always_touched() {
    let pkg = make_pkg("app", ".", &[]);
    let files = vec!["README.md".to_string()];
    assert!(is_package_touched(&pkg, &files, false));
}

#[test]
fn monorepo_root_package_always_touched() {
    let pkg = make_pkg("root", ".", &[]);
    let files = vec!["something.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_package_touched_by_own_files() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["packages/api/src/main.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_package_not_touched_by_other_files() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["packages/site/index.ts".to_string()];
    assert!(!is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_package_touched_by_shared_path() {
    let pkg = make_pkg("api", "packages/api", &["packages/shared/"]);
    let files = vec!["packages/shared/types.ts".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_shared_path_trailing_slash_trimmed() {
    let pkg = make_pkg("api", "packages/api", &["lib/"]);
    let files = vec!["lib/utils.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_no_changed_files() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files: Vec<String> = vec![];
    assert!(!is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_path_with_dot_slash_prefix() {
    let pkg = make_pkg("api", "./packages/api", &[]);
    let files = vec!["packages/api/src/main.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn single_package_mode_always_touched() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["unrelated/file.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, false));
}

#[test]
fn monorepo_empty_path_is_root() {
    let pkg = make_pkg("root", "", &[]);
    let files = vec!["anything.rs".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_exact_shared_path_file() {
    let pkg = make_pkg("api", "packages/api", &["shared-config.json"]);
    let files = vec!["shared-config.json".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_multiple_shared_paths() {
    let pkg = make_pkg("api", "packages/api", &["lib/", "proto/"]);
    let files = vec!["proto/schema.proto".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_similar_prefix_no_false_positive() {
    let pkg = make_pkg("api", "packages/api", &[]);
    let files = vec!["packages/api-docs/README.md".to_string()];
    assert!(!is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_shared_path_with_trailing_slash() {
    let pkg = make_pkg("api", "packages/api", &["packages/shared/"]);
    let files = vec!["packages/shared/types.ts".to_string()];
    assert!(is_package_touched(&pkg, &files, true));
}

#[test]
fn monorepo_empty_changed_files_single_package() {
    let pkg = make_pkg("app", "packages/app", &[]);
    let files: Vec<String> = vec![];
    assert!(is_package_touched(&pkg, &files, false));
}

#[test]
fn parse_force_version_single_repo() {
    let fv = "1.2.3";
    let result: Option<(Option<&str>, &str)> = if let Some(at_pos) = fv.find('@') {
        let name = &fv[..at_pos];
        let version = &fv[at_pos + 1..];
        Some((Some(name), version))
    } else {
        Some((None, fv))
    };
    assert_eq!(result, Some((None, "1.2.3")));
}

#[test]
fn parse_force_version_monorepo() {
    let fv = "api@2.0.0";
    let result: Option<(Option<&str>, &str)> = if let Some(at_pos) = fv.find('@') {
        let name = &fv[..at_pos];
        let version = &fv[at_pos + 1..];
        Some((Some(name), version))
    } else {
        Some((None, fv))
    };
    assert_eq!(result, Some((Some("api"), "2.0.0")));
}

#[test]
fn parse_force_version_with_v_prefix() {
    let fv = "v3.0.0";
    let clean = fv.strip_prefix('v').unwrap_or(fv);
    assert!(semver::Version::parse(clean).is_ok());
}

#[test]
fn parse_force_version_invalid_semver() {
    let fv = "not-a-version";
    let clean = fv.strip_prefix('v').unwrap_or(fv);
    assert!(semver::Version::parse(clean).is_err());
}

mod lockfile_flow {
    use crate::formats::lockfiles::{self, UpdateOutcome};
    use crate::test_utils::{commit_file, git, init_repo, with_cwd};
    use crate::timing::Timing;
    use std::path::Path;

    fn program_on_path(program: &str) -> bool {
        std::process::Command::new(program)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn add_bare_remote(dir: &Path) -> tempfile::TempDir {
        let bare = tempfile::tempdir().unwrap();
        git(dir, &["init", "--bare", &bare.path().to_string_lossy()]);
        git(
            dir,
            &["remote", "add", "origin", &bare.path().to_string_lossy()],
        );
        bare
    }

    fn write_cargo_workspace(dir: &Path) {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\"crates/alpha\", \"crates/beta\"]\n",
        )
        .unwrap();
        for member in ["alpha", "beta"] {
            let crate_dir = dir.join("crates").join(member);
            std::fs::create_dir_all(crate_dir.join("src")).unwrap();
            std::fs::write(
                crate_dir.join("Cargo.toml"),
                format!("[package]\nname = \"{member}\"\nversion = \"1.0.0\"\nedition = \"2021\"\n\n[dependencies]\n"),
            )
            .unwrap();
            std::fs::write(crate_dir.join("src").join("lib.rs"), "").unwrap();
        }
        std::fs::write(
            dir.join(".ferrflow"),
            r#"{"workspace": {"releaseCommitMode": "commit", "updateLockfiles": true}, "package": [
                {"name": "alpha", "path": "crates/alpha", "versionedFiles": [{"path": "crates/alpha/Cargo.toml", "format": "toml"}], "changelog": "crates/alpha/CHANGELOG.md"},
                {"name": "beta", "path": "crates/beta", "versionedFiles": [{"path": "crates/beta/Cargo.toml", "format": "toml"}], "changelog": "crates/beta/CHANGELOG.md"}
            ]}"#,
        )
        .unwrap();
    }

    fn run_release(dir: &Path) -> anyhow::Result<()> {
        let config = dir.join(".ferrflow");
        with_cwd(dir, || {
            crate::monorepo::release(
                Some(&config),
                false,
                false,
                false,
                false,
                None,
                None,
                false,
                false,
                &mut Timing::new(false),
            )
        })
    }

    fn alpha_version_in_lock(lock: &str) -> Option<String> {
        let mut in_alpha = false;
        for line in lock.lines() {
            let line = line.trim();
            if line == "[[package]]" {
                in_alpha = false;
            } else if line == "name = \"alpha\"" {
                in_alpha = true;
            } else if in_alpha && let Some(v) = line.strip_prefix("version = \"") {
                return Some(v.trim_end_matches('"').to_string());
            }
        }
        None
    }

    #[test]
    fn cargo_workspace_refreshes_only_bumped_member() {
        if !program_on_path("cargo") {
            return;
        }
        let (dir, _repo) = init_repo();
        let root = dir.path();
        write_cargo_workspace(root);

        let gen_lock = std::process::Command::new("cargo")
            .current_dir(root)
            .args(["generate-lockfile", "--offline"])
            .output()
            .unwrap();
        assert!(
            gen_lock.status.success(),
            "generate-lockfile failed: {}",
            String::from_utf8_lossy(&gen_lock.stderr)
        );

        commit_file(root, "seed.txt", "x", "chore: seed", 1_970_000_000);
        commit_file(
            root,
            "crates/alpha/feature.txt",
            "y",
            "feat: alpha change",
            1_970_000_001,
        );
        let _remote = add_bare_remote(root);

        let before = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
        assert_eq!(alpha_version_in_lock(&before).as_deref(), Some("1.0.0"));
        let beta_before = before.contains("name = \"beta\"");

        run_release(root).unwrap();

        let after = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
        assert_eq!(
            alpha_version_in_lock(&after).as_deref(),
            Some("1.1.0"),
            "alpha entry must reflect the bump in Cargo.lock"
        );
        assert_eq!(
            beta_before,
            after.contains("name = \"beta\""),
            "beta entry must be untouched"
        );

        let committed = git(root, &["show", "--name-only", "--format=", "HEAD"]);
        assert!(
            committed.contains("Cargo.lock"),
            "refreshed Cargo.lock must land in the release commit, got: {committed}"
        );
    }

    #[test]
    fn disabled_leaves_lockfile_untouched() {
        if !program_on_path("cargo") {
            return;
        }
        let (dir, _repo) = init_repo();
        let root = dir.path();
        write_cargo_workspace(root);
        std::fs::write(
            root.join(".ferrflow"),
            r#"{"workspace": {"releaseCommitMode": "commit"}, "package": [
                {"name": "alpha", "path": "crates/alpha", "versionedFiles": [{"path": "crates/alpha/Cargo.toml", "format": "toml"}], "changelog": "crates/alpha/CHANGELOG.md"},
                {"name": "beta", "path": "crates/beta", "versionedFiles": [{"path": "crates/beta/Cargo.toml", "format": "toml"}], "changelog": "crates/beta/CHANGELOG.md"}
            ]}"#,
        )
        .unwrap();

        let gen_lock = std::process::Command::new("cargo")
            .current_dir(root)
            .args(["generate-lockfile", "--offline"])
            .output()
            .unwrap();
        assert!(gen_lock.status.success());

        commit_file(root, "seed.txt", "x", "chore: seed", 1_971_000_000);
        commit_file(
            root,
            "crates/alpha/feature.txt",
            "y",
            "feat: alpha change",
            1_971_000_001,
        );
        let _remote = add_bare_remote(root);

        let before = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();

        run_release(root).unwrap();

        let after = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
        assert_eq!(
            before, after,
            "Cargo.lock must be byte-identical when updateLockfiles is unset"
        );

        let committed = git(root, &["show", "--name-only", "--format=", "HEAD"]);
        assert!(
            !committed.contains("Cargo.lock"),
            "Cargo.lock must not be staged when the feature is off, got: {committed}"
        );
    }

    #[test]
    fn missing_package_manager_is_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mix.exs"),
            "defmodule App.MixProject do\nend\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("mix.lock"), "%{}\n").unwrap();

        let outcome = lockfiles::update_for_manifest(dir.path(), "mix.exs").unwrap();
        if program_on_path("mix") {
            assert!(matches!(
                outcome,
                UpdateOutcome::Updated { .. } | UpdateOutcome::Failed { .. }
            ));
        } else {
            assert_eq!(
                outcome,
                UpdateOutcome::NotOnPath {
                    program: "mix".to_string()
                }
            );
        }
    }
}

mod cycle_detection {
    use crate::config::Config;
    use crate::monorepo::run::run_release_logic;
    use crate::test_utils::{commit_file, init_repo, with_cwd};
    use crate::timing::Timing;
    use std::path::Path;

    fn write_pkg(dir: &Path, name: &str) {
        std::fs::create_dir_all(dir.join(name)).unwrap();
        std::fs::write(
            dir.join(name).join("package.json"),
            format!("{{\n  \"name\": \"{name}\",\n  \"version\": \"1.0.0\"\n}}\n"),
        )
        .unwrap();
    }

    #[test]
    fn mutual_dependency_aborts_release_without_writing() {
        let (dir, _repo) = init_repo();
        let root = dir.path();
        write_pkg(root, "api");
        write_pkg(root, "web");
        std::fs::write(
            root.join(".ferrflow"),
            r#"{"package": [
                {"name": "api", "path": "api", "dependsOn": ["web"], "versionedFiles": [{"path": "api/package.json", "format": "json"}]},
                {"name": "web", "path": "web", "dependsOn": ["api"], "versionedFiles": [{"path": "web/package.json", "format": "json"}]}
            ]}"#,
        )
        .unwrap();
        commit_file(
            root,
            "api/feature.txt",
            "x",
            "feat: api change",
            1_972_000_000,
        );

        let config_path = root.join(".ferrflow");
        let config = Config::load(root, Some(&config_path)).unwrap();

        let err = with_cwd(root, || {
            run_release_logic(
                root,
                &config,
                false,
                false,
                false,
                false,
                false,
                None,
                None,
                false,
                false,
                &mut Timing::new(false),
            )?;
            Ok(())
        })
        .unwrap_err();

        assert_eq!(
            err.downcast_ref::<crate::error_code::ErrorCode>()
                .map(|c| c.0),
            Some(8003),
            "expected MONOREPO_DEPENDENCY_CYCLE"
        );
        let message = err
            .chain()
            .map(|cause| cause.to_string())
            .find(|m| m.starts_with("cycle detected"))
            .expect("cycle message in the error chain");
        assert!(
            message.contains("api") && message.contains("web"),
            "{message}"
        );

        let api = std::fs::read_to_string(root.join("api/package.json")).unwrap();
        let web = std::fs::read_to_string(root.join("web/package.json")).unwrap();
        assert!(
            api.contains("\"version\": \"1.0.0\""),
            "api untouched: {api}"
        );
        assert!(
            web.contains("\"version\": \"1.0.0\""),
            "web untouched: {web}"
        );
    }
}
