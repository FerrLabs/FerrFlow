use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;

use super::publish_drafts_with;
use crate::config::Config;
use crate::forge::{Forge, MergeRequestResult, ReleaseResult};

#[derive(Default)]
struct DraftForge {
    drafts: HashMap<String, u64>,
    lookup_fails_for: Vec<String>,
    publish_fails: bool,
    lookups: Mutex<Vec<String>>,
    published: Mutex<Vec<u64>>,
}

impl Forge for DraftForge {
    fn create_release(
        &self,
        _tag: &str,
        _body: &str,
        _prerelease: bool,
        _draft: bool,
    ) -> Result<ReleaseResult> {
        unreachable!("pending drafts are published, never created")
    }

    fn find_draft_release(&self, tag: &str) -> Result<Option<u64>> {
        self.lookups.lock().unwrap().push(tag.to_string());
        if self.lookup_fails_for.iter().any(|t| t == tag) {
            anyhow::bail!("lookup failed for {tag}");
        }
        Ok(self.drafts.get(tag).copied())
    }

    fn publish_release(&self, release_id: u64) -> Result<()> {
        if self.publish_fails {
            anyhow::bail!("publish refused");
        }
        self.published.lock().unwrap().push(release_id);
        Ok(())
    }

    fn create_merge_request(
        &self,
        _head: &str,
        _base: &str,
        _title: &str,
        _body: &str,
    ) -> Result<MergeRequestResult> {
        unreachable!("not exercised")
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

fn write_version(root: &Path, pkg: &str, version: &str) {
    std::fs::create_dir_all(root.join(pkg)).unwrap();
    std::fs::write(
        root.join(pkg).join("package.json"),
        format!(r#"{{"name":"{pkg}","version":"{version}"}}"#),
    )
    .unwrap();
}

fn monorepo_config() -> Config {
    serde_json::from_str(
        r#"{"package":[
            {"name":"api","path":"api","versionedFiles":[{"path":"api/package.json","format":"json"}]},
            {"name":"web","path":"web","versionedFiles":[{"path":"web/package.json","format":"json"}]},
            {"name":"docs","path":"docs"}
        ]}"#,
    )
    .unwrap()
}

fn run(forge: &DraftForge, config: &Config, root: &Path) -> Vec<String> {
    let mut outputs = Vec::new();
    publish_drafts_with(forge, config, root, false, &mut outputs);
    outputs
}

#[test]
fn a_draft_for_the_current_version_is_published() {
    let dir = tempfile::tempdir().unwrap();
    write_version(dir.path(), "api", "1.2.0");
    write_version(dir.path(), "web", "3.0.0");
    let forge = DraftForge {
        drafts: HashMap::from([("api@v1.2.0".to_string(), 7)]),
        ..Default::default()
    };

    let outputs = run(&forge, &monorepo_config(), dir.path());

    assert_eq!(*forge.published.lock().unwrap(), vec![7]);
    assert_eq!(outputs.len(), 1, "{outputs:?}");
    assert!(
        outputs[0].contains("Published draft release"),
        "{outputs:?}"
    );
    assert!(outputs[0].contains("api@v1.2.0"), "{outputs:?}");
}

#[test]
fn only_packages_with_a_readable_version_are_looked_up() {
    let dir = tempfile::tempdir().unwrap();
    write_version(dir.path(), "api", "1.2.0");
    let forge = DraftForge::default();

    let outputs = run(&forge, &monorepo_config(), dir.path());

    assert!(outputs.is_empty(), "{outputs:?}");
    assert_eq!(
        *forge.lookups.lock().unwrap(),
        vec!["api@v1.2.0".to_string()],
        "web has no version file on disk and docs declares none"
    );
    assert!(forge.published.lock().unwrap().is_empty());
}

#[test]
fn a_single_package_repo_looks_up_the_unprefixed_tag() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name":"app","version":"0.4.1"}"#,
    )
    .unwrap();
    let config: Config = serde_json::from_str(
        r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"package.json","format":"json"}]}]}"#,
    )
    .unwrap();
    let forge = DraftForge {
        drafts: HashMap::from([("v0.4.1".to_string(), 3)]),
        ..Default::default()
    };

    run(&forge, &config, dir.path());

    assert_eq!(*forge.published.lock().unwrap(), vec![3]);
}

#[test]
fn a_failed_publish_reports_nothing_and_does_not_stop_the_others() {
    let dir = tempfile::tempdir().unwrap();
    write_version(dir.path(), "api", "1.2.0");
    write_version(dir.path(), "web", "3.0.0");
    let forge = DraftForge {
        drafts: HashMap::from([("api@v1.2.0".to_string(), 7), ("web@v3.0.0".to_string(), 8)]),
        publish_fails: true,
        ..Default::default()
    };

    let outputs = run(&forge, &monorepo_config(), dir.path());

    assert!(outputs.is_empty(), "{outputs:?}");
    assert_eq!(forge.lookups.lock().unwrap().len(), 2);
}

#[test]
fn a_failed_lookup_moves_on_to_the_next_package() {
    let dir = tempfile::tempdir().unwrap();
    write_version(dir.path(), "api", "1.2.0");
    write_version(dir.path(), "web", "3.0.0");
    let forge = DraftForge {
        drafts: HashMap::from([("web@v3.0.0".to_string(), 8)]),
        lookup_fails_for: vec!["api@v1.2.0".to_string()],
        ..Default::default()
    };

    let outputs = run(&forge, &monorepo_config(), dir.path());

    assert_eq!(*forge.published.lock().unwrap(), vec![8]);
    assert_eq!(outputs.len(), 1);
    assert!(outputs[0].contains("web@v3.0.0"), "{outputs:?}");
}
