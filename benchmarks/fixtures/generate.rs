// Generates synthetic git repos for benchmarking ferrflow.
//
// Usage: generate-fixtures [output_dir]
//
// Creates four fixtures:
//   single/       - single-package repo, 100 commits
//   mono-small/   - 10 packages, 100 commits
//   mono-medium/  - 50 packages, 500 commits
//   mono-large/   - 200 packages, 10000 commits
//
// Builds trees incrementally: only the changed subtree is rebuilt per commit.
// mono-large (200 packages, 10k commits) finishes in seconds.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use git2::{FileMode, Oid, Repository, Signature, Time};

const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "refactor", "perf", "chore", "docs", "ci", "test",
];
const SCOPES: &[&str] = &[
    "core", "api", "cli", "config", "parser", "auth", "db", "cache", "logging", "events",
];
const WORDS_A: &[&str] = &[
    "update",
    "add",
    "remove",
    "refactor",
    "improve",
    "fix",
    "handle",
    "support",
    "implement",
    "optimize",
];
const WORDS_B: &[&str] = &[
    "feature",
    "endpoint",
    "handler",
    "logic",
    "validation",
    "error",
    "check",
    "flow",
    "config",
    "output",
];

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn usize(&mut self, max: usize) -> usize {
        (self.next_u64() % max as u64) as usize
    }

    fn pick<'a>(&mut self, items: &'a [&str]) -> &'a str {
        items[self.usize(items.len())]
    }
}

fn rand_message(rng: &mut Rng, scope: &str) -> String {
    let t = rng.pick(COMMIT_TYPES);
    let bang = if rng.usize(20) == 0 { "!" } else { "" };
    let a = rng.pick(WORDS_A);
    let b = rng.pick(WORDS_B);
    format!("{t}({scope}){bang}: {a} {b}")
}

fn rand_time(rng: &mut Rng, now: i64) -> Time {
    let days = rng.usize(365) as i64;
    let hours = rng.usize(24) as i64;
    let mins = rng.usize(60) as i64;
    let offset = days * 86400 + hours * 3600 + mins * 60;
    Time::new(now - offset, 0)
}

fn sig(time: &Time) -> Signature<'_> {
    Signature::new("FerrFlow Bench", "bench@ferrflow.dev", time).unwrap()
}

// A cached tree node. Children are either blobs (leaf) or subtrees.
enum Entry {
    Blob(Oid),
    Tree(TreeNode),
}

struct TreeNode {
    entries: HashMap<String, Entry>,
    cached_oid: Option<Oid>,
}

impl TreeNode {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            cached_oid: None,
        }
    }

    fn invalidate(&mut self) {
        self.cached_oid = None;
    }

    // Insert a blob at a path (possibly nested). Invalidates caches along the way.
    fn insert_blob(&mut self, path: &str, blob_oid: Oid) {
        self.invalidate();
        if let Some(slash) = path.find('/') {
            let dir = &path[..slash];
            let rest = &path[slash + 1..];
            let child = self
                .entries
                .entry(dir.to_string())
                .or_insert_with(|| Entry::Tree(TreeNode::new()));
            match child {
                Entry::Tree(node) => node.insert_blob(rest, blob_oid),
                _ => panic!("path conflict: {dir} is a blob, not a tree"),
            }
        } else {
            self.entries.insert(path.to_string(), Entry::Blob(blob_oid));
        }
    }

    // Write this tree (and any dirty subtrees) to the repo. Reuses cached OIDs.
    fn write(&mut self, repo: &Repository) -> Result<Oid> {
        if let Some(oid) = self.cached_oid {
            return Ok(oid);
        }

        let mut builder = repo.treebuilder(None)?;
        for (name, entry) in &mut self.entries {
            match entry {
                Entry::Blob(oid) => {
                    builder.insert(name, *oid, FileMode::Blob.into())?;
                }
                Entry::Tree(node) => {
                    let oid = node.write(repo)?;
                    builder.insert(name, oid, FileMode::Tree.into())?;
                }
            }
        }
        let oid = builder.write()?;
        self.cached_oid = Some(oid);
        Ok(oid)
    }
}

struct RepoBuilder {
    root: TreeNode,
    // Track accumulated content for dummy files so we can append.
    dummy_content: HashMap<String, Vec<u8>>,
}

impl RepoBuilder {
    fn new() -> Self {
        Self {
            root: TreeNode::new(),
            dummy_content: HashMap::new(),
        }
    }

    fn set_file(&mut self, repo: &Repository, path: &str, content: &[u8]) -> Result<()> {
        let blob_oid = repo.blob(content)?;
        self.root.insert_blob(path, blob_oid);
        Ok(())
    }

    fn append_dummy(&mut self, repo: &Repository, path: &str) -> Result<()> {
        let content = self.dummy_content.entry(path.to_string()).or_default();
        content.extend_from_slice(b"change\n");
        let blob_oid = repo.blob(content)?;
        self.root.insert_blob(path, blob_oid);
        Ok(())
    }

