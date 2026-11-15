//! Sign a short-lived GitHub App JWT.
//!
//! GitHub Apps authenticate by presenting a JWT signed with the App's RSA
//! private key (`iss` = client id, `iat` / `exp` tight, RS256). The JWT is
//! traded for an installation access token by the `github_client`
//! module — App JWTs themselves can't call data plane endpoints.
//!
//! Phase 3 moves the signing call to Cloud KMS. The function signature
//! here stays stable so that swap is a one-file change.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct AppClaims {
    iat: i64,
    exp: i64,
    iss: String,
}

/// Sign a GitHub App JWT valid for ~9 minutes. GitHub caps App JWT lifetime
/// at 10 min; we leave a minute of headroom for clock skew on their side.
/// `iat` is pre-dated by 60s for the same reason.
pub fn sign_app_jwt(client_id: &str, private_key_pem: &str, now: DateTime<Utc>) -> Result<String> {
    let iat = now.timestamp() - 60;
    let exp = now.timestamp() + 9 * 60;
    let claims = AppClaims {
        iat,
        exp,
        iss: client_id.to_string(),
    };
    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .context("GitHub App private key is not a valid RSA PEM")?;
    let header = Header::new(Algorithm::RS256);
    encode(&header, &claims, &key).context("failed to sign GitHub App JWT")
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{DecodingKey, Validation, decode};

    // Test-only RSA keypair. NOT a production key — generated at fixture
    // time so tests can round-trip sign/verify without pulling a runtime
    // dep to build keys on the fly.
    const TEST_PRIVATE_KEY: &str = include_str!("../../tests/fixtures/ferrflow_test_rsa.pem");
    const TEST_PUBLIC_KEY: &str = include_str!("../../tests/fixtures/ferrflow_test_rsa.pub.pem");

    #[derive(Debug, serde::Deserialize)]
    struct RoundTrip {
        iss: String,
        iat: i64,
        exp: i64,
    }

    #[test]
    fn sign_then_verify_roundtrip() {
        let now = Utc::now();
        let jwt = sign_app_jwt("Iv23liTEST", TEST_PRIVATE_KEY, now).expect("sign");
        let key = DecodingKey::from_rsa_pem(TEST_PUBLIC_KEY.as_bytes()).expect("pub key");
        let mut v = Validation::new(Algorithm::RS256);
        // No audience/issuer set by default validators we care about in the
        // roundtrip — just verify signature + exp.
        v.validate_aud = false;
        let data = decode::<RoundTrip>(&jwt, &key, &v).expect("decode");
        assert_eq!(data.claims.iss, "Iv23liTEST");
        assert!(data.claims.exp > data.claims.iat);
        // Window check: iat pre-dated by ~60s, exp ~9min out.
        let lifetime = data.claims.exp - data.claims.iat;
        assert!(
            (9 * 60..=10 * 60 + 5).contains(&lifetime),
            "unexpected lifetime {lifetime}s"
        );
    }

    #[test]
    fn rejects_malformed_pem() {
        let err = sign_app_jwt("Iv23liTEST", "not a pem", Utc::now()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("RSA PEM"), "unexpected error: {msg}");
    }
}
