use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::git::Repository;

const CACHE_DIR_NAME: &str = "ferrflow-cache";
const MAX_AGE: Duration = Duration::from_secs(5 * 60);
const PRUNE_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const PRUNE_MAX_ENTRIES: usize = 50;
/// A staged write is renamed within milliseconds. One that is still sitting
/// there an hour later is the residue of a killed process, not work in flight.
const TMP_MAX_AGE: Duration = Duration::from_secs(60 * 60);

#[derive(Serialize, Deserialize)]
pub struct CachedRun {
    pub json: Option<String>,
    pub text_lines: Vec<String>,
}

pub struct CacheKey {
    head: String,
    tags_hash: String,
    config_hash: String,
    files_hash: String,
    variant: &'static str,
}

impl CacheKey {
    fn filename(&self) -> String {
        let joined = [
            self.head.as_str(),
            &self.tags_hash,
            &self.config_hash,
            &self.files_hash,
            self.variant,
        ]
        .join("\n");
        format!("{}.json", sha256_hex(joined.as_bytes()))
    }
}

pub fn cache_dir(repo: &Repository) -> PathBuf {
    repo.git_dir().join(CACHE_DIR_NAME)
}

pub fn compute_key(
    repo: &Repository,
    root: &Path,
    config: &Config,
    explicit_config: Option<&Path>,
    variant: &'static str,
) -> Option<CacheKey> {
    let head = repo.head_id().ok()?.to_string();
    let tags_hash = hash_tag_refs(repo);
    let config_hash = hash_config(root, explicit_config);
    let files_hash = hash_versioned_files(config, root);
    Some(CacheKey {
        head,
        tags_hash,
        config_hash,
        files_hash,
        variant,
    })
}

/// Hashes the contents of every configured versioned file.
///
/// `check` reports the current version by reading these files, not by reading
/// git, so a key built only from HEAD, tags and the config serves a stale plan
/// after a hand-edited version or a release that wrote files without
/// committing. That is a wrong answer delivered confidently, which is worse
/// than a slow one.
///
/// Contents rather than mtime and size: `1.0.0` and `1.1.0` are the same
/// length, so size alone misses the realistic edit, and mtime granularity is
/// coarse on some filesystems, which would make both the check and its test
/// depend on timing. Reading a handful of small manifests is still far cheaper
/// than the history walk this cache exists to avoid.
fn hash_versioned_files(config: &Config, root: &Path) -> String {
    let mut hasher = Sha256::new();
    for pkg in &config.packages {
        for file in &pkg.versioned_files {
            let path = root.join(&file.path);
            hasher.update(file.path.as_bytes());
            match std::fs::read(&path) {
                Ok(bytes) => hasher.update(&bytes),
                // A missing file is itself part of the state: it must hash
                // differently from the same file present.
                Err(_) => hasher.update(b"<missing>"),
            }
        }
    }
    hex::encode(hasher.finalize())
}

fn hash_tag_refs(repo: &Repository) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Ok(references) = repo.references()
        && let Ok(tags) = references.tags()
    {
        for reference in tags.flatten() {
            let name = String::from_utf8_lossy(reference.name().as_bstr()).into_owned();
            let oid = reference.id().detach().to_string();
            lines.push(format!("{name}={oid}"));
        }
    }
    lines.sort();
    sha256_hex(lines.join("\n").as_bytes())
}

