use wasm_bindgen::prelude::*;

use ferrflow::changelog::{self, GitLog};
use ferrflow::config::{self, Config, ConfigFormatHandler};
use ferrflow::conventional_commits;
use ferrflow::versioning;

#[wasm_bindgen]
pub fn determine_bump(message: &str) -> String {
    conventional_commits::determine_bump(message).to_string()
}

#[wasm_bindgen]
pub fn compute_next_version(current: &str, bump: &str, strategy: &str) -> Result<String, JsError> {
    let bump_type = match bump {
        "major" => ferrflow::conventional_commits::BumpType::Major,
        "minor" => ferrflow::conventional_commits::BumpType::Minor,
        "patch" => ferrflow::conventional_commits::BumpType::Patch,
        _ => ferrflow::conventional_commits::BumpType::None,
    };

    let strategy_type = match strategy {
        "calver" => ferrflow::config::VersioningStrategy::Calver,
        "calver-short" => ferrflow::config::VersioningStrategy::CalverShort,
        "calver-seq" => ferrflow::config::VersioningStrategy::CalverSeq,
        "sequential" => ferrflow::config::VersioningStrategy::Sequential,
        "zerover" => ferrflow::config::VersioningStrategy::Zerover,
        _ => ferrflow::config::VersioningStrategy::Semver,
    };

    versioning::compute_next_version(current, bump_type, strategy_type)
        .map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn build_changelog_section(version: &str, commits_json: &str) -> Result<String, JsError> {
    let raw: Vec<CommitInput> =
        serde_json::from_str(commits_json).map_err(|e| JsError::new(&e.to_string()))?;

    let commits: Vec<GitLog> = raw
        .into_iter()
        .map(|c| GitLog {
            hash: c.hash.unwrap_or_else(|| "0000000".to_string()),
            message: c.message,
        })
        .collect();

    Ok(changelog::build_section(version, &commits))
}

#[wasm_bindgen]
pub fn validate_config(config_json: &str) -> String {
    match serde_json::from_str::<Config>(config_json) {
        Ok(config) => {
            let mut errors: Vec<String> = Vec::new();

            if config.packages.is_empty() {
                errors.push("At least one package is required".to_string());
            }

            for (i, pkg) in config.packages.iter().enumerate() {
                if pkg.name.is_empty() {
                    errors.push(format!("Package {} has no name", i + 1));
                }
                if pkg.path.is_empty() {
                    errors.push(format!("Package '{}' has no path", pkg.name));
                }
            }

            if errors.is_empty() {
                r#"{"valid":true}"#.to_string()
            } else {
                serde_json::json!({ "valid": false, "errors": errors }).to_string()
            }
        }
        Err(e) => serde_json::json!({ "valid": false, "errors": [e.to_string()] }).to_string(),
    }
}

#[wasm_bindgen]
pub fn serialize_config(config_json: &str, format: &str) -> Result<String, JsError> {
    let config: Config =
        serde_json::from_str(config_json).map_err(|e| JsError::new(&e.to_string()))?;

    let handler: &dyn ConfigFormatHandler = match format {
        "toml" => config::format_handler(config::ConfigFileFormat::Toml),
        "json5" => config::format_handler(config::ConfigFileFormat::Json5),
        _ => config::format_handler(config::ConfigFileFormat::Json),
    };

    handler
        .serialize(&config)
        .map_err(|e| JsError::new(&e.to_string()))
}

#[derive(serde::Deserialize)]
struct CommitInput {
    message: String,
    hash: Option<String>,
}
