use std::collections::{HashMap, HashSet};

use crate::config::PackageConfig;
use crate::conventional_commits::BumpType;
use crate::error_code;

#[derive(Debug)]
pub(crate) struct Cycle {
    path: Vec<String>,
}

impl Cycle {
    pub(crate) fn path(&self) -> &[String] {
        &self.path
    }

    pub(crate) fn into_error(self) -> anyhow::Error {
        let mut rendered = self.path;
        if let Some(first) = rendered.first().cloned() {
            rendered.push(first);
        }
        anyhow::anyhow!("cycle detected: {}", rendered.join(" → "))
            .context(error_code::MONOREPO_DEPENDENCY_CYCLE)
    }
}

/// The packages a further cascade round would move, given what is already
/// bumped, and the bump each would take. Each dependency edge carries its own
/// [`PropagatePolicy`], so an edge can decline to propagate at all.
///
/// A package is returned when the strongest bump reaching it is stronger than
/// what it currently holds, so a package fed by two edges settles on the
/// strongest rather than on whichever arrived first. Iterate until this
/// returns nothing to reach the fixpoint.
///
/// The release cascade and `graph --impact` both call this, which is what
/// makes the preview agree with what a release does.
pub(crate) fn cascade_round(
    packages: &[PackageConfig],
    bumped: &HashMap<String, BumpType>,
) -> Vec<(usize, BumpType)> {
    let mut joined = Vec::new();
    for (idx, pkg) in packages.iter().enumerate() {
        let held = bumped.get(&pkg.name).copied().unwrap_or(BumpType::None);
        let incoming = pkg
            .depends_on
            .iter()
            .filter_map(|dep| {
                let upstream = bumped.get(dep.name())?;
                Some(dep.propagate().resolve(*upstream))
            })
            .max()
            .unwrap_or(BumpType::None);
        if incoming > held {
            joined.push((idx, incoming));
        }
    }
    joined
}

pub(crate) fn release_order(packages: &[PackageConfig]) -> Result<Vec<usize>, Cycle> {
    let index_of: HashMap<&str, usize> = packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| (pkg.name.as_str(), i))
        .collect();

    let adjacency: Vec<Vec<usize>> = packages
        .iter()
        .map(|pkg| {
            pkg.depends_on
                .iter()
                .filter_map(|dep| index_of.get(dep.name()).copied())
                .collect()
        })
        .collect();

    let mut order = Vec::with_capacity(packages.len());
    for component in tarjan_sccs(&adjacency) {
        let is_cycle = component.len() > 1 || adjacency[component[0]].contains(&component[0]);
        if is_cycle {
            let path = cycle_path(&adjacency, &component)
                .into_iter()
                .map(|i| packages[i].name.clone())
                .collect();
            return Err(Cycle { path });
        }
        order.push(component[0]);
    }
    Ok(order)
}

fn tarjan_sccs(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct Walk<'a> {
        adjacency: &'a [Vec<usize>],
        next_index: usize,
        index: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        components: Vec<Vec<usize>>,
    }

    impl Walk<'_> {
        fn connect(&mut self, v: usize) {
            self.index[v] = Some(self.next_index);
            self.lowlink[v] = self.next_index;
            self.next_index += 1;
            self.stack.push(v);
            self.on_stack[v] = true;

            for w in self.adjacency[v].clone() {
                match self.index[w] {
                    None => {
                        self.connect(w);
                        self.lowlink[v] = self.lowlink[v].min(self.lowlink[w]);
                    }
                    Some(w_index) if self.on_stack[w] => {
                        self.lowlink[v] = self.lowlink[v].min(w_index);
                    }
                    Some(_) => {}
                }
            }

            if self.lowlink[v] == self.index[v].expect("v was just indexed") {
                let mut component = Vec::new();
                loop {
                    let w = self.stack.pop().expect("stack holds at least v");
                    self.on_stack[w] = false;
                    component.push(w);
                    if w == v {
                        break;
                    }
                }
                self.components.push(component);
            }
        }
    }

    let n = adjacency.len();
    let mut walk = Walk {
        adjacency,
        next_index: 0,
        index: vec![None; n],
        lowlink: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        components: Vec::new(),
    };
    for v in 0..n {
        if walk.index[v].is_none() {
            walk.connect(v);
        }
    }
    walk.components
}

