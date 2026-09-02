use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::error::{ApiError, ApiResult, code};
use crate::state::AppState;

const SIGNATURE_HEADER: &str = "x-hub-signature-256";
const SIGNATURE_PREFIX: &str = "sha256=";
const EVENT_HEADER: &str = "x-github-event";
const PURCHASE_EVENT: &str = "marketplace_purchase";

#[derive(Debug, Deserialize)]
struct MarketplaceEvent {
    action: String,
    marketplace_purchase: MarketplacePurchase,
}

#[derive(Debug, Deserialize)]
struct MarketplacePurchase {
    account: Account,
    plan: Plan,
}

#[derive(Debug, Deserialize)]
struct Account {
    login: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct Plan {
    name: String,
}

fn signature_matches(secret: &str, body: &[u8], header: &str) -> bool {
    let Some(hex_digest) = header.strip_prefix(SIGNATURE_PREFIX) else {
        return false;
    };
    let Ok(expected) = hex::decode(hex_digest) else {
        return false;
    };
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts a key of any length");
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}

fn header<'h>(headers: &'h HeaderMap, name: &str) -> Option<&'h str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn unauthorized(message: &'static str) -> ApiError {
    ApiError::new(code::SIGNATURE_INVALID, StatusCode::UNAUTHORIZED, message)
}

#[tracing::instrument(name = "ferrflow_marketplace_event", skip(state, headers, body))]
pub async fn marketplace(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<StatusCode> {
    let secret = state.marketplace_webhook_secret().ok_or_else(|| {
        ApiError::new(
            code::NOT_CONFIGURED,
            StatusCode::SERVICE_UNAVAILABLE,
            "marketplace webhooks are not configured on this deploy",
        )
    })?;

    let signature =
        header(&headers, SIGNATURE_HEADER).ok_or_else(|| unauthorized("missing signature"))?;

    if !signature_matches(secret, &body, signature) {
        return Err(unauthorized("signature does not match the request body"));
    }

    if header(&headers, EVENT_HEADER) != Some(PURCHASE_EVENT) {
        return Ok(StatusCode::NO_CONTENT);
    }

    let event: MarketplaceEvent = serde_json::from_slice(&body).map_err(|e| {
        ApiError::bad_request(
            code::VALIDATION_FAILED,
            format!("marketplace_purchase payload: {e}"),
        )
    })?;

    tracing::info!(
        action = %event.action,
        plan = %event.marketplace_purchase.plan.name,
        account = %event.marketplace_purchase.account.login,
        account_type = %event.marketplace_purchase.account.kind,
        "marketplace listing event",
    );

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "a-marketplace-secret";

    fn sign(secret: &str, body: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        format!(
            "{SIGNATURE_PREFIX}{}",
            hex::encode(mac.finalize().into_bytes())
        )
    }

    #[test]
    fn a_signature_over_the_exact_body_matches() {
        let body = br#"{"action":"purchased"}"#;
        assert!(signature_matches(SECRET, body, &sign(SECRET, body)));
    }

    #[test]
    fn a_signature_over_a_different_body_is_rejected() {
        let signature = sign(SECRET, br#"{"action":"purchased"}"#);
        assert!(!signature_matches(
            SECRET,
            br#"{"action":"cancelled"}"#,
            &signature
        ));
    }

    #[test]
    fn a_signature_from_another_secret_is_rejected() {
        let body = br#"{"action":"purchased"}"#;
        assert!(!signature_matches(SECRET, body, &sign("another", body)));
    }

    #[test]
    fn a_signature_without_the_sha256_prefix_is_rejected() {
        let body = br#"{"action":"purchased"}"#;
        let bare = sign(SECRET, body)
            .strip_prefix(SIGNATURE_PREFIX)
            .unwrap()
            .to_string();
        assert!(!signature_matches(SECRET, body, &bare));
    }

    #[test]
    fn a_signature_that_is_not_hex_is_rejected() {
        let body = br#"{"action":"purchased"}"#;
        assert!(!signature_matches(SECRET, body, "sha256=not-a-hex-digest"));
    }

    #[test]
    fn a_truncated_digest_is_rejected() {
        let body = br#"{"action":"purchased"}"#;
        let full = sign(SECRET, body);
        assert!(!signature_matches(SECRET, body, &full[..full.len() - 2]));
    }

    #[test]
    fn a_purchase_payload_deserialises_the_fields_we_log() {
        let event: MarketplaceEvent = serde_json::from_slice(
            br#"{
                "action": "purchased",
                "marketplace_purchase": {
                    "account": { "login": "acme", "type": "Organization" },
                    "plan": { "name": "Free" }
                }
            }"#,
        )
        .unwrap();
        assert_eq!(event.action, "purchased");
        assert_eq!(event.marketplace_purchase.account.login, "acme");
        assert_eq!(event.marketplace_purchase.account.kind, "Organization");
        assert_eq!(event.marketplace_purchase.plan.name, "Free");
    }
}
