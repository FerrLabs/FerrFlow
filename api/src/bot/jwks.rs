use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use jsonwebtoken::{Algorithm, jwk::JwkSet};
use std::sync::LazyLock;

const CACHE_TTL: Duration = Duration::from_hours(1);

struct Cached {
    fetched_at: Instant,
    jwks: JwkSet,
}

static CACHE: LazyLock<Mutex<HashMap<String, Cached>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!("ferrflow-api/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        // A JWKS endpoint that answers with a redirect is either broken or
        // being tampered with; either way we would rather fail than follow it.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client builder never fails on these options")
});

/// Fetch a JWKS document, cached for an hour per URL.
///
/// `url` is a compile-time constant in this crate (GitHub's Actions OIDC
/// endpoint), never caller-supplied, so there is nothing here for an SSRF
/// check to protect.
pub async fn load(url: &str) -> Result<JwkSet> {
    if let Some(hit) = cached(url) {
        return Ok(hit);
    }
    let resp = CLIENT
        .get(url)
        .send()
        .await
        .with_context(|| format!("fetch JWKS {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "JWKS endpoint {url} returned HTTP {}",
            resp.status()
        ));
    }
    let jwks: JwkSet = resp
        .json()
        .await
        .with_context(|| format!("parse JWKS {url}"))?;
    insert(url.to_string(), jwks.clone());
    Ok(jwks)
}

fn cached(url: &str) -> Option<JwkSet> {
    let cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = cache.get(url)?;
    (entry.fetched_at.elapsed() < CACHE_TTL).then(|| entry.jwks.clone())
}

fn insert(url: String, jwks: JwkSet) {
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.insert(
        url,
        Cached {
            fetched_at: Instant::now(),
            jwks,
        },
    );
}

/// The algorithm to verify with, taken from the token header but confined to
/// the asymmetric families. An OIDC issuer never signs with a shared secret,
/// so an `HS*` header is an attempt to have us verify a forgery with a public
/// key as the HMAC secret — forcing RS256 makes the JWK decode reject it.
pub fn pick_algorithm(header: &jsonwebtoken::Header) -> Algorithm {
    match header.alg {
        Algorithm::RS256
        | Algorithm::RS384
        | Algorithm::RS512
        | Algorithm::ES256
        | Algorithm::ES384
        | Algorithm::PS256
        | Algorithm::PS384
        | Algorithm::PS512 => header.alg,
        _ => Algorithm::RS256,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(alg: Algorithm) -> jsonwebtoken::Header {
        jsonwebtoken::Header::new(alg)
    }

    #[test]
    fn asymmetric_algorithms_pass_through() {
        for alg in [
            Algorithm::RS256,
            Algorithm::RS512,
            Algorithm::ES256,
            Algorithm::PS384,
        ] {
            assert_eq!(pick_algorithm(&header(alg)), alg);
        }
    }

    // The alg-confusion attack: claim HS256 so the verifier treats the public
    // key as an HMAC secret the attacker also knows.
    #[test]
    fn hmac_algorithms_are_forced_to_rs256() {
        for alg in [Algorithm::HS256, Algorithm::HS384, Algorithm::HS512] {
            assert_eq!(pick_algorithm(&header(alg)), Algorithm::RS256);
        }
    }
}
