use anyhow::{Context, Result};

use crate::error_code::{self, ErrorCodeExt};

use super::Config;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum ConfigFileFormat {
    Json,
    Json5,
    Toml,
    Dotfile,
}

pub trait ConfigFormatHandler {
    fn filename(&self) -> &str;
    fn parse(&self, content: &str) -> Result<Config>;
    fn serialize(&self, config: &Config) -> Result<String>;
}

pub(crate) struct JsonFormat;
pub(crate) struct Json5Format;
pub(crate) struct TomlFormat;
pub(crate) struct DotfileFormat;

pub(crate) fn snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

const CAMEL_CASE_KEYS: &[&str] = &[
    "tag_template",
    "versioned_files",
    "shared_paths",
    "recover_missed_releases",
    "release_commit_mode",
    "release_commit_scope",
    "auto_merge_releases",
    "skip_ci",
    "commit_skip_markers",
    "pre_bump",
    "post_bump",
    "pre_commit",
    "pre_publish",
    "post_publish",
    "on_failure",
    "floating_tags",
    "orphaned_tag_strategy",
    "prerelease_identifier",
    "token_env",
    "allow_dirty",
    "no_verify",
    "display_name",
];

pub(crate) fn to_camel_case_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let new_map = map
                .into_iter()
                .map(|(k, v)| {
                    let new_key = if CAMEL_CASE_KEYS.contains(&k.as_str()) {
                        snake_to_camel(&k)
                    } else {
                        k
                    };
                    (new_key, to_camel_case_keys(v))
                })
                .collect();
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(to_camel_case_keys).collect())
        }
        other => other,
    }
}

impl ConfigFormatHandler for JsonFormat {
    fn filename(&self) -> &str {
        "ferrflow.json"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        serde_json::from_str(content)
            .with_context(|| "Failed to parse ferrflow.json")
            .error_code(error_code::CONFIG_PARSE_JSON)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        let value = serde_json::to_value(config)?;
        let camel = to_camel_case_keys(value);
        let mut out = serde_json::to_string_pretty(&camel)?;
        out.push('\n');
        Ok(out)
    }
}

impl ConfigFormatHandler for Json5Format {
    fn filename(&self) -> &str {
        "ferrflow.json5"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        json5::from_str(content)
            .with_context(|| "Failed to parse ferrflow.json5")
            .error_code(error_code::CONFIG_PARSE_JSON5)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        // json5 crate has no serializer; valid JSON is valid JSON5
        let value = serde_json::to_value(config)?;
        let camel = to_camel_case_keys(value);
        let mut out = serde_json::to_string_pretty(&camel)?;
        out.push('\n');
        Ok(out)
    }
}

impl ConfigFormatHandler for TomlFormat {
    fn filename(&self) -> &str {
        "ferrflow.toml"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        toml_edit::de::from_str(content)
            .with_context(|| "Failed to parse ferrflow.toml")
            .error_code(error_code::CONFIG_PARSE_TOML)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        toml_edit::ser::to_string_pretty(config)
            .with_context(|| "Failed to serialize to TOML")
            .error_code(error_code::CONFIG_SERIALIZE_TOML)
    }
}

impl ConfigFormatHandler for DotfileFormat {
    fn filename(&self) -> &str {
        ".ferrflow"
    }
    fn parse(&self, content: &str) -> Result<Config> {
        ConfigFormatHandler::parse(&JsonFormat, content)
            .with_context(|| "Failed to parse .ferrflow")
            .error_code(error_code::CONFIG_PARSE_DOTFILE)
    }
    fn serialize(&self, config: &Config) -> Result<String> {
        ConfigFormatHandler::serialize(&JsonFormat, config)
            .with_context(|| "Failed to serialize .ferrflow")
            .error_code(error_code::CONFIG_SERIALIZE_DOTFILE)
    }
}

/// Ordered by priority: json > json5 > toml > .ferrflow
pub(crate) const CONFIG_FORMATS: &[&dyn ConfigFormatHandler] =
    &[&JsonFormat, &Json5Format, &TomlFormat, &DotfileFormat];

pub fn format_handler(fmt: ConfigFileFormat) -> &'static dyn ConfigFormatHandler {
    match fmt {
        ConfigFileFormat::Json => &JsonFormat,
        ConfigFileFormat::Json5 => &Json5Format,
        ConfigFileFormat::Toml => &TomlFormat,
        ConfigFileFormat::Dotfile => &DotfileFormat,
    }
}
