//! `POST /v1/ferrflow/token` — hosted FerrFlow bot token service.
//!
//! 1. The runner requests a GitHub OIDC token with audience
//!    `ferrflow.ferrlabs.com` and POSTs it here.
//! 2. We verify it against GitHub's JWKS (signature, issuer, audience, exp).
//! 3. We sign a 9-minute GitHub App JWT locally.
//! 4. We look up the FerrFlow App installation on the OIDC token's
//!    `repository_owner` (org first, user fallback).
//! 5. We call `POST /app/installations/{id}/access_tokens` with
//!    `repositories: [<repo>]` so the minted token reaches exactly the one
//!    repository that asked.
//!
//! There is no auth middleware on this route — the OIDC token *is* the auth.

use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::bot::{
    app_jwt,
    github_client::{GhError, Permissions},
    oidc_verify,
};
use crate::error::{ApiError, ApiResult, code};
use crate::state::AppState;

/// GitHub's OIDC tokens sit around 1–2 KB. The bounds exist to reject junk
/// before it reaches the JWT parser, not to be a precise spec limit.
const MIN_TOKEN_LEN: usize = 20;
const MAX_TOKEN_LEN: usize = 8192;

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Installation access token (`ghs_...`), usable as
    /// `Authorization: token <token>` or as the git HTTP basic-auth password
    /// for `x-access-token` pushes.
    pub token: String,
    /// When the token stops working. GitHub issues them for about an hour.
    pub expires_at: DateTime<Utc>,
    /// Echo of the OIDC `repository` claim, so the runner can check it got
    /// what it asked for.
    pub repository: String,
}

impl TokenRequest {
    fn validate(&self) -> Result<(), ApiError> {
        let len = self.token.len();
        if (MIN_TOKEN_LEN..=MAX_TOKEN_LEN).contains(&len) {
            return Ok(());
        }
        Err(ApiError::bad_request(
            code::VALIDATION_FAILED,
            format!(
                "token: expected between {MIN_TOKEN_LEN} and {MAX_TOKEN_LEN} characters, got {len}"
            ),
        ))
    }
}

#[tracing::instrument(
    name = "ferrflow_token_issue",
    skip(state, body),
    fields(
        repository = tracing::field::Empty,
        workflow = tracing::field::Empty,
        run_id = tracing::field::Empty,
    ),
)]
pub async fn issue_token(
    State(state): State<AppState>,
    Json(body): Json<TokenRequest>,
) -> ApiResult<(StatusCode, Json<TokenResponse>)> {
    body.validate()?;

    let (cfg, github) = state.github_app().ok_or_else(|| {
        ApiError::new(
            code::NOT_CONFIGURED,
            StatusCode::SERVICE_UNAVAILABLE,
            "FerrFlow bot token service is not configured on this deploy",
        )
    })?;

    let claims = oidc_verify::verify(&body.token, &cfg.expected_audience)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "FerrFlow OIDC verification failed");
            ApiError::new(
                code::OIDC_INVALID,
                StatusCode::UNAUTHORIZED,
                "OIDC token rejected",
            )
        })?;

    let span = tracing::Span::current();
    span.record("repository", tracing::field::display(&claims.repository));
    span.record("workflow", tracing::field::display(&claims.workflow));
    span.record("run_id", tracing::field::display(&claims.run_id));

    // `repository` is always `<owner>/<name>`. Splitting rather than trusting
    // is defence in depth: we already rejected on signature, but a malformed
    // value would otherwise decide which repo a token is scoped to.
    let (owner, name) = claims.repository.split_once('/').ok_or_else(|| {
        tracing::warn!(
            repository = %claims.repository,
            "FerrFlow OIDC repository claim not in owner/name shape"
        );
        ApiError::new(
            code::OIDC_INVALID,
            StatusCode::UNAUTHORIZED,
            "OIDC token repository claim malformed",
        )
    })?;

    if owner != claims.repository_owner {
        tracing::warn!(
            repository = %claims.repository,
            repository_owner = %claims.repository_owner,
            "FerrFlow OIDC owner / repository mismatch",
        );
        return Err(ApiError::new(
            code::REPO_MISMATCH,
            StatusCode::UNAUTHORIZED,
            "OIDC repository owner does not match repository",
        ));
    }

    let app_jwt =
        app_jwt::sign_app_jwt(&cfg.client_id, &cfg.private_key_pem, Utc::now()).map_err(|e| {
            tracing::error!(error = %e, "failed to sign FerrFlow App JWT");
            ApiError::new(
                code::GITHUB_UPSTREAM,
                StatusCode::BAD_GATEWAY,
                "failed to build App JWT",
            )
        })?;

    let install_id = github
        .find_installation(&app_jwt, owner)
        .await
        .map_err(map_gh_error)?;

    let token = github
        .create_installation_token(&app_jwt, install_id, name, &Permissions::default())
        .await
        .map_err(map_gh_error)?;

    Ok((
        StatusCode::OK,
        Json(TokenResponse {
            token: token.token,
            expires_at: token.expires_at,
            repository: claims.repository,
        }),
    ))
}

fn map_gh_error(err: GhError) -> ApiError {
    match err {
        GhError::InstallNotFound => ApiError::not_found(
            code::INSTALL_NOT_FOUND,
            "FerrFlow GitHub App is not installed on this owner",
        ),
        GhError::RateLimited => {
            tracing::warn!("FerrFlow hit GitHub App rate limit");
            ApiError::new(
                code::RATE_LIMITED,
                StatusCode::TOO_MANY_REQUESTS,
                "upstream rate limit reached, try again shortly",
            )
        }
        GhError::Upstream(e) => {
            tracing::error!(error = %e, "FerrFlow upstream GitHub error");
            ApiError::new(
                code::GITHUB_UPSTREAM,
                StatusCode::BAD_GATEWAY,
                "upstream GitHub error",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(token: &str) -> TokenRequest {
        TokenRequest {
            token: token.to_string(),
        }
    }

    #[test]
    fn validation_rejects_absurdly_short_tokens() {
        let err = request("too-short").validate().unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, code::VALIDATION_FAILED);
    }

    #[test]
    fn validation_rejects_oversized_tokens() {
        assert!(request(&"a".repeat(MAX_TOKEN_LEN + 1)).validate().is_err());
    }

    #[test]
    fn validation_accepts_the_boundaries() {
        assert!(request(&"a".repeat(MIN_TOKEN_LEN)).validate().is_ok());
        assert!(request(&"a".repeat(MAX_TOKEN_LEN)).validate().is_ok());
    }

    // The runner deserialises this response; renaming a field breaks every
    // released binary at once.
    #[test]
    fn response_shape_roundtrip() {
        let expires_at = "2026-04-21T12:34:56Z".parse::<DateTime<Utc>>().unwrap();
        let resp = TokenResponse {
            token: "ghs_exampleTokenValue".to_string(),
            expires_at,
            repository: "FerrLabs/FerrFlow".to_string(),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["token"], "ghs_exampleTokenValue");
        assert_eq!(v["repository"], "FerrLabs/FerrFlow");
        assert_eq!(v["expires_at"], "2026-04-21T12:34:56Z");
    }

    #[tokio::test]
    async fn an_unconfigured_deploy_answers_503_not_500() {
        let state = AppState::new(None);
        let err = issue_token(State(state), Json(request(&"a".repeat(64))))
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.code, code::NOT_CONFIGURED);
    }
}
