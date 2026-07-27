use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use anyhow::Result;

use crate::config::Config;
use crate::forge::{Forge, MergeRequestResult, ReleaseResult};
use crate::git::{Repository, open_repo};
use crate::hooks::HookContext;

use super::checkpoint::{Checkpoint, Phase};
use super::execute::{ReleasePlan, execute_release};
use super::summary::TagToCreate;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {:?} failed: {}{}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn init_workdir(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    git(path, &["init", "-b", "main"]);
    git(path, &["config", "user.email", "test@ferrflow.dev"]);
    git(path, &["config", "user.name", "FerrFlow Test"]);
    git(path, &["config", "commit.gpgsign", "false"]);
    git(path, &["config", "tag.gpgsign", "false"]);
}

fn commit_file(path: &Path, name: &str, contents: &str, message: &str) {
    std::fs::write(path.join(name), contents).unwrap();
    git(path, &["add", name]);
    git(path, &["commit", "-m", message]);
}

/// A real local repo wired to a real bare remote, which is what the push
/// helpers need — they shell out to `git push` and inspect `ls-remote`, so
/// there is nothing meaningful to mock at that layer.
struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    remote: PathBuf,
    repo: Repository,
    config: Config,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let remote = dir.path().join("remote.git");
        std::fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "--bare", "-b", "main"]);

        let root = dir.path().join("local");
        init_workdir(&root);
        git(
            &root,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        commit_file(&root, "README.md", "hello", "chore: initial");
        git(&root, &["push", "origin", "main:main"]);

        let mut config = Config::default();
        config.workspace.remote = "origin".to_string();
        config.workspace.branch = "main".to_string();

        let repo = open_repo(&root).unwrap();
        Self {
            _dir: dir,
            root,
            remote,
            repo,
            config,
        }
    }

    fn git(&self, args: &[&str]) -> String {
        git(&self.root, args)
    }

    fn remote_tags(&self) -> Vec<String> {
        git(&self.remote, &["tag", "-l"])
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// Publishes `tag` on the remote at a commit the local repo does not have,
    /// without moving the remote branch. That is the concurrent-release shape:
    /// our branch push still fast-forwards, but the tag push collides.
    fn publish_divergent_remote_tag(&self, tag: &str) {
        let helper = self._dir.path().join("helper");
        init_workdir(&helper);
        git(
            &helper,
            &["remote", "add", "origin", self.remote.to_str().unwrap()],
        );
        commit_file(&helper, "other.txt", "concurrent", "feat: concurrent work");
        git(
            &helper,
            &["tag", "-a", tag, "-m", "published by the winner"],
        );
        git(&helper, &["push", "origin", &format!("refs/tags/{tag}")]);
    }
}

#[derive(Default)]
struct RecordingForge {
    create_calls: Mutex<Vec<String>>,
}

impl Forge for RecordingForge {
    fn create_release(
        &self,
        tag: &str,
        _body: &str,
        _prerelease: bool,
        _draft: bool,
    ) -> Result<ReleaseResult> {
        self.create_calls.lock().unwrap().push(tag.to_string());
        Ok(ReleaseResult {
            id: Some(1),
            url: Some(format!("https://forge/{tag}")),
        })
    }

    fn find_draft_release(&self, _tag: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    fn publish_release(&self, _release_id: u64) -> Result<()> {
        Ok(())
    }

    fn create_merge_request(
        &self,
        _head: &str,
        _base: &str,
        _title: &str,
        _body: &str,
    ) -> Result<MergeRequestResult> {
        unreachable!("release execution does not open merge requests in commit mode")
    }

    fn enable_auto_merge(&self, _mr: &MergeRequestResult) -> Result<()> {
        unreachable!("not exercised")
    }

    fn mr_noun(&self) -> &'static str {
        "pull request"
    }

    fn release_noun(&self) -> &'static str {
        "release"
    }

    fn find_comment(&self, _pr_id: u64, _marker: &str) -> Result<Option<u64>> {
        unreachable!("not exercised")
    }

    fn create_comment(&self, _pr_id: u64, _body: &str) -> Result<()> {
        unreachable!("not exercised")
    }

    fn update_comment(&self, _pr_id: u64, _comment_id: u64, _body: &str) -> Result<()> {
        unreachable!("not exercised")
    }

    fn find_open_pr(&self, _head: &str, _base: &str) -> Result<Option<u64>> {
        unreachable!("not exercised")
    }

    fn update_merge_request(
        &self,
        _id: u64,
        _title: &str,
        _body: &str,
    ) -> Result<MergeRequestResult> {
        unreachable!("not exercised")
    }
}

fn tag_to_create(tag: &str, pkg: &str, version: &str) -> TagToCreate {
    (
        tag.to_string(),
        format!("{pkg} {version}"),
        "release notes".to_string(),
        pkg.to_string(),
        version.to_string(),
        1,
        false,
    )
}

