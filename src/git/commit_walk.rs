use anyhow::Result;
use gix::ObjectId;
use gix::revision::walk::Sorting;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::commits::{GitLog, get_commits_since_oid, subject_has_skip_marker};
use super::repo::Repository;

const SHARED_WALK_THRESHOLD: usize = 2;

pub struct CommitWalkCache {
    skip_markers: Vec<String>,
    calls: AtomicUsize,
    built: Mutex<Option<Arc<BuiltWalk>>>,
}

struct BuiltWalk {
    commits: Vec<DecodedCommit>,
    pos: HashMap<ObjectId, usize>,
}

struct DecodedCommit {
    log: GitLog,
    skipped: bool,
    parents: Vec<ObjectId>,
}

impl CommitWalkCache {
    pub fn new(skip_markers: Vec<String>) -> Self {
        Self {
            skip_markers,
            calls: AtomicUsize::new(0),
            built: Mutex::new(None),
        }
    }

    pub fn commits_since(&self, repo: &Repository, stop: Option<ObjectId>) -> Result<Vec<GitLog>> {
        let already_built = self
            .built
            .lock()
            .expect("commit walk cache poisoned")
            .is_some();
        if !already_built && self.calls.fetch_add(1, Ordering::Relaxed) < SHARED_WALK_THRESHOLD {
            return get_commits_since_oid(repo, stop, &self.skip_markers);
        }
        let built = self.get_or_build(repo)?;
        let hidden = match stop {
            None => HashSet::new(),
            Some(stop) => {
                let Some(&start) = built.pos.get(&stop) else {
                    return get_commits_since_oid(repo, Some(stop), &self.skip_markers);
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
            .filter(|(i, c)| !c.skipped && !hidden.contains(i))
            .map(|(_, c)| c.log.clone())
            .collect())
    }

    fn get_or_build(&self, repo: &Repository) -> Result<Arc<BuiltWalk>> {
        let mut guard = self.built.lock().expect("commit walk cache poisoned");
        if let Some(built) = guard.as_ref() {
            return Ok(Arc::clone(built));
        }
        let built = Arc::new(build(repo, &self.skip_markers)?);
        *guard = Some(Arc::clone(&built));
        Ok(built)
    }
}

fn build(repo: &Repository, skip_markers: &[String]) -> Result<BuiltWalk> {
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
        let parents: Vec<ObjectId> = info.parent_ids.iter().copied().collect();
        let decoded = repo.find_commit(info.id).ok().and_then(|commit| {
            commit
                .message_raw()
                .ok()
                .map(|raw| String::from_utf8_lossy(raw).into_owned())
        });
        let (message, skipped) = match decoded {
            Some(message) => {
                let skipped = subject_has_skip_marker(&message, skip_markers);
                (message, skipped)
            }
            None => (String::new(), true),
        };
        pos.insert(info.id, commits.len());
        commits.push(DecodedCommit {
            log: GitLog {
                id: info.id.to_string(),
                hash: info.id.to_string()[..8].to_string(),
                message,
            },
            skipped,
            parents,
        });
    }
    Ok(BuiltWalk { commits, pos })
}
