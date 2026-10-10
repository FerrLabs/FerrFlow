use std::path::Path;

use regex::Regex;

use super::{DiscoveredPackage, Source, directory_name, normalise_glob};
use crate::config::package::FileFormat;

const MANIFEST: &str = "go.mod";

pub(super) fn source(root: &Path) -> Source {
    Source {
        globs: uses(root),
        read: read_module,
    }
}

fn uses(root: &Path) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(root.join("go.work")) else {
        return Vec::new();
    };
    let mut in_block = false;
    let mut paths = Vec::new();
    for line in raw.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if in_block {
            if line.starts_with(')') {
                in_block = false;
            } else if !line.is_empty() {
                paths.push(line);
            }
            continue;
        }
        let Some(rest) = line
            .strip_prefix("use")
            .filter(|r| r.starts_with(char::is_whitespace) || r.starts_with('('))
        else {
            continue;
        };
        let rest = rest.trim();
        match rest.strip_prefix('(') {
            Some(_) => in_block = true,
            None if !rest.is_empty() => paths.push(rest),
            None => {}
        }
    }
    paths
        .into_iter()
        .map(|p| normalise_glob(p.trim_matches('"')))
        .collect()
}

fn read_module(_root: &Path, dir: &Path, rel: &str) -> Option<DiscoveredPackage> {
    let raw = std::fs::read_to_string(dir.join(MANIFEST)).ok()?;
    let module = Regex::new(r"(?m)^module\s+(\S+)").ok()?;
    let name = module
        .captures(&raw)
        .map(|c| c[1].trim_matches('"').to_string())
        .or_else(|| directory_name(dir))?;
    Some(DiscoveredPackage {
        name,
        path: rel.to_string(),
        manifest: MANIFEST,
        format: FileFormat::GoMod,
    })
}
