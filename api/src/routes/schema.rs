use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::error::{ApiError, code};

const SCHEMA: &str = ferrflow::schema::BUNDLED_SCHEMA;

fn current_major() -> u64 {
    ferrflow::VERSION
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn schema_response() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/schema+json")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .header(
            header::ETAG,
            format!("\"ferrflow-schema-{}\"", current_major()),
        )
        .body(Body::from(SCHEMA))
        .expect("static schema headers are always valid")
}

pub async fn schema() -> Response {
    schema_response()
}

pub async fn schema_versioned(Path(version): Path<String>) -> Response {
    let requested = version.trim_start_matches('v');
    let major: Option<u64> = requested.split('.').next().and_then(|s| s.parse().ok());

    match major {
        Some(m) if m == current_major() => schema_response(),
        Some(m) => ApiError::not_found(
            code::SCHEMA_NOT_FOUND,
            format!(
                "no bundled schema for major v{m}; this API serves the current major (v{}). \
                 Historical per-major schemas are served from tagged sources — follow-up.",
                current_major()
            ),
        )
        .into_response(),
        None => ApiError::new(
            code::SCHEMA_NOT_FOUND,
            StatusCode::BAD_REQUEST,
            format!("invalid schema version '{version}'"),
        )
        .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_major_matches_the_linked_library() {
        let expected: u64 = ferrflow::VERSION
            .split('.')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(current_major(), expected);
    }

    #[tokio::test]
    async fn schema_is_served_with_schema_content_type() {
        let resp = schema().await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/schema+json"
        );
        assert!(resp.headers().get(header::ETAG).is_some());
    }

    #[tokio::test]
    async fn unknown_major_is_404() {
        let major = current_major() + 1;
        let resp = schema_versioned(Path(format!("v{major}"))).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn current_major_path_serves_the_schema() {
        let resp = schema_versioned(Path(format!("v{}", current_major()))).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
