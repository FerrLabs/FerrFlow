use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::error_code::{self, ErrorCodeExt};

const AUDIENCE: &str = "pypi";
const DEFAULT_MINT_ENDPOINT: &str = "https://pypi.org/_/oidc/mint-token";
const REQUEST_URL_ENV: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
const REQUEST_TOKEN_ENV: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";

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
    let endpoint = mint_endpoint(repository_url)?;
    let agent = crate::http::agent();

    let id_token: IdTokenResponse = agent
        .get(&audience_url(&request_url))
        .header("Authorization", &format!("Bearer {request_token}"))
        .header("Accept", "application/json")
        .call()
        .context("publisher pypi: requesting a GitHub OIDC token failed")
        .error_code(error_code::CONFIG_INVALID_PATH)?
        .body_mut()
        .read_json()
        .context("publisher pypi: the GitHub OIDC token response was not the expected JSON")
        .error_code(error_code::CONFIG_INVALID_PATH)?;

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
        .error_code(error_code::CONFIG_INVALID_PATH)?
        .body_mut()
        .read_json()
        .context("publisher pypi: the mint-token response was not the expected JSON")
        .error_code(error_code::CONFIG_INVALID_PATH)?;

    Ok(minted.token)
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name)
        .map_err(|_| {
            anyhow!(
                "publisher pypi: trustedPublishing needs `{name}`, which GitHub Actions provides \
                 only when the job declares `permissions: id-token: write`"
            )
        })
        .error_code(error_code::CONFIG_INVALID_PATH)
}

fn audience_url(base: &str) -> String {
    let separator = if base.contains('?') { '&' } else { '?' };
    format!("{base}{separator}audience={AUDIENCE}")
}

fn mint_endpoint(repository_url: Option<&str>) -> Result<String> {
    let Some(raw) = repository_url else {
        return Ok(DEFAULT_MINT_ENDPOINT.to_string());
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
        .error_code(error_code::CONFIG_INVALID_PATH)?;
    Ok(format!("https://{host}/_/oidc/mint-token"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_audience_is_appended_to_a_bare_request_url() {
        assert_eq!(
            audience_url("https://token.actions.githubusercontent.com/"),
            "https://token.actions.githubusercontent.com/?audience=pypi"
        );
    }

    #[test]
    fn the_audience_joins_an_existing_query_string() {
        assert_eq!(
            audience_url("https://token.actions.githubusercontent.com/?api-version=2.0"),
            "https://token.actions.githubusercontent.com/?api-version=2.0&audience=pypi"
        );
    }

    #[test]
    fn the_default_index_mints_on_pypi_org() {
        assert_eq!(mint_endpoint(None).unwrap(), DEFAULT_MINT_ENDPOINT);
    }

    #[test]
    fn a_custom_index_mints_on_its_own_host() {
        assert_eq!(
            mint_endpoint(Some("https://test.pypi.org/legacy/")).unwrap(),
            "https://test.pypi.org/_/oidc/mint-token"
        );
    }

    #[test]
    fn a_plaintext_index_is_refused() {
        let err = mint_endpoint(Some("http://pypi.internal/simple")).expect_err("must error");
        assert!(format!("{err:?}").contains("https registry url"));
    }

    #[test]
    fn a_url_without_a_host_is_refused() {
        assert!(mint_endpoint(Some("https:///legacy/")).is_err());
        assert!(mint_endpoint(Some("pypi.internal")).is_err());
    }
}
