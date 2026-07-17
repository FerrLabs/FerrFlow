use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error_code::{self, ErrorCodeExt};

const CHECKPOINT_FILENAME: &str = "ferrflow.checkpoint.json";
const CHECKPOINT_SCHEMA: u32 = 1;

/// Coarse-grained sequence of release phases, ordered by the side
/// effects they produce. The checkpoint records the highest phase that
/// completed successfully so a resume can skip back up to that point.
///
/// Why coarse-grained? The release pipeline already groups related side
/// effects (all tags created together, all tags pushed together, all
/// releases created together). Tracking finer per-tag state means
/// duplicating the orchestration logic in two places — release-time and
/// resume-time — and a mid-batch crash inside any single step (e.g.
/// 5 of 10 tags pushed) is rare in practice and still recoverable
/// because the underlying git/forge operations are idempotent: pushing
/// an already-existing tag, or creating an already-existing release, is
/// a no-op that the existing code already handles gracefully. So we
/// rewind to the start of the *phase*, not the start of the *item*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Pending,
    CommitDone,
    TagsCreated,
    Pushed,
    ReleasesCreated,
    PostPublishDone,
}

/// On-disk crash-resume marker for `ferrflow release`.
///
/// Written progressively at `.git/ferrflow.checkpoint.json` as each
/// phase of the release succeeds. On the next run we load it back, and
/// if HEAD still matches we skip every phase up to and including the
/// recorded one. Cleared on successful completion of the release.
///
/// HEAD-pinning is what makes the resume safe: if the user (or another
/// process) advanced HEAD between runs, the recorded commit_sha won't
/// match anymore and we refuse to auto-resume — the user has to delete
/// the checkpoint manually or rerun against the recorded commit. This
/// avoids the worst failure mode (replaying old tags onto a new commit
/// graph). See #549.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub schema_version: u32,
    /// The HEAD commit SHA observed when the release started — what the
    /// in-progress release was operating on. Used to detect "HEAD
    /// moved" on resume.
    pub head_sha: String,
    /// Unix-epoch seconds, for debugging. Not used by the
    /// resume policy itself.
    pub started_at: u64,
    /// Highest phase that finished without erroring.
    pub phase: Phase,
    /// The release commit's SHA, populated once the commit phase
    /// finishes. Surfaces in the resume log line so the user can verify
    /// the resume is operating on what they expect.
    pub commit_sha: Option<String>,
    /// Tags that this release expects to create. Recorded at start so a
    /// resume can sanity-check that the in-flight release matches the
    /// tag set we'd recompute today.
    pub tag_names: Vec<String>,
}

impl Checkpoint {
    pub fn new(head_sha: String, tag_names: Vec<String>) -> Self {
        Self {
            schema_version: CHECKPOINT_SCHEMA,
            head_sha,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            phase: Phase::Pending,
            commit_sha: None,
            tag_names,
        }
    }

