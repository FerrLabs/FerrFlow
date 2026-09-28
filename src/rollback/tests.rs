use std::path::{Path, PathBuf};

use crate::monorepo::run::checkpoint::{Checkpoint, Phase, RecordedPublish, RecordedTag};
use crate::test_utils::{commit_file, git, init_repo_at, with_cwd};

const SINGLE: &str = r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"VERSION","format":"txt"}]}]}"#;

const MONOREPO: &str = r#"{"package":[
    {"name":"api","path":"api","versionedFiles":[{"path":"api/VERSION","format":"txt"}]},
    {"name":"web","path":"web","versionedFiles":[{"path":"web/VERSION","format":"txt"}]}
]}"#;

struct Fixture {
    _dir: tempfile::TempDir,
    local: PathBuf,
    remote: PathBuf,
}

impl Fixture {
    fn new(config: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let remote = dir.path().join("remote.git");
        std::fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "--bare", "-b", "main"]);

        let local = dir.path().join("local");
        std::fs::create_dir_all(local.join("api")).unwrap();
        std::fs::create_dir_all(local.join("web")).unwrap();
        init_repo_at(&local);
        git(
            &local,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        std::fs::write(local.join(".ferrflow"), config).unwrap();
        git(&local, &["add", ".ferrflow"]);
        commit_file(&local, "VERSION", "1.0.0", "feat: first", 1_900_000_000);
        git(&local, &["push", "origin", "main:main"]);

        Self {
            _dir: dir,
            local,
            remote,
        }
    }

    fn release(&self, tags: &[&str]) -> Checkpoint {
        let before = self.head();
        commit_file(
            &self.local,
            "VERSION",
            "1.1.0",
            "chore(release): 1.1.0",
            1_900_000_100,
        );
        let release_sha = self.head();
        let mut checkpoint = Checkpoint::new(before, tags.iter().map(|t| t.to_string()).collect());
        checkpoint.commit_sha = Some(release_sha.clone());
        for tag in tags {
            git(&self.local, &["tag", "-a", tag, "-m", tag]);
            git(&self.local, &["push", "origin", tag]);
            checkpoint.created_tags.push(RecordedTag {
                name: tag.to_string(),
                sha: release_sha.clone(),
            });
        }
        checkpoint.advance(Phase::Pushed);
        checkpoint
    }

    fn save(&self, checkpoint: &Checkpoint) {
        checkpoint.save(&self.local).unwrap();
    }

    fn head(&self) -> String {
        git(&self.local, &["rev-parse", "HEAD"]).trim().to_string()
    }

    fn head_subject(&self) -> String {
        git(&self.local, &["log", "-1", "--format=%s"])
            .trim()
            .to_string()
    }

    fn local_tags(&self) -> Vec<String> {
        tag_list(&self.local)
    }

    fn remote_tags(&self) -> Vec<String> {
        tag_list(&self.remote)
    }

    fn has_checkpoint(&self) -> bool {
        Checkpoint::path(&self.local).exists()
    }

    fn rollback(&self, packages: &[&str], yes: bool) -> anyhow::Result<()> {
        let packages: Vec<String> = packages.iter().map(|p| p.to_string()).collect();
        let mut outcome = None;
        with_cwd(&self.local, || {
            outcome = Some(super::run(&packages, yes, None));
            Ok(())
        })
        .unwrap();
        outcome.unwrap()
    }
}

