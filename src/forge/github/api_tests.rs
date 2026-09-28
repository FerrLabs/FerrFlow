use serde_json::json;

use super::GitHubForge;
use crate::error_code::code_from_error;
use crate::forge::test_server::{FakeServer, Reply};
use crate::forge::{AuthoredCommit, FileAddition, Forge, MergeRequestResult};

fn forge(server: &FakeServer) -> GitHubForge {
    GitHubForge {
        token: "gh-secret".to_string(),
        slug: "owner/repo".to_string(),
        api_base: server.url().to_string(),
        agent: crate::http::agent(),
    }
}

fn pr(id: u64, node: &str) -> MergeRequestResult {
    MergeRequestResult {
        id,
        auto_merge_key: node.to_string(),
    }
}

fn code(err: &anyhow::Error) -> Option<String> {
    code_from_error(err)
}

#[test]
fn create_release_posts_the_payload_with_bearer_auth_and_reads_id_and_url() {
    let server = FakeServer::start(vec![Reply::json(
        201,
        json!({ "id": 55, "html_url": "https://github.com/owner/repo/releases/tag/v1.2.0" }),
    )]);

    let result = forge(&server)
        .create_release("v1.2.0", "notes", true, false)
        .unwrap();

    assert_eq!(result.id, Some(55));
    assert_eq!(
        result.url.as_deref(),
        Some("https://github.com/owner/repo/releases/tag/v1.2.0")
    );
    let req = server.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/repos/owner/repo/releases");
    assert_eq!(req.header("authorization"), Some("Bearer gh-secret"));
    assert_eq!(req.header("x-github-api-version"), Some("2022-11-28"));
    let body = req.json();
    assert_eq!(body["tag_name"], "v1.2.0");
    assert_eq!(body["name"], "v1.2.0");
    assert_eq!(body["body"], "notes");
    assert_eq!(body["prerelease"], true);
    assert_eq!(body["draft"], false);
}

#[test]
fn an_existing_release_is_an_error_with_the_create_release_code() {
    let server = FakeServer::start(vec![Reply::json(
        422,
        json!({ "message": "Validation Failed", "errors": [{ "code": "already_exists" }] }),
    )]);

    let err = forge(&server)
        .create_release("v1.2.0", "notes", false, false)
        .unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3001"));
    assert!(format!("{err:#}").contains("v1.2.0"), "{err:#}");
}

#[test]
fn a_release_created_with_an_unparsable_body_still_succeeds_without_id() {
    let server = FakeServer::start(vec![Reply::raw(201, "not json")]);

    let result = forge(&server)
        .create_release("v1.2.0", "notes", false, true)
        .unwrap();

    assert_eq!(result.id, None);
    assert_eq!(result.url, None);
}

#[test]
fn find_draft_release_walks_every_full_page_until_it_finds_the_tag() {
    let first_page: Vec<_> = (0..100)
        .map(|i| json!({ "id": i, "tag_name": format!("v0.{i}.0"), "draft": true }))
        .collect();
    let server = FakeServer::start(vec![
        Reply::json(200, json!(first_page)),
        Reply::json(
            200,
            json!([
                { "id": 900, "tag_name": "v2.0.0", "draft": false },
                { "id": 901, "tag_name": "v2.0.0", "draft": true },
            ]),
        ),
    ]);

    let found = forge(&server).find_draft_release("v2.0.0").unwrap();

    assert_eq!(found, Some(901));
    let requests = server.requests();
    let paths: Vec<_> = requests.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/repos/owner/repo/releases?per_page=100&page=1",
            "/repos/owner/repo/releases?per_page=100&page=2",
        ]
    );
}

