pub mod bot;
pub mod config;
pub mod engine;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use axum::middleware;
use ferrlabs_api_version::{ApiVersion, ApiVersionContext, negotiate};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::state::AppState;

/// The contract version this build serves.
///
/// Distinct from the package version on purpose: the API ships many times
/// without the contract moving. Bump this only when a request or response
/// shape changes in a way an existing caller would notice, and land a
/// `Transform` in the same commit.
const CURRENT_API_VERSION: ApiVersion = match ApiVersion::from_ymd(2026, 8, 8) {
    Some(v) => v,
    None => panic!("CURRENT_API_VERSION is not a calendar date"),
};

/// The oldest contract still served. Anything older is `410 Gone`.
///
/// Equal to the current one while the contract has never moved: no earlier
/// date was ever promised. It stays put when `CURRENT_API_VERSION` advances —
/// that gap is the compatibility window.
const OLDEST_SUPPORTED_API_VERSION: ApiVersion = CURRENT_API_VERSION;

/// Product-scoped, like the sibling APIs: `FerrLabs` is both the org and a
/// product, so an org-wide header name would be ambiguous exactly here.
const API_VERSION_HEADER: &str = "x-ferrflow-api-version";

/// The negotiation context. Registry left empty: the contract has not moved
/// since it shipped, so there is nothing to rewrite yet.
fn api_version_context() -> ApiVersionContext {
    ApiVersionContext::new(
        API_VERSION_HEADER,
        CURRENT_API_VERSION,
        OLDEST_SUPPORTED_API_VERSION,
    )
    .expect("API version header and bounds are compile-time constants")
}

pub fn build_router(state: AppState, allowed_origins: &[String]) -> Router {
    let api = routes::router();

    Router::new()
        // Le contrat se négocie par l'en-tête `x-ferrflow-api-version` : les
        // routes vivent donc à la racine, sans version dans le chemin.
        .merge(api.clone())
        // ⚠️ DETTE À ÉCHÉANCE — RETIRER APRÈS LE 2027-02-08.
        //
        // Ces deux montages ne versionnent plus rien ; ils n'existent que
        // pour les binaires FerrFlow déjà distribués, qui ont
        // `/v1/ferrflow/token` (et, avant la bascule vers api.ferrflow.com,
        // `/api/v1/ferrflow/token`) compilé en dur et ne peuvent pas être
        // mis à jour à distance. Six mois suffisent largement à ce que les
        // pipelines repassent sur une version récente.
        //
        // Pour retirer : supprimer les deux `.nest` ci-dessous et les tests
        // `legacy_v1_*` plus bas. Voir la section « Dette à échéance » du
        // CLAUDE.md à la racine.
        .nest("/v1", api.clone())
        .nest("/api/v1", api)
        // Après le routage, pour lire la route résolue et non le chemin brut.
        .layer(middleware::from_fn_with_state(
            api_version_context(),
            negotiate,
        ))
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
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::HeaderName::from_static(API_VERSION_HEADER),
        ])
        // Sans l'expose, le navigateur masque la version servie et un client
        // ne peut pas savoir quel contrat il a réellement obtenu.
        .expose_headers([header::HeaderName::from_static(
            ferrlabs_api_version::SERVED_HEADER,
        )])
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
        let (status, body) = get("/ferrflow/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "ferrflow-api");
    }

    /// La version servie est annoncée sur chaque réponse : c'est ce qui
    /// remplace le préfixe de chemin, donc ce qui doit être vérifié.
    #[tokio::test]
    async fn the_served_contract_version_is_announced() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/ferrflow/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(
            resp.headers()
                .contains_key(ferrlabs_api_version::SERVED_HEADER),
            "la réponse doit porter {}",
            ferrlabs_api_version::SERVED_HEADER
        );
    }

    /// ⚠️ À SUPPRIMER AVEC LES MONTAGES `/v1` ET `/api/v1` — APRÈS LE
    /// 2027-02-08. Les binaires FerrFlow déjà distribués ont ces chemins
    /// compilés en dur ; tant qu'ils circulent, ce test protège leur CI.
    #[tokio::test]
    async fn legacy_v1_mounts_serve_the_same_routes() {
        for path in ["/v1/ferrflow/health", "/api/v1/ferrflow/health"] {
            let (status, body) = get(path).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert_eq!(body["service"], "ferrflow-api", "{path}");
        }
    }

    #[tokio::test]
    async fn an_unknown_path_is_404() {
        let (status, _) = get("/ferrflow/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn validate_reports_a_bad_config_without_a_5xx() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ferrflow/validate")
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
                    .uri("/ferrflow/preview")
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
                    .uri("/ferrflow/token")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"token":"{}"}}"#, "a".repeat(64))))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
