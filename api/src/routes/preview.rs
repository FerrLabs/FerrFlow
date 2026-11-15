use std::collections::BTreeMap;

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::engine::{self, CommitInput, PackagePreview};
use crate::error::{ApiError, ApiResult, code};

#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub config: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub commits: Vec<CommitInput>,
    #[serde(default)]
    pub current_versions: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub packages: Vec<PackagePreview>,
}

pub async fn preview(Json(req): Json<PreviewRequest>) -> ApiResult<Json<PreviewResponse>> {
    let config = engine::parse_config(&req.config, req.format.as_deref())
        .map_err(|message| ApiError::new(code::CONFIG_INVALID, StatusCode::BAD_REQUEST, message))?;

    let packages = engine::compute_preview(&config, &req.commits, &req.current_versions);
    Ok(Json(PreviewResponse { packages }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn preview_returns_bumped_package() {
        let req = PreviewRequest {
            config: r#"{"package":[{"name":"app","path":"."}]}"#.to_string(),
            format: Some("json".to_string()),
            commits: vec![CommitInput {
                message: "feat: add endpoint".to_string(),
                hash: Some("a1b2c3d".to_string()),
                files: vec![],
            }],
            current_versions: BTreeMap::from([("app".to_string(), "1.2.3".to_string())]),
        };
        let Json(body) = preview(Json(req)).await.unwrap();
        assert_eq!(body.packages.len(), 1);
        assert_eq!(body.packages[0].next, "1.3.0");
        assert_eq!(body.packages[0].bump, "minor");
    }

    #[tokio::test]
    async fn invalid_config_is_a_400() {
        let req = PreviewRequest {
            config: "}{".to_string(),
            format: Some("json".to_string()),
            commits: vec![],
            current_versions: BTreeMap::new(),
        };
        let err = preview(Json(req)).await.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, code::CONFIG_INVALID);
    }
}