    pub fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(".git").join(CHECKPOINT_FILENAME)
    }

    /// Returns Ok(Some(_)) when a parseable checkpoint exists, Ok(None)
    /// when the file is absent, and Err on a corrupt file — callers
    /// should surface the corruption to the user rather than silently
    /// nuking work-in-progress.
    pub fn load(repo_root: &Path) -> Result<Option<Self>> {
        let path = Self::path(repo_root);
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Cannot read {}", path.display()))
                    .error_code(error_code::JSON_READ);
            }
        };
        let cp: Checkpoint = serde_json::from_str(&raw)
            .with_context(|| {
                format!(
                    "Cannot parse {} — delete it manually if you want a fresh release",
                    path.display()
                )
            })
            .error_code(error_code::JSON_PARSE)?;
        if cp.schema_version != CHECKPOINT_SCHEMA {
            return Err(anyhow::anyhow!(
                "checkpoint at {} has schema {} but this build expects {}; delete it to start fresh",
                path.display(),
                cp.schema_version,
                CHECKPOINT_SCHEMA
            ))
            .error_code(error_code::JSON_PARSE);
        }
        Ok(Some(cp))
    }

    /// Atomic write via temp + rename so a crash mid-save leaves the
    /// previous checkpoint intact rather than a half-written one.
    pub fn save(&self, repo_root: &Path) -> Result<()> {
        let final_path = Self::path(repo_root);
        let tmp_path = final_path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(self)
            .with_context(|| "serialize release checkpoint")
            .error_code(error_code::JSON_WRITE)?;
        std::fs::write(&tmp_path, bytes)
            .with_context(|| format!("write {}", tmp_path.display()))
            .error_code(error_code::JSON_WRITE)?;
        std::fs::rename(&tmp_path, &final_path)
            .with_context(|| format!("rename to {}", final_path.display()))
            .error_code(error_code::JSON_WRITE)?;
        Ok(())
    }

    pub fn delete(repo_root: &Path) -> Result<()> {
        let path = Self::path(repo_root);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e)
                .with_context(|| format!("delete {}", path.display()))
                .error_code(error_code::JSON_WRITE),
        }
    }

    pub fn advance(&mut self, phase: Phase) {
        if phase > self.phase {
            self.phase = phase;
        }
    }

    pub fn is_done(&self, phase: Phase) -> bool {
        self.phase >= phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        dir
    }

    #[test]
    fn load_returns_none_when_no_file() {
        let dir = init_test_repo();
        assert!(Checkpoint::load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = init_test_repo();
        let mut cp = Checkpoint::new("abc123".to_string(), vec!["v1.0.0".to_string()]);
        cp.advance(Phase::CommitDone);
        cp.commit_sha = Some("def456".to_string());
        cp.save(dir.path()).unwrap();
        let loaded = Checkpoint::load(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.head_sha, "abc123");
        assert_eq!(loaded.phase, Phase::CommitDone);
        assert_eq!(loaded.commit_sha.as_deref(), Some("def456"));
        assert_eq!(loaded.tag_names, vec!["v1.0.0".to_string()]);
    }

    #[test]
    fn delete_removes_the_file() {
        let dir = init_test_repo();
        let cp = Checkpoint::new("abc".into(), vec![]);
        cp.save(dir.path()).unwrap();
        assert!(Checkpoint::path(dir.path()).exists());
        Checkpoint::delete(dir.path()).unwrap();
        assert!(!Checkpoint::path(dir.path()).exists());
    }

    #[test]
    fn delete_when_absent_is_noop() {
        let dir = init_test_repo();
        Checkpoint::delete(dir.path()).expect("absent delete must be Ok");
    }

    #[test]
    fn corrupt_file_surfaces_an_error() {
        let dir = init_test_repo();
        std::fs::write(Checkpoint::path(dir.path()), "{ not valid json").unwrap();
        let err = Checkpoint::load(dir.path()).expect_err("parse error expected");
        let msg = format!("{err:?}");
        assert!(msg.contains("parse") || msg.contains("delete it manually"));
    }

    #[test]
    fn schema_mismatch_is_rejected() {
        let dir = init_test_repo();
        let payload = r#"{
            "schema_version": 999,
            "head_sha": "abc",
            "started_at": 0,
            "phase": "pending",
            "commit_sha": null,
            "tag_names": []
        }"#;
        std::fs::write(Checkpoint::path(dir.path()), payload).unwrap();
        let err = Checkpoint::load(dir.path()).expect_err("schema mismatch expected");
        assert!(format!("{err:?}").contains("schema"));
    }

    #[test]
    fn advance_only_moves_forward() {
        let mut cp = Checkpoint::new("a".into(), vec![]);
        cp.advance(Phase::TagsCreated);
        assert_eq!(cp.phase, Phase::TagsCreated);
        // Trying to "advance" back to a smaller phase is a no-op.
        cp.advance(Phase::Pending);
        assert_eq!(cp.phase, Phase::TagsCreated);
    }

    #[test]
    fn phases_are_strictly_ordered() {
        assert!(Phase::Pending < Phase::CommitDone);
        assert!(Phase::CommitDone < Phase::TagsCreated);
        assert!(Phase::TagsCreated < Phase::Pushed);
        assert!(Phase::Pushed < Phase::ReleasesCreated);
        assert!(Phase::ReleasesCreated < Phase::PostPublishDone);
    }

    #[test]
    fn is_done_threshold_check() {
        let mut cp = Checkpoint::new("a".into(), vec![]);
        assert!(cp.is_done(Phase::Pending));
        assert!(!cp.is_done(Phase::CommitDone));
        cp.advance(Phase::TagsCreated);
        assert!(cp.is_done(Phase::CommitDone));
        assert!(cp.is_done(Phase::TagsCreated));
        assert!(!cp.is_done(Phase::Pushed));
    }

    #[test]
    fn save_is_atomic_via_tmp_rename() {
        let dir = init_test_repo();
        let cp = Checkpoint::new("a".into(), vec![]);
        cp.save(dir.path()).unwrap();
        // After save, no .tmp file is left behind.
        let tmp = Checkpoint::path(dir.path()).with_extension("json.tmp");
        assert!(!tmp.exists(), "tmp file should be cleaned up after rename");
    }
}
