use serde_json::Value;

use crate::config::package::{FileFormat, VersionedFile};

use super::join_path;

const GENERIC_MARKER_SELECTOR: &str =
    r"v?(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z][0-9A-Za-z.+-]*)?)[^\n]*x-release-please-version";

const CARGO_VERSION_PATHS: &[&str] = &["$.package.version", "$.workspace.package.version"];

#[derive(Debug, PartialEq)]
pub(super) enum Mapped {
    Generic(VersionedFile),
    Direct(VersionedFile),
    Manual(String),
}

pub(super) fn map(pkg_path: &str, entry: &Value) -> Mapped {
    let (kind, path, jsonpath) = match entry {
        Value::String(path) => ("generic", path.as_str(), None),
        Value::Object(map) => {
            let Some(path) = map.get("path").and_then(Value::as_str) else {
                return Mapped::Manual(manual(pkg_path, entry, "entry has no `path`"));
            };
            let kind = map.get("type").and_then(Value::as_str).unwrap_or("generic");
            (kind, path, map.get("jsonpath").and_then(Value::as_str))
        }
        _ => return Mapped::Manual(manual(pkg_path, entry, "entry is not a string or object")),
    };
    let resolved = resolve(pkg_path, path);

    match (kind, jsonpath) {
        ("generic", _) => Mapped::Generic(VersionedFile {
            path: resolved,
            format: FileFormat::Txt,
            selector: Some(GENERIC_MARKER_SELECTOR.to_string()),
        }),
        ("toml", Some(jp)) if is_cargo_manifest(&resolved) && CARGO_VERSION_PATHS.contains(&jp) => {
            Mapped::Direct(VersionedFile {
                path: resolved,
                format: FileFormat::Toml,
                selector: None,
            })
        }
        ("json", Some("$.version")) => Mapped::Direct(VersionedFile {
            path: resolved,
            format: FileFormat::Json,
            selector: None,
        }),
        _ => Mapped::Manual(manual(
            pkg_path,
            entry,
            "no ferrflow versionedFiles equivalent",
        )),
    }
}

fn resolve(pkg_path: &str, path: &str) -> String {
    match path.strip_prefix('/') {
        Some(from_root) => from_root.to_string(),
        None => join_path(pkg_path, path),
    }
}

fn is_cargo_manifest(path: &str) -> bool {
    path.rsplit('/').next() == Some("Cargo.toml")
}

