use serde_json::json;

use super::BitbucketForge;
use crate::error_code::code_from_error;
use crate::forge::Forge;
use crate::forge::test_server::{FakeServer, Reply};

fn cloud(server: &FakeServer) -> BitbucketForge {
    BitbucketForge {
        token: "bb-secret".to_string(),
        slug: "workspace/repo".to_string(),
        api_base: server.url().to_string(),
        is_cloud: true,
        agent: crate::http::agent(),
    }
}

#[test]
fn a_cloud_release_resolves_the_tag_and_returns_its_page() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!({
            "name": "v1.0.0",
            "links": { "html": { "href": "https://bitbucket.org/workspace/repo/commits/tag/v1.0.0" } }
        }),
    )]);

    let result = cloud(&server)
        .create_release("v1.0.0", "notes", false, false)
        .unwrap();

    assert_eq!(
        result.url.as_deref(),
        Some("https://bitbucket.org/workspace/repo/commits/tag/v1.0.0")
    );
    let req = server.only_request();
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/repositories/workspace/repo/refs/tags/v1.0.0");
    assert_eq!(req.header("authorization"), Some("Bearer bb-secret"));
}

#[test]
fn a_missing_cloud_tag_is_a_create_release_error() {
    let server = FakeServer::start(vec![Reply::json(
        404,
        json!({ "error": { "message": "tag not found" } }),
    )]);

    let err = cloud(&server)
        .create_release("v1.0.0", "notes", false, false)
        .unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3301"));
    assert!(format!("{err:#}").contains("v1.0.0"), "{err:#}");
}

#[test]
fn a_cloud_tag_response_without_links_has_no_url() {
    let server = FakeServer::start(vec![Reply::raw(200, "not json")]);

    let result = cloud(&server)
        .create_release("v1.0.0", "notes", false, false)
        .unwrap();

    assert_eq!(result.url, None);
}
