use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::error_code::{self, ErrorCodeExt};

const DEFAULT_INDEX: &str = "https://pypi.org";
const DEFAULT_AUDIENCE: &str = "pypi";
const REQUEST_URL_ENV: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
const REQUEST_TOKEN_ENV: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";

#[derive(Deserialize)]
struct AudienceResponse {
    audience: String,
}

#[derive(Deserialize)]
struct IdTokenResponse {
    value: String,
}

#[derive(Deserialize)]
struct MintedTokenResponse {
    token: String,
}

pub fn mint(repository_url: Option<&str>) -> Result<String> {
    let request_url = required_env(REQUEST_URL_ENV)?;
    let request_token = required_env(REQUEST_TOKEN_ENV)?;
    let index = index_base(repository_url)?;
    mint_on(&index, &request_url, &request_token)
}

fn mint_on(index: &str, request_url: &str, request_token: &str) -> Result<String> {
    let agent = crate::http::agent();
    let audience = index_audience(&agent, index)?;

    let id_token: IdTokenResponse = agent
        .get(request_url)
        .query("audience", &audience)
        .header("Authorization", &format!("Bearer {request_token}"))
        .header("Accept", "application/json")
        .call()
        .context("publisher pypi: requesting a GitHub OIDC token failed")
        .error_code(error_code::PUBLISH_FAILED)?
        .body_mut()
        .read_json()
        .context("publisher pypi: the GitHub OIDC token response was not the expected JSON")
        .error_code(error_code::PUBLISH_FAILED)?;

    let endpoint = format!("{index}/_/oidc/mint-token");
    let minted: MintedTokenResponse = agent
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send_json(serde_json::json!({ "token": id_token.value }))
        .with_context(|| {
            format!(
                "publisher pypi: the token exchange at {endpoint} was refused; \
                 check that this repository and workflow are registered as a trusted publisher \
                 for the project"
            )
        })
        .error_code(error_code::PUBLISH_FAILED)?
        .body_mut()
        .read_json()
        .context("publisher pypi: the mint-token response was not the expected JSON")
        .error_code(error_code::PUBLISH_FAILED)?;

    Ok(minted.token)
}

fn index_audience(agent: &ureq::Agent, index: &str) -> Result<String> {
    let url = format!("{index}/_/oidc/audience");
    let fetched = agent
        .get(&url)
        .header("Accept", "application/json")
        .call()
        .map_err(anyhow::Error::from)
        .and_then(|mut response| {
            response
                .body_mut()
                .read_json::<AudienceResponse>()
                .map_err(anyhow::Error::from)
        });
    match (fetched, fallback_audience(index)) {
        (Ok(response), _) => Ok(response.audience),
        (Err(_), Some(audience)) => Ok(audience.to_string()),
        (Err(err), None) => Err(err)
            .with_context(|| {
                format!(
                    "publisher pypi: could not read the trusted publishing audience from {url}; \
                     check that the index supports trusted publishing"
                )
            })
            .error_code(error_code::PUBLISH_FAILED),
    }
}

fn fallback_audience(index: &str) -> Option<&'static str> {
    (index == DEFAULT_INDEX).then_some(DEFAULT_AUDIENCE)
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name)
        .map_err(|_| {
            anyhow!(
                "publisher pypi: trustedPublishing needs `{name}`, which GitHub Actions provides \
                 only when the job declares `permissions: id-token: write`"
            )
        })
        .error_code(error_code::PUBLISHER_MISCONFIGURED)
}