fn hash_config(root: &Path, explicit_config: Option<&Path>) -> String {
    match Config::source_path(root, explicit_config) {
        Some(path) => match std::fs::read(&path) {
            Ok(bytes) => sha256_hex(&bytes),
            Err(_) => sha256_hex(b""),
        },
        None => sha256_hex(b""),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn read(dir: &Path, key: &CacheKey) -> Option<CachedRun> {
    let path = dir.join(key.filename());
    let metadata = std::fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    if !is_fresh(modified, SystemTime::now()) {
        return None;
    }
    let bytes = std::fs::read(&path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn is_fresh(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified)
        .map(|age| age <= MAX_AGE)
        .unwrap_or(true)
}

pub fn write(dir: &Path, key: &CacheKey, run: &CachedRun) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(serialized) = serde_json::to_vec(run) else {
        return;
    };
    let final_path = dir.join(key.filename());
    let tmp_path = dir.join(format!("{}.{}.tmp", key.filename(), std::process::id()));
    if std::fs::write(&tmp_path, &serialized).is_err() {
        let _ = std::fs::remove_file(&tmp_path);
        return;
    }
    if std::fs::rename(&tmp_path, &final_path).is_err() {
        let _ = std::fs::remove_file(&tmp_path);
        return;
    }
    prune(dir);
}

pub fn clear(repo: &Repository) -> Result<()> {
    let dir = cache_dir(repo);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

pub fn clear_cwd() -> Result<()> {
    let repo = crate::git::open_repo(&std::env::current_dir()?)?;
    let dir = cache_dir(&repo);
    clear(&repo)?;
    tracing::info!("Cleared FerrFlow cache at {}", dir.display());
    Ok(())
}

fn prune(dir: &Path) {
    prune_at(dir, SystemTime::now());
}

fn prune_at(dir: &Path, now: SystemTime) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<(PathBuf, SystemTime)> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        let extension = path.extension().and_then(|e| e.to_str());
        if extension != Some("json") && extension != Some("tmp") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let age = now.duration_since(modified).ok();

        // Orphaned staging files from a run that died between the write and the
        // rename. They carry no extension the cache reads, so nothing else ever
        // removed them and the directory grew without bound.
        if extension == Some("tmp") {
            if age.map(|age| age > TMP_MAX_AGE).unwrap_or(false) {
                let _ = std::fs::remove_file(&path);
            }
            continue;
        }

        if age.map(|age| age > PRUNE_MAX_AGE).unwrap_or(false) {
            let _ = std::fs::remove_file(&path);
            continue;
        }
        entries.push((path, modified));
    }

    if entries.len() > PRUNE_MAX_ENTRIES {
        entries.sort_by_key(|(_, modified)| *modified);
        let excess = entries.len() - PRUNE_MAX_ENTRIES;
        for (path, _) in entries.into_iter().take(excess) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(head: &str, tags: &str, config: &str) -> CacheKey {
        CacheKey {
            head: head.to_string(),
            tags_hash: tags.to_string(),
            config_hash: config.to_string(),
            files_hash: "f".to_string(),
            variant: "text",
        }
    }

    fn sample() -> CachedRun {
        CachedRun {
            json: Some("{\"packages\":[]}".to_string()),
            text_lines: vec!["● api  1.0.0 → 1.1.0  (minor)".to_string()],
        }
    }

    #[test]
    fn round_trips_through_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let k = key("head", "tags", "config");
        write(dir.path(), &k, &sample());
        let got = read(dir.path(), &k).expect("cache hit");
        assert_eq!(got.json.as_deref(), Some("{\"packages\":[]}"));
        assert_eq!(got.text_lines, sample().text_lines);
    }

    #[test]
    fn miss_when_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path(), &key("nope", "nope", "nope")).is_none());
    }

    #[test]
    fn tampered_file_falls_back_to_miss() {
        let dir = tempfile::tempdir().unwrap();
        let k = key("head", "tags", "config");
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join(k.filename()), b"{ this is not json").unwrap();
        assert!(read(dir.path(), &k).is_none());
    }

    #[test]
    fn is_fresh_window() {
        let now = SystemTime::now();
        assert!(is_fresh(now, now));
        assert!(is_fresh(now - Duration::from_secs(60), now));
        assert!(!is_fresh(now - (MAX_AGE + Duration::from_secs(1)), now));
    }

    #[test]
    fn write_to_readonly_parent_is_noop_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing_parent = dir.path().join("does-not-exist");
        std::fs::write(&missing_parent, b"i am a file, not a dir").unwrap();
        let cache_dir = missing_parent.join("cache");
        let k = key("head", "tags", "config");
        write(&cache_dir, &k, &sample());
        assert!(read(&cache_dir, &k).is_none());
    }

    #[test]
    fn filename_changes_with_each_key_component() {
        let base = key("h", "t", "c").filename();
        assert_ne!(base, key("h2", "t", "c").filename());
        assert_ne!(base, key("h", "t2", "c").filename());
        assert_ne!(base, key("h", "t", "c2").filename());
        let json_variant = CacheKey {
            head: "h".into(),
            tags_hash: "t".into(),
            config_hash: "c".into(),
            files_hash: "f".into(),
            variant: "json",
        };
        assert_ne!(base, json_variant.filename());
    }

    #[test]
    fn a_full_size_key_round_trips_with_a_seven_digit_pid_suffix() {
        let hash = "a".repeat(64);
        let k = CacheKey {
            head: hash.clone(),
            tags_hash: hash.clone(),
            config_hash: hash.clone(),
            files_hash: hash,
            variant: "text",
        };
        assert!(k.filename().len() + ".4194304.tmp".len() <= 255);

        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), &k, &sample());
        assert!(read(dir.path(), &k).is_some());
    }

    #[test]
    fn prune_caps_total_entries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        for i in 0..(PRUNE_MAX_ENTRIES + 5) {
            std::fs::write(dir.path().join(format!("entry-{i}.json")), b"{}").unwrap();
        }
        prune(dir.path());
        let remaining = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(remaining, PRUNE_MAX_ENTRIES);
    }

    #[test]
    fn prune_drops_entries_older_than_seven_days() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join("a.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("b.json"), b"{}").unwrap();
        let far_future = SystemTime::now() + PRUNE_MAX_AGE + Duration::from_secs(60);
        prune_at(dir.path(), far_future);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    mod key {
        use super::super::*;
        use crate::test_utils::{commit_file, git, init_repo};

        fn write_config(dir: &std::path::Path, body: &str) {
            std::fs::write(dir.join(".ferrflow"), body).unwrap();
        }

        fn filename(repo: &Repository, root: &std::path::Path) -> String {
            let config = Config::load(root, None).expect("config");
            compute_key(repo, root, &config, None, "text")
                .expect("key")
                .filename()
        }

        #[test]
        fn key_is_stable_for_unchanged_inputs() {
            let (dir, repo) = init_repo();
            write_config(dir.path(), r#"{"package":[{"name":"a","path":"."}]}"#);
            commit_file(dir.path(), "f.txt", "x", "feat: a", 1_900_000_000);
            assert_eq!(filename(&repo, dir.path()), filename(&repo, dir.path()));
        }

        #[test]
        fn key_busts_on_new_head() {
            let (dir, repo) = init_repo();
            write_config(dir.path(), r#"{"package":[{"name":"a","path":"."}]}"#);
            commit_file(dir.path(), "f.txt", "x", "feat: a", 1_900_000_000);
            let before = filename(&repo, dir.path());
            commit_file(dir.path(), "g.txt", "y", "feat: b", 1_900_000_001);
            assert_ne!(before, filename(&repo, dir.path()));
        }

        #[test]
        fn key_busts_on_new_tag() {
            let (dir, repo) = init_repo();
            write_config(dir.path(), r#"{"package":[{"name":"a","path":"."}]}"#);
            commit_file(dir.path(), "f.txt", "x", "feat: a", 1_900_000_000);
            let before = filename(&repo, dir.path());
            git(dir.path(), &["tag", "v1.0.0"]);
            assert_ne!(before, filename(&repo, dir.path()));
        }

        #[test]
        fn key_busts_on_config_edit() {
            let (dir, repo) = init_repo();
            write_config(dir.path(), r#"{"package":[{"name":"a","path":"."}]}"#);
            commit_file(dir.path(), "f.txt", "x", "feat: a", 1_900_000_000);
            let before = filename(&repo, dir.path());
            write_config(
                dir.path(),
                r#"{"package":[{"name":"a","path":".","versioning":"calver"}]}"#,
            );
            assert_ne!(before, filename(&repo, dir.path()));
        }

        /// A config with one package whose version lives in `package.json`,
        /// which is what `check` reads to render the current version.
        fn write_versioned_config(dir: &std::path::Path) {
            write_config(
                dir,
                r#"{"package":[{"name":"a","path":".","versionedFiles":[{"path":"package.json","format":"json"}]}]}"#,
            );
        }

        #[test]
        fn key_busts_on_an_uncommitted_version_edit() {
            // `check` reads the current version off disk, so a hand edit or a
            // release that wrote files without committing changes the answer
            // while HEAD, the tags and the config all stay put.
            let (dir, repo) = init_repo();
            write_versioned_config(dir.path());
            commit_file(
                dir.path(),
                "package.json",
                r#"{"name":"a","version":"1.0.0"}"#,
                "feat: a",
                1_900_000_000,
            );
            let before = filename(&repo, dir.path());

            std::fs::write(
                dir.path().join("package.json"),
                r#"{"name":"a","version":"1.1.0"}"#,
            )
            .unwrap();

            assert_ne!(
                before,
                filename(&repo, dir.path()),
                "a stale plan served for five minutes is a wrong answer, not a slow one"
            );
        }

        #[test]
        fn key_is_stable_when_the_versioned_file_is_untouched() {
            let (dir, repo) = init_repo();
            write_versioned_config(dir.path());
            commit_file(
                dir.path(),
                "package.json",
                r#"{"name":"a","version":"1.0.0"}"#,
                "feat: a",
                1_900_000_000,
            );

            assert_eq!(
                filename(&repo, dir.path()),
                filename(&repo, dir.path()),
                "hashing contents must not make the key depend on when it was computed"
            );
        }

        #[test]
        fn key_busts_when_a_versioned_file_disappears() {
            let (dir, repo) = init_repo();
            write_versioned_config(dir.path());
            commit_file(
                dir.path(),
                "package.json",
                r#"{"name":"a","version":"1.0.0"}"#,
                "feat: a",
                1_900_000_000,
            );
            let before = filename(&repo, dir.path());

            std::fs::remove_file(dir.path().join("package.json")).unwrap();

            assert_ne!(
                before,
                filename(&repo, dir.path()),
                "an absent file is state too, and must not hash like the file being present"
            );
        }
    }

    #[test]
    fn prune_removes_temp_files_left_by_a_killed_run() {
        // A staged write that never got renamed. It carries no extension the
        // cache reads, so before this nothing removed it and the directory grew
        // without bound on a repo with interrupted runs.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("scratch.1234.tmp"), b"{}").unwrap();

        prune_at(
            dir.path(),
            SystemTime::now() + TMP_MAX_AGE + Duration::from_secs(60),
        );

        assert!(!dir.path().join("scratch.1234.tmp").exists());
    }

    #[test]
    fn prune_leaves_a_temp_file_a_write_may_still_be_using() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("scratch.1234.tmp"), b"{}").unwrap();

        prune_at(dir.path(), SystemTime::now());

        assert!(
            dir.path().join("scratch.1234.tmp").exists(),
            "a rename in flight must not have its staging file pulled out from under it"
        );
    }

    #[test]
    fn prune_still_ignores_files_that_are_neither_json_nor_temp() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), b"not ours").unwrap();

        prune_at(
            dir.path(),
            SystemTime::now() + PRUNE_MAX_AGE + Duration::from_secs(60),
        );

        assert!(dir.path().join("README.md").exists());
    }

    #[test]
    fn prune_drops_an_old_json_entry_and_an_old_temp_file_together() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("keep.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("scratch.tmp"), b"{}").unwrap();

        prune_at(
            dir.path(),
            SystemTime::now() + PRUNE_MAX_AGE + Duration::from_secs(60),
        );

        assert!(!dir.path().join("keep.json").exists());
        assert!(!dir.path().join("scratch.tmp").exists());
    }
}
