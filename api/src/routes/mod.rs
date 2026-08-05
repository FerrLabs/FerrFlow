use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

mod health;
mod latest;
mod openapi;
mod preview;
mod schema;
mod token;
mod validate;

/// The `/v1/ferrflow/*` surface. Mounted twice by [`crate::build_router`] —
/// once at `/v1` and once at `/api/v1` — because released FerrFlow binaries
/// call the `/api/v1` form and cannot be updated retroactively.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ferrflow/health", get(health::health))
        .route("/ferrflow/schema", get(schema::schema))
        .route("/ferrflow/schema/{version}", get(schema::schema_versioned))
        .route("/ferrflow/latest", get(latest::latest))
        .route("/ferrflow/validate", post(validate::validate))
        .route("/ferrflow/preview", post(preview::preview))
        .route("/ferrflow/openapi.json", get(openapi::openapi))
        .route("/ferrflow/token", post(token::issue_token))
}
