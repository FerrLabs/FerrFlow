use anyhow::{Context, Result};

use super::{AuthoredCommit, Forge, MergeRequestResult, ReleaseResult};
use crate::error_code::{self, ErrorCodeExt};

const PER_PAGE: u32 = 100;

const MAX_PAGES: u32 = 100;

pub struct GitHubForge {
    pub token: String,
    pub slug: String,
    pub api_base: String,
    pub agent: ureq::Agent,
}

impl GitHubForge {
    fn paginated_json_array(&self, base_url: &str, what: &str) -> Result<Vec<serde_json::Value>> {
        let mut all = Vec::new();
        for page in 1..=MAX_PAGES {
            let url = format!("{base_url}?per_page={PER_PAGE}&page={page}");
            let body: serde_json::Value = self
                .agent
                .get(&url)
                .header("Authorization", &format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "ferrflow")
                .call()
                .with_context(|| format!("Failed to list {what}"))?
                .body_mut()
                .read_json()
                .with_context(|| format!("Failed to parse {what} response"))?;
            let page_items = match body.as_array() {
                Some(arr) if !arr.is_empty() => arr.clone(),
                _ => return Ok(all),
            };
            let len = page_items.len();
            all.extend(page_items);
            if (len as u32) < PER_PAGE {
                return Ok(all);
            }
        }
        Ok(all)
    }
}

const CREATE_COMMIT_MUTATION: &str = "\
mutation($input: CreateCommitOnBranchInput!) { \
  createCommitOnBranch(input: $input) { commit { oid } } \
}";

impl GitHubForge {
    fn graphql(&self, body: serde_json::Value, what: &str) -> Result<serde_json::Value> {
        let url = format!("{}/graphql", self.api_base);
        let response: serde_json::Value = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(body)
            .with_context(|| format!("{what} failed"))
            .error_code(error_code::GITHUB_GRAPHQL_REQUEST)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse GraphQL response")
            .error_code(error_code::GITHUB_GRAPHQL_PARSE)?;

        if let Some(message) = graphql_error_message(&response) {
            return Err(anyhow::anyhow!("{what} failed: {message}"))
                .error_code(error_code::GITHUB_GRAPHQL_ERROR);
        }
        Ok(response)
    }
}

pub(super) fn graphql_error_message(response: &serde_json::Value) -> Option<String> {
    let errors = response.get("errors")?.as_array()?;
    let first = errors.first()?;
    Some(
        first["message"]
            .as_str()
            .unwrap_or("unknown GraphQL error")
            .to_string(),
    )
}

pub(super) fn build_commit_input(
    slug: &str,
    commit: &crate::forge::AuthoredCommit<'_>,
) -> serde_json::Value {
    let additions: Vec<serde_json::Value> = commit
        .additions
        .iter()
        .map(|a| serde_json::json!({ "path": a.path, "contents": a.base64_contents }))
        .collect();
    let deletions: Vec<serde_json::Value> = commit
        .deletions
        .iter()
        .map(|path| serde_json::json!({ "path": path }))
        .collect();

    let (headline, body) = split_commit_message(commit.message);
    let mut message = serde_json::json!({ "headline": headline });
    if let Some(body) = body {
        message["body"] = serde_json::Value::String(body);
    }

    serde_json::json!({
        "branch": {
            "repositoryNameWithOwner": slug,
            "branchName": commit.branch,
        },
        "expectedHeadOid": commit.expected_head_oid,
        "message": message,
        "fileChanges": {
            "additions": additions,
            "deletions": deletions,
        },
    })
}

fn split_commit_message(message: &str) -> (String, Option<String>) {
    match message.split_once('\n') {
        Some((headline, rest)) => {
            let body = rest.trim_start_matches('\n').trim_end();
            let body = (!body.is_empty()).then(|| body.to_string());
            (headline.trim_end().to_string(), body)
        }
        None => (message.trim_end().to_string(), None),
    }
}

impl Forge for GitHubForge {
    fn authors_verified_commits(&self) -> bool {
        true
    }

    fn create_commit_on_branch(&self, commit: &AuthoredCommit<'_>) -> Result<String> {
        let input = build_commit_input(&self.slug, commit);
        let response = self.graphql(
            serde_json::json!({
                "query": CREATE_COMMIT_MUTATION,
                "variables": { "input": input },
            }),
            "createCommitOnBranch",
        )?;

        response["data"]["createCommitOnBranch"]["commit"]["oid"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("createCommitOnBranch returned no commit oid"))
            .error_code(error_code::GITHUB_GRAPHQL_ERROR)
    }

