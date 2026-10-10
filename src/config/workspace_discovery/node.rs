use std::path::Path;

use super::{DiscoveredPackage, Source, directory_name, normalise_glob};
use crate::config::package::FileFormat;

const MANIFEST: &str = "package.json";

pub(super) fn source(root: &Path) -> Source {
    let mut globs = globs_from_package_json(root);
    globs.extend(globs_from_pnpm_workspace(root));
    globs.sort();
    globs.dedup();
    Source {
        globs,
        read: read_manifest,
    }
}

fn globs_from_package_json(root: &Path) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(root.join(MANIFEST)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    match value.get("workspaces") {
        Some(serde_json::Value::Array(items)) => string_list(items),
        Some(serde_json::Value::Object(obj)) => obj
            .get("packages")
            .and_then(|p| p.as_array())
            .map(|items| string_list(items))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn globs_from_pnpm_workspace(root: &Path) -> Vec<String> {
    let raw = ["pnpm-workspace.yaml", "pnpm-workspace.yml"]
        .iter()
        .find_map(|name| std::fs::read_to_string(root.join(name)).ok());
    let Some(raw) = raw else {
        return Vec::new();
    };
    let Ok(value) = serde_norway::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    value
        .get("packages")
        .and_then(|p| p.as_array())
        .map(|items| string_list(items))
        .unwrap_or_default()
}

fn string_list(items: &[serde_json::Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|v| v.as_str())
        .map(|raw| match raw.strip_prefix('!') {
            Some(negated) => format!("!{}", normalise_glob(negated)),
            None => normalise_glob(raw),
        })
        .collect()
}

fn read_manifest(_root: &Path, dir: &Path, rel: &str) -> Option<DiscoveredPackage> {
    let raw = std::fs::read_to_string(dir.join(MANIFEST)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let name = value
        .get("name")
        .and_then(|n| n.as_str())
        .map(str::to_string)
        .or_else(|| directory_name(dir))?;
    Some(DiscoveredPackage {
        name,
        path: rel.to_string(),
        manifest: MANIFEST,
        format: FileFormat::Json,
    })
}
