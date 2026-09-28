use serde_json::json;

use super::BotTokenExchange;
use crate::forge::test_server::{FakeServer, Reply};
use crate::test_utils::ENV_LOCK;

fn exchange_at(server: &FakeServer, audience: &str) -> BotTokenExchange {
    BotTokenExchange {
        endpoint: format!("{}/ferrflow/token", server.url()),
        audience: audience.to_string(),
    }
}

fn with_runner_env<T>(request_url: &str, request_token: &str, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let names = [
        "ACTIONS_ID_TOKEN_REQUEST_URL",
        "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
    ];
    let saved: Vec<_> = names.iter().map(|n| std::env::var(n).ok()).collect();
    unsafe {
        std::env::set_var(names[0], request_url);
        std::env::set_var(names[1], request_token);
    }
    let out = f();
    for (name, value) in names.iter().zip(saved) {
        unsafe {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
    out
}

#[test]
fn the_oidc_request_carries_the_runner_token_and_the_encoded_audience() {
    let server = FakeServer::start(vec![Reply::json(200, json!({ "value": "oidc-jwt" }))]);
    let exchange = exchange_at(&server, "ferrflow ferrlabs&x");

    let token = with_runner_env(
        &format!("{}/oidc?api-version=2.0", server.url()),
        "runner-secret",
        || exchange.request_oidc_token(&crate::http::agent()),
    )
    .unwrap();

    assert_eq!(token, "oidc-jwt");
    let req = server.only_request();
    assert_eq!(req.method, "GET");
    assert_eq!(
        req.path,
        "/oidc?api-version=2.0&audience=ferrflow%20ferrlabs%26x"
    );
    assert_eq!(req.header("authorization"), Some("Bearer runner-secret"));
}

#[test]
fn a_bare_request_url_gets_the_audience_as_its_only_query() {
    let server = FakeServer::start(vec![Reply::json(200, json!({ "value": "oidc-jwt" }))]);
    let exchange = exchange_at(&server, "ferrflow.ferrlabs.com");

    with_runner_env(&format!("{}/oidc", server.url()), "t", || {
        exchange.request_oidc_token(&crate::http::agent())
    })
    .unwrap();

    assert_eq!(
        server.only_request().path,
        "/oidc?audience=ferrflow.ferrlabs.com"
    );
}

#[test]
fn a_runner_that_refuses_or_garbles_the_oidc_token_is_an_error() {
    let server = FakeServer::start(vec![
        Reply::json(403, json!({})),
        Reply::raw(200, "not json"),
        Reply::json(200, json!({ "value": "" })),
    ]);
    let exchange = exchange_at(&server, "aud");
    let request_url = format!("{}/oidc", server.url());

    let errors: Vec<String> = (0..3)
        .map(|_| {
            with_runner_env(&request_url, "t", || {
                exchange.request_oidc_token(&crate::http::agent())
            })
            .unwrap_err()
            .to_string()
        })
        .collect();

    assert!(
        errors[0].contains("failed to request OIDC token"),
        "{}",
        errors[0]
    );
    assert!(errors[1].contains("not valid JSON"), "{}", errors[1]);
    assert!(errors[2].contains("`value`"), "{}", errors[2]);
}

#[test]
fn the_exchange_posts_the_oidc_token_and_returns_the_issued_token() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!({
            "token": "ghs_installation",
            "expires_at": "2026-09-28T12:00:00Z",
            "repository": "FerrLabs/FerrFlow"
        }),
    )]);

    let issued = exchange_at(&server, "aud")
        .exchange(&crate::http::agent(), "oidc-jwt")
        .unwrap();

    assert_eq!(issued.token, "ghs_installation");
    assert_eq!(issued.expires_at, "2026-09-28T12:00:00Z");
    assert_eq!(issued.repository, "FerrLabs/FerrFlow");
    let req = server.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/ferrflow/token");
    assert_eq!(req.json(), json!({ "token": "oidc-jwt" }));
}

#[test]
fn optional_fields_may_be_absent_from_the_issued_token() {
    let server = FakeServer::start(vec![Reply::json(200, json!({ "token": "ghs_x" }))]);

    let issued = exchange_at(&server, "aud")
        .exchange(&crate::http::agent(), "oidc-jwt")
        .unwrap();

    assert_eq!(issued.token, "ghs_x");
    assert!(issued.expires_at.is_empty());
    assert!(issued.repository.is_empty());
}

#[test]
fn a_rate_limited_exchange_is_retried_and_then_succeeds() {
    let server = FakeServer::start(vec![
        Reply::json(429, json!({})),
        Reply::json(200, json!({ "token": "ghs_after_retry" })),
    ]);

    let issued = exchange_at(&server, "aud")
        .exchange(&crate::http::agent(), "oidc-jwt")
        .unwrap();

    assert_eq!(issued.token, "ghs_after_retry");
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].json(), json!({ "token": "oidc-jwt" }));
}

#[test]
fn a_rejected_oidc_token_fails_at_once_with_a_clear_message() {
    let server = FakeServer::start(vec![
        Reply::json(401, json!({})),
        Reply::json(200, json!({ "token": "never reached" })),
    ]);

    let err = exchange_at(&server, "aud")
        .exchange(&crate::http::agent(), "oidc-jwt")
        .unwrap_err();

    assert!(
        err.to_string().contains("OIDC verification failed (401)"),
        "{err}"
    );
    assert_eq!(server.requests().len(), 1, "a 401 must not be retried");
}

#[test]
fn a_missing_installation_points_at_the_app_install_page() {
    let server = FakeServer::start(vec![Reply::json(404, json!({}))]);

    let err = exchange_at(&server, "aud")
        .exchange(&crate::http::agent(), "oidc-jwt")
        .unwrap_err();

    assert!(
        err.to_string().contains("github.com/apps/ferrflow"),
        "{err}"
    );
}

#[test]
fn an_unusable_issued_token_response_is_an_error() {
    let server = FakeServer::start(vec![
        Reply::json(200, json!({ "token": "" })),
        Reply::raw(200, "<html>"),
    ]);
    let exchange = exchange_at(&server, "aud");
    let agent = crate::http::agent();

    let empty = exchange.exchange(&agent, "oidc-jwt").unwrap_err();
    let garbled = exchange.exchange(&agent, "oidc-jwt").unwrap_err();

    assert!(
        empty.to_string().contains("did not contain a token"),
        "{empty}"
    );
    assert!(garbled.to_string().contains("not valid JSON"), "{garbled}");
}
