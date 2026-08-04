use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::{FileFormat, VersionedFile};
use crate::error_code::{self, ErrorCodeExt};

/// Dependency tables rewritten in a `package.json`.
const JSON_SECTIONS: &[&str] = &[
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
];

/// Dependency tables rewritten in a `Cargo.toml`.
const TOML_SECTIONS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

/// A manifest rewrite that has been computed but not yet applied, so a dry run
/// can report exactly what a real run would write.
pub struct PlannedUpdate {
    path: PathBuf,
    content: String,
}

impl PlannedUpdate {
    pub fn apply(&self) -> Result<()> {
        std::fs::write(&self.path, &self.content)
            .with_context(|| format!("Cannot write {}", self.path.display()))
            .error_code(error_code::JSON_WRITE)
    }
}

/// Computes the rewrite of `dep_name`'s version constraint to `new_version` in
/// `vf`, keeping the file's formatting and the constraint's operator.
///
/// `Ok(None)` means there is nothing to do: a format with no defined notion of
/// a dependency table, a missing or unparseable manifest, an absent dependency,
/// a constraint we refuse to touch, or one already on `new_version`. This never
/// fails a release over a manifest it does not understand.
pub fn plan_dependency_update(
    vf: &VersionedFile,
    repo_root: &Path,
    dep_name: &str,
    new_version: &str,
) -> Result<Option<PlannedUpdate>> {
    let path = super::join_within_repo(repo_root, &vf.path)?;
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };

    let updated = match vf.format {
        FileFormat::Json => rewrite_json(&content, dep_name, new_version),
        FileFormat::Toml => rewrite_toml(&content, dep_name, new_version),
        _ => None,
    };

    Ok(updated.map(|content| PlannedUpdate { path, content }))
}

/// Whether a format has a dependency table this module knows how to rewrite.
pub fn supports_dependency_updates(format: &FileFormat) -> bool {
    matches!(format, FileFormat::Json | FileFormat::Toml)
}

/// Replaces the version inside a constraint while keeping its operator, so
/// `^1.2.3` becomes `^2.0.0` rather than a bare pin.
///
/// Returns `None` for anything that is not a plain operator + version:
/// wildcards, `workspace:*`, `file:`/`git:` specs, multi-part ranges. Those
/// carry intent we cannot preserve, and silently rewriting them would be worse
/// than leaving them for a human.
fn rewrite_constraint(existing: &str, new_version: &str) -> Option<String> {
    let trimmed = existing.trim();
    if trimmed.is_empty()
        || trimmed.contains(char::is_whitespace)
        || trimmed.contains(':')
        || trimmed.contains(',')
        || trimmed.contains("||")
    {
        return None;
    }

    let digit_at = trimmed.find(|c: char| c.is_ascii_digit())?;
    let (prefix, version) = trimmed.split_at(digit_at);

    if !prefix
        .chars()
        .all(|c| matches!(c, '^' | '~' | '>' | '<' | '=' | 'v'))
    {
        return None;
    }
    // `1.x` / `1.*` are ranges, not pins — the same reasoning as above.
    if version
        .chars()
        .any(|c| matches!(c, 'x' | 'X' | '*' | '|' | ' '))
    {
        return None;
    }
    if !version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
    {
        return None;
    }

    Some(format!("{prefix}{new_version}"))
}

fn rewrite_json(content: &str, dep_name: &str, new_version: &str) -> Option<String> {
    if serde_json::from_str::<serde_json::Value>(content).is_err() {
        return None;
    }

    let mut out = content.to_string();
    let mut changed = false;
    // Walk sections back to front: every splice shifts the offsets after it,
    // and later sections sit later in the file.
    let mut spans: Vec<(usize, usize)> = JSON_SECTIONS
        .iter()
        .filter_map(|section| {
            super::json::find_nested_string_value_span(content, section, dep_name)
        })
        .collect();
    spans.sort_unstable();
    spans.dedup();

    for (start, end) in spans.into_iter().rev() {
        let existing = &content[start..end];
        let Some(replacement) = rewrite_constraint(existing, new_version) else {
            continue;
        };
        if replacement == existing {
            continue;
        }
        out.replace_range(start..end, &replacement);
        changed = true;
    }

    changed.then_some(out)
}

