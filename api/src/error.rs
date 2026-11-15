use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub mod code {
    pub const CONFIG_INVALID: &str = "FERRFLOW_CONFIG_INVALID";
    pub const SCHEMA_NOT_FOUND: &str = "FERRFLOW_SCHEMA_NOT_FOUND";
    pub const PLATFORM_INVALID: &str = "FERRFLOW_PLATFORM_INVALID";
    pub const RELEASE_UPSTREAM: &str = "FERRFLOW_RELEASE_UPSTREAM";
    pub const NOT_CONFIGURED: &str = "FERRFLOW_NOT_CONFIGURED";
    pub const OIDC_INVALID: &str = "FERRFLOW_OIDC_INVALID";
    pub const REPO_MISMATCH: &str = "FERRFLOW_REPO_MISMATCH";
    pub const REF_NOT_ALLOWED: &str = "FERRFLOW_REF_NOT_ALLOWED";
    pub const INSTALL_NOT_FOUND: &str = "FERRFLOW_INSTALL_NOT_FOUND";
    pub const RATE_LIMITED: &str = "FERRFLOW_RATE_LIMITED";
    pub const GITHUB_UPSTREAM: &str = "FERRFLOW_GITHUB_UPSTREAM";
    pub const SIGNATURE_INVALID: &str = "FERRFLOW_SIGNATURE_INVALID";
    pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
}

pub type ApiResult<T> = Result<T, ApiError>;

/// Every failure this API can return. The wire shape is `{ code, error }`,
/// byte-compatible with what `ferrlabs-api` served on these routes — the CLI
/// and the docs both match on `code`, so it is part of the contract.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    error: &'a str,
}

impl ApiError {
    pub fn new(code: &'static str, status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, StatusCode::BAD_REQUEST, message)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                code: self.code,
                error: &self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    // The `{code, error}` envelope is what callers match on. A field rename
    // here is a silent breaking change for every client.
    #[tokio::test]
    async fn error_body_keeps_the_code_and_error_fields() {
        let resp = ApiError::bad_request(code::CONFIG_INVALID, "bad config").into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let bytes = to_bytes(resp.into_body(), 4096).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], "FERRFLOW_CONFIG_INVALID");
        assert_eq!(body["error"], "bad config");
    }

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let resp = ApiError::not_found(code::SCHEMA_NOT_FOUND, "nope").into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
