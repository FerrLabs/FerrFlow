pub mod bot;
pub mod config;
pub mod engine;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn build_router(state: AppState, allowed_origins: &[String]) -> Router {
    let api = routes::router();

    Router::new()
        // `/api/v1` is not a legacy wart we can drop: every released FerrFlow
        // binary before the api.ferrflow.com move POSTs its OIDC token to
        // `/api/v1/ferrflow/token`, and binaries cannot be updated in place.
        .nest("/v1", api.clone())
        .nest("/api/v1", api)
        .layer(cors(allowed_origins))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Every endpoint here is public and unauthenticated, and none of them read a
/// cookie, so there is no credential to leak to a hostile origin. An empty
/// allowlist therefore means `*` rather than "deny everything" — that is what
/// makes the schema and validate endpoints usable from a browser at all.
fn cors(allowed_origins: &[String]) -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
        .max_age(std::time::Duration::from_secs(3600));

    let parsed: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    if parsed.is_empty() {
        layer.allow_origin(Any)
    } else {
        layer.allow_origin(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn app() -> Router {
        build_router(AppState::new(None), &[])
    }

    async fn get(path: &str) -> (StatusCode, serde_json::Value) {
        let resp = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn health_answers_on_the_canonical_path() {
        let (status, body) = get("/v1/ferrflow/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "ferrflow-api");
    }

    // Released binaries POST to `/api/v1/ferrflow/token`. If this mount ever
    // disappears, every pinned FerrFlow version breaks at once.
    #[tokio::test]
    async fn the_legacy_api_v1_mount_serves_the_same_routes() {
        let (status, body) = get("/api/v1/ferrflow/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["service"], "ferrflow-api");
    }

    #[tokio::test]
    async fn an_unknown_path_is_404() {
        let (status, _) = get("/v1/ferrflow/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn validate_reports_a_bad_config_without_a_5xx() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/ferrflow/validate")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"config":"}{","format":"json"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["valid"], false);
    }

    #[tokio::test]
    async fn preview_computes_a_bump_end_to_end() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/ferrflow/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"config":"{\"package\":[{\"name\":\"app\",\"path\":\".\"}]}","format":"json","commits":[{"message":"feat!: rewrite"}],"current_versions":{"app":"1.2.3"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["packages"][0]["next"], "2.0.0");
        assert_eq!(body["packages"][0]["bump"], "major");
    }

    // Without a GitHub App key the token route must degrade to 503, not panic
    // or 500 — a deploy missing its Vault mount should stay diagnosable.
    #[tokio::test]
    async fn token_without_a_configured_app_is_503() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/ferrflow/token")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"token":"{}"}}"#, "a".repeat(64))))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