    fn commit(
        &mut self,
        repo: &Repository,
        parent: Option<Oid>,
        msg: &str,
        time: &Time,
    ) -> Result<Oid> {
        let tree_id = self.root.write(repo)?;
        let tree = repo.find_tree(tree_id)?;
        let s = sig(time);

        let oid = match parent {
            Some(pid) => {
                let p = repo.find_commit(pid)?;
                repo.commit(Some("HEAD"), &s, &s, msg, &tree, &[&p])?
            }
            None => repo.commit(Some("HEAD"), &s, &s, msg, &tree, &[])?,
        };
        Ok(oid)
    }
}

fn ensure_clean(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("rm {}", path.display()))?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn create_single(base: &Path, rng: &mut Rng, now: i64) -> Result<()> {
    let dir = base.join("single");
    ensure_clean(&dir)?;

    let repo = Repository::init(&dir)?;
    let mut b = RepoBuilder::new();

    b.set_file(
        &repo,
        ".ferrflow",
        br#"{
  "package": [
    {
      "name": "myapp",
      "path": ".",
      "changelog": "CHANGELOG.md",
      "versioned_files": [
        { "path": "package.json", "format": "json" }
      ]
    }
  ]
}"#,
    )?;
    b.set_file(
        &repo,
        "package.json",
        b"{\n  \"name\": \"myapp\",\n  \"version\": \"0.1.0\"\n}\n",
    )?;
    b.set_file(&repo, "dummy.txt", b"")?;

    let t = rand_time(rng, now);
    let oid = b.commit(&repo, None, "chore: initial commit", &t)?;

    let obj = repo.find_object(oid, None)?;
    repo.tag_lightweight("v0.1.0", &obj, false)?;

    let mut parent = oid;
    for _ in 0..100 {
        b.append_dummy(&repo, "dummy.txt")?;
        let scope = rng.pick(SCOPES);
        let msg = rand_message(rng, scope);
        let t = rand_time(rng, now);
        parent = b.commit(&repo, Some(parent), &msg, &t)?;
    }

    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
    println!("Created single fixture: 100 commits");
    Ok(())
}

fn create_mono(
    base: &Path,
    name: &str,
    pkg_count: usize,
    commit_count: usize,
    rng: &mut Rng,
    now: i64,
) -> Result<()> {
    let dir = base.join(name);
    ensure_clean(&dir)?;

    let repo = Repository::init(&dir)?;
    let mut b = RepoBuilder::new();

    let packages: Vec<String> = (1..=pkg_count).map(|i| format!("pkg-{i:03}")).collect();

    // Write .ferrflow config.
    let pkg_entries: Vec<String> = packages
        .iter()
        .map(|p| {
            format!(
                r#"    {{
      "name": "{p}",
      "path": "packages/{p}",
      "changelog": "packages/{p}/CHANGELOG.md",
      "versioned_files": [
        {{ "path": "packages/{p}/package.json", "format": "json" }}
      ]
    }}"#
            )
        })
        .collect();
    let config = format!("{{\n  \"package\": [\n{}\n  ]\n}}", pkg_entries.join(",\n"));
    b.set_file(&repo, ".ferrflow", config.as_bytes())?;

    for p in &packages {
        let content = format!("{{\n  \"name\": \"{p}\",\n  \"version\": \"0.1.0\"\n}}\n");
        b.set_file(
            &repo,
            &format!("packages/{p}/package.json"),
            content.as_bytes(),
        )?;
    }

    b.set_file(&repo, "dummy.txt", b"")?;

    let t = rand_time(rng, now);
    let oid = b.commit(&repo, None, "chore: initial commit", &t)?;

    let obj = repo.find_object(oid, None)?;
    for p in &packages {
        repo.tag_lightweight(&format!("{p}@v0.1.0"), &obj, false)?;
    }

    let mut parent = oid;
    for i in 1..=commit_count {
        let pkg = &packages[rng.usize(pkg_count)];
        let path = format!("packages/{pkg}/dummy.txt");
        b.append_dummy(&repo, &path)?;

        let msg = rand_message(rng, pkg);
        let t = rand_time(rng, now);
        parent = b.commit(&repo, Some(parent), &msg, &t)?;

        if commit_count >= 1000 && i % 2000 == 0 {
            println!("  {name}: {i}/{commit_count}");
        }
    }

    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
    println!("Created {name} fixture: {pkg_count} packages, {commit_count} commits");
    Ok(())
}

fn main() -> Result<()> {
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/fixtures".into());
    let base = Path::new(&output);
    fs::create_dir_all(base)?;

    let now = Utc::now().timestamp();
    let mut rng = Rng::new(42);

    println!("Generating benchmark fixtures...");
    create_single(base, &mut rng, now)?;
    create_mono(base, "mono-small", 10, 100, &mut rng, now)?;
    create_mono(base, "mono-medium", 50, 500, &mut rng, now)?;
    create_mono(base, "mono-large", 200, 10000, &mut rng, now)?;
    println!("Done.");

    Ok(())
}