#[test]
fn find_draft_release_stops_at_a_short_page() {
    let server = FakeServer::start(vec![
        Reply::json(
            200,
            json!([{ "id": 1, "tag_name": "v1.0.0", "draft": false }]),
        ),
        Reply::json(
            200,
            json!([{ "id": 2, "tag_name": "v1.0.0", "draft": true }]),
        ),
    ]);

    assert_eq!(forge(&server).find_draft_release("v1.0.0").unwrap(), None);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_malformed_release_list_is_a_list_releases_error() {
    let server = FakeServer::start(vec![Reply::raw(200, "<html>oops</html>")]);

    let err = forge(&server).find_draft_release("v1.0.0").unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3002"));
    assert!(format!("{err:#}").contains("Failed to parse"), "{err:#}");
}

#[test]
fn publish_release_patches_draft_false_on_that_release() {
    let server = FakeServer::start(vec![Reply::json(200, json!({}))]);

    forge(&server).publish_release(7).unwrap();

    let req = server.only_request();
    assert_eq!(req.method, "PATCH");
    assert_eq!(req.path, "/repos/owner/repo/releases/7");
    assert_eq!(req.json(), json!({ "draft": false }));
}

#[test]
fn a_failed_publish_carries_the_publish_code() {
    let server = FakeServer::start(vec![Reply::json(404, json!({}))]);

    let err = forge(&server).publish_release(7).unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3004"));
}

#[test]
fn delete_release_sends_a_delete_and_maps_failure() {
    let server = FakeServer::start(vec![Reply::raw(204, ""), Reply::json(404, json!({}))]);
    let forge = forge(&server);

    forge.delete_release(9).unwrap();
    let err = forge.delete_release(10).unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3016"));
    let requests = server.requests();
    assert_eq!(requests[0].method, "DELETE");
    assert_eq!(requests[0].path, "/repos/owner/repo/releases/9");
    assert_eq!(requests[1].path, "/repos/owner/repo/releases/10");
}

#[test]
fn create_merge_request_returns_the_number_and_node_id() {
    let server = FakeServer::start(vec![Reply::json(
        201,
        json!({ "number": 42, "node_id": "PR_kw42" }),
    )]);

    let mr = forge(&server)
        .create_merge_request("release/v1", "main", "chore(release): v1", "body")
        .unwrap();

    assert_eq!(mr.id, 42);
    assert_eq!(mr.auto_merge_key, "PR_kw42");
    let req = server.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/repos/owner/repo/pulls");
    let body = req.json();
    assert_eq!(body["head"], "release/v1");
    assert_eq!(body["base"], "main");
    assert_eq!(body["title"], "chore(release): v1");
    assert_eq!(body["body"], "body");
}

#[test]
fn a_rejected_pr_surfaces_the_status_and_the_api_message() {
    let server = FakeServer::start(vec![Reply::json(
        422,
        json!({ "message": "A pull request already exists for owner:release/v1." }),
    )]);

    let err = forge(&server)
        .create_merge_request("release/v1", "main", "t", "b")
        .unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3005"));
    let msg = format!("{err:#}");
    assert!(msg.contains("http status 422"), "{msg}");
    assert!(msg.contains("A pull request already exists"), "{msg}");
}

#[test]
fn a_pr_response_without_node_id_or_json_is_rejected() {
    let server = FakeServer::start(vec![
        Reply::json(201, json!({ "number": 42 })),
        Reply::json(201, json!({ "node_id": "PR_x" })),
        Reply::raw(201, "{"),
    ]);
    let forge = forge(&server);

    let missing_node = forge.create_merge_request("h", "b", "t", "x").unwrap_err();
    let missing_number = forge.create_merge_request("h", "b", "t", "x").unwrap_err();
    let unparsable = forge.create_merge_request("h", "b", "t", "x").unwrap_err();

    assert_eq!(code(&missing_node).as_deref(), Some("E3007"));
    assert_eq!(code(&missing_number).as_deref(), Some("E3007"));
    assert_eq!(code(&unparsable).as_deref(), Some("E3006"));
}

#[test]
fn enable_auto_merge_sends_the_node_id_to_graphql() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!({ "data": { "enablePullRequestAutoMerge": { "pullRequest": { "number": 42 } } } }),
    )]);

    forge(&server)
        .enable_auto_merge(&pr(42, "PR_kw42"))
        .unwrap();

    let req = server.only_request();
    assert_eq!(req.path, "/graphql");
    assert_eq!(req.header("authorization"), Some("Bearer gh-secret"));
    let body = req.json();
    assert_eq!(body["variables"]["prId"], "PR_kw42");
    assert!(
        body["query"]
            .as_str()
            .unwrap()
            .contains("mergeMethod: SQUASH")
    );
}

