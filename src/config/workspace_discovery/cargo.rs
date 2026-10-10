use std::path::Path;

use toml_edit::{DocumentMut, Item};

use super::{DiscoveredPackage, Source, directory_name, normalise_glob};
use crate::config::package::FileFormat;

const MANIFEST: &str = "Cargo.toml";

pub(super) fn source(root: &Path) -> Source {
    Source {
        globs: globs(root),
        read: read_member,
    }
}

fn load(dir: &Path) -> Option<DocumentMut> {
    std::fs::read_to_string(dir.join(MANIFEST))
        .ok()?
        .parse()
        .ok()
}

fn globs(root: &Path) -> Vec<String> {
    let Some(doc) = load(root) else {
        return Vec::new();
    };
    let Some(workspace) = doc.get("workspace") else {
        return Vec::new();
    };
    let listed = |key: &str, prefix: &str| -> Vec<String> {
        workspace
            .get(key)
            .and_then(Item::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
            .map(|raw| format!("{prefix}{}", normalise_glob(raw)))
            .collect()
    };
    let mut globs = listed("members", "");
    globs.extend(listed("exclude", "!"));
    globs
}

fn read_member(root: &Path, dir: &Path, rel: &str) -> Option<DiscoveredPackage> {
    let doc = load(dir)?;
    let package = doc.get("package")?;
    if rel != "." && inherits_version(package) {
        return workspace_root_package(root);
    }
    let name = package
        .get("name")
        .and_then(Item::as_str)
        .map(str::to_string)
        .or_else(|| directory_name(dir))?;
    Some(DiscoveredPackage {
        name,
        path: rel.to_string(),
        manifest: MANIFEST,
        format: FileFormat::Toml,
    })
}

fn inherits_version(package: &Item) -> bool {
    package
        .get("version")
        .and_then(Item::as_table_like)
        .and_then(|table| table.get("workspace"))
        .and_then(Item::as_bool)
        .unwrap_or(false)
}

fn workspace_root_package(root: &Path) -> Option<DiscoveredPackage> {
    let doc = load(root)?;
    doc.get("workspace")?
        .get("package")?
        .get("version")?
        .as_str()?;
    let name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(Item::as_str)
        .map(str::to_string)
        .or_else(|| directory_name(root))?;
    Some(DiscoveredPackage {
        name,
        path: ".".to_string(),
        manifest: MANIFEST,
        format: FileFormat::Toml,
    })
}