/// Drives the publish half of `execute_release`. The checkpoint starts at
/// `TagsCreated` so the commit and tag-creation phases are skipped — the test
/// creates the tags itself — and execution lands directly on push → publish.
fn run_publish_phase(
    harness: &Harness,
    tags: &[TagToCreate],
    forge: &dyn Forge,
) -> (Result<()>, Vec<(String, ReleaseResult)>) {
    let mut checkpoint = Checkpoint::new(
        harness.git(&["rev-parse", "HEAD"]).trim().to_string(),
        tags.iter().map(|(t, ..)| t.clone()).collect(),
    );
    checkpoint.advance(Phase::TagsCreated);

    let hook_contexts: Vec<(HookContext, usize)> = Vec::new();
    let mut files_to_commit: Vec<String> = Vec::new();
    let mut files_per_package: HashMap<String, Vec<String>> = HashMap::new();
    let mut pkg_outputs: Vec<(String, Vec<String>)> = Vec::new();
    let mut shared_outputs: Vec<String> = Vec::new();
    let mut forge_results: Vec<(String, ReleaseResult)> = Vec::new();

    let result = {
        let mut plan = ReleasePlan {
            repo: &harness.repo,
            config: &harness.config,
            root: &harness.root,
            target_branch: "main",
            dry_run: false,
            verbose: false,
            force: false,
            draft: false,
            tags_to_create: tags,
            hook_contexts: &hook_contexts,
            files_to_commit: &mut files_to_commit,
            files_per_package: &mut files_per_package,
            pkg_outputs: &mut pkg_outputs,
            shared_outputs: &mut shared_outputs,
            forge_results: &mut forge_results,
            checkpoint: Some(&mut checkpoint),
            forge: Some(forge),
        };
        execute_release(&mut plan)
    };

    (result, forge_results)
}

// The guarantee behind #770: git is the source of truth and lands first, so a
// failed tag push must leave the forge untouched. Before the reorder, releases
// were published first and this scenario left N releases pointing at a tag the
// remote never received.
#[test]
fn a_failed_tag_push_publishes_no_release() {
    let harness = Harness::new();
    let tag = "app@v1.0.0";

    harness.publish_divergent_remote_tag(tag);
    harness.git(&["tag", "-a", tag, "-m", "app 1.0.0"]);

    let tags = vec![tag_to_create(tag, "app", "1.0.0")];
    let forge = RecordingForge::default();
    let (result, forge_results) = run_publish_phase(&harness, &tags, &forge);

    let err = result.expect_err("pushing a tag that diverged on the remote must fail");
    assert!(
        format!("{err:#}").contains("already exist on remote"),
        "expected the divergent-tag diagnosis, got: {err:#}"
    );

    assert!(
        forge.create_calls.lock().unwrap().is_empty(),
        "no release may be published when the tag never reached the remote"
    );
    assert!(forge_results.is_empty());
}

#[test]
fn a_successful_push_lands_the_tag_then_publishes_it() {
    let harness = Harness::new();
    let tag = "app@v1.0.0";

    harness.git(&["tag", "-a", tag, "-m", "app 1.0.0"]);

    let tags = vec![tag_to_create(tag, "app", "1.0.0")];
    let forge = RecordingForge::default();
    let (result, forge_results) = run_publish_phase(&harness, &tags, &forge);

    result.expect("push and publish should both succeed");

    assert_eq!(harness.remote_tags(), vec![tag.to_string()]);
    assert_eq!(
        *forge.create_calls.lock().unwrap(),
        vec![tag.to_string()],
        "the release is created exactly once, after the tag landed"
    );
    assert_eq!(forge_results.len(), 1);
}

// The annotated tag carries the changelog body. #770 removed `target_commitish`
// so the forge can no longer mint a lightweight tag of its own that shadows it.
#[test]
fn the_pushed_tag_keeps_its_annotation() {
    let harness = Harness::new();
    let tag = "app@v1.0.0";

    harness.git(&["tag", "-a", tag, "-m", "app 1.0.0 release notes"]);

    let tags = vec![tag_to_create(tag, "app", "1.0.0")];
    let forge = RecordingForge::default();
    let (result, _) = run_publish_phase(&harness, &tags, &forge);
    result.expect("push and publish should both succeed");

    let kind = git(&harness.remote, &["cat-file", "-t", tag]);
    assert_eq!(
        kind.trim(),
        "tag",
        "an annotated tag object must land on the remote, not a lightweight ref"
    );
    let message = git(&harness.remote, &["tag", "-l", "-n99", tag]);
    assert!(
        message.contains("release notes"),
        "the annotation must survive the push, got: {message}"
    );
}
