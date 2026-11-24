//! Verify a GitHub Actions OIDC ID token.
//!
//! GitHub issues these to any workflow that requests the `id-token: write`
//! permission. Each token is scoped to a single workflow run on a single
//! repository and carries the repo, ref, run id and actor as claims we act
//! on. Verification is standard OIDC: fetch JWKS, match `kid`, check `iss`,
//! `aud`, `exp`, signature.

use anyhow::{Context, Result, anyhow};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;

use super::jwks;

/// Issuer on every GitHub Actions OIDC token. Stable — changing it would
/// break every Renovate/Chainguard-style integration in existence.
pub const GHA_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// JWKS endpoint. Hard-coded rather than discovered through
/// `.well-known/openid-configuration`: the issuer points at the same origin,
/// so discovery would only add a round trip per cold cache.
pub const GHA_JWKS_URL: &str = "https://token.actions.githubusercontent.com/.well-known/jwks";

/// `aud` is a string in GitHub's tokens but the JWT spec allows an array, and
/// a verifier that only handles one shape is a verifier that can be bypassed
/// by sending the other.
#[derive(Debug, Default, Deserialize)]
#[serde(untagged)]
pub enum Audience {
    #[default]
    None,
    Single(String),
    Many(Vec<String>),
}

impl Audience {
    pub fn contains(&self, needle: &str) -> bool {
        match self {
            Audience::None => false,
            Audience::Single(s) => s == needle,
            Audience::Many(xs) => xs.iter().any(|s| s == needle),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GhaClaims {
    /// `<owner>/<name>`, e.g. `FerrLabs/FerrFlow`. Drives which repository
    /// the minted installation token is scoped to.
    pub repository: String,
    /// `<owner>`. Redundant with the first half of `repository`, but GitHub
    /// emits it and the handler cross-checks the two.
    pub repository_owner: String,
    #[serde(rename = "ref")]
    pub ref_: String,
    pub event_name: String,
    pub workflow: String,
    pub run_id: String,
    pub actor: String,
    pub sub: String,
    pub iss: String,
    pub aud: Audience,
    pub exp: u64,
}

/// Verify a GitHub Actions OIDC JWT and return its claims.
///
/// The failure surface is deliberately collapsed to `anyhow::Error`: the
/// caller turns any error into a 401. The real reason is logged on our side
/// so ops can tell clock skew from a spoofed audience.
pub async fn verify(jwt: &str, expected_audience: &str) -> Result<GhaClaims> {
    let header = decode_header(jwt).context("JWT header malformed")?;
    let kid = header
        .kid
        .clone()
        .ok_or_else(|| anyhow!("JWT header missing kid"))?;

    let set = jwks::load(GHA_JWKS_URL).await?;
    verify_with_jwks(jwt, &kid, expected_audience, &set)
}

fn verify_with_jwks(
    jwt: &str,
    kid: &str,
    expected_audience: &str,
    set: &JwkSet,
) -> Result<GhaClaims> {
    let jwk = set
        .find(kid)
        .ok_or_else(|| anyhow!("no JWKS entry matches kid {kid}"))?;
    let key = DecodingKey::from_jwk(jwk).context("unsupported JWK")?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[GHA_ISSUER]);
    validation.set_audience(&[expected_audience]);
    // Runners can sit on the token for a few seconds before POSTing it.
    validation.leeway = 30;

    let data = decode::<GhaClaims>(jwt, &key, &validation)
        .context("JWT failed signature or standard claim validation")?;

    // `jsonwebtoken` already checked `aud`, but it does so before our untagged
    // enum has collapsed the two possible shapes. Re-checking against the
    // parsed value costs nothing and closes the gap.
    if !data.claims.aud.contains(expected_audience) {
        return Err(anyhow!(
            "JWT aud does not include expected audience {expected_audience}"
        ));
    }

    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::errors::ErrorKind;
    use jsonwebtoken::{EncodingKey, Header, encode};

    use super::*;

    const TEST_PRIVATE_KEY: &str = include_str!("../../tests/fixtures/ferrflow_test_rsa.pem");
    const TEST_PUBLIC_KEY: &str = include_str!("../../tests/fixtures/ferrflow_test_rsa.pub.pem");
    const TEST_MODULUS: &str = "4r7LVJmQyk0QQHW4_qdVvpQJTomYDVL-Mc9nHvfw0OxYUHfFllb5uYXzGmqRvL_4gAW_4k9U29xI5xJMIqOT1V9yV7n0HFnMfZfNI-okH3m6vZiXwrfCEhzmxd1mXo2SqsaZLOxKiWplDdD8SZQIU7tno3D7l50hWd2wNyctG3a-nam7k-VcoZeC21JJ5k-fgHmaqq9rDT4EMnkWYOPgbrT8mwb4nWkU_es-s3tZLqg1USc7TUXMekkrNgzWtskiyJJxaregRuLWU2_WglusWNPQfuFwKemrFuvlze4tX0at5IDb_8WLw30BXnAHAxMYMkx6OQ53RRzWB0F5Wkngew";
    const KID: &str = "test-kid";
    const AUDIENCE: &str = "ferrflow.ferrlabs.com";

    fn test_jwks() -> JwkSet {
        serde_json::from_value(serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "kid": KID,
                "alg": "RS256",
                "use": "sig",
                "n": TEST_MODULUS,
                "e": "AQAB",
            }]
        }))
        .unwrap()
    }

    fn sign(alg: Algorithm, key: &EncodingKey) -> String {
        let mut header = Header::new(alg);
        header.kid = Some(KID.to_string());
        let claims = serde_json::json!({
            "repository": "acme/widget",
            "repository_owner": "acme",
            "ref": "refs/heads/main",
            "event_name": "push",
            "workflow": "Release",
            "run_id": "123456",
            "actor": "someone",
            "sub": "repo:acme/widget:ref:refs/heads/main",
            "iss": GHA_ISSUER,
            "aud": AUDIENCE,
            "exp": 4_102_444_800_u64,
        });
        encode(&header, &claims, key).unwrap()
    }

    fn rsa_key() -> EncodingKey {
        EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY.as_bytes()).unwrap()
    }

    fn assert_invalid_algorithm(jwt: &str) {
        let err = verify_with_jwks(jwt, KID, AUDIENCE, &test_jwks()).unwrap_err();
        let kind = err
            .root_cause()
            .downcast_ref::<jsonwebtoken::errors::Error>()
            .map(jsonwebtoken::errors::Error::kind);
        assert!(matches!(kind, Some(ErrorKind::InvalidAlgorithm)), "{err:#}");
    }

    #[test]
    fn an_rs256_token_verifies() {
        let jwt = sign(Algorithm::RS256, &rsa_key());
        let claims = verify_with_jwks(&jwt, KID, AUDIENCE, &test_jwks()).unwrap();
        assert_eq!(claims.repository, "acme/widget");
    }

    #[test]
    fn other_rsa_algorithms_are_rejected_even_with_the_right_key() {
        for alg in [
            Algorithm::RS384,
            Algorithm::RS512,
            Algorithm::PS256,
            Algorithm::PS384,
            Algorithm::PS512,
        ] {
            assert_invalid_algorithm(&sign(alg, &rsa_key()));
        }
    }

    #[test]
    fn an_hmac_token_keyed_with_the_public_key_is_rejected() {
        let key = EncodingKey::from_secret(TEST_PUBLIC_KEY.as_bytes()).unwrap();
        for alg in [Algorithm::HS256, Algorithm::HS384, Algorithm::HS512] {
            assert_invalid_algorithm(&sign(alg, &key));
        }
    }

    #[test]
    fn audience_matches_a_bare_string() {
        let aud: Audience = serde_json::from_str(r#""ferrflow.ferrlabs.com""#).unwrap();
        assert!(aud.contains("ferrflow.ferrlabs.com"));
        assert!(!aud.contains("evil.example"));
    }

    // A token carrying `aud: ["evil", "ferrflow.ferrlabs.com"]` is legitimate
    // per the JWT spec; one carrying only "evil" must not pass.
    #[test]
    fn audience_matches_inside_an_array() {
        let aud: Audience =
            serde_json::from_str(r#"["evil.example","ferrflow.ferrlabs.com"]"#).unwrap();
        assert!(aud.contains("ferrflow.ferrlabs.com"));

        let aud: Audience = serde_json::from_str(r#"["evil.example"]"#).unwrap();
        assert!(!aud.contains("ferrflow.ferrlabs.com"));
    }

    #[test]
    fn a_missing_audience_matches_nothing() {
        assert!(!Audience::None.contains("ferrflow.ferrlabs.com"));
    }

    #[test]
    fn claims_parse_from_a_realistic_token_body() {
        let body = r#"{
            "repository": "FerrLabs/FerrFlow",
            "repository_owner": "FerrLabs",
            "ref": "refs/heads/main",
            "event_name": "push",
            "workflow": "Release",
            "run_id": "123456",
            "actor": "BryanFRD",
            "sub": "repo:FerrLabs/FerrFlow:ref:refs/heads/main",
            "iss": "https://token.actions.githubusercontent.com",
            "aud": "ferrflow.ferrlabs.com",
            "exp": 1900000000,
            "some_future_claim": "ignored"
        }"#;
        let claims: GhaClaims = serde_json::from_str(body).unwrap();
        assert_eq!(claims.repository, "FerrLabs/FerrFlow");
        assert_eq!(claims.repository_owner, "FerrLabs");
        assert!(claims.aud.contains("ferrflow.ferrlabs.com"));
    }

    #[tokio::test]
    async fn a_garbage_token_never_reaches_the_network() {
        let err = verify("not-a-jwt", "ferrflow.ferrlabs.com")
            .await
            .unwrap_err();
        assert!(format!("{err:#}").contains("header malformed"), "{err:#}");
    }
}
