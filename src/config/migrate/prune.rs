use anyhow::Result;
use serde_json::{Map, Value};

use crate::config::Config;
use crate::config::format::to_camel_case_keys;
use crate::config::package::PackageConfig;
use crate::config::workspace::WorkspaceConfig;

const ALWAYS_WRITTEN_PACKAGE_KEYS: &[&str] = &["name", "path"];

pub(super) fn to_pretty_json(config: &Config) -> Result<String> {
    let mut root = serde_json::to_value(config)?;
    if let Value::Object(map) = &mut root {
        strip_defaults(map, &serde_json::to_value(Config::default())?, &[]);
        if let Some(Value::Object(workspace)) = map.get_mut("workspace") {
            strip_defaults(
                workspace,
                &serde_json::to_value(WorkspaceConfig::default())?,
                &[],
            );
        }
        map.retain(|_, v| v.as_object().is_none_or(|o| !o.is_empty()));
        if let Some(Value::Array(packages)) = map.get_mut("package") {
            let defaults = serde_json::to_value(PackageConfig::default())?;
            for package in packages.iter_mut().filter_map(Value::as_object_mut) {
                strip_defaults(package, &defaults, ALWAYS_WRITTEN_PACKAGE_KEYS);
            }
        }
    }
    let mut out = serde_json::to_string_pretty(&to_camel_case_keys(root))?;
    out.push('\n');
    Ok(out)
}

fn strip_defaults(map: &mut Map<String, Value>, defaults: &Value, keep: &[&str]) {
    map.retain(|key, value| keep.contains(&key.as_str()) || defaults.get(key) != Some(value));
}
