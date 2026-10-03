use serde_json::json;

use super::{GitLabForge, GitLabToken};
use crate::error_code::code_from_error;
use crate::forge::test_server::{FakeServer, Reply};
use crate::forge::{Forge, MergeRequestResult};

fn forge_with(server: &FakeServer, token_kind: GitLabToken) -> GitLabForge {
    GitLabForge {
        token: "gl-secret".to_string(),
        token_kind,
        slug: "group/sub/app".to_string(),
        api_base: server.url().to_string(),
        agent: crate::http::agent(),
    }
}

fn forge(server: &FakeServer) -> GitLabForge {
    forge_with(server, GitLabToken::Private)
}

fn mr(id: u64) -> MergeRequestResult {
    MergeRequestResult {
        id,
        auto_merge_key: id.to_string(),
    }
}

const PROJECT: &str = "/projects/group%2Fsub%2Fapp";

#[test]
fn create_release_posts_to_the_encoded_project_with_a_private_token() {
    let server = FakeServer::start(vec![Reply::json(
        201,
        json!({ "_links": { "self": "https://gitlab.com/group/sub/app/-/releases/v1.0.0" } }),
    )]);

    let result = forge(&server)
        .create_release("v1.0.0", "notes", false, false)
        .unwrap();

    assert_eq!(
        result.url.as_deref(),
        Some("https://gitlab.com/group/sub/app/-/releases/v1.0.0")
    );
    assert_eq!(result.id, None);
    let req = server.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, format!("{PROJECT}/releases"));
    assert_eq!(req.header("private-token"), Some("gl-secret"));
    assert_eq!(req.header("job-token"), None);
    let body = req.json();
    assert_eq!(body["tag_name"], "v1.0.0");
    assert_eq!(body["name"], "v1.0.0");
    assert_eq!(body["description"], "notes");
    assert!(body.get("upcoming_release").is_none(), "{body}");
}

#[test]
fn a_prerelease_is_sent_as_an_upcoming_release() {
    let server = FakeServer::start(vec![Reply::json(201, json!({}))]);

    forge(&server)
        .create_release("v1.0.0-rc.1", "notes", true, false)
        .unwrap();

    assert_eq!(server.only_request().json()["upcoming_release"], true);
}

#[test]
fn a_ci_job_token_is_sent_in_the_job_token_header() {
    let server = FakeServer::start(vec![Reply::json(201, json!({}))]);

    forge_with(&server, GitLabToken::Job)
        .create_release("v1.0.0", "notes", false, false)
        .unwrap();

    let req = server.only_request();
    assert_eq!(req.header("job-token"), Some("gl-secret"));
    assert_eq!(req.header("private-token"), None);
}

#[test]
fn a_rejected_release_carries_the_create_release_code() {
    let server = FakeServer::start(vec![Reply::json(
        409,
        json!({ "message": "Release already exists" }),
    )]);

    let err = forge(&server)
        .create_release("v1.0.0", "notes", false, false)
        .unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3101"));
}

#[test]
fn create_merge_request_returns_the_iid() {
    let server = FakeServer::start(vec![Reply::json(201, json!({ "id": 999, "iid": 15 }))]);

    let result = forge(&server)
        .create_merge_request("release/v1", "main", "chore(release): v1", "body")
        .unwrap();

    assert_eq!(result.id, 15);
    assert_eq!(result.auto_merge_key, "15");
    let req = server.only_request();
    assert_eq!(req.path, format!("{PROJECT}/merge_requests"));
    let body = req.json();
    assert_eq!(body["source_branch"], "release/v1");
    assert_eq!(body["target_branch"], "main");
    assert_eq!(body["title"], "chore(release): v1");
    assert_eq!(body["description"], "body");
}

#[test]
fn a_conflicting_mr_surfaces_the_status_and_the_api_message() {
    let server = FakeServer::start(vec![Reply::json(
        409,
        json!({ "message": ["Another open merge request already exists for this source branch: !14"] }),
    )]);

    let err = forge(&server)
        .create_merge_request("release/v1", "main", "t", "b")
        .unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3102"));
    let msg = format!("{err:#}");
    assert!(msg.contains("http status 409"), "{msg}");
    assert!(msg.contains("already exists"), "{msg}");
}

#[test]
fn an_mr_response_without_iid_or_json_is_rejected() {
    let server = FakeServer::start(vec![
        Reply::json(201, json!({ "id": 999 })),
        Reply::raw(201, "not json"),
    ]);
    let forge = forge(&server);

    let missing = forge.create_merge_request("h", "b", "t", "x").unwrap_err();
    let unparsable = forge.create_merge_request("h", "b", "t", "x").unwrap_err();

    assert_eq!(code_from_error(&missing).as_deref(), Some("E3104"));
    assert_eq!(code_from_error(&unparsable).as_deref(), Some("E3103"));
}

#[test]
fn enable_auto_merge_asks_to_merge_when_the_pipeline_succeeds() {
    let server = FakeServer::start(vec![Reply::json(200, json!({}))]);

    forge(&server).enable_auto_merge(&mr(15)).unwrap();

    let req = server.only_request();
    assert_eq!(req.method, "PUT");
    assert_eq!(req.path, format!("{PROJECT}/merge_requests/15/merge"));
    assert_eq!(
        req.json(),
        json!({ "merge_when_pipeline_succeeds": true, "squash": true })
    );
}