#[test]
fn a_graphql_error_on_auto_merge_fails_with_its_message() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!({ "errors": [{ "message": "Auto merge is not allowed for this repository" }] }),
    )]);

    let err = forge(&server)
        .enable_auto_merge(&pr(42, "PR_kw42"))
        .unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3010"));
    let msg = format!("{err:#}");
    assert!(msg.contains("PR #42"), "{msg}");
    assert!(msg.contains("Auto merge is not allowed"), "{msg}");
}

#[test]
fn a_transport_failure_on_auto_merge_uses_the_request_code() {
    let server = FakeServer::start(vec![Reply::json(502, json!({}))]);

    let err = forge(&server)
        .enable_auto_merge(&pr(42, "PR_kw42"))
        .unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3008"));
}

#[test]
fn find_comment_returns_the_first_comment_carrying_the_marker() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!([
            { "id": 1, "body": "looks good" },
            { "id": 2, "body": "<!-- ferrflow -->\npreview" },
            { "id": 3, "body": "<!-- ferrflow --> again" },
        ]),
    )]);

    let found = forge(&server).find_comment(5, "<!-- ferrflow -->").unwrap();

    assert_eq!(found, Some(2));
    assert_eq!(
        server.only_request().path,
        "/repos/owner/repo/issues/5/comments?per_page=100&page=1"
    );
}

#[test]
fn find_comment_on_an_empty_thread_is_none() {
    let server = FakeServer::start(vec![Reply::json(200, json!([]))]);

    assert_eq!(forge(&server).find_comment(5, "marker").unwrap(), None);
}

#[test]
fn create_and_update_comment_hit_the_issue_comment_endpoints() {
    let server = FakeServer::start(vec![
        Reply::json(201, json!({ "id": 77 })),
        Reply::json(200, json!({ "id": 77 })),
    ]);
    let forge = forge(&server);

    forge.create_comment(5, "first").unwrap();
    forge.update_comment(5, 77, "second").unwrap();

    let requests = server.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/repos/owner/repo/issues/5/comments");
    assert_eq!(requests[0].json(), json!({ "body": "first" }));
    assert_eq!(requests[1].method, "PATCH");
    assert_eq!(requests[1].path, "/repos/owner/repo/issues/comments/77");
    assert_eq!(requests[1].json(), json!({ "body": "second" }));
}

#[test]
fn a_failed_comment_write_is_an_error() {
    let server = FakeServer::start(vec![
        Reply::json(403, json!({})),
        Reply::json(404, json!({})),
    ]);
    let forge = forge(&server);

    assert!(forge.create_comment(5, "x").is_err());
    assert!(forge.update_comment(5, 77, "x").is_err());
}

#[test]
fn find_open_pr_filters_by_owner_qualified_head_and_base() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!([{ "number": 12 }, { "number": 13 }]),
    )]);

    let found = forge(&server).find_open_pr("release/v1", "main").unwrap();

    assert_eq!(found, Some(12));
    assert_eq!(
        server.only_request().path,
        "/repos/owner/repo/pulls?state=open&head=owner:release/v1&base=main"
    );
}

#[test]
fn find_open_pr_without_a_match_is_none_and_errors_are_coded() {
    let server = FakeServer::start(vec![
        Reply::json(200, json!([])),
        Reply::json(500, json!({})),
        Reply::raw(200, "nope"),
    ]);
    let forge = forge(&server);

    assert_eq!(forge.find_open_pr("h", "main").unwrap(), None);
    let transport = forge.find_open_pr("h", "main").unwrap_err();
    let parse = forge.find_open_pr("h", "main").unwrap_err();

    assert_eq!(code(&transport).as_deref(), Some("E3011"));
    assert_eq!(code(&parse).as_deref(), Some("E3006"));
}

#[test]
fn update_merge_request_patches_title_and_body_and_keeps_the_id() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!({ "number": 3, "node_id": "PR_three" }),
    )]);

    let mr = forge(&server)
        .update_merge_request(3, "new title", "new body")
        .unwrap();

    assert_eq!(mr.id, 3);
    assert_eq!(mr.auto_merge_key, "PR_three");
    let req = server.only_request();
    assert_eq!(req.method, "PATCH");
    assert_eq!(req.path, "/repos/owner/repo/pulls/3");
    assert_eq!(
        req.json(),
        json!({ "title": "new title", "body": "new body" })
    );
}

