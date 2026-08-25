use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use crate::error_code::{self, ErrorCodeExt};

use super::format::{DotfileFormat, Json5Format, JsonFormat, TomlFormat};
use super::{Config, ConfigFormatHandler, PackageConfig};

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

const MAX_DEPTH: usize = 8;

const FRAGMENT_ONLY_KEYS: &[&str] = &["workspace", "include", "package"];

pub(super) fn resolve(config: &mut Config, config_path: &Path, repo_root: &Path) -> Result<()> {
    if config.include.is_empty() {
        return Ok(());
    }

    let config_dir = config_path.parent().unwrap_or(repo_root);
    let patterns = std::mem::take(&mut config.include);
    let mut included = Vec::new();

    for pattern in &patterns {
        let matches = expand(pattern, config_dir, config_path);
        if matches.is_empty() {
            Err(anyhow::anyhow!(
                "include pattern matched no file: {pattern}\n\
                 Remove the pattern, or fix the path relative to {}.",
                config_dir.display()
            ))
            .error_code(error_code::CONFIG_INCLUDE_NOT_FOUND)?;
        }
        for path in matches {
            included.push(load_fragment(&path, repo_root)?);
        }
    }

    config.packages.extend(included);
    reject_duplicate_names(&config.packages)?;
    Ok(())
}

fn load_fragment(path: &Path, repo_root: &Path) -> Result<PackageConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))
        .error_code(error_code::CONFIG_READ_FAILED)?;

    let handler = handler_for(path);
    let value = handler
        .parse_value(&content)
        .with_context(|| format!("in included file {}", path.display()))?;

    let object = value.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "included file {} must contain a single package object",
            path.display()
        )
    })?;

    for key in FRAGMENT_ONLY_KEYS {
        if object.contains_key(*key) {
            Err(anyhow::anyhow!(
                "included file {} declares `{key}`, which only belongs in the root config.\n\
                 An included file describes one package, using the same keys as a `package` entry.",
                path.display()
            ))
            .error_code(error_code::CONFIG_INCLUDE_INVALID)?;
        }
    }

    let mut package: PackageConfig = serde_json::from_value(value)
        .with_context(|| format!("in included file {}", path.display()))
        .error_code(error_code::CONFIG_INCLUDE_INVALID)?;

    let fragment_dir = path.parent().unwrap_or(repo_root);
    package.path = fragment_path(&package.path, fragment_dir, repo_root, path)?;
    Ok(package)
}

fn fragment_path(
    declared: &str,
    fragment_dir: &Path,
    repo_root: &Path,
    fragment: &Path,
) -> Result<String> {
    let joined = if declared.is_empty() {
        fragment_dir.to_path_buf()
    } else {
        fragment_dir.join(declared)
    };

    let normalised = normalise(&joined);
    let root = normalise(repo_root);

    let relative = normalised.strip_prefix(&root).map_err(|_| {
        anyhow::anyhow!(
            "included file {} resolves to {}, which is outside the repository root {}",
            fragment.display(),
            normalised.display(),
            root.display()
        )
    });

    let relative = match relative {
        Ok(rel) => rel,
        Err(err) => return Err(err).error_code(error_code::CONFIG_INCLUDE_OUTSIDE_ROOT),
    };

    Ok(to_slash(relative))
}

pub(super) fn reject_duplicate_names(packages: &[PackageConfig]) -> Result<()> {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    for package in packages {
        if let Some(first) = seen.insert(&package.name, &package.path) {
            Err(anyhow::anyhow!(
                "duplicate package name `{}`, declared at both `{first}` and `{}`",
                package.name,
                package.path
            ))
            .error_code(error_code::CONFIG_DUPLICATE_PACKAGE)?;
        }
    }
    Ok(())
}

fn handler_for(path: &Path) -> &'static dyn ConfigFormatHandler {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "json5" => &Json5Format,
        "toml" => &TomlFormat,
        _ if filename == ".ferrflow" => &DotfileFormat,
        _ => &JsonFormat,
    }
}

fn is_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?', '[', '{'])
}

fn expand(pattern: &str, base: &Path, own_path: &Path) -> Vec<PathBuf> {
    if !is_glob(pattern) {
        let candidate = base.join(pattern);
        return if candidate.is_file() {
            vec![candidate]
        } else {
            Vec::new()
        };
    }

    let mut found = Vec::new();
    collect(base, base, 0, pattern, own_path, &mut found);
    found.sort();
    found
}

fn collect(
    base: &Path,
    dir: &Path,
    depth: usize,
    pattern: &str,
    own_path: &Path,
    found: &mut Vec<PathBuf>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name) {
                continue;
            }
            collect(base, &path, depth + 1, pattern, own_path, found);
        } else if path != own_path
            && let Ok(rel) = path.strip_prefix(base)
            && glob_match::glob_match(pattern, &to_slash(rel))
        {
            found.push(path);
        }
    }
}