#[test]
fn enable_auto_merge_falls_back_to_an_immediate_merge() {
    let server = FakeServer::start(vec![
        Reply::json(405, json!({ "message": "Method Not Allowed" })),
        Reply::json(200, json!({})),
    ]);

    forge(&server).enable_auto_merge(&mr(15)).unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1].json(),
        json!({ "squash": true, "should_remove_source_branch": true })
    );
}

#[test]
fn enable_auto_merge_fails_when_both_merge_attempts_fail() {
    let server = FakeServer::start(vec![
        Reply::json(405, json!({})),
        Reply::json(406, json!({})),
    ]);

    let err = forge(&server).enable_auto_merge(&mr(15)).unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3105"));
    assert!(format!("{err:#}").contains("!15"), "{err:#}");
}

#[test]
fn find_comment_pages_through_notes_until_the_marker() {
    let first_page: Vec<_> = (0..100)
        .map(|i| json!({ "id": i, "body": "unrelated" }))
        .collect();
    let server = FakeServer::start(vec![
        Reply::json(200, json!(first_page)),
        Reply::json(200, json!([{ "id": 500, "body": "x <!-- ferrflow --> y" }])),
    ]);

    let found = forge(&server)
        .find_comment(15, "<!-- ferrflow -->")
        .unwrap();

    assert_eq!(found, Some(500));
    let requests = server.requests();
    assert_eq!(
        requests[1].path,
        format!("{PROJECT}/merge_requests/15/notes?per_page=100&page=2")
    );
}

#[test]
fn a_malformed_notes_page_is_an_error() {
    let server = FakeServer::start(vec![Reply::raw(200, "<html>")]);

    let err = forge(&server).find_comment(15, "marker").unwrap_err();

    assert!(format!("{err:#}").contains("MR notes"), "{err:#}");
}

#[test]
fn create_and_update_comment_hit_the_notes_endpoints() {
    let server = FakeServer::start(vec![
        Reply::json(201, json!({})),
        Reply::json(200, json!({})),
    ]);
    let forge = forge(&server);

    forge.create_comment(15, "first").unwrap();
    forge.update_comment(15, 77, "second").unwrap();

    let requests = server.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(
        requests[0].path,
        format!("{PROJECT}/merge_requests/15/notes")
    );
    assert_eq!(requests[0].json(), json!({ "body": "first" }));
    assert_eq!(requests[1].method, "PUT");
    assert_eq!(
        requests[1].path,
        format!("{PROJECT}/merge_requests/15/notes/77")
    );
    assert_eq!(requests[1].json(), json!({ "body": "second" }));
}

#[test]
fn find_open_pr_filters_opened_mrs_by_source_and_target() {
    let server = FakeServer::start(vec![Reply::json(200, json!([{ "iid": 14 }, { "iid": 3 }]))]);

    let found = forge(&server).find_open_pr("release/v1", "main").unwrap();

    assert_eq!(found, Some(14));
    assert_eq!(
        server.only_request().path,
        format!(
            "{PROJECT}/merge_requests?state=opened&source_branch=release%2Fv1&target_branch=main"
        )
    );
}

#[test]
fn find_open_pr_encodes_a_branch_with_query_characters() {
    let server = FakeServer::start(vec![Reply::json(200, json!([{ "iid": 9 }]))]);

    let found = forge(&server).find_open_pr("fix/a&b#c+d", "main").unwrap();

    assert_eq!(found, Some(9));
    assert_eq!(
        server.only_request().path,
        format!(
            "{PROJECT}/merge_requests?state=opened&source_branch=fix%2Fa%26b%23c%2Bd&target_branch=main"
        )
    );
}

#[test]
fn find_open_pr_without_a_match_is_none_and_errors_are_coded() {
    let server = FakeServer::start(vec![
        Reply::json(200, json!([])),
        Reply::json(401, json!({})),
        Reply::raw(200, "{"),
    ]);
    let forge = forge(&server);

    assert_eq!(forge.find_open_pr("h", "main").unwrap(), None);
    let transport = forge.find_open_pr("h", "main").unwrap_err();
    let parse = forge.find_open_pr("h", "main").unwrap_err();

    assert_eq!(code_from_error(&transport).as_deref(), Some("E3106"));
    assert_eq!(code_from_error(&parse).as_deref(), Some("E3103"));
}

#[test]
fn update_merge_request_puts_title_and_description() {
    let server = FakeServer::start(vec![Reply::json(200, json!({ "iid": 14 }))]);

    let result = forge(&server)
        .update_merge_request(14, "new title", "new body")
        .unwrap();

    assert_eq!(result.id, 14);
    assert_eq!(result.auto_merge_key, "14");
    let req = server.only_request();
    assert_eq!(req.method, "PUT");
    assert_eq!(req.path, format!("{PROJECT}/merge_requests/14"));
    assert_eq!(
        req.json(),
        json!({ "title": "new title", "description": "new body" })
    );
}

#[test]
fn a_refused_mr_update_names_the_mr_and_the_reason() {
    let server = FakeServer::start(vec![Reply::json(
        403,
        json!({ "message": "403 Forbidden" }),
    )]);

    let err = forge(&server)
        .update_merge_request(14, "t", "b")
        .unwrap_err();

    assert_eq!(code_from_error(&err).as_deref(), Some("E3107"));
    let msg = format!("{err:#}");
    assert!(msg.contains("!14"), "{msg}");
    assert!(msg.contains("403 Forbidden"), "{msg}");
}
