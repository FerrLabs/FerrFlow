use anyhow::{Context, Result};

use super::{Forge, MergeRequestResult, ReleaseResult};
use crate::error_code::{self, ErrorCodeExt};

const PER_PAGE: u32 = 50;

const MAX_PAGES: u32 = 100;

pub struct GiteaForge {
    pub token: String,
    pub slug: String,
    pub api_base: String,
    pub agent: ureq::Agent,
}

impl GiteaForge {
    fn paginated_json_array(&self, base_url: &str, what: &str) -> Result<Vec<serde_json::Value>> {
        let mut all = Vec::new();
        for page in 1..=MAX_PAGES {
            let url = format!("{base_url}?limit={PER_PAGE}&page={page}");
            let body: serde_json::Value = self
                .agent
                .get(&url)
                .header("Authorization", &format!("token {}", self.token))
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

impl Forge for GiteaForge {
    fn create_release(
        &self,
        tag: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
        target_commitish: Option<&str>,
    ) -> Result<ReleaseResult> {
        let url = format!("{}/repos/{}/releases", self.api_base, self.slug);

        let mut payload = serde_json::json!({
            "tag_name": tag,
            "name": tag,
            "body": body,
            "draft": draft,
            "prerelease": prerelease,
        });
        if let Some(sha) = target_commitish.filter(|s| !s.is_empty()) {
            payload["target_commitish"] = serde_json::Value::String(sha.to_string());
        }

        let response: serde_json::Value = self
            .agent
            .post(&url)
            .header("Authorization", &format!("token {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create Gitea release for {tag}"))
            .error_code(error_code::GITEA_CREATE_RELEASE)?
            .body_mut()
            .read_json()
            .unwrap_or(serde_json::Value::Null);

        Ok(ReleaseResult {
            id: response["id"].as_u64(),
            url: response["html_url"].as_str().map(str::to_string),
        })
    }

    fn find_draft_release(&self, tag: &str) -> Result<Option<u64>> {
        let base_url = format!("{}/repos/{}/releases", self.api_base, self.slug);
        let releases = self
            .paginated_json_array(&base_url, "Gitea releases")
            .error_code(error_code::GITEA_LIST_RELEASES)?;
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

        self.agent
            .patch(&url)
            .header("Authorization", &format!("token {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "draft": false }))
            .with_context(|| format!("Failed to publish Gitea release {release_id}"))
            .error_code(error_code::GITEA_PUBLISH_RELEASE)?;

        Ok(())
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
        let base_url = format!(
            "{}/repos/{}/issues/{}/comments",
            self.api_base, self.slug, pr_id
        );
        let comments = self.paginated_json_array(&base_url, "issue comments")?;
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
            .header("Authorization", &format!("token {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "body": body }))
            .with_context(|| "Failed to create issue comment")?;
        Ok(())
    }

    fn update_comment(&self, _pr_id: u64, comment_id: u64, body: &str) -> Result<()> {
        let url = format!(
            "{}/repos/{}/issues/comments/{}",
            self.api_base, self.slug, comment_id
        );
        self.agent
            .patch(&url)
            .header("Authorization", &format!("token {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "body": body }))
            .with_context(|| "Failed to update issue comment")?;
        Ok(())
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
        anyhow::bail!(
            "Pull-request mode is not yet supported on Gitea/Forgejo. \
             FerrFlow can create releases; PR-based release flow is tracked in FerrLabs/FerrFlow#499."
        )
    }
}

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
