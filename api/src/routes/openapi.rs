use axum::{
    body::Body,
    http::header,
    response::{IntoResponse, Response},
};

const SPEC: &str = include_str!("../../openapi/ferrflow.json");

pub async fn openapi() -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        Body::from(SPEC),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_is_valid_openapi_31_covering_the_endpoints() {
        let doc: serde_json::Value = serde_json::from_str(SPEC).expect("openapi spec must parse");
        assert!(
            doc["openapi"].as_str().unwrap().starts_with("3.1"),
            "expected OpenAPI 3.1"
        );
        let paths = &doc["paths"];
        for path in [
            "/ferrflow/health",
            "/ferrflow/schema",
            "/ferrflow/latest",
            "/ferrflow/validate",
            "/ferrflow/preview",
            "/ferrflow/token",
        ] {
            assert!(paths.get(path).is_some(), "missing path {path}");
        }
    }
}
