use std::path::Path;

/// A `package.json` found by expanding the JS workspace globs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscoveredPackage {
    /// `name` from the manifest, falling back to the directory name when the
    /// manifest is nameless (private workspace roots often are).
    pub name: String,
    /// Slash-separated path relative to the repo root.
    pub path: String,
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

/// Deep enough for `apps/*/packages/*` style layouts without walking a whole
/// monorepo's build output.
const MAX_DEPTH: usize = 6;

/// Reads workspace globs from `package.json` and `pnpm-workspace.yaml`, then
/// returns every `package.json` under a directory matching one of them.
///
/// Returns an empty vec when the repo declares no workspace, which is the
/// signal to fall back to a single root package.
pub(super) fn discover(root: &Path) -> Vec<DiscoveredPackage> {
    let globs = workspace_globs(root);
    if globs.is_empty() {
        return Vec::new();
    }

    let mut found = Vec::new();
    collect(root, root, 0, &globs, &mut found);
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found.dedup_by(|a, b| a.path == b.path);

    let excluded: Vec<&str> = globs.iter().filter_map(|g| g.strip_prefix('!')).collect();
    found.retain(|p| !excluded.iter().any(|g| glob_match::glob_match(g, &p.path)));
    found
}

fn workspace_globs(root: &Path) -> Vec<String> {
    let mut globs = globs_from_package_json(root);
    globs.extend(globs_from_pnpm_workspace(root));
    globs.sort();
    globs.dedup();
    globs
}

fn globs_from_package_json(root: &Path) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(root.join("package.json")) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    match value.get("workspaces") {
        // npm / yarn classic: "workspaces": ["packages/*"]
        Some(serde_json::Value::Array(items)) => string_list(items),
        // yarn berry: "workspaces": { "packages": ["packages/*"] }
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
        .map(normalise_glob)
        .collect()
}

/// `./packages/*` and `packages/*/` both mean `packages/*`.
fn normalise_glob(raw: &str) -> String {
    raw.trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn collect(
    root: &Path,
    dir: &Path,
    depth: usize,
    globs: &[String],
    found: &mut Vec<DiscoveredPackage>,
) {
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
            && globs.iter().any(|g| matches_glob(g, &rel))
            && let Some(pkg) = read_manifest(&path, &rel)
        {
            found.push(pkg);
        }
        collect(root, &path, depth + 1, globs, found);
    }
}

/// Negated globs (`!packages/internal`) never include anything; `discover`
/// subtracts them after the walk.
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

fn read_manifest(dir: &Path, rel: &str) -> Option<DiscoveredPackage> {
    let manifest = dir.join("package.json");
    let raw = std::fs::read_to_string(&manifest).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let name = value
        .get("name")
        .and_then(|n| n.as_str())
        .map(str::to_string)
        .or_else(|| dir.file_name().and_then(|n| n.to_str()).map(str::to_string))?;
    Some(DiscoveredPackage {
        name,
        path: rel.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn pkg(name: &str) -> String {
        format!("{{\"name\": \"{name}\", \"version\": \"1.0.0\"}}")
    }

    #[test]
    fn npm_workspaces_array_is_expanded() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
        write(root, "packages/a/package.json", &pkg("@acme/a"));
        write(root, "packages/b/package.json", &pkg("@acme/b"));

        let found = discover(root);
        assert_eq!(
            found,
            vec![
                DiscoveredPackage {
                    name: "@acme/a".into(),
                    path: "packages/a".into()
                },
                DiscoveredPackage {
                    name: "@acme/b".into(),
                    path: "packages/b".into()
                },
            ]
        );
    }

    #[test]
    fn yarn_berry_object_form_is_expanded() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "package.json",
            r#"{"workspaces": {"packages": ["libs/*"]}}"#,
        );
        write(root, "libs/one/package.json", &pkg("one"));

        let found = discover(root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "libs/one");
    }

    #[test]
    fn pnpm_workspace_globs_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"name": "root"}"#);
        write(root, "pnpm-workspace.yaml", "packages:\n  - 'apps/*'\n");
        write(root, "apps/web/package.json", &pkg("@acme/web"));

        let found = discover(root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "@acme/web");
        assert_eq!(found[0].path, "apps/web");
    }

    // node_modules holds thousands of package.json files; walking into it would
    // both be slow and scaffold dependencies as if they were our packages.
    #[test]
    fn node_modules_is_never_walked() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
        write(root, "packages/a/package.json", &pkg("@acme/a"));
        write(
            root,
            "packages/a/node_modules/left-pad/package.json",
            &pkg("left-pad"),
        );
        write(root, "node_modules/react/package.json", &pkg("react"));

        let found = discover(root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "@acme/a");
    }

    #[test]
    fn nested_globs_are_matched() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"workspaces": ["apps/*/pkg/*"]}"#);
        write(root, "apps/web/pkg/ui/package.json", &pkg("ui"));

        let found = discover(root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "apps/web/pkg/ui");
    }

    #[test]
    fn a_directory_without_a_manifest_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
        std::fs::create_dir_all(root.join("packages/empty")).unwrap();
        write(root, "packages/real/package.json", &pkg("real"));

        let found = discover(root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "real");
    }

    #[test]
    fn a_nameless_manifest_falls_back_to_its_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"workspaces": ["packages/*"]}"#);
        write(root, "packages/tools/package.json", r#"{"private": true}"#);

        let found = discover(root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "tools");
    }

    #[test]
    fn no_workspace_declaration_discovers_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"name": "solo"}"#);
        write(root, "packages/a/package.json", &pkg("@acme/a"));

        assert!(discover(root).is_empty());
    }

    #[test]
    fn negated_globs_exclude_their_matches() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "package.json",
            r#"{"workspaces": ["packages/*", "!packages/internal"]}"#,
        );
        write(root, "packages/keep/package.json", &pkg("keep"));
        write(root, "packages/internal/package.json", &pkg("internal"));

        let found = discover(root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "keep");
    }
}
