//! Thin wrapper over the GitHub REST API for the two calls the hosted bot
//! actually makes: find the installation on a target owner, mint an
//! installation access token scoped to a single repository.

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use reqwest::{StatusCode, header};
use serde::{Deserialize, Serialize};

/// Errors surfaced to the handler. Variants map directly to FERRFLOW_* error
/// codes at the route layer.
#[derive(Debug)]
pub enum GhError {
    /// The targeted owner (org or user) has no FerrFlow installation.
    InstallNotFound,
    /// Anything else GitHub responded with that isn't a clean success —
    /// 5xx, network, JSON parse failure. The inner error is logged on our
    /// side, not propagated to the caller.
    Upstream(anyhow::Error),
    /// We hit the App-level rate limit on our App JWT. Usually means
    /// runaway traffic from a single org; the route maps this to a 429
    /// with FERRFLOW_RATE_LIMITED so callers back off.
    RateLimited,
}

impl std::fmt::Display for GhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GhError::InstallNotFound => f.write_str("github installation not found"),
            GhError::Upstream(e) => write!(f, "github upstream error: {e:#}"),
            GhError::RateLimited => f.write_str("github rate limited"),
        }
    }
}

impl std::error::Error for GhError {}

/// Permissions block sent on `POST /app/installations/{id}/access_tokens`.
/// All constants — Phase 1 mints exactly one permission set, the one
/// FerrFlow needs to push tags, open release PRs, and publish releases.
#[derive(Debug, Serialize)]
pub struct Permissions {
    pub contents: &'static str,
    pub metadata: &'static str,
    pub pull_requests: &'static str,
}

impl Default for Permissions {
    fn default() -> Self {
        // contents:write → push tags and create releases
        // pull_requests:write → open release PRs when release-PR flow is on
        // metadata:read → required alongside any other permission by GitHub
        Permissions {
            contents: "write",
            metadata: "read",
            pull_requests: "write",
        }
    }
}

#[derive(Debug, Deserialize)]
struct InstallationResponse {
    id: u64,
}

#[derive(Debug, Deserialize)]
pub struct InstallationToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AccessTokenRequest<'a> {
    repositories: Vec<&'a str>,
    permissions: &'a Permissions,
}

pub struct GithubClient {
    http: reqwest::Client,
    base_url: String,
}

impl GithubClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("ferrflow-bot/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client builder never panics on these options");
        GithubClient {
            http,
            base_url: base_url.into(),
        }
    }

    /// Resolve the installation id of the FerrFlow GitHub App on a target
    /// owner. GitHub doesn't expose a single "any account type" endpoint —
    /// try the org route first, fall back to the user route on 404. The
    /// Renovate-Cloud client does the same.
    pub async fn find_installation(&self, app_jwt: &str, owner: &str) -> Result<u64, GhError> {
        let org_url = format!("{}/orgs/{}/installation", self.base_url, owner);
        let resp = self.send_get(&org_url, app_jwt).await?;
        match resp.status() {
            StatusCode::OK => parse_installation(resp).await,
            StatusCode::NOT_FOUND => {
                let user_url = format!("{}/users/{}/installation", self.base_url, owner);
                let resp = self.send_get(&user_url, app_jwt).await?;
                match resp.status() {
                    StatusCode::OK => parse_installation(resp).await,
                    StatusCode::NOT_FOUND => Err(GhError::InstallNotFound),
                    s if is_rate_limited(s, &resp) => Err(GhError::RateLimited),
                    s => Err(GhError::Upstream(anyhow!(
                        "unexpected status from user installation lookup: {s}"
                    ))),
                }
            }
            s if is_rate_limited(s, &resp) => Err(GhError::RateLimited),
            s => Err(GhError::Upstream(anyhow!(
                "unexpected status from org installation lookup: {s}"
            ))),
        }
    }

    /// Mint a short-lived installation token scoped to `repo` (unqualified
    /// name — GitHub infers the owner from the install). The `permissions`
    /// block is the intersection of what the App has and what we ask for;
    /// GitHub refuses the call if we ask for more than the App holds.
    pub async fn create_installation_token(
        &self,
        app_jwt: &str,
        install_id: u64,
        repo: &str,
        permissions: &Permissions,
    ) -> Result<InstallationToken, GhError> {
        let url = format!(
            "{}/app/installations/{}/access_tokens",
            self.base_url, install_id
        );
        let body = AccessTokenRequest {
            repositories: vec![repo],
            permissions,
        };
        let resp = self
            .http
            .post(&url)
            .bearer_auth(app_jwt)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&body)
            .send()
            .await
            .map_err(|e| GhError::Upstream(anyhow!(e)))?;
        match resp.status() {
            StatusCode::CREATED | StatusCode::OK => resp
                .json::<InstallationToken>()
                .await
                .map_err(|e| GhError::Upstream(anyhow!(e))),
            s if is_rate_limited(s, &resp) => Err(GhError::RateLimited),
            s => {
                let detail = resp.text().await.unwrap_or_default();
                Err(GhError::Upstream(anyhow!(
                    "access_tokens call failed: {s}: {detail}"
                )))
            }
        }
    }

    async fn send_get(&self, url: &str, app_jwt: &str) -> Result<reqwest::Response, GhError> {
        self.http
            .get(url)
            .bearer_auth(app_jwt)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| GhError::Upstream(anyhow!(e)))
    }
}

async fn parse_installation(resp: reqwest::Response) -> Result<u64, GhError> {
    let install: InstallationResponse = resp
        .json()
        .await
        .map_err(|e| GhError::Upstream(anyhow!(e)))?;
    Ok(install.id)
}

fn is_rate_limited(status: StatusCode, resp: &reqwest::Response) -> bool {
    if status != StatusCode::FORBIDDEN && status != StatusCode::TOO_MANY_REQUESTS {
        return false;
    }
    resp.headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim() == "0")
        || status == StatusCode::TOO_MANY_REQUESTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_response_deserialises() {
        // A minimal realistic sample of /orgs/{org}/installation. GitHub
        // actually returns ~40 fields; serde's `deny_unknown_fields` would
        // break on any addition, so we stay lenient.
        let body = r#"{
            "id": 12345678,
            "account": { "login": "FerrLabs", "type": "Organization" },
            "app_id": 987654,
            "target_type": "Organization"
        }"#;
        let parsed: InstallationResponse = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.id, 12_345_678);
    }

    #[test]
    fn access_token_response_deserialises() {
        let body = r#"{
            "token": "ghs_thisIsAnInstallationTokenExampleValue",
            "expires_at": "2026-04-21T12:34:56Z",
            "permissions": { "contents": "write", "metadata": "read", "pull_requests": "write" },
            "repository_selection": "selected"
        }"#;
        let parsed: InstallationToken = serde_json::from_str(body).unwrap();
        assert!(parsed.token.starts_with("ghs_"));
        assert_eq!(parsed.expires_at.to_rfc3339(), "2026-04-21T12:34:56+00:00");
    }

    #[test]
    fn default_permissions_match_phase1_contract() {
        let p = Permissions::default();
        assert_eq!(p.contents, "write");
        assert_eq!(p.metadata, "read");
        assert_eq!(p.pull_requests, "write");
    }

    #[test]
    fn access_token_request_serialises_compactly() {
        let p = Permissions::default();
        let body = AccessTokenRequest {
            repositories: vec!["FerrFlow"],
            permissions: &p,
        };
        let v = serde_json::to_value(&body).unwrap();
        assert_eq!(v["repositories"][0], "FerrFlow");
        assert_eq!(v["permissions"]["contents"], "write");
    }
}
