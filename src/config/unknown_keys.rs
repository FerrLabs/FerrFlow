use std::collections::HashSet;
use std::fmt;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::schema::BUNDLED_SCHEMA;

const SCHEMA_KEY: &str = "$schema";
const MIN_SIMILARITY: f64 = 0.85;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Segment {
    Key(String),
    Index(usize),
}

#[derive(Debug, PartialEq)]
pub(crate) struct UnknownKey {
    pub path: Vec<Segment>,
    pub suggestion: Option<String>,
}

impl fmt::Display for UnknownKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.path.iter().enumerate() {
            match segment {
                Segment::Key(key) if i == 0 => write!(f, "{key}")?,
                Segment::Key(key) => write!(f, ".{key}")?,
                Segment::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}

pub(crate) fn find_unknown_keys<T: DeserializeOwned>(
    value: Value,
    schema_prefix: &[Segment],
) -> Vec<UnknownKey> {
    let mut paths = Vec::new();
    let _: Result<T, _> = serde_ignored::deserialize(value, |path| paths.push(segments(&path)));
    if paths.is_empty() {
        return Vec::new();
    }
    let schema: Value = serde_json::from_str(BUNDLED_SCHEMA).unwrap_or(Value::Null);
    paths
        .into_iter()
        .filter(|path| !matches!(path.last(), Some(Segment::Key(key)) if key == SCHEMA_KEY))
        .filter(|path| !documented(&schema, schema_prefix, path))
        .map(|path| {
            let suggestion = suggest(&schema, schema_prefix, &path);
            UnknownKey { path, suggestion }
        })
        .collect()
}

pub(crate) fn warn_unknown_keys<T: DeserializeOwned>(
    value: Value,
    source: &Path,
    schema_prefix: &[Segment],
) {
    static REPORTED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let reported = REPORTED.get_or_init(Default::default);
    let shown = std::env::current_dir()
        .ok()
        .and_then(|cwd| source.strip_prefix(cwd).ok())
        .unwrap_or(source)
        .display();
    for key in find_unknown_keys::<T>(value, schema_prefix) {
        let message = match &key.suggestion {
            Some(suggestion) => {
                format!(
                    "Warning: unknown key `{key}` in {shown}, ignored. Did you mean `{suggestion}`?"
                )
            }
            None => format!("Warning: unknown key `{key}` in {shown}, ignored."),
        };
        let first_time = reported
            .lock()
            .map(|mut seen| seen.insert(message.clone()))
            .unwrap_or(true);
        if first_time {
            tracing::warn!("{message}");
        }
    }
}

fn segments(path: &serde_ignored::Path<'_>) -> Vec<Segment> {
    use serde_ignored::Path;
    match path {
        Path::Root => Vec::new(),
        Path::Seq { parent, index } => {
            let mut out = segments(parent);
            out.push(Segment::Index(*index));
            out
        }
        Path::Map { parent, key } => {
            let mut out = segments(parent);
            out.push(Segment::Key(key.clone()));
            out
        }
        Path::Some { parent }
        | Path::NewtypeStruct { parent }
        | Path::NewtypeVariant { parent } => segments(parent),
    }
}

fn documented(schema: &Value, prefix: &[Segment], path: &[Segment]) -> bool {
    prefix
        .iter()
        .chain(path)
        .try_fold(schema, |node, segment| child(schema, node, segment))
        .is_some()
}

fn suggest(schema: &Value, prefix: &[Segment], path: &[Segment]) -> Option<String> {
    let (Segment::Key(unknown), parent) = path.split_last()? else {
        return None;
    };
    let node = prefix
        .iter()
        .chain(parent)
        .try_fold(schema, |node, segment| child(schema, node, segment))?;
    let candidates = resolve(schema, node)?.get("properties")?.as_object()?;
    nearest(unknown, candidates.keys().map(String::as_str))
}

fn child<'a>(root: &'a Value, node: &'a Value, segment: &Segment) -> Option<&'a Value> {
    let node = resolve(root, node)?;
    match segment {
        Segment::Key(key) => node
            .get("properties")
            .and_then(|properties| properties.get(key))
            .or_else(|| node.get("additionalProperties").filter(|v| v.is_object())),
        Segment::Index(_) => node.get("items"),
    }
}

fn resolve<'a>(root: &'a Value, node: &'a Value) -> Option<&'a Value> {
    match node.get("$ref").and_then(Value::as_str) {
        Some(reference) => root.pointer(reference.strip_prefix('#')?),
        None => Some(node),
    }
}

fn nearest<'a>(unknown: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let wanted = normalize(unknown);
    candidates
        .filter(|candidate| *candidate != SCHEMA_KEY)
        .map(|candidate| {
            (
                candidate,
                strsim::jaro_winkler(&wanted, &normalize(candidate)),
            )
        })
        .filter(|(_, score)| *score >= MIN_SIMILARITY)
        .fold(None, |best: Option<(&str, f64)>, current| match best {
            Some(best) if best.1 >= current.1 => Some(best),
            _ => Some(current),
        })
        .map(|(candidate, _)| candidate.to_string())
}

