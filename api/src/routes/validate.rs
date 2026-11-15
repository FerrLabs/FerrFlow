use std::collections::BTreeMap;

use axum::Json;
use ferrflow::validate::{ValidationEntry, ValidationLevel, ValidationResult, validate_files};
use serde::Deserialize;

use crate::engine;

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub config: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

pub async fn validate(Json(req): Json<ValidateRequest>) -> Json<ValidationResult> {
    match engine::parse_config(&req.config, req.format.as_deref()) {
        Ok(config) => {
            let files = req
                .files
                .into_iter()
                .map(|(path, contents)| (path, contents.into_bytes()))
                .collect();
            Json(validate_files(&config, files))
        }
        Err(message) => Json(config_error(message)),
    }
}

fn config_error(message: String) -> ValidationResult {
    let mut result = ValidationResult::from_entries(vec![ValidationEntry {
        level: ValidationLevel::Error,
        path: "(config)".to_string(),
        message,
    }]);
    result.config_file = None;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unparseable_config_is_a_failed_validation_not_an_error() {
        let req = ValidateRequest {
            config: "not json at all {".to_string(),
            format: Some("json".to_string()),
            files: BTreeMap::new(),
        };
        let Json(result) = validate(Json(req)).await;
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn version_mismatch_across_files_is_reported() {
        let mut files = BTreeMap::new();
        files.insert(
            "package.json".to_string(),
            r#"{"name":"app","version":"1.0.0"}"#.to_string(),
        );
        files.insert(
            "Cargo.toml".to_string(),
            "[package]\nname = \"app\"\nversion = \"1.1.0\"\n".to_string(),
        );
        let req = ValidateRequest {
            config: r#"{"package":[{"name":"app","path":".","versionedFiles":[{"path":"package.json","format":"json"},{"path":"Cargo.toml","format":"toml"}]}]}"#.to_string(),
            format: Some("json".to_string()),
            files,
        };
        let Json(result) = validate(Json(req)).await;
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("1.0.0") && e.message.contains("1.1.0"))
        );
    }
}
