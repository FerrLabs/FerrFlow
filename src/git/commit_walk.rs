use anyhow::Result;
use gix::ObjectId;
use gix::revision::walk::Sorting;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::commits::{GitLog, decode_commit, walk_commit_ids};
use super::repo::Repository;

const SHARED_WALK_THRESHOLD: usize = 2;

pub struct CommitWalkCache {
    skip_markers: Vec<String>,
    calls: AtomicUsize,
    built: Mutex<Option<Arc<BuiltWalk>>>,
}

struct BuiltWalk {
    commits: Vec<WalkedCommit>,
    pos: HashMap<ObjectId, usize>,
}

struct WalkedCommit {
    id: ObjectId,
    parents: Vec<ObjectId>,
    log: OnceLock<Option<GitLog>>,
}

impl CommitWalkCache {
    pub fn new(skip_markers: Vec<String>) -> Self {
        Self {
            skip_markers,
            calls: AtomicUsize::new(0),
            built: Mutex::new(None),
        }
    }

    pub fn commit_ids_since(
        &self,
        repo: &Repository,
        stop: Option<ObjectId>,
    ) -> Result<Vec<ObjectId>> {
        let already_built = self
            .built
            .lock()
            .expect("commit walk cache poisoned")
            .is_some();
        if !already_built && self.calls.fetch_add(1, Ordering::Relaxed) < SHARED_WALK_THRESHOLD {
            return walk_commit_ids(repo, stop);
        }
        let built = self.get_or_build(repo)?;
        let hidden = match stop {
            None => HashSet::new(),
            Some(stop) => {
                let Some(&start) = built.pos.get(&stop) else {
                    return walk_commit_ids(repo, Some(stop));
                };
                let mut hidden = HashSet::from([start]);
                let mut queue = VecDeque::from([start]);
                while let Some(i) = queue.pop_front() {
                    for parent in &built.commits[i].parents {
                        if let Some(&pi) = built.pos.get(parent)
                            && hidden.insert(pi)
                        {
                            queue.push_back(pi);
                        }
                    }
                }
                hidden
            }
        };
        Ok(built
            .commits
            .iter()
            .enumerate()
            .filter(|(i, _)| !hidden.contains(i))
            .map(|(_, c)| c.id)
            .collect())
    }

    pub fn decode(&self, repo: &Repository, ids: &[ObjectId]) -> Vec<GitLog> {
        let built = self
            .built
            .lock()
            .expect("commit walk cache poisoned")
            .clone();
        ids.iter()
            .filter_map(|&id| {
                let walked = built
                    .as_ref()
                    .and_then(|b| b.pos.get(&id).map(|&i| &b.commits[i]));
                match walked {
                    Some(walked) => walked
                        .log
                        .get_or_init(|| decode_commit(repo, id, &self.skip_markers))
                        .clone(),
                    None => decode_commit(repo, id, &self.skip_markers),
                }
            })
            .collect()
    }

    fn get_or_build(&self, repo: &Repository) -> Result<Arc<BuiltWalk>> {
        let mut guard = self.built.lock().expect("commit walk cache poisoned");
        if let Some(built) = guard.as_ref() {
            return Ok(Arc::clone(built));
        }
        let built = Arc::new(build(repo)?);
        *guard = Some(Arc::clone(&built));
        Ok(built)
    }
}

fn build(repo: &Repository) -> Result<BuiltWalk> {
    let head = repo.head_id()?.detach();
    let walk = repo
        .rev_walk([head])
        .use_commit_graph(true)
        .sorting(Sorting::BreadthFirst)
        .all()?;

    let mut commits = Vec::new();
    let mut pos = HashMap::new();
    for info in walk {
        let info = info?;
        pos.insert(info.id, commits.len());
        commits.push(WalkedCommit {
            id: info.id,
            parents: info.parent_ids.iter().copied().collect(),
            log: OnceLock::new(),
        });
    }
    Ok(BuiltWalk { commits, pos })
}