    fn set_branch(&self, branch: &str, oid: &str) -> Result<()> {
        let url = format!(
            "{}/repos/{}/git/refs/heads/{branch}",
            self.api_base, self.slug
        );
        let patched = self
            .agent
            .patch(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "sha": oid, "force": true }));

        match patched {
            Ok(_) => Ok(()),
            Err(_) => {
                let create_url = format!("{}/repos/{}/git/refs", self.api_base, self.slug);
                self.agent
                    .post(&create_url)
                    .header("Authorization", &format!("Bearer {}", self.token))
                    .header("User-Agent", "ferrflow")
                    .send_json(serde_json::json!({
                        "ref": format!("refs/heads/{branch}"),
                        "sha": oid,
                    }))
                    .map(|_| ())
                    .with_context(|| format!("Failed to point branch '{branch}' at {oid}"))
                    .error_code(error_code::GITHUB_SET_BRANCH)
            }
        }
    }
    fn create_release(
        &self,
        tag: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
    ) -> Result<ReleaseResult> {
        let url = format!("{}/repos/{}/releases", self.api_base, self.slug);

        let payload = serde_json::json!({
            "tag_name": tag,
            "name": tag,
            "body": body,
            "draft": draft,
            "prerelease": prerelease,
        });
        let response: serde_json::Value = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create GitHub release for {tag}"))
            .error_code(error_code::GITHUB_CREATE_RELEASE)?
            .body_mut()
            .read_json()
            .unwrap_or(serde_json::Value::Null);

        Ok(ReleaseResult {
            id: response["id"].as_u64(),
            url: response["html_url"].as_str().map(str::to_string),
        })
    }

    fn delete_release(&self, id: u64) -> Result<()> {
        let url = format!("{}/repos/{}/releases/{id}", self.api_base, self.slug);
        self.agent
            .delete(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "ferrflow")
            .call()
            .map(|_| ())
            .with_context(|| format!("Failed to delete release #{id}"))
            .error_code(error_code::GITHUB_DELETE_RELEASE)
    }

    fn find_draft_release(&self, tag: &str) -> Result<Option<u64>> {
        let base_url = format!("{}/repos/{}/releases", self.api_base, self.slug);
        let releases = self
            .paginated_json_array(&base_url, "GitHub releases")
            .error_code(error_code::GITHUB_LIST_RELEASES)?;
        for release in releases {
            if release["draft"].as_bool() == Some(true)
                && release["tag_name"].as_str() == Some(tag)
                && let Some(id) = release["id"].as_u64()
            {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    fn publish_release(&self, release_id: u64) -> Result<()> {
        let url = format!(
            "{}/repos/{}/releases/{release_id}",
            self.api_base, self.slug
        );

        let payload = serde_json::json!({
            "draft": false,
        });

        self.agent
            .patch(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to publish GitHub release {release_id}"))
            .error_code(error_code::GITHUB_PUBLISH_RELEASE)?;

        Ok(())
    }

    fn create_merge_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<MergeRequestResult> {
        let url = format!("{}/repos/{}/pulls", self.api_base, self.slug);

        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base,
        });

        let response: serde_json::Value = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create PR from {head} to {base}"))
            .error_code(error_code::GITHUB_CREATE_PR)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse PR response")
            .error_code(error_code::GITHUB_PARSE_PR)?;

        let number = response["number"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("PR response missing number field"))
            .error_code(error_code::GITHUB_PR_MISSING_FIELD)?;

        let node_id = response["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("PR response missing node_id field"))
            .error_code(error_code::GITHUB_PR_MISSING_FIELD)?
            .to_string();

        Ok(MergeRequestResult {
            id: number,
            auto_merge_key: node_id,
        })
    }

    fn enable_auto_merge(&self, mr: &MergeRequestResult) -> Result<()> {
        let query = serde_json::json!({
            "query": "mutation($prId: ID!) { enablePullRequestAutoMerge(input: { pullRequestId: $prId, mergeMethod: SQUASH }) { pullRequest { number } } }",
            "variables": { "prId": mr.auto_merge_key },
        });

        let graphql_url = format!("{}/graphql", self.api_base);
        let response: serde_json::Value = self
            .agent
            .post(&graphql_url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(query)
            .with_context(|| format!("Failed to enable auto-merge on PR #{}", mr.id))
            .error_code(error_code::GITHUB_AUTO_MERGE)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse GraphQL response")
            .error_code(error_code::GITHUB_GRAPHQL_PARSE)?;

        if let Some(errors) = response.get("errors") {
            let msg = errors[0]["message"]
                .as_str()
                .unwrap_or("unknown GraphQL error");
            return Err(anyhow::anyhow!("Auto-merge failed on PR #{}: {msg}", mr.id))
                .error_code(error_code::GITHUB_AUTO_MERGE_FAILED);
        }

        Ok(())
    }

    fn mr_noun(&self) -> &'static str {
        "PR"
    }

    fn release_noun(&self) -> &'static str {
        "GitHub Release"
    }

    fn find_comment(&self, pr_id: u64, marker: &str) -> Result<Option<u64>> {
        let base_url = format!(
            "{}/repos/{}/issues/{}/comments",
            self.api_base, self.slug, pr_id
        );
        let comments = self.paginated_json_array(&base_url, "PR comments")?;
        for comment in comments {
            if let Some(body) = comment["body"].as_str()
                && body.contains(marker)
                && let Some(id) = comment["id"].as_u64()
            {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    fn create_comment(&self, pr_id: u64, body: &str) -> Result<()> {
        let url = format!(
            "{}/repos/{}/issues/{}/comments",
            self.api_base, self.slug, pr_id
        );
        self.agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "body": body }))
            .with_context(|| "Failed to create PR comment")?;
        Ok(())
    }

    fn update_comment(&self, _pr_id: u64, comment_id: u64, body: &str) -> Result<()> {
        let url = format!(
            "{}/repos/{}/issues/comments/{}",
            self.api_base, self.slug, comment_id
        );
        self.agent
            .patch(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "body": body }))
            .with_context(|| "Failed to update PR comment")?;
        Ok(())
    }

    fn find_open_pr(&self, head: &str, base: &str) -> Result<Option<u64>> {
        let owner = self.slug.split('/').next().unwrap_or_default();
        let url = format!(
            "{}/repos/{}/pulls?state=open&head={}:{}&base={}",
            self.api_base, self.slug, owner, head, base
        );
        let response: serde_json::Value = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .call()
            .with_context(|| format!("Failed to list open PRs for {head}"))
            .error_code(error_code::GITHUB_FIND_PR)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse PR list response")
            .error_code(error_code::GITHUB_PARSE_PR)?;

        Ok(response
            .as_array()
            .and_then(|prs| prs.first())
            .and_then(|pr| pr["number"].as_u64()))
    }

    fn update_merge_request(&self, id: u64, title: &str, body: &str) -> Result<MergeRequestResult> {
        let url = format!("{}/repos/{}/pulls/{}", self.api_base, self.slug, id);
        let response: serde_json::Value = self
            .agent
            .patch(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "title": title, "body": body }))
            .with_context(|| format!("Failed to update PR #{id}"))
            .error_code(error_code::GITHUB_UPDATE_PR)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse PR update response")
            .error_code(error_code::GITHUB_PARSE_PR)?;

        let node_id = response["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("PR response missing node_id field"))
            .error_code(error_code::GITHUB_PR_MISSING_FIELD)?
            .to_string();

        Ok(MergeRequestResult {
            id,
            auto_merge_key: node_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_forge() -> GitHubForge {
        GitHubForge {
            token: "test-token".to_string(),
            slug: "owner/repo".to_string(),
            api_base: "https://api.github.com".to_string(),
            agent: ureq::Agent::new_with_defaults(),
        }
    }

    #[test]
    fn mr_noun_returns_pr() {
        assert_eq!(make_forge().mr_noun(), "PR");
    }

    #[test]
    fn release_noun_returns_github_release() {
        assert_eq!(make_forge().release_noun(), "GitHub Release");
    }

    #[test]
    fn struct_fields_accessible() {
        let forge = make_forge();
        assert_eq!(forge.token, "test-token");
        assert_eq!(forge.slug, "owner/repo");
    }

    #[test]
    fn find_draft_release_parses_empty_array() {
        let response: serde_json::Value = serde_json::json!([]);
        let releases = response.as_array().unwrap();
        let found = releases.iter().find(|r| {
            r["draft"].as_bool() == Some(true) && r["tag_name"].as_str() == Some("v1.0.0")
        });
        assert!(found.is_none());
    }

    #[test]
    fn find_draft_release_parses_draft() {
        let response: serde_json::Value = serde_json::json!([
            {"id": 1, "tag_name": "v1.0.0", "draft": false},
            {"id": 2, "tag_name": "v1.1.0", "draft": true},
            {"id": 3, "tag_name": "v1.2.0", "draft": true},
        ]);
        let releases = response.as_array().unwrap();
        let found = releases
            .iter()
            .find(|r| {
                r["draft"].as_bool() == Some(true) && r["tag_name"].as_str() == Some("v1.1.0")
            })
            .and_then(|r| r["id"].as_u64());
        assert_eq!(found, Some(2));
    }

    #[test]
    fn find_draft_release_ignores_non_draft() {
        let response: serde_json::Value = serde_json::json!([
            {"id": 1, "tag_name": "v1.0.0", "draft": false},
        ]);
        let releases = response.as_array().unwrap();
        let found = releases
            .iter()
            .find(|r| {
                r["draft"].as_bool() == Some(true) && r["tag_name"].as_str() == Some("v1.0.0")
            })
            .and_then(|r| r["id"].as_u64());
        assert!(found.is_none());
    }

    #[test]
    fn find_draft_release_matches_exact_tag() {
        let response: serde_json::Value = serde_json::json!([
            {"id": 10, "tag_name": "v2.0.0", "draft": true},
            {"id": 20, "tag_name": "v2.0.0-beta.1", "draft": true},
        ]);
        let releases = response.as_array().unwrap();
        let found = releases
            .iter()
            .find(|r| {
                r["draft"].as_bool() == Some(true) && r["tag_name"].as_str() == Some("v2.0.0")
            })
            .and_then(|r| r["id"].as_u64());
        assert_eq!(found, Some(10));
    }

    #[test]
    fn create_release_payload_structure() {
        let payload = serde_json::json!({
            "tag_name": "v1.0.0",
            "name": "v1.0.0",
            "body": "Release notes",
            "draft": true,
            "prerelease": false,
        });
        assert_eq!(payload["tag_name"], "v1.0.0");
        assert_eq!(payload["draft"], true);
        assert_eq!(payload["prerelease"], false);
        assert_eq!(payload["body"], "Release notes");
    }

    #[test]
    fn publish_release_payload_structure() {
        let payload = serde_json::json!({"draft": false});
        assert_eq!(payload["draft"], false);
    }

    #[test]
    fn create_pr_payload_structure() {
        let payload = serde_json::json!({
            "title": "chore(release): v1.0.0",
            "body": "Release PR",
            "head": "release/v1.0.0",
            "base": "main",
        });
        assert_eq!(payload["head"], "release/v1.0.0");
        assert_eq!(payload["base"], "main");
    }

    fn authored<'a>(
        branch: &'a str,
        message: &'a str,
        additions: Vec<(&str, &str)>,
        deletions: Vec<&str>,
    ) -> AuthoredCommit<'a> {
        AuthoredCommit {
            branch,
            expected_head_oid: "deadbeef",
            message,
            additions: additions
                .into_iter()
                .map(|(path, contents)| crate::forge::FileAddition {
                    path: path.to_string(),
                    base64_contents: contents.to_string(),
                })
                .collect(),
            deletions: deletions.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn the_commit_input_carries_no_author_committer_or_signature() {
        // GitHub only signs a bot's commit when the request has none of these,
        // so their absence is the whole point rather than an omission.
        let commit = authored("main", "chore(release): v1.1.0", vec![], vec![]);
        let input = build_commit_input("owner/repo", &commit);

        let rendered = input.to_string();
        for forbidden in ["author", "committer", "signature"] {
            assert!(
                !rendered.contains(forbidden),
                "`{forbidden}` in the input would cost us the verified mark: {rendered}"
            );
        }
    }

    #[test]
    fn the_commit_input_targets_the_branch_and_pins_the_head() {
        let commit = authored(
            "ferrflow/release-main",
            "chore(release): v1.1.0",
            vec![],
            vec![],
        );
        let input = build_commit_input("FerrLabs/FerrFlow", &commit);

        assert_eq!(
            input["branch"]["repositoryNameWithOwner"],
            "FerrLabs/FerrFlow"
        );
        assert_eq!(input["branch"]["branchName"], "ferrflow/release-main");
        assert_eq!(input["expectedHeadOid"], "deadbeef");
    }

    #[test]
    fn additions_and_deletions_both_reach_the_payload() {
        let commit = authored(
            "main",
            "chore(release): v1.1.0",
            vec![("Cargo.toml", "dmVyc2lvbg==")],
            vec!["old.txt"],
        );
        let input = build_commit_input("owner/repo", &commit);

        assert_eq!(input["fileChanges"]["additions"][0]["path"], "Cargo.toml");
        assert_eq!(
            input["fileChanges"]["additions"][0]["contents"],
            "dmVyc2lvbg=="
        );
        assert_eq!(input["fileChanges"]["deletions"][0]["path"], "old.txt");
    }

    #[test]
    fn a_single_line_message_has_no_body() {
        let commit = authored("main", "chore(release): v1.1.0", vec![], vec![]);
        let input = build_commit_input("owner/repo", &commit);

        assert_eq!(input["message"]["headline"], "chore(release): v1.1.0");
        assert!(input["message"].get("body").is_none());
    }

    #[test]
    fn a_message_body_is_split_off_the_headline() {
        let commit = authored(
            "main",
            "chore(release): v1.1.0\n\n- app 1.1.0 (3 commits)\n- lib 2.0.0 (1 commit)",
            vec![],
            vec![],
        );
        let input = build_commit_input("owner/repo", &commit);

        assert_eq!(input["message"]["headline"], "chore(release): v1.1.0");
        assert_eq!(
            input["message"]["body"],
            "- app 1.1.0 (3 commits)\n- lib 2.0.0 (1 commit)"
        );
    }

    #[test]
    fn a_graphql_error_is_surfaced_rather_than_swallowed() {
        let response = serde_json::json!({
            "errors": [{ "message": "Expected branch head to be deadbeef" }]
        });

        assert_eq!(
            graphql_error_message(&response).as_deref(),
            Some("Expected branch head to be deadbeef")
        );
        assert!(graphql_error_message(&serde_json::json!({ "data": {} })).is_none());
    }

    #[test]
    fn auto_merge_graphql_payload() {
        let query = serde_json::json!({
            "query": "mutation($prId: ID!) { enablePullRequestAutoMerge(input: { pullRequestId: $prId, mergeMethod: SQUASH }) { pullRequest { number } } }",
            "variables": { "prId": "PR_abc123" },
        });
        assert!(
            query["query"]
                .as_str()
                .unwrap()
                .contains("enablePullRequestAutoMerge")
        );
        assert_eq!(query["variables"]["prId"], "PR_abc123");
    }

    #[test]
    fn graphql_error_detection() {
        let response: serde_json::Value = serde_json::json!({
            "errors": [{"message": "Some error"}]
        });
        let errors = response.get("errors");
        assert!(errors.is_some());
        let msg = errors.unwrap()[0]["message"].as_str().unwrap();
        assert_eq!(msg, "Some error");
    }

    #[test]
    fn graphql_no_errors() {
        let response: serde_json::Value = serde_json::json!({
            "data": {"enablePullRequestAutoMerge": {"pullRequest": {"number": 42}}}
        });
        assert!(response.get("errors").is_none());
    }

    #[test]
    fn pr_response_parsing() {
        let response: serde_json::Value = serde_json::json!({
            "number": 42,
            "node_id": "PR_kwDOabc123"
        });
        let number = response["number"].as_u64().unwrap();
        let node_id = response["node_id"].as_str().unwrap();
        assert_eq!(number, 42);
        assert_eq!(node_id, "PR_kwDOabc123");
    }

    #[test]
    fn pr_response_missing_number() {
        let response: serde_json::Value = serde_json::json!({"node_id": "PR_abc"});
        assert!(response["number"].as_u64().is_none());
    }

    #[test]
    fn pr_response_missing_node_id() {
        let response: serde_json::Value = serde_json::json!({"number": 1});
        assert!(response["node_id"].as_str().is_none());
    }

    #[test]
    fn api_base_github_com() {
        let forge = make_forge();
        assert_eq!(forge.api_base, "https://api.github.com");
    }

    #[test]
    fn api_base_github_enterprise() {
        let forge = GitHubForge {
            token: "tok".to_string(),
            slug: "owner/repo".to_string(),
            api_base: "https://github.corp.com/api/v3".to_string(),
            agent: ureq::Agent::new_with_defaults(),
        };
        assert_eq!(forge.api_base, "https://github.corp.com/api/v3");
    }
}
