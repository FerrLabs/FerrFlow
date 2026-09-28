use serde_json::json;

use super::GiteaForge;
use crate::error_code::code_from_error;
use crate::forge::Forge;
use crate::forge::test_server::{FakeServer, Reply};

fn forge(server: &FakeServer) -> GiteaForge {
    GiteaForge {
        token: "gitea-secret".to_string(),
        slug: "owner/repo".to_string(),
        api_base: server.url().to_string(),
        agent: crate::http::agent(),
    }
}

#[test]
fn create_release_posts_the_payload_with_token_auth() {
    let server = FakeServer::start(vec![Reply::json(
        201,
        json!({ "id": 31, "html_url": "https://codeberg.org/owner/repo/releases/tag/v1.0.0" }),
    )]);

    let result = forge(&server)
        .create_release("v1.0.0", "notes", false, true)
        .unwrap();

    assert_eq!(result.id, Some(31));
    assert_eq!(
        result.url.as_deref(),
        Some("https://codeberg.org/owner/repo/releases/tag/v1.0.0")
    );
    let req = server.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/repos/owner/repo/releases");
    assert_eq!(req.header("authorization"), Some("token gitea-secret"));
    let body = req.json();
    assert_eq!(body["tag_name"], "v1.0.0");
    assert_eq!(body["name"], "v1.0.0");
    assert_eq!(body["body"], "notes");
    assert_eq!(body["draft"], true);
    assert_eq!(body["prerelease"], false);
}

#[test]
fn a_rejected_release_carries_the_create_release_code() {
    let server = FakeServer::start(vec![Reply::json(409, json!({}))]);

    let err = forge(&server)
        .create_release("v1.0.0", "notes", false, false)
        .unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3201"));
}

#[test]
fn find_draft_release_pages_by_fifty_and_matches_draft_and_tag() {
    let first_page: Vec<_> = (0..50)
        .map(|i| json!({ "id": i, "tag_name": "v2.0.0", "draft": false }))
        .collect();
    let server = FakeServer::start(vec![
        Reply::json(200, json!(first_page)),
        Reply::json(
            200,
            json!([
                { "id": 70, "tag_name": "v2.0.0-rc.1", "draft": true },
                { "id": 71, "tag_name": "v2.0.0", "draft": true },
            ]),
        ),
    ]);

    let found = forge(&server).find_draft_release("v2.0.0").unwrap();

    assert_eq!(found, Some(71));
    let requests = server.requests();
    assert_eq!(
        requests[0].path,
        "/repos/owner/repo/releases?limit=50&page=1"
    );
    assert_eq!(
        requests[1].path,
        "/repos/owner/repo/releases?limit=50&page=2"
    );
}

#[test]
fn find_draft_release_errors_carry_the_list_code() {
    let server = FakeServer::start(vec![Reply::json(500, json!({}))]);

    let err = forge(&server).find_draft_release("v1.0.0").unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3202"));
}

#[test]
fn publish_release_patches_draft_false() {
    let server = FakeServer::start(vec![
        Reply::json(200, json!({})),
        Reply::json(404, json!({})),
    ]);
    let forge = forge(&server);

    forge.publish_release(31).unwrap();
    let err = forge.publish_release(32).unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3203"));
    let requests = server.requests();
    assert_eq!(requests[0].method, "PATCH");
    assert_eq!(requests[0].path, "/repos/owner/repo/releases/31");
    assert_eq!(requests[0].json(), json!({ "draft": false }));
}

#[test]
fn find_comment_matches_the_marker_on_the_issue_thread() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!([
            { "id": 1, "body": "hello" },
            { "id": 2, "body": "<!-- ferrflow --> preview" },
        ]),
    )]);

    let found = forge(&server).find_comment(8, "<!-- ferrflow -->").unwrap();

    assert_eq!(found, Some(2));
    assert_eq!(
        server.only_request().path,
        "/repos/owner/repo/issues/8/comments?limit=50&page=1"
    );
}

#[test]
fn create_and_update_comment_hit_the_issue_comment_endpoints() {
    let server = FakeServer::start(vec![
        Reply::json(201, json!({})),
        Reply::json(200, json!({})),
        Reply::json(403, json!({})),
    ]);
    let forge = forge(&server);

    forge.create_comment(8, "first").unwrap();
    forge.update_comment(8, 40, "second").unwrap();
    assert!(forge.create_comment(8, "third").is_err());

    let requests = server.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/repos/owner/repo/issues/8/comments");
    assert_eq!(requests[0].json(), json!({ "body": "first" }));
    assert_eq!(requests[1].method, "PATCH");
    assert_eq!(requests[1].path, "/repos/owner/repo/issues/comments/40");
    assert_eq!(requests[1].json(), json!({ "body": "second" }));
}
