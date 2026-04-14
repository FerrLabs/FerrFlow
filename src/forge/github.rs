use anyhow::{Context, Result};

use super::{Forge, MergeRequestResult};
use crate::error_code::{self, ErrorCodeExt};

pub struct GitHubForge {
    pub token: String,
    pub slug: String,
    pub api_base: String,
}

impl Forge for GitHubForge {
    fn create_release(&self, tag: &str, body: &str, prerelease: bool, draft: bool) -> Result<()> {
        let url = format!("{}/repos/{}/releases", self.api_base, self.slug);

        let payload = serde_json::json!({
            "tag_name": tag,
            "name": tag,
            "body": body,
            "draft": draft,
            "prerelease": prerelease,
        });

        ureq::post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create GitHub release for {tag}"))
            .error_code(error_code::GITHUB_CREATE_RELEASE)?;

        Ok(())
    }

    fn find_draft_release(&self, tag: &str) -> Result<Option<u64>> {
        let url = format!("{}/repos/{}/releases", self.api_base, self.slug);

        let response: serde_json::Value = ureq::get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .call()
            .with_context(|| "Failed to list GitHub releases")
            .error_code(error_code::GITHUB_LIST_RELEASES)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse releases response")
            .error_code(error_code::GITHUB_PARSE_RELEASES)?;

        let empty = vec![];
        let releases = response.as_array().unwrap_or(&empty);
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

        ureq::patch(&url)
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

        let response: serde_json::Value = ureq::post(&url)
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
        let response: serde_json::Value = ureq::post(&graphql_url)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_forge() -> GitHubForge {
        GitHubForge {
            token: "test-token".to_string(),
            slug: "owner/repo".to_string(),
            api_base: "https://api.github.com".to_string(),
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
        // Simulate parsing logic used in find_draft_release
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
        };
        assert_eq!(forge.api_base, "https://github.corp.com/api/v3");
    }
}
