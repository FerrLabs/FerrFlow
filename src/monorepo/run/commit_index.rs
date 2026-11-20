use crate::config::{Config, NestedPackages};
use crate::git::{Repository, get_changed_files_for_commit};
use gix::ObjectId;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

const PARALLEL_DIFF_THRESHOLD: usize = 768;

struct PackageOwnership {
    words: usize,
    always: Vec<usize>,
    by_path: HashMap<String, Vec<usize>>,
    exclusive_roots: Vec<usize>,
    exclusive: bool,
    shared: Vec<(String, usize)>,
}

impl PackageOwnership {
    fn new(config: &Config) -> Self {
        let mut ownership = Self {
            words: config.packages.len().div_ceil(64).max(1),
            always: Vec::new(),
            by_path: HashMap::new(),
            exclusive_roots: Vec::new(),
            exclusive: config.workspace.nested_packages == NestedPackages::Exclusive,
            shared: Vec::new(),
        };
        for (idx, pkg) in config.packages.iter().enumerate() {
            match pkg.relative_path() {
                Some(path) => ownership
                    .by_path
                    .entry(path.to_string())
                    .or_default()
                    .push(idx),
                None if config.nested_package_paths(pkg).is_empty() => ownership.always.push(idx),
                None => ownership.exclusive_roots.push(idx),
            }
            ownership.shared.extend(
                pkg.shared_paths
                    .iter()
                    .map(|shared| (shared.trim_end_matches('/').to_string(), idx)),
            );
        }
        ownership
    }

    fn row(&self, files: &[String]) -> Vec<u64> {
        let mut row = vec![0; self.words];
        if files.is_empty() {
            row.fill(u64::MAX);
            return row;
        }
        self.always.iter().for_each(|&idx| set_bit(&mut row, idx));
        for file in files {
            self.mark_owners(file, &mut row);
            self.shared
                .iter()
                .filter(|(shared, _)| file.starts_with(shared.as_str()))
                .for_each(|&(_, idx)| set_bit(&mut row, idx));
        }
        row
    }

    fn mark_owners(&self, file: &str, row: &mut [u64]) {
        let mut owners = file
            .match_indices('/')
            .filter_map(|(end, _)| self.by_path.get(&file[..end]));
        if self.exclusive {
            let deepest = owners.next_back();
            deepest
                .unwrap_or(&self.exclusive_roots)
                .iter()
                .for_each(|&idx| set_bit(row, idx));
        } else {
            owners.flatten().for_each(|&idx| set_bit(row, idx));
        }
    }
}

fn set_bit(row: &mut [u64], idx: usize) {
    row[idx / 64] |= 1 << (idx % 64);
}

fn has_bit(row: &[u64], idx: usize) -> bool {
    row[idx / 64] & (1 << (idx % 64)) != 0
}

#[derive(Default)]
struct Rows {
    slot: HashMap<ObjectId, usize>,
    bits: Vec<u64>,
}

pub(super) struct CommitIndex {
    ownership: PackageOwnership,
    rows: Mutex<Rows>,
}

impl CommitIndex {
    pub(super) fn new(config: &Config) -> Self {
        Self {
            ownership: PackageOwnership::new(config),
            rows: Mutex::default(),
        }
    }

    pub(super) fn retain_touching(
        &self,
        repo: &Repository,
        package: usize,
        mut commits: Vec<ObjectId>,
    ) -> Vec<ObjectId> {
        self.fill(repo, &commits);
        let rows = self.rows.lock().expect("commit index poisoned");
        let words = self.ownership.words;
        commits.retain(|id| {
            rows.slot
                .get(id)
                .is_none_or(|&slot| has_bit(&rows.bits[slot * words..][..words], package))
        });
        commits
    }

    fn fill(&self, repo: &Repository, commits: &[ObjectId]) {
        let missing: Vec<ObjectId> = {
            let rows = self.rows.lock().expect("commit index poisoned");
            commits
                .iter()
                .copied()
                .filter(|id| !rows.slot.contains_key(id))
                .collect()
        };
        if missing.is_empty() {
            return;
        }
        let computed = self.compute_rows(repo, &missing);

        let mut rows = self.rows.lock().expect("commit index poisoned");
        for (id, row) in computed {
            if rows.slot.contains_key(&id) {
                continue;
            }
            let slot = rows.slot.len();
            rows.slot.insert(id, slot);
            rows.bits.extend(row);
        }
    }

