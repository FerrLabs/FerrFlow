use anyhow::{Context, Result, bail};

use super::{Forge, MergeRequestResult, ReleaseResult};
use crate::error_code::{self, ErrorCodeExt};

const PR_UNSUPPORTED: &str = "Pull-request mode is not yet supported on Bitbucket. FerrFlow creates the release tag; \
     PR-based release flow is tracked in FerrLabs/FerrFlow#656.";

pub struct BitbucketForge {
    pub token: String,
    pub slug: String,
    pub api_base: String,
    pub is_cloud: bool,
    pub agent: ureq::Agent,
}

impl Forge for BitbucketForge {
    fn create_release(
        &self,
        tag: &str,
        _body: &str,
        _prerelease: bool,
        _draft: bool,
    ) -> Result<ReleaseResult> {
        if !self.is_cloud {
            return Ok(ReleaseResult::default());
        }

        let url = format!(
            "{}/repositories/{}/refs/tags/{tag}",
            self.api_base, self.slug
        );
        let response: serde_json::Value = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "ferrflow")
            .call()
            .with_context(|| format!("Failed to resolve Bitbucket tag {tag}"))
            .error_code(error_code::BITBUCKET_CREATE_RELEASE)?
            .body_mut()
            .read_json()
            .unwrap_or(serde_json::Value::Null);

        Ok(ReleaseResult {
            id: None,
            url: tag_html_url(&response),
        })
    }

    fn find_draft_release(&self, _tag: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    fn publish_release(&self, _release_id: u64) -> Result<()> {
        Ok(())
    }

    fn create_merge_request(
        &self,
        _head: &str,
        _base: &str,
        _title: &str,
        _body: &str,
    ) -> Result<MergeRequestResult> {
        bail!("{PR_UNSUPPORTED}")
    }

    fn enable_auto_merge(&self, _mr: &MergeRequestResult) -> Result<()> {
        bail!("Auto-merge is not yet supported on Bitbucket (PR mode). See FerrLabs/FerrFlow#656.")
    }

    fn mr_noun(&self) -> &'static str {
        "PR"
    }

    fn release_noun(&self) -> &'static str {
        "Bitbucket tag"
    }

    fn find_comment(&self, _pr_id: u64, _marker: &str) -> Result<Option<u64>> {
        bail!("{PR_UNSUPPORTED}")
    }

    fn create_comment(&self, _pr_id: u64, _body: &str) -> Result<()> {
        bail!("{PR_UNSUPPORTED}")
    }

    fn update_comment(&self, _pr_id: u64, _comment_id: u64, _body: &str) -> Result<()> {
        bail!("{PR_UNSUPPORTED}")
    }

    fn find_open_pr(&self, _head: &str, _base: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    fn update_merge_request(
        &self,
        _id: u64,
        _title: &str,
        _body: &str,
    ) -> Result<MergeRequestResult> {
        bail!("{PR_UNSUPPORTED}")
    }
}

fn tag_html_url(response: &serde_json::Value) -> Option<String> {
    response["links"]["html"]["href"]
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_forge(is_cloud: bool) -> BitbucketForge {
        BitbucketForge {
            token: "test-token".to_string(),
            slug: "workspace/repo".to_string(),
            api_base: if is_cloud {
                "https://api.bitbucket.org/2.0".to_string()
            } else {
                "https://bitbucket.example.com/rest/api/1.0".to_string()
            },
            is_cloud,
            agent: ureq::Agent::new_with_defaults(),
        }
    }

    #[test]
    fn release_noun_is_bitbucket_tag() {
        assert_eq!(make_forge(true).release_noun(), "Bitbucket tag");
    }

    #[test]
    fn extracts_the_canonical_tag_url_from_the_api_shape() {
        let response = serde_json::json!({
            "name": "v1.0.0",
            "type": "tag",
            "links": {
                "html": { "href": "https://bitbucket.org/workspace/repo/commits/tag/v1.0.0" },
                "self": { "href": "https://api.bitbucket.org/2.0/repositories/workspace/repo/refs/tags/v1.0.0" }
            }
        });
        assert_eq!(
            tag_html_url(&response).as_deref(),
            Some("https://bitbucket.org/workspace/repo/commits/tag/v1.0.0")
        );
        assert_eq!(tag_html_url(&serde_json::Value::Null), None);
    }

    #[test]
    fn server_release_is_tag_only_without_a_network_call() {
        let result = make_forge(false)
            .create_release("v1.0.0", "notes", false, false)
            .unwrap();
        assert_eq!(result.id, None);
        assert_eq!(result.url, None);
    }

    #[test]
    fn drafts_are_a_noop() {
        let forge = make_forge(true);
        assert_eq!(forge.find_draft_release("v1.0.0").unwrap(), None);
        assert!(forge.publish_release(1).is_ok());
    }

    #[test]
    fn pr_mode_and_comments_are_not_supported() {
        let forge = make_forge(true);
        assert!(forge.create_merge_request("h", "b", "t", "body").is_err());
        assert!(
            forge
                .enable_auto_merge(&MergeRequestResult {
                    id: 1,
                    auto_merge_key: String::new(),
                })
                .is_err()
        );
        assert!(forge.find_comment(1, "marker").is_err());
        assert!(forge.create_comment(1, "body").is_err());
        assert!(forge.update_comment(1, 2, "body").is_err());
    }
}
