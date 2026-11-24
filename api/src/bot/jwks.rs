use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use jsonwebtoken::jwk::JwkSet;
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