fn tag_list(dir: &Path) -> Vec<String> {
    git(dir, &["tag", "-l"])
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn without_a_checkpoint_there_is_nothing_to_do() {
    let fx = Fixture::new(SINGLE);
    git(&fx.local, &["tag", "v1.0.0"]);
    let head = fx.head();

    fx.rollback(&[], true).unwrap();

    assert_eq!(fx.head(), head);
    assert_eq!(fx.local_tags(), ["v1.0.0"]);
}

#[test]
fn a_dry_run_leaves_tags_commit_and_checkpoint_untouched() {
    let fx = Fixture::new(SINGLE);
    let checkpoint = fx.release(&["v1.1.0"]);
    fx.save(&checkpoint);
    let head = fx.head();

    fx.rollback(&[], false).unwrap();

    assert_eq!(fx.head(), head);
    assert_eq!(fx.local_tags(), ["v1.1.0"]);
    assert_eq!(fx.remote_tags(), ["v1.1.0"]);
    assert!(fx.has_checkpoint());
}

#[test]
fn applying_deletes_the_tag_everywhere_reverts_the_release_and_clears_the_checkpoint() {
    let fx = Fixture::new(SINGLE);
    let checkpoint = fx.release(&["v1.1.0"]);
    fx.save(&checkpoint);

    fx.rollback(&[], true).unwrap();

    assert!(fx.local_tags().is_empty(), "{:?}", fx.local_tags());
    assert!(fx.remote_tags().is_empty(), "{:?}", fx.remote_tags());
    assert!(
        fx.head_subject().starts_with("Revert"),
        "{}",
        fx.head_subject()
    );
    assert_eq!(
        std::fs::read_to_string(fx.local.join("VERSION")).unwrap(),
        "1.0.0"
    );
    assert!(!fx.has_checkpoint());
}

#[test]
fn a_tag_someone_moved_on_the_remote_survives_the_rollback() {
    let fx = Fixture::new(SINGLE);
    let mut checkpoint = fx.release(&["v1.1.0"]);
    checkpoint.created_tags[0].sha = "0".repeat(40);
    fx.save(&checkpoint);

    fx.rollback(&[], true).unwrap();

    assert_eq!(fx.remote_tags(), ["v1.1.0"]);
}

#[test]
fn a_package_published_to_an_immutable_registry_blocks_the_whole_rollback() {
    let fx = Fixture::new(SINGLE);
    let mut checkpoint = fx.release(&["v1.1.0"]);
    checkpoint.published.push(RecordedPublish {
        package: "app".into(),
        kind: "cargo".into(),
        immutable: true,
    });
    fx.save(&checkpoint);
    let head = fx.head();

    let err = fx.rollback(&[], true).unwrap_err();

    assert!(
        format!("{err:#}").contains("cannot be unpublished"),
        "{err:#}"
    );
    assert_eq!(fx.head(), head);
    assert_eq!(fx.remote_tags(), ["v1.1.0"]);
    assert!(
        fx.has_checkpoint(),
        "a refused rollback must keep its record"
    );
}

#[test]
fn naming_one_package_only_removes_its_tag_and_keeps_the_release_commit() {
    let fx = Fixture::new(MONOREPO);
    let checkpoint = fx.release(&["api@v1.1.0", "web@v1.1.0"]);
    fx.save(&checkpoint);
    let head = fx.head();

    fx.rollback(&["api"], true).unwrap();

    assert_eq!(fx.remote_tags(), ["web@v1.1.0"]);
    assert_eq!(fx.local_tags(), ["web@v1.1.0"]);
    assert_eq!(
        fx.head(),
        head,
        "the release commit also bumped web, so it must not be reverted"
    );
}

#[test]
fn a_tag_no_package_claims_is_left_out_of_a_narrowed_rollback() {
    let fx = Fixture::new(MONOREPO);
    let checkpoint = fx.release(&["api@v1.1.0", "stray-1.1.0"]);
    fx.save(&checkpoint);

    fx.rollback(&["api"], true).unwrap();

    assert_eq!(fx.remote_tags(), ["stray-1.1.0"]);
}

#[test]
fn a_mutable_publish_in_a_monorepo_does_not_block_its_package() {
    let fx = Fixture::new(MONOREPO);
    let mut checkpoint = fx.release(&["api@v1.1.0", "web@v1.1.0"]);
    checkpoint.published.push(RecordedPublish {
        package: "web".into(),
        kind: "docker".into(),
        immutable: false,
    });
    checkpoint.published.push(RecordedPublish {
        package: "api".into(),
        kind: "npm".into(),
        immutable: true,
    });
    fx.save(&checkpoint);
    let head = fx.head();

    fx.rollback(&[], true).unwrap();

    assert_eq!(fx.remote_tags(), ["api@v1.1.0"]);
    assert_eq!(fx.head(), head, "a partly blocked run must not revert");
}
