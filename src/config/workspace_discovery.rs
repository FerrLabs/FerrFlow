use std::path::Path;

use super::package::FileFormat;

mod cargo;
mod go;
mod gradle;
mod node;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DiscoveredPackage {
    pub name: String,
    pub path: String,
    pub manifest: &'static str,
    pub format: FileFormat,
}

type ReadManifest = fn(root: &Path, dir: &Path, rel: &str) -> Option<DiscoveredPackage>;

struct Source {
    globs: Vec<String>,
    read: ReadManifest,
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
    "coverage",
    "vendor",
];

const MAX_DEPTH: usize = 6;

pub(super) fn discover(root: &Path) -> Vec<DiscoveredPackage> {
    let mut found: Vec<DiscoveredPackage> = [
        node::source(root),
        cargo::source(root),
        go::source(root),
        gradle::source(root),
    ]
    .iter()
    .filter(|source| !source.globs.is_empty())
    .flat_map(|source| source.collect(root))
    .collect();
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found.dedup_by(|a, b| a.path == b.path);
    found
}

pub(super) fn discover_node(root: &Path) -> Vec<DiscoveredPackage> {
    let source = node::source(root);
    let mut found = if source.globs.is_empty() {
        Vec::new()
    } else {
        source.collect(root)
    };
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found.dedup_by(|a, b| a.path == b.path);
    found
}

impl Source {
    fn collect(&self, root: &Path) -> Vec<DiscoveredPackage> {
        let mut found = Vec::new();
        if self.globs.iter().any(|g| g == ".") {
            found.extend((self.read)(root, root, "."));
        }
        self.walk(root, root, 0, &mut found);

        let excluded: Vec<&str> = self
            .globs
            .iter()
            .filter_map(|g| g.strip_prefix('!'))
            .collect();
        found.retain(|p| !excluded.iter().any(|g| glob_match::glob_match(g, &p.path)));
        found
    }

    fn walk(&self, root: &Path, dir: &Path, depth: usize, found: &mut Vec<DiscoveredPackage>) {
        if depth > MAX_DEPTH {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.') || SKIP_DIRS.contains(&name) {
                continue;
            }
            if let Some(rel) = relative_slash_path(root, &path)
                && self.globs.iter().any(|g| matches_glob(g, &rel))
                && let Some(pkg) = (self.read)(root, &path, &rel)
            {
                found.push(pkg);
            }
            self.walk(root, &path, depth + 1, found);
        }
    }
}

fn normalise_glob(raw: &str) -> String {
    let trimmed = raw.trim_start_matches("./").trim_end_matches('/');
    if trimmed.is_empty() {
        ".".to_string()
    } else {
        trimmed.to_string()
    }
}

fn matches_glob(glob: &str, rel: &str) -> bool {
    !glob.starts_with('!') && glob_match::glob_match(glob, rel)
}

fn relative_slash_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(
        rel.components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn directory_name(dir: &Path) -> Option<String> {
    dir.file_name().and_then(|n| n.to_str()).map(str::to_string)
}