fn normalize(key: &str) -> String {
    key.chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::config::{Config, PackageConfig};

    fn reported(value: Value) -> Vec<(String, Option<String>)> {
        find_unknown_keys::<Config>(value, &[])
            .into_iter()
            .map(|key| (key.to_string(), key.suggestion))
            .collect()
    }

    #[test]
    fn misspelled_hooks_are_reported_with_their_path_and_the_hook_they_meant() {
        let found = reported(json!({
            "workspace": { "hooks": { "post-bump": "a", "postBumpp": "b" } },
            "package": [{ "name": "core", "path": ".", "versionedFiles": [] }]
        }));
        assert_eq!(
            found,
            vec![
                (
                    "workspace.hooks.post-bump".to_string(),
                    Some("postBump".to_string())
                ),
                (
                    "workspace.hooks.postBumpp".to_string(),
                    Some("postBump".to_string())
                ),
            ]
        );
    }

    #[test]
    fn keys_inside_a_package_entry_carry_the_index() {
        let found = reported(json!({
            "package": [
                { "name": "a", "path": "a", "versionedFiles": [] },
                { "name": "b", "path": "b", "versionedFiles": [], "chnagelog": "CHANGELOG.md" }
            ]
        }));
        assert_eq!(
            found,
            vec![(
                "package[1].chnagelog".to_string(),
                Some("changelog".to_string())
            )]
        );
    }

    #[test]
    fn keys_under_a_free_form_map_resolve_through_additional_properties() {
        let found = reported(json!({
            "workspace": { "registries": { "kellnr": { "url": "https://k", "tokenEnvv": "T" } } }
        }));
        assert_eq!(
            found,
            vec![(
                "workspace.registries.kellnr.tokenEnvv".to_string(),
                Some("tokenEnv".to_string())
            )]
        );
    }

    #[test]
    fn retired_keys_the_schema_still_documents_are_not_reported() {
        assert!(
            reported(json!({
                "workspace": { "telemetry": false, "anonymous_telemetry": false }
            }))
            .is_empty()
        );
    }

    #[test]
    fn an_unrelated_key_gets_no_suggestion() {
        assert_eq!(
            reported(json!({ "whatever": true })),
            vec![("whatever".to_string(), None)]
        );
    }

    #[test]
    fn valid_configs_report_nothing() {
        assert!(
            reported(json!({
                "$schema": "https://ferrflow.com/schema/ferrflow.json",
                "workspace": {
                    "tagTemplate": "v{{version}}",
                    "build_metadata": "git rev-parse --short HEAD",
                    "hooks": { "pre_bump": "true", "postBump": "true", "onFailure": "continue" },
                    "registries": { "kellnr": { "url": "https://k", "token_env": "T" } }
                },
                "package": [{
                    "name": "core",
                    "path": ".",
                    "versioned_files": [{ "path": "Cargo.toml", "format": "toml" }]
                }]
            }))
            .is_empty()
        );
    }

    #[test]
    fn included_fragments_are_checked_against_the_package_schema() {
        let found = find_unknown_keys::<PackageConfig>(
            json!({ "name": "core", "versionedFiless": [] }),
            &[Segment::Key("package".into()), Segment::Index(0)],
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].to_string(), "versionedFiless");
        assert_eq!(found[0].suggestion.as_deref(), Some("versionedFiles"));
    }
}