fn index_base(repository_url: Option<&str>) -> Result<String> {
    let Some(raw) = repository_url else {
        return Ok(DEFAULT_INDEX.to_string());
    };
    let host = raw
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .filter(|host| !host.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "publisher pypi: trustedPublishing needs an https registry url to derive the \
                 token endpoint from, got `{raw}`"
            )
        })
        .error_code(error_code::PUBLISHER_MISCONFIGURED)?;
    Ok(format!("https://{host}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::test_server::{FakeServer, Reply};

    fn audience(value: &str) -> Reply {
        Reply::json(200, serde_json::json!({ "audience": value }))
    }

    fn oidc_token() -> Reply {
        Reply::json(200, serde_json::json!({ "value": "oidc-jwt" }))
    }

    #[test]
    fn the_default_index_is_pypi_org() {
        assert_eq!(index_base(None).unwrap(), DEFAULT_INDEX);
    }

    #[test]
    fn a_custom_index_keeps_only_its_host() {
        assert_eq!(
            index_base(Some("https://test.pypi.org/legacy/")).unwrap(),
            "https://test.pypi.org"
        );
    }

    #[test]
    fn a_plaintext_index_is_refused() {
        let err = index_base(Some("http://pypi.internal/simple")).expect_err("must error");
        assert!(format!("{err:?}").contains("https registry url"));
    }

    #[test]
    fn a_url_without_a_host_is_refused() {
        assert!(index_base(Some("https:///legacy/")).is_err());
        assert!(index_base(Some("pypi.internal")).is_err());
    }

    #[test]
    fn only_pypi_org_falls_back_to_the_pypi_audience() {
        assert_eq!(fallback_audience(DEFAULT_INDEX), Some("pypi"));
        assert_eq!(fallback_audience("https://test.pypi.org"), None);
        assert_eq!(fallback_audience("https://pypi.internal"), None);
    }

    #[test]
    fn a_missing_runner_request_url_names_the_permission_to_add() {
        let _guard = crate::test_utils::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let saved = std::env::var(REQUEST_URL_ENV).ok();
        unsafe { std::env::remove_var(REQUEST_URL_ENV) };
        let result = mint(Some("https://test.pypi.org/legacy/"));
        if let Some(value) = saved {
            unsafe { std::env::set_var(REQUEST_URL_ENV, value) };
        }

        let msg = format!("{:#}", result.unwrap_err());
        assert!(msg.contains(REQUEST_URL_ENV), "{msg}");
        assert!(msg.contains("id-token: write"), "{msg}");
    }

    #[test]
    fn the_oidc_token_is_requested_for_the_audience_the_index_advertises() {
        let server = FakeServer::start(vec![
            audience("testpypi"),
            oidc_token(),
            Reply::json(200, serde_json::json!({ "token": "pypi-upload-token" })),
        ]);
        let request_url = format!("{}/oidc?api-version=2.0", server.url());

        let token = mint_on(server.url(), &request_url, "runner-secret").unwrap();

        assert_eq!(token, "pypi-upload-token");
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].path, "/_/oidc/audience");
        assert_eq!(requests[1].path, "/oidc?api-version=2.0&audience=testpypi");
        assert_eq!(
            requests[1].header("authorization"),
            Some("Bearer runner-secret")
        );
        assert_eq!(requests[2].method, "POST");
        assert_eq!(requests[2].path, "/_/oidc/mint-token");
        assert_eq!(
            requests[2].json(),
            serde_json::json!({ "token": "oidc-jwt" })
        );
    }

    #[test]
    fn an_index_without_an_audience_stops_before_asking_github() {
        let server = FakeServer::start(vec![Reply::json(404, serde_json::json!({}))]);
        let request_url = format!("{}/oidc", server.url());

        let err = mint_on(server.url(), &request_url, "runner-secret").unwrap_err();

        assert_eq!(server.only_request().path, "/_/oidc/audience");
        let msg = format!("{err:#}");
        assert!(
            msg.contains(&format!("{}/_/oidc/audience", server.url())),
            "{msg}"
        );
        assert!(!msg.contains("trusted publisher for the project"), "{msg}");
    }

    #[test]
    fn a_refused_exchange_names_the_mint_endpoint_and_the_trusted_publisher() {
        let server = FakeServer::start(vec![
            audience("pypi"),
            oidc_token(),
            Reply::json(400, serde_json::json!({})),
        ]);
        let request_url = format!("{}/oidc", server.url());

        let err = mint_on(server.url(), &request_url, "runner-secret").unwrap_err();

        let msg = format!("{err:#}");
        assert!(
            msg.contains(&format!("{}/_/oidc/mint-token", server.url())),
            "{msg}"
        );
        assert!(msg.contains("trusted publisher"), "{msg}");
    }

    #[test]
    fn a_refused_or_garbled_oidc_token_stops_before_the_exchange() {
        let server = FakeServer::start(vec![
            audience("pypi"),
            Reply::json(403, serde_json::json!({})),
            audience("pypi"),
            Reply::raw(200, "not json"),
        ]);
        let request_url = format!("{}/oidc", server.url());

        let refused = mint_on(server.url(), &request_url, "runner-secret").unwrap_err();
        let garbled = mint_on(server.url(), &request_url, "runner-secret").unwrap_err();

        assert_eq!(server.requests().len(), 4);
        assert!(
            format!("{refused:#}").contains("requesting a GitHub OIDC token failed"),
            "{refused:#}"
        );
        assert!(
            format!("{garbled:#}").contains("not the expected JSON"),
            "{garbled:#}"
        );
    }
}