fn manual(pkg_path: &str, entry: &Value, reason: &str) -> String {
    format!(
        "package {pkg_path}: extra-files entry {entry} ({reason}), add it to `versionedFiles` by hand."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::VersionFile;
    use crate::formats::txt::TxtVersionFile;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const LUA_LINE: &str = "return \"0.5.0\" -- x-release-please-version\n";
    const README_LINE: &str =
        "    version='v0.5.0'  -- use the latest tagged releases x-release-please-version\n";

    fn generic_selector() -> String {
        match map(".", &json!({"type": "generic", "path": "f"})) {
            Mapped::Generic(vf) => vf.selector.expect("selector"),
            other => panic!("expected generic, got {other:?}"),
        }
    }

    fn file_with(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn selector_reads_a_quoted_version_on_the_marker_line() {
        let f = file_with(LUA_LINE);
        let v = TxtVersionFile
            .read_version_with_selector(f.path(), Some(&generic_selector()))
            .unwrap();
        assert_eq!(v, "0.5.0");
    }

    #[test]
    fn selector_leaves_a_leading_v_outside_the_capture() {
        let f = file_with(README_LINE);
        let v = TxtVersionFile
            .read_version_with_selector(f.path(), Some(&generic_selector()))
            .unwrap();
        assert_eq!(v, "0.5.0");
    }

    #[test]
    fn selector_write_replaces_only_the_version() {
        let f = file_with(README_LINE);
        TxtVersionFile
            .write_version_with_selector(f.path(), "0.6.0", Some(&generic_selector()))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(f.path()).unwrap(),
            "    version='v0.6.0'  -- use the latest tagged releases x-release-please-version\n"
        );
    }

    #[test]
    fn selector_ignores_versions_on_lines_without_the_marker() {
        let f = file_with(&format!("local lua = \"5.1.0\"\n{LUA_LINE}"));
        TxtVersionFile
            .write_version_with_selector(f.path(), "0.6.0", Some(&generic_selector()))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(f.path()).unwrap(),
            "local lua = \"5.1.0\"\nreturn \"0.6.0\" -- x-release-please-version\n"
        );
    }

    #[test]
    fn selector_keeps_a_prerelease_suffix_in_the_capture() {
        let f = file_with("v = \"1.0.0-rc.1\" # x-release-please-version\n");
        let v = TxtVersionFile
            .read_version_with_selector(f.path(), Some(&generic_selector()))
            .unwrap();
        assert_eq!(v, "1.0.0-rc.1");
    }

    #[test]
    fn selector_does_not_match_a_file_without_the_marker() {
        let f = file_with("return \"0.5.0\"\n");
        assert!(
            TxtVersionFile
                .read_version_with_selector(f.path(), Some(&generic_selector()))
                .is_err()
        );
    }

    #[test]
    fn plain_string_entry_is_a_generic_file() {
        assert!(matches!(
            map("pkg", &json!("docs/VERSION.md")),
            Mapped::Generic(vf) if vf.path == "pkg/docs/VERSION.md"
        ));
    }

    #[test]
    fn leading_slash_anchors_the_path_at_the_repo_root() {
        assert!(matches!(
            map("pkg", &json!({"type": "generic", "path": "/README.md"})),
            Mapped::Generic(vf) if vf.path == "README.md"
        ));
    }

    #[test]
    fn toml_workspace_package_version_on_cargo_toml_maps_to_toml() {
        let mapped = map(
            ".",
            &json!({"type": "toml", "path": "Cargo.toml", "jsonpath": "$.workspace.package.version"}),
        );
        assert_eq!(
            mapped,
            Mapped::Direct(VersionedFile {
                path: "Cargo.toml".to_string(),
                format: FileFormat::Toml,
                selector: None,
            })
        );
    }

    #[test]
    fn toml_package_version_on_a_nested_cargo_toml_maps_to_toml() {
        assert!(matches!(
            map(
                "crates/core",
                &json!({"type": "toml", "path": "Cargo.toml", "jsonpath": "$.package.version"})
            ),
            Mapped::Direct(vf) if vf.path == "crates/core/Cargo.toml"
        ));
    }

    #[test]
    fn json_version_maps_to_json() {
        assert!(matches!(
            map(".", &json!({"type": "json", "path": "manifest.json", "jsonpath": "$.version"})),
            Mapped::Direct(vf) if matches!(vf.format, FileFormat::Json) && vf.path == "manifest.json"
        ));
    }

    #[test]
    fn other_jsonpaths_need_manual_review_with_the_exact_entry() {
        for entry in [
            json!({"type": "json", "path": "package.json", "jsonpath": "$.metadata.version"}),
            json!({"type": "toml", "path": "pyproject.toml", "jsonpath": "$.project.version"}),
            json!({"type": "toml", "path": "Cargo.toml", "jsonpath": "$.dependencies.foo.version"}),
            json!({"type": "toml", "path": "Cargo.toml"}),
            json!({"type": "yaml", "path": "chart.yaml", "jsonpath": "$.version"}),
            json!({"type": "xml", "path": "pom.xml", "xpath": "//project/version"}),
            json!({"type": "pom", "path": "pom.xml"}),
        ] {
            match map(".", &entry) {
                Mapped::Manual(msg) => assert!(msg.contains(&entry.to_string()), "{msg}"),
                other => panic!("{entry} should need review, got {other:?}"),
            }
        }
    }

    #[test]
    fn malformed_entries_need_manual_review() {
        assert!(matches!(map(".", &json!(42)), Mapped::Manual(_)));
        assert!(matches!(
            map(".", &json!({"type": "generic"})),
            Mapped::Manual(_)
        ));
    }
}