    fn compute_rows(&self, repo: &Repository, missing: &[ObjectId]) -> Vec<(ObjectId, Vec<u64>)> {
        let row_of = |repo: &Repository, id: ObjectId| {
            let files = get_changed_files_for_commit(repo, id).unwrap_or_default();
            (id, self.ownership.row(&files))
        };
        if crate::concurrency::max_jobs() < 2
            || missing.len() < PARALLEL_DIFF_THRESHOLD
            || rayon::current_num_threads() < 2
        {
            return missing.iter().map(|&id| row_of(repo, id)).collect();
        }
        let shared = repo.clone().into_sync();
        missing
            .par_iter()
            .map_init(
                || shared.to_thread_local(),
                |worker, &id| row_of(worker, id),
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PackageConfig, WorkspaceConfig};
    use std::path::Path;

    fn pkg(name: &str, path: &str) -> PackageConfig {
        PackageConfig {
            name: name.to_string(),
            path: path.to_string(),
            ..Default::default()
        }
    }

    fn config(nested: NestedPackages, packages: Vec<PackageConfig>) -> Config {
        Config {
            include: Vec::new(),
            workspace: WorkspaceConfig {
                nested_packages: nested,
                ..Default::default()
            },
            packages,
        }
    }

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    fn touched_names(config: &Config, paths: &[&str]) -> Vec<String> {
        let row = PackageOwnership::new(config).row(&files(paths));
        config
            .packages
            .iter()
            .enumerate()
            .filter(|(idx, _)| has_bit(&row, *idx))
            .map(|(_, pkg)| pkg.name.clone())
            .collect()
    }

    fn reference_touched(config: &Config, paths: &[&str]) -> Vec<String> {
        let files = files(paths);
        config
            .packages
            .iter()
            .filter(|pkg| {
                files.is_empty()
                    || pkg.is_touched_by(&files, true, &config.nested_package_paths(pkg))
            })
            .map(|pkg| pkg.name.clone())
            .collect()
    }

    fn layered_packages() -> Vec<PackageConfig> {
        let mut svc_docs = pkg("svc", "services/svc");
        svc_docs.shared_paths = vec!["docs/".to_string()];
        vec![
            pkg("libs", "libs"),
            pkg("core", "libs/core"),
            pkg("deep", "./libs/core/deep/"),
            pkg("libs2", "libs2"),
            pkg("core-x", "libs/coreX"),
            svc_docs,
            pkg("svc-twin", "services/svc"),
        ]
    }

    const CORPUS: &[&[&str]] = &[
        &["libs/a.txt"],
        &["libs/core/b.txt"],
        &["libs/core/deep/c.txt"],
        &["libs/core/deep/x/y/z.txt"],
        &["libs2/a.txt"],
        &["libs/coreX/a.txt"],
        &["libs/core"],
        &["libs"],
        &["services/svc/main.rs"],
        &["docs/guide.md"],
        &["docsX/guide.md"],
        &["README.md"],
        &["libs/a.txt", "libs2/a.txt"],
        &["libs/core/b.txt", "README.md"],
        &["services/svc/a", "libs/core/deep/c"],
        &[],
    ];

    #[test]
    fn the_index_matches_per_package_filtering_for_every_nesting_mode() {
        for nested in [NestedPackages::Exclusive, NestedPackages::Shared] {
            let mut packages = layered_packages();
            let config_without_root = config(nested, packages.clone());
            packages.push(pkg("root", "."));
            let config_with_root = config(nested, packages);
            for config in [&config_without_root, &config_with_root] {
                for paths in CORPUS {
                    assert_eq!(
                        touched_names(config, paths),
                        reference_touched(config, paths),
                        "{nested:?} with {} packages, files {paths:?}",
                        config.packages.len()
                    );
                }
            }
        }
    }

    #[test]
    fn an_exclusive_parent_does_not_own_what_a_nested_package_owns() {
        let config = config(NestedPackages::Exclusive, layered_packages());
        assert_eq!(touched_names(&config, &["libs/a.txt"]), ["libs"]);
        assert_eq!(touched_names(&config, &["libs/core/b.txt"]), ["core"]);
        assert_eq!(touched_names(&config, &["libs/core/deep/c.txt"]), ["deep"]);
    }

    #[test]
    fn a_shared_parent_owns_what_its_children_own() {
        let config = config(NestedPackages::Shared, layered_packages());
        assert_eq!(
            touched_names(&config, &["libs/core/deep/c.txt"]),
            ["libs", "core", "deep"]
        );
    }

    #[test]
    fn a_sibling_sharing_a_name_prefix_is_not_inside_the_package() {
        let config = config(NestedPackages::Exclusive, layered_packages());
        assert_eq!(touched_names(&config, &["libs2/a.txt"]), ["libs2"]);
        assert_eq!(touched_names(&config, &["libs/coreX/a.txt"]), ["core-x"]);
    }

    #[test]
    fn packages_sharing_a_path_both_own_it() {
        let config = config(NestedPackages::Exclusive, layered_packages());
        assert_eq!(
            touched_names(&config, &["services/svc/main.rs"]),
            ["svc", "svc-twin"]
        );
    }

    #[test]
    fn a_file_owned_by_no_package_touches_none() {
        let config = config(NestedPackages::Exclusive, layered_packages());
        assert!(touched_names(&config, &["README.md"]).is_empty());
        assert!(touched_names(&config, &["libs"]).is_empty());
    }

    #[test]
    fn a_root_package_owns_only_what_no_other_package_owns() {
        let mut packages = layered_packages();
        packages.push(pkg("root", "."));
        let exclusive = config(NestedPackages::Exclusive, packages.clone());
        assert_eq!(touched_names(&exclusive, &["README.md"]), ["root"]);
        assert_eq!(touched_names(&exclusive, &["libs/a.txt"]), ["libs"]);

        let shared = config(NestedPackages::Shared, packages);
        assert_eq!(touched_names(&shared, &["libs/a.txt"]), ["libs", "root"]);
    }

    #[test]
    fn shared_paths_touch_their_package_outside_its_own_directory() {
        let config = config(NestedPackages::Exclusive, layered_packages());
        assert_eq!(touched_names(&config, &["docs/guide.md"]), ["svc"]);
        assert!(touched_names(&config, &["other/guide.md"]).is_empty());
    }

    #[test]
    fn a_commit_without_changed_files_touches_every_package() {
        let mut packages = layered_packages();
        packages.push(pkg("root", "."));
        let config = config(NestedPackages::Exclusive, packages);
        assert_eq!(touched_names(&config, &[]).len(), config.packages.len());
    }

    #[test]
    fn more_than_sixty_four_packages_use_every_bit_word() {
        let packages: Vec<PackageConfig> = (0..130)
            .map(|i| pkg(&format!("p{i}"), &format!("pkgs/p{i}")))
            .collect();
        let config = config(NestedPackages::Exclusive, packages);
        assert_eq!(touched_names(&config, &["pkgs/p0/a"]), ["p0"]);
        assert_eq!(touched_names(&config, &["pkgs/p63/a"]), ["p63"]);
        assert_eq!(touched_names(&config, &["pkgs/p64/a"]), ["p64"]);
        assert_eq!(touched_names(&config, &["pkgs/p129/a"]), ["p129"]);
        assert_eq!(
            touched_names(&config, &["pkgs/p1/a", "pkgs/p100/b"]),
            ["p1", "p100"]
        );
    }

    fn fast_import(dir: &Path, history: &[(&str, &[&str])]) {
        use std::io::Write;
        let mut stream = String::new();
        for (i, (subject, paths)) in history.iter().enumerate() {
            let body = format!("{subject}\n");
            stream.push_str("commit refs/heads/main\n");
            stream.push_str(&format!(
                "committer Test <test@test.com> {} +0000\n",
                1_950_000_000 + i
            ));
            stream.push_str(&format!("data {}\n{body}", body.len()));
            for path in *paths {
                let content = format!("{i}\n");
                stream.push_str(&format!(
                    "M 100644 inline {path}\ndata {}\n{content}",
                    content.len()
                ));
            }
        }
        let mut child = crate::test_utils::git_command()
            .current_dir(dir)
            .args(["fast-import", "--quiet"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("git should be on PATH");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stream.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success(), "git fast-import failed");
    }

    fn history_ids(repo: &Repository) -> Vec<ObjectId> {
        let head = repo.head_id().unwrap().detach();
        repo.rev_walk([head])
            .all()
            .unwrap()
            .map(|info| info.unwrap().id)
            .collect()
    }

    fn in_pool<T: Send>(threads: usize, f: impl FnOnce() -> T + Send) -> T {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(f)
    }

    fn per_package_reference(
        repo: &Repository,
        config: &Config,
        package: usize,
        ids: &[ObjectId],
    ) -> Vec<ObjectId> {
        let pkg = &config.packages[package];
        let nested = config.nested_package_paths(pkg);
        ids.iter()
            .copied()
            .filter(|&id| {
                let files = get_changed_files_for_commit(repo, id).unwrap();
                files.is_empty() || pkg.is_touched_by(&files, true, &nested)
            })
            .collect()
    }

    fn assert_index_matches_reference(dir: &Path, config: &Config, repo: &Repository) {
        let ids = history_ids(repo);
        let index = CommitIndex::new(config);
        for package in 0..config.packages.len() {
            assert_eq!(
                index.retain_touching(repo, package, ids.clone()),
                per_package_reference(repo, config, package, &ids),
                "package {} in {}",
                config.packages[package].name,
                dir.display()
            );
        }
    }

    fn history_config() -> Config {
        config(
            NestedPackages::Exclusive,
            vec![
                pkg("api", "api"),
                pkg("sdk", "sdk"),
                pkg("sdk-core", "sdk/core"),
                pkg("root", "."),
            ],
        )
    }

    #[test]
    fn the_index_keeps_exactly_the_commits_per_package_filtering_keeps() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        fast_import(
            dir.path(),
            &[
                ("chore: seed", &["README.md"]),
                ("fix(api): one", &["api/a.txt"]),
                ("fix(sdk): two", &["sdk/a.txt"]),
                ("fix(core): three", &["sdk/core/a.txt"]),
                ("chore: empty", &[]),
                ("fix: both", &["api/b.txt", "sdk/core/b.txt"]),
                ("chore: root only", &["Makefile"]),
            ],
        );
        let repo = crate::git::open_repo(dir.path()).unwrap();
        assert_index_matches_reference(dir.path(), &history_config(), &repo);
    }

    #[test]
    fn an_empty_commit_is_kept_for_every_package() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        fast_import(
            dir.path(),
            &[("fix(api): one", &["api/a.txt"]), ("chore: empty", &[])],
        );
        let repo = crate::git::open_repo(dir.path()).unwrap();
        let config = history_config();
        let ids = history_ids(&repo);
        let empty = ids[0];
        let index = CommitIndex::new(&config);
        for package in 0..config.packages.len() {
            assert!(
                index
                    .retain_touching(&repo, package, ids.clone())
                    .contains(&empty),
                "package {} lost the empty commit",
                config.packages[package].name
            );
        }
    }

