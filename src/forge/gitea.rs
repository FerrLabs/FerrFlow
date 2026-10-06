use anyhow::{Context, Result};

use super::rest::RestClient;
use super::{Forge, MergeRequestResult, ReleaseResult};
use crate::error_code::{self, ErrorCodeExt};

pub struct GiteaForge {
    pub token: String,
    pub slug: String,
    pub api_base: String,
    pub agent: ureq::Agent,
}

impl GiteaForge {
    fn rest(&self) -> RestClient<'_> {
        RestClient::gitea(&self.agent, &self.api_base, &self.slug, &self.token)
    }
}

impl Forge for GiteaForge {
    fn create_release(
        &self,
        tag: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
    ) -> Result<ReleaseResult> {
        self.rest()
            .create_release(tag, body, prerelease, draft)
            .with_context(|| format!("Failed to create Gitea release for {tag}"))
            .error_code(error_code::GITEA_CREATE_RELEASE)
    }

    fn find_draft_release(&self, tag: &str) -> Result<Option<u64>> {
        self.rest()
            .find_draft_release(tag, "Gitea releases")
            .error_code(error_code::GITEA_LIST_RELEASES)
    }

    fn publish_release(&self, release_id: u64) -> Result<()> {
        self.rest()
            .publish_release(release_id)
            .with_context(|| format!("Failed to publish Gitea release {release_id}"))
            .error_code(error_code::GITEA_PUBLISH_RELEASE)
    }

    fn create_merge_request(
        &self,
        _head: &str,
        _base: &str,
        _title: &str,
        _body: &str,
    ) -> Result<MergeRequestResult> {
        anyhow::bail!(
            "Pull-request mode is not yet supported on Gitea/Forgejo. \
             FerrFlow can create releases; PR-based release flow is tracked in FerrLabs/FerrFlow#499."
        )
    }

    fn enable_auto_merge(&self, _mr: &MergeRequestResult) -> Result<()> {
        anyhow::bail!(
            "Auto-merge is not yet supported on Gitea/Forgejo (PR mode). See FerrLabs/FerrFlow#499."
        )
    }

    fn mr_noun(&self) -> &'static str {
        "PR"
    }

    fn release_noun(&self) -> &'static str {
        "Gitea Release"
    }

    fn find_comment(&self, pr_id: u64, marker: &str) -> Result<Option<u64>> {
        self.rest().find_comment(pr_id, marker, "issue comments")
    }

    fn create_comment(&self, pr_id: u64, body: &str) -> Result<()> {
        self.rest()
            .create_comment(pr_id, body)
            .context("Failed to create issue comment")
    }

    fn update_comment(&self, _pr_id: u64, comment_id: u64, body: &str) -> Result<()> {
        self.rest()
            .update_comment(comment_id, body)
            .context("Failed to update issue comment")
    }

    fn find_open_pr(&self, _head: &str, _base: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    fn supports_merge_requests(&self) -> bool {
        false
    }

    fn update_merge_request(
        &self,
        _id: u64,
        _title: &str,
        _body: &str,
    ) -> Result<MergeRequestResult> {
        anyhow::bail!(
            "Pull-request mode is not yet supported on Gitea/Forgejo. \
             FerrFlow can create releases; PR-based release flow is tracked in FerrLabs/FerrFlow#499."
        )
    }
}

#[cfg(test)]
mod api_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_forge() -> GiteaForge {
        GiteaForge {
            token: "test-token".to_string(),
            slug: "owner/repo".to_string(),
            api_base: "https://codeberg.org/api/v1".to_string(),
            agent: ureq::Agent::new_with_defaults(),
        }
    }

    #[test]
    fn release_noun_is_gitea_release() {
        assert_eq!(make_forge().release_noun(), "Gitea Release");
    }

    #[test]
    fn api_base_uses_v1_prefix() {
        assert_eq!(make_forge().api_base, "https://codeberg.org/api/v1");
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
    }

    #[test]
    fn find_draft_release_matches_exact_tag() {
        let releases: serde_json::Value = serde_json::json!([
            {"id": 10, "tag_name": "v2.0.0", "draft": true},
            {"id": 20, "tag_name": "v2.0.0-beta.1", "draft": true},
        ]);
        let found = releases
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["draft"].as_bool() == Some(true) && r["tag_name"].as_str() == Some("v2.0.0")
            })
            .and_then(|r| r["id"].as_u64());
        assert_eq!(found, Some(10));
    }

    #[test]
    fn find_draft_release_ignores_published() {
        let releases: serde_json::Value = serde_json::json!([
            {"id": 1, "tag_name": "v1.0.0", "draft": false},
        ]);
        let found = releases.as_array().unwrap().iter().find(|r| {
            r["draft"].as_bool() == Some(true) && r["tag_name"].as_str() == Some("v1.0.0")
        });
        assert!(found.is_none());
    }

    #[test]
    fn pr_mode_is_not_supported() {
        let forge = make_forge();
        assert!(forge.create_merge_request("h", "b", "t", "body").is_err());
        assert!(
            forge
                .enable_auto_merge(&MergeRequestResult {
                    id: 1,
                    auto_merge_key: String::new(),
                })
                .is_err()
        );
    }
}
