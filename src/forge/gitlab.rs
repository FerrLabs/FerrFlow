use anyhow::{Context, Result};
use colored::Colorize;

use super::{Forge, MergeRequestResult, ReleaseResult};
use crate::error_code::{self, ErrorCodeExt};

/// See `github::PER_PAGE` — same justification, same number.
const PER_PAGE: u32 = 100;
const MAX_PAGES: u32 = 100;

pub struct GitLabForge {
    pub token: String,
    pub slug: String,
    pub api_base: String,
    pub agent: ureq::Agent,
}

impl GitLabForge {
    fn encoded_project_id(&self) -> String {
        self.slug.replace('/', "%2F")
    }

    /// Walk every page of a GitLab list endpoint. Same shape as the
    /// GitHub helper: stop when a page returns fewer than `PER_PAGE`
    /// items. GitLab supports the `?per_page=&page=` parameters
    /// identically to GitHub, so the universal trick works. See #524.
    fn paginated_json_array(&self, base_url: &str, what: &str) -> Result<Vec<serde_json::Value>> {
        let mut all = Vec::new();
        for page in 1..=MAX_PAGES {
            let url = format!("{base_url}?per_page={PER_PAGE}&page={page}");
            let body: serde_json::Value = self
                .agent
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
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

impl Forge for GitLabForge {
    fn create_release(
        &self,
        tag: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
    ) -> Result<ReleaseResult> {
        if draft {
            tracing::warn!(
                "{}",
                "Warning: --draft is not supported on GitLab. The release will be created as \
                 published immediately. Use a tag-protected branch or a manual approval gate if \
                 you need a review step before publishing."
                    .yellow()
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

        let response: serde_json::Value = self
            .agent
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create GitLab release for {tag}"))
            .error_code(error_code::GITLAB_CREATE_RELEASE)?
            .body_mut()
            .read_json()
            .unwrap_or(serde_json::Value::Null);

        Ok(ReleaseResult {
            id: None,
            url: response["_links"]["self"].as_str().map(str::to_string),
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

        let response: serde_json::Value = self
            .agent
            .post(&url)
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

        let payload = serde_json::json!({
            "merge_when_pipeline_succeeds": true,
            "squash": true,
        });

        let result = self
            .agent
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(payload);

        if result.is_ok() {
            return Ok(());
        }

        let payload = serde_json::json!({
            "squash": true,
            "should_remove_source_branch": true,
        });

        self.agent
            .put(&url)
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
        let base_url = format!(
            "{}/projects/{}/merge_requests/{}/notes",
            self.api_base,
            self.encoded_project_id(),
            mr_id
        );
        // Paginate — long-lived release MRs can accumulate hundreds of
        // notes from CI / bots / reviewers. See #524.
        let notes = self.paginated_json_array(&base_url, "MR notes")?;
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
        self.agent
            .post(&url)
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
        self.agent
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "body": body }))
            .with_context(|| "Failed to update MR note")?;
        Ok(())
    }

    fn find_open_pr(&self, head: &str, base: &str) -> Result<Option<u64>> {
        let project = self.encoded_project_id();
        let url = format!(
            "{}/projects/{project}/merge_requests?state=opened&source_branch={head}&target_branch={base}",
            self.api_base
        );
        let response: serde_json::Value = self
            .agent
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .call()
            .with_context(|| format!("Failed to list open MRs for {head}"))
            .error_code(error_code::GITLAB_FIND_MR)?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse MR list response")
            .error_code(error_code::GITLAB_PARSE_MR)?;

        Ok(response
            .as_array()
            .and_then(|mrs| mrs.first())
            .and_then(|mr| mr["iid"].as_u64()))
    }

    fn update_merge_request(&self, id: u64, title: &str, body: &str) -> Result<MergeRequestResult> {
        let project = self.encoded_project_id();
        let url = format!("{}/projects/{project}/merge_requests/{id}", self.api_base);
        self.agent
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(serde_json::json!({ "title": title, "description": body }))
            .with_context(|| format!("Failed to update MR !{id}"))
            .error_code(error_code::GITLAB_UPDATE_MR)?;

        Ok(MergeRequestResult {
            id,
            auto_merge_key: id.to_string(),
        })
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
            agent: ureq::Agent::new_with_defaults(),
        };
        assert_eq!(forge.encoded_project_id(), "owner%2Frepo");
    }

    #[test]
    fn encoded_project_id_subgroup() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "group/subgroup/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
            agent: ureq::Agent::new_with_defaults(),
        };
        assert_eq!(forge.encoded_project_id(), "group%2Fsubgroup%2Frepo");
    }

    #[test]
    fn mr_noun_returns_mr() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
            agent: ureq::Agent::new_with_defaults(),
        };
        assert_eq!(forge.mr_noun(), "MR");
    }

    #[test]
    fn release_noun_returns_gitlab_release() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
            agent: ureq::Agent::new_with_defaults(),
        };
        assert_eq!(forge.release_noun(), "GitLab Release");
    }

    #[test]
    fn find_draft_release_always_none() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
            agent: ureq::Agent::new_with_defaults(),
        };
        assert_eq!(forge.find_draft_release("v1.0.0").unwrap(), None);
    }

    #[test]
    fn publish_release_noop() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "owner/repo".to_string(),
            api_base: "https://gitlab.com/api/v4".to_string(),
            agent: ureq::Agent::new_with_defaults(),
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
            agent: ureq::Agent::new_with_defaults(),
        };
        assert_eq!(forge.api_base, "https://gitlab.internal/api/v4");
    }
}