fn rewrite_toml(content: &str, dep_name: &str, new_version: &str) -> Option<String> {
    let mut doc = content.parse::<toml_edit::DocumentMut>().ok()?;
    let mut changed = false;

    for section in TOML_SECTIONS {
        let Some(table) = doc.get_mut(section).and_then(|i| i.as_table_like_mut()) else {
            continue;
        };
        let Some(entry) = table.get_mut(dep_name) else {
            continue;
        };

        // `dep = "1.2"` and `dep = { version = "1.2", path = ".." }` are both
        // common; a path/git-only entry has no version to rewrite.
        if let Some(existing) = entry.as_str() {
            if let Some(replacement) = rewrite_constraint(existing, new_version)
                && replacement != existing
            {
                *entry = toml_edit::value(replacement);
                changed = true;
            }
        } else if let Some(inline) = entry.as_table_like_mut()
            && let Some(version) = inline.get_mut("version")
            && let Some(existing) = version.as_str()
            && let Some(replacement) = rewrite_constraint(existing, new_version)
            && replacement != existing
        {
            *version = toml_edit::value(replacement);
            changed = true;
        }
    }

    changed.then(|| doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_caret_and_replaces_the_version() {
        assert_eq!(rewrite_constraint("^1.2.3", "2.0.0").unwrap(), "^2.0.0");
    }

    #[test]
    fn keeps_other_operators() {
        for (existing, expected) in [
            ("~1.2.3", "~2.0.0"),
            (">=1.2.3", ">=2.0.0"),
            ("=1.2.3", "=2.0.0"),
            ("v1.2.3", "v2.0.0"),
            ("1.2.3", "2.0.0"),
        ] {
            assert_eq!(rewrite_constraint(existing, "2.0.0").unwrap(), expected);
        }
    }

    // Everything below carries intent a version pin would destroy. Leaving
    // them alone is the whole safety story of this module.
    #[test]
    fn refuses_specs_it_cannot_preserve() {
        for existing in [
            "*",
            "workspace:*",
            "workspace:^1.2.3",
            "file:../core",
            "git:git@github.com:a/b.git",
            "npm:core@1.2.3",
            ">=1.0.0 <2.0.0",
            "^1.0.0 || ^2.0.0",
            "1.x",
            "1.*",
            "latest",
            "",
        ] {
            assert!(
                rewrite_constraint(existing, "2.0.0").is_none(),
                "{existing:?} must be left untouched"
            );
        }
    }

    #[test]
    fn rewrites_a_package_json_dependency_in_place() {
        let original = "{\n  \"name\": \"cli\",\n  \"version\": \"2.1.0\",\n  \"dependencies\": {\n    \"core\": \"^1.4.0\",\n    \"other\": \"^9.9.9\"\n  }\n}\n";
        let out = rewrite_json(original, "core", "2.0.0").expect("rewrites");
        assert!(out.contains("\"core\": \"^2.0.0\""));
        assert!(
            out.contains("\"other\": \"^9.9.9\""),
            "unrelated dependencies must not move"
        );
        assert!(
            out.contains("\"version\": \"2.1.0\""),
            "the package's own version is not this module's job"
        );
    }

    // A name appearing in several tables must be updated in all of them, and
    // the offsets must survive multiple splices in one pass.
    #[test]
    fn rewrites_every_section_that_mentions_the_dependency() {
        let original = "{\n  \"dependencies\": {\n    \"core\": \"^1.4.0\"\n  },\n  \"devDependencies\": {\n    \"core\": \"~1.4.0\"\n  },\n  \"peerDependencies\": {\n    \"core\": \"1.4.0\"\n  }\n}\n";
        let out = rewrite_json(original, "core", "2.0.0").expect("rewrites");
        assert!(out.contains("\"core\": \"^2.0.0\""));
        assert!(out.contains("\"core\": \"~2.0.0\""));
        assert!(out.contains("\"core\": \"2.0.0\""));
    }

    #[test]
    fn a_missing_json_dependency_changes_nothing() {
        let original = "{\n  \"dependencies\": {\n    \"other\": \"^1.0.0\"\n  }\n}\n";
        assert!(rewrite_json(original, "core", "2.0.0").is_none());
    }

    #[test]
    fn rewrites_a_cargo_string_dependency() {
        let original = "[package]\nname = \"cli\"\nversion = \"2.1.0\"\n\n[dependencies]\ncore = \"1.4\"\nserde = \"1\"\n";
        let out = rewrite_toml(original, "core", "2.0.0").expect("rewrites");
        assert!(out.contains("core = \"2.0.0\""));
        assert!(out.contains("serde = \"1\""), "unrelated deps stay put");
        assert!(out.contains("version = \"2.1.0\""), "own version untouched");
    }

    #[test]
    fn rewrites_the_version_inside_an_inline_table() {
        let original = "[dependencies]\ncore = { version = \"1.4\", path = \"../core\", features = [\"a\"] }\n";
        let out = rewrite_toml(original, "core", "2.0.0").expect("rewrites");
        assert!(out.contains("version = \"2.0.0\""));
        assert!(
            out.contains("path = \"../core\"") && out.contains("features = [\"a\"]"),
            "the rest of the entry must survive: {out}"
        );
    }

    // A path-only or git-only dependency has no version to move.
    #[test]
    fn leaves_a_dependency_with_no_version_alone() {
        let original = "[dependencies]\ncore = { path = \"../core\" }\n";
        assert!(rewrite_toml(original, "core", "2.0.0").is_none());
    }

    #[test]
    fn rewrites_dev_and_build_dependencies_too() {
        let original =
            "[dev-dependencies]\ncore = \"1.4\"\n\n[build-dependencies]\ncore = \"~1.4\"\n";
        let out = rewrite_toml(original, "core", "2.0.0").expect("rewrites");
        assert!(out.contains("core = \"2.0.0\""));
        assert!(out.contains("core = \"~2.0.0\""));
    }

    fn versioned(path: &str, format: FileFormat) -> VersionedFile {
        VersionedFile {
            path: path.to_string(),
            format,
            selector: None,
        }
    }

    // Planning must not touch the file — that is what makes `--dry-run` able to
    // report the rewrite it would perform.
    #[test]
    fn planning_reports_the_change_without_writing_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("package.json");
        let original = "{\n  \"dependencies\": {\n    \"core\": \"^1.0.0\"\n  }\n}\n";
        std::fs::write(&path, original).unwrap();

        let vf = versioned("package.json", FileFormat::Json);
        let planned = plan_dependency_update(&vf, dir.path(), "core", "2.0.0")
            .unwrap()
            .expect("a rewrite is planned");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);

        planned.apply().unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("\"core\": \"^2.0.0\"")
        );
    }

    // A second pass over an already-current manifest must plan nothing, or the
    // release commit would carry a file with no diff.
    #[test]
    fn an_already_current_manifest_plans_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("package.json");
        std::fs::write(
            &path,
            "{\n  \"dependencies\": {\n    \"core\": \"^2.0.0\"\n  }\n}\n",
        )
        .unwrap();

        let vf = versioned("package.json", FileFormat::Json);
        assert!(
            plan_dependency_update(&vf, dir.path(), "core", "2.0.0")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_missing_manifest_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let vf = versioned("package.json", FileFormat::Json);
        assert!(
            plan_dependency_update(&vf, dir.path(), "core", "2.0.0")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn only_json_and_toml_are_supported() {
        assert!(supports_dependency_updates(&FileFormat::Json));
        assert!(supports_dependency_updates(&FileFormat::Toml));
        for other in [FileFormat::Gradle, FileFormat::Xml, FileFormat::Txt] {
            assert!(!supports_dependency_updates(&other));
        }
    }
}