fn to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn load(root: &Path) -> Result<Config> {
        Config::load(root, None)
    }

    fn names_and_paths(config: &Config) -> Vec<(String, String)> {
        config
            .packages
            .iter()
            .map(|p| (p.name.clone(), p.path.clone()))
            .collect()
    }

    #[test]
    fn glob_collects_one_package_per_fragment() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["projects/*/ferrflow.json"]}"#,
        );
        write(root, "projects/api/ferrflow.json", r#"{"name":"api"}"#);
        write(root, "projects/web/ferrflow.json", r#"{"name":"web"}"#);

        let config = load(root).unwrap();

        assert_eq!(
            names_and_paths(&config),
            vec![
                ("api".to_string(), "projects/api".to_string()),
                ("web".to_string(), "projects/web".to_string()),
            ]
        );
    }

    #[test]
    fn fragment_path_defaults_to_its_own_directory_not_the_repo_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["a/b/c/ferrflow.json"]}"#,
        );
        write(root, "a/b/c/ferrflow.json", r#"{"name":"deep"}"#);

        let config = load(root).unwrap();

        assert_eq!(config.packages[0].path, "a/b/c");
    }

    #[test]
    fn fragment_path_is_relative_to_the_fragment_not_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["tools/ferrflow.json"]}"#,
        );
        write(
            root,
            "tools/ferrflow.json",
            r#"{"name":"cli","path":"cli"}"#,
        );

        let config = load(root).unwrap();

        assert_eq!(config.packages[0].path, "tools/cli");
    }

    #[test]
    fn fragment_escaping_the_repo_root_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["pkg/ferrflow.json"]}"#,
        );
        write(
            root,
            "pkg/ferrflow.json",
            r#"{"name":"escapee","path":"../../outside"}"#,
        );

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("outside the repository root"), "{err}");
    }

    #[test]
    fn a_pattern_matching_nothing_is_an_error_rather_than_a_silent_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["projects/*/ferrflow.json"]}"#,
        );

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("matched no file"), "{err}");
    }

    #[test]
    fn a_missing_literal_path_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["projects/api/ferrflow.json"]}"#,
        );

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("matched no file"), "{err}");
    }

    #[test]
    fn duplicate_names_across_fragments_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["projects/*/ferrflow.json"]}"#,
        );
        write(root, "projects/one/ferrflow.json", r#"{"name":"dup"}"#);
        write(root, "projects/two/ferrflow.json", r#"{"name":"dup"}"#);

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("duplicate package name `dup`"), "{err}");
    }

    #[test]
    fn a_fragment_colliding_with_an_inline_package_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["projects/*/ferrflow.json"],"package":[{"name":"api","path":"legacy"}]}"#,
        );
        write(root, "projects/api/ferrflow.json", r#"{"name":"api"}"#);

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("duplicate package name `api`"), "{err}");
    }

    #[test]
    fn a_fragment_declaring_workspace_settings_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["pkg/ferrflow.json"]}"#,
        );
        write(
            root,
            "pkg/ferrflow.json",
            r#"{"name":"api","workspace":{"versioning":"semver"}}"#,
        );

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("declares `workspace`"), "{err}");
    }

    #[test]
    fn a_fragment_using_the_root_package_array_shape_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["pkg/ferrflow.json"]}"#,
        );
        write(root, "pkg/ferrflow.json", r#"{"package":[{"name":"api"}]}"#);

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("declares `package`"), "{err}");
    }

    #[test]
    fn nested_includes_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["pkg/ferrflow.json"]}"#,
        );
        write(
            root,
            "pkg/ferrflow.json",
            r#"{"name":"api","include":["deeper/ferrflow.json"]}"#,
        );

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("declares `include`"), "{err}");
    }

    #[test]
    fn inline_and_included_packages_merge() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["projects/*/ferrflow.json"],"package":[{"name":"root-pkg","path":"."}]}"#,
        );
        write(root, "projects/api/ferrflow.json", r#"{"name":"api"}"#);

        let config = load(root).unwrap();

        assert_eq!(
            names_and_paths(&config),
            vec![
                ("root-pkg".to_string(), ".".to_string()),
                ("api".to_string(), "projects/api".to_string()),
            ]
        );
    }

    #[test]
    fn fragments_may_use_a_different_format_than_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["api/ferrflow.toml"]}"#,
        );
        write(root, "api/ferrflow.toml", "name = \"api\"\n");

        let config = load(root).unwrap();

        assert_eq!(
            names_and_paths(&config),
            vec![("api".to_string(), "api".to_string())]
        );
    }

    #[test]
    fn fragments_keep_package_level_settings_and_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"include":["projects/*/ferrflow.json"]}"#,
        );
        write(root, "projects/api/ferrflow.json", r#"{"name":"api"}"#);
        write(
            root,
            "projects/web/ferrflow.json",
            r#"{"name":"web","dependsOn":["api"],"versioning":"calver"}"#,
        );

        let config = load(root).unwrap();
        let web = config.packages.iter().find(|p| p.name == "web").unwrap();

        assert_eq!(web.depends_on.len(), 1);
        assert_eq!(web.depends_on[0].name(), "api");
        assert!(web.versioning.is_some());
    }

    #[test]
    fn skipped_directories_are_not_scanned() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "ferrflow.json", r#"{"include":["**/ferrflow.json"]}"#);
        write(root, "projects/api/ferrflow.json", r#"{"name":"api"}"#);
        write(
            root,
            "node_modules/evil/ferrflow.json",
            r#"{"name":"evil"}"#,
        );

        let config = load(root).unwrap();
        let names: Vec<&str> = config.packages.iter().map(|p| p.name.as_str()).collect();

        assert_eq!(names, vec!["api"]);
    }

    #[test]
    fn an_inline_package_without_a_path_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "ferrflow.json", r#"{"package":[{"name":"api"}]}"#);

        let err = format!("{:#}", load(root).unwrap_err());

        assert!(err.contains("has no `path`"), "{err}");
    }

    #[test]
    fn a_config_without_include_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "ferrflow.json",
            r#"{"package":[{"name":"api","path":"api"}]}"#,
        );

        let config = load(root).unwrap();

        assert_eq!(
            names_and_paths(&config),
            vec![("api".to_string(), "api".to_string())]
        );
    }
}