    #[test]
    fn scoping_an_empty_range_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        let repo = crate::git::open_repo(dir.path()).unwrap();
        let index = CommitIndex::new(&history_config());
        assert!(index.retain_touching(&repo, 0, Vec::new()).is_empty());
    }

    #[test]
    fn a_second_scope_reuses_the_rows_of_the_first() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        fast_import(
            dir.path(),
            &[
                ("fix(api): one", &["api/a.txt"]),
                ("fix(sdk): two", &["sdk/a.txt"]),
            ],
        );
        let repo = crate::git::open_repo(dir.path()).unwrap();
        let ids = history_ids(&repo);
        let index = CommitIndex::new(&history_config());
        let first = index.retain_touching(&repo, 0, ids.clone());
        let second = index.retain_touching(&repo, 0, ids.clone());
        assert_eq!(first, second);
        assert_eq!(index.rows.lock().unwrap().slot.len(), ids.len());
    }

    #[test]
    fn the_parallel_fill_matches_per_package_filtering() {
        let dir = tempfile::tempdir().unwrap();
        crate::test_utils::init_repo_at(dir.path());
        let subjects: Vec<(String, Vec<String>)> = (0..PARALLEL_DIFF_THRESHOLD + 40)
            .map(|i| {
                let paths = match i % 5 {
                    0 => vec![format!("api/f{i}.txt")],
                    1 => vec![format!("sdk/f{i}.txt")],
                    2 => vec![format!("sdk/core/f{i}.txt")],
                    3 => vec![],
                    _ => vec![format!("api/f{i}.txt"), format!("Makefile{i}")],
                };
                (format!("fix: change {i}"), paths)
            })
            .collect();
        let history: Vec<(&str, Vec<&str>)> = subjects
            .iter()
            .map(|(s, p)| (s.as_str(), p.iter().map(String::as_str).collect()))
            .collect();
        let history_refs: Vec<(&str, &[&str])> =
            history.iter().map(|(s, p)| (*s, p.as_slice())).collect();
        fast_import(dir.path(), &history_refs);

        let repo = crate::git::open_repo(dir.path()).unwrap();
        let ids = history_ids(&repo);
        assert!(ids.len() > PARALLEL_DIFF_THRESHOLD);
        let config = history_config();
        let index = CommitIndex::new(&config);
        let filled = in_pool(4, || {
            let worker = crate::git::open_repo(dir.path()).unwrap();
            (0..config.packages.len())
                .map(|package| index.retain_touching(&worker, package, ids.clone()))
                .collect::<Vec<_>>()
        });
        for (package, kept) in filled.iter().enumerate() {
            assert_eq!(
                *kept,
                per_package_reference(&repo, &config, package, &ids),
                "package {}",
                config.packages[package].name
            );
        }
    }
}