fn cycle_path(adjacency: &[Vec<usize>], component: &[usize]) -> Vec<usize> {
    let members: HashSet<usize> = component.iter().copied().collect();
    let start = component[0];

    let mut path = vec![start];
    let mut current = start;
    loop {
        let next = adjacency[current]
            .iter()
            .copied()
            .find(|w| members.contains(w))
            .expect("a node in a strongly connected component has an in-component edge");
        if let Some(pos) = path.iter().position(|&n| n == next) {
            return path[pos..].to_vec();
        }
        path.push(next);
        current = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, deps: &[&str]) -> PackageConfig {
        PackageConfig {
            version_source: None,
            name: name.to_string(),
            path: name.to_string(),
            versioned_files: vec![],
            changelog: None,
            shared_paths: vec![],
            depends_on: deps
                .iter()
                .map(|s| crate::config::Dependency::Name(s.to_string()))
                .collect(),
            update_lockfiles: None,
            versioning: None,
            tag_template: None,
            version_template: None,
            floating_tags: None,
            latest_tag: None,
            hooks: None,
            publishers: vec![],
        }
    }

    fn names(packages: &[PackageConfig], order: &[usize]) -> Vec<String> {
        order.iter().map(|&i| packages[i].name.clone()).collect()
    }

    fn cycle_message(err: &anyhow::Error) -> String {
        err.chain()
            .map(|cause| cause.to_string())
            .find(|m| m.starts_with("cycle detected"))
            .expect("a cycle message in the error chain")
    }

    #[test]
    fn no_dependencies_preserves_config_order() {
        let pkgs = [pkg("a", &[]), pkg("b", &[]), pkg("c", &[])];
        let order = release_order(&pkgs).expect("acyclic");
        assert_eq!(names(&pkgs, &order), ["a", "b", "c"]);
    }

    #[test]
    fn dependency_is_released_before_its_dependent() {
        let pkgs = [pkg("web", &["api"]), pkg("api", &[])];
        let order = release_order(&pkgs).expect("acyclic");
        assert_eq!(names(&pkgs, &order), ["api", "web"]);
    }

    #[test]
    fn linear_chain_orders_deepest_dependency_first() {
        let pkgs = [pkg("a", &["b"]), pkg("b", &["c"]), pkg("c", &[])];
        let order = release_order(&pkgs).expect("acyclic");
        assert_eq!(names(&pkgs, &order), ["c", "b", "a"]);
    }

    #[test]
    fn unknown_dependency_names_are_ignored() {
        let pkgs = [pkg("a", &["ghost"]), pkg("b", &[])];
        let order = release_order(&pkgs).expect("acyclic");
        assert_eq!(names(&pkgs, &order), ["a", "b"]);
    }

    #[test]
    fn two_package_cycle_is_rejected() {
        let pkgs = [pkg("a", &["b"]), pkg("b", &["a"])];
        let err = release_order(&pkgs).unwrap_err().into_error();
        assert_eq!(
            err.downcast_ref::<error_code::ErrorCode>().map(|c| c.0),
            Some(8003)
        );
        let msg = cycle_message(&err);
        assert!(
            msg == "cycle detected: a → b → a" || msg == "cycle detected: b → a → b",
            "{msg}"
        );
    }

    #[test]
    fn self_dependency_is_rejected() {
        let pkgs = [pkg("a", &["a"])];
        let err = release_order(&pkgs).unwrap_err().into_error();
        assert_eq!(cycle_message(&err), "cycle detected: a → a");
    }

    #[test]
    fn three_package_cycle_renders_full_path() {
        let pkgs = [pkg("a", &["b"]), pkg("b", &["c"]), pkg("c", &["a"])];
        let err = release_order(&pkgs).unwrap_err().into_error();
        let msg = cycle_message(&err);
        let path = msg
            .trim_start_matches("cycle detected: ")
            .split(" → ")
            .collect::<Vec<_>>();
        assert_eq!(path.first(), path.last(), "loop is closed: {msg}");
        let unique: HashSet<&str> = path.iter().copied().collect();
        assert_eq!(unique, HashSet::from(["a", "b", "c"]), "{msg}");
    }
}