#[test]
fn a_refused_pr_update_names_the_pr_and_the_reason() {
    let server = FakeServer::start(vec![Reply::json(
        403,
        json!({ "message": "Resource not accessible by integration" }),
    )]);

    let err = forge(&server)
        .update_merge_request(3, "t", "b")
        .unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3012"));
    let msg = format!("{err:#}");
    assert!(msg.contains("PR #3"), "{msg}");
    assert!(msg.contains("Resource not accessible"), "{msg}");
}

fn release_commit() -> AuthoredCommit<'static> {
    AuthoredCommit {
        branch: "main",
        expected_head_oid: "abc123",
        message: "chore(release): v1.1.0",
        additions: vec![FileAddition {
            path: "Cargo.toml".to_string(),
            base64_contents: "dmVyc2lvbg==".to_string(),
        }],
        deletions: vec![],
    }
}

#[test]
fn create_commit_on_branch_returns_the_new_oid() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!({ "data": { "createCommitOnBranch": { "commit": { "oid": "fff999" } } } }),
    )]);

    let oid = forge(&server)
        .create_commit_on_branch(&release_commit())
        .unwrap();

    assert_eq!(oid, "fff999");
    let req = server.only_request();
    assert_eq!(req.path, "/graphql");
    let body = req.json();
    assert!(
        body["query"]
            .as_str()
            .unwrap()
            .contains("createCommitOnBranch")
    );
    assert_eq!(body["variables"]["input"]["expectedHeadOid"], "abc123");
    assert_eq!(
        body["variables"]["input"]["branch"]["repositoryNameWithOwner"],
        "owner/repo"
    );
}

#[test]
fn a_stale_head_graphql_error_fails_the_commit_with_the_error_code() {
    let server = FakeServer::start(vec![Reply::json(
        200,
        json!({ "errors": [{ "message": "Expected branch to point to abc123" }] }),
    )]);

    let err = forge(&server)
        .create_commit_on_branch(&release_commit())
        .unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3014"));
    assert!(
        format!("{err:#}").contains("Expected branch to point to abc123"),
        "{err:#}"
    );
}

#[test]
fn a_commit_response_without_oid_or_json_is_rejected() {
    let server = FakeServer::start(vec![
        Reply::json(200, json!({ "data": { "createCommitOnBranch": null } })),
        Reply::raw(200, "]"),
        Reply::json(401, json!({})),
    ]);
    let forge = forge(&server);
    let commit = release_commit();

    let missing = forge.create_commit_on_branch(&commit).unwrap_err();
    let unparsable = forge.create_commit_on_branch(&commit).unwrap_err();
    let refused = forge.create_commit_on_branch(&commit).unwrap_err();

    assert_eq!(code(&missing).as_deref(), Some("E3014"));
    assert_eq!(code(&unparsable).as_deref(), Some("E3009"));
    assert_eq!(code(&refused).as_deref(), Some("E3013"));
}

#[test]
fn set_branch_force_moves_an_existing_ref_in_one_call() {
    let server = FakeServer::start(vec![Reply::json(200, json!({}))]);

    forge(&server).set_branch("release/v1", "fff999").unwrap();

    let req = server.only_request();
    assert_eq!(req.method, "PATCH");
    assert_eq!(req.path, "/repos/owner/repo/git/refs/heads/release/v1");
    assert_eq!(req.json(), json!({ "sha": "fff999", "force": true }));
}

#[test]
fn set_branch_creates_the_ref_when_it_does_not_exist() {
    let server = FakeServer::start(vec![
        Reply::json(422, json!({ "message": "Reference does not exist" })),
        Reply::json(201, json!({})),
    ]);

    forge(&server).set_branch("release/v1", "fff999").unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].method, "POST");
    assert_eq!(requests[1].path, "/repos/owner/repo/git/refs");
    assert_eq!(
        requests[1].json(),
        json!({ "ref": "refs/heads/release/v1", "sha": "fff999" })
    );
}

#[test]
fn set_branch_fails_when_neither_move_nor_create_works() {
    let server = FakeServer::start(vec![
        Reply::json(422, json!({})),
        Reply::json(403, json!({})),
    ]);

    let err = forge(&server).set_branch("main", "fff999").unwrap_err();

    assert_eq!(code(&err).as_deref(), Some("E3015"));
    assert!(format!("{err:#}").contains("'main'"), "{err:#}");
}
