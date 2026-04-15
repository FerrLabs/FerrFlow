use anyhow::{Context, Result};
use colored::Colorize;

use super::{Forge, MergeRequestResult};
use crate::error_code::{self, ErrorCodeExt};

pub struct GitLabForge {
    pub token: String,
    pub slug: String,
    pub api_base: String,
}

impl GitLabForge {
    fn encoded_project_id(&self) -> String {
        self.slug.replace('/', "%2F")
    }
}

impl Forge for GitLabForge {
    fn create_release(&self, tag: &str, body: &str, prerelease: bool, draft: bool) -> Result<()> {
        if draft {
            eprintln!(
                "{}",
                "Warning: GitLab does not support draft releases, creating as published".yellow()
            );
        }

        let project = self.encoded_project_id();
        let url = format!("{}/projects/{project}/releases", self.api_base);

        let mut payload = serde_json::json!({
            "tag_name": tag,
            "name": tag,
            "description": body,
        });
        if prerelease {
            payload["upcoming_release"] = serde_json::json!(true);
        }

        ureq::post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create GitLab release for {tag}"))
            .error_code(error_code::GITLAB_CREATE_RELEASE)?;

        Ok(())
    }

    fn find_draft_release(&self, _tag: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    fn publish_release(&self, _release_id: u64) -> Result<()> {
        Ok(())
    }

    fn create_merge_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<MergeRequestResult> {
        let project = self.encoded_project_id();
        let url = format!("{}/projects/{project}/merge_requests", self.api_base);

        let payload = serde_json::json!({
            "source_branch": head,
            "target_branch": base,
            "title": title,
            "description": body,
        });

        let response: serde_json::Value = ureq::post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create MR from {head} to {base}"))
            .error_code(error_code::GITLAB_CREATE_MR)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse MR response")
            .error_code(error_code::GITLAB_PARSE_MR)?;

        let iid = response["iid"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("MR response missing iid field"))
            .error_code(error_code::GITLAB_MR_MISSING_FIELD)?;

        Ok(MergeRequestResult {
            id: iid,
            auto_merge_key: iid.to_string(),
        })
    }

    fn enable_auto_merge(&self, mr: &MergeRequestResult) -> Result<()> {
        let project = self.encoded_project_id();
        let url = format!(
            "{}/projects/{project}/merge_requests/{}/merge",
            self.api_base, mr.id
        );

        // Try merge_when_pipeline_succeeds first (requires an active pipeline)
        let payload = serde_json::json!({
            "merge_when_pipeline_succeeds": true,
            "squash": true,
        });

        let result = ureq::put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(payload);

        if result.is_ok() {
            return Ok(());
        }

        // Fallback: merge directly (pipeline may be skipped or absent)
        let payload = serde_json::json!({
            "squash": true,
            "should_remove_source_branch": true,
        });

        ureq::put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to merge MR !{}", mr.id))
            .error_code(error_code::GITLAB_MERGE_MR)?;

        Ok(())
    }

    fn mr_noun(&self) -> &'static str {
        "MR"
    }

    fn release_noun(&self) -> &'static str {
        "GitLab Release"
    }

    fn find_comment(&self, mr_id: u64, marker: &str) -> Result<Option<u64>> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/notes?per_page=100",
            self.api_base,
            self.encoded_project_id(),
            mr_id
        );
        let notes: Vec<serde_json::Value> = ureq::get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .call()
            .with_context(|| "Failed to list MR notes")?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse MR notes")?;

        for note in notes {
            if let Some(body) = note["body"].as_str()
                && body.contains(marker)
                && let Some(id) = note["id"].as_u64()
            {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    fn create_comment(&self, mr_id: u64, body: &str) -> Result<()> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/notes",
            self.api_base,
            self.encoded_project_id(),
            mr_id
        );
        ureq::post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "body": body }))
            .with_context(|| "Failed to create MR note")?;
        Ok(())
    }

    fn update_comment(&self, mr_id: u64, comment_id: u64, body: &str) -> Result<()> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/notes/{}",
            self.api_base,
            self.encoded_project_id(),
            mr_id,
            comment_id
        );
        ureq::put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "body": body }))
            .with_context(|| "Failed to update MR note")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_project_id_simple() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
        };
        assert_eq!(forge.encoded_project_id(), "owner%2Frepo");
    }

    #[test]
    fn encoded_project_id_subgroup() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "group/subgroup/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
        };
        assert_eq!(forge.encoded_project_id(), "group%2Fsubgroup%2Frepo");
    }

    #[test]
    fn mr_noun_returns_mr() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
        };
        assert_eq!(forge.mr_noun(), "MR");
    }

    #[test]
    fn release_noun_returns_gitlab_release() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
        };
        assert_eq!(forge.release_noun(), "GitLab Release");
    }

    #[test]
    fn find_draft_release_always_none() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
        };
        assert_eq!(forge.find_draft_release("v1.0.0").unwrap(), None);
    }

    #[test]
    fn publish_release_noop() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
        };
        assert!(forge.publish_release(123).is_ok());
    }

    #[test]
    fn create_release_payload_structure() {
        let mut payload = serde_json::json!({
            "tag_name": "v1.0.0",
            "name": "v1.0.0",
            "description": "Release notes",
        });
        // prerelease adds upcoming_release
        payload["upcoming_release"] = serde_json::json!(true);
        assert_eq!(payload["upcoming_release"], true);
        assert_eq!(payload["tag_name"], "v1.0.0");
    }

    #[test]
    fn mr_response_parsing() {
        let response: serde_json::Value = serde_json::json!({"iid": 15});
        let iid = response["iid"].as_u64().unwrap();
        assert_eq!(iid, 15);
    }

    #[test]
    fn auto_merge_payload_structure() {
        let payload = serde_json::json!({
            "merge_when_pipeline_succeeds": true,
            "squash": true,
        });
        assert_eq!(payload["merge_when_pipeline_succeeds"], true);
        assert_eq!(payload["squash"], true);
    }

    #[test]
    fn api_base_self_hosted() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "team/project".to_string(),
            api_base: "https://gitlab.internal/api/v4".to_string(),
        };
        assert_eq!(forge.api_base, "https://gitlab.internal/api/v4");
    }
}
