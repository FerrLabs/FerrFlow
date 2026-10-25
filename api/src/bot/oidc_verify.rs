//! Verify a GitHub Actions OIDC ID token.
//!
//! GitHub issues these to any workflow that requests the `id-token: write`
//! permission. Each token is scoped to a single workflow run on a single
//! repository and carries the repo, ref, run id and actor as claims we act
//! on. Verification is standard OIDC: fetch JWKS, match `kid`, check `iss`,
//! `aud`, `exp`, signature.

use anyhow::{Context, Result, anyhow};
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
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

    let alg = jwks::pick_algorithm(&header);
    let set = jwks::load(GHA_JWKS_URL).await?;
    let jwk = set
        .find(&kid)
        .ok_or_else(|| anyhow!("no JWKS entry matches kid {kid}"))?;
    let key = DecodingKey::from_jwk(jwk).context("unsupported JWK")?;

    let mut validation = Validation::new(alg);
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
    use super::*;

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
