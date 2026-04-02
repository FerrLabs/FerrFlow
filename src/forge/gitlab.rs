use anyhow::{Context, Result};

use super::{Forge, MergeRequestResult};

pub struct GitLabForge {
    pub token: String,
    pub slug: String,
}

impl GitLabForge {
    fn encoded_project_id(&self) -> String {
        self.slug.replace('/', "%2F")
    }
}

impl Forge for GitLabForge {
    fn create_release(&self, tag: &str, body: &str, prerelease: bool) -> Result<()> {
        let project = self.encoded_project_id();
        let url = format!("https://gitlab.com/api/v4/projects/{project}/releases");

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
            .with_context(|| format!("Failed to create GitLab release for {tag}"))?;

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
        let url = format!("https://gitlab.com/api/v4/projects/{project}/merge_requests");

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
            .with_context(|| format!("Failed to create MR from {head} to {base}"))?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse MR response")?;

        let iid = response["iid"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("MR response missing iid field"))?;

        Ok(MergeRequestResult {
            id: iid,
            auto_merge_key: iid.to_string(),
        })
    }

    fn enable_auto_merge(&self, mr: &MergeRequestResult) -> Result<()> {
        let project = self.encoded_project_id();
        let url = format!(
            "https://gitlab.com/api/v4/projects/{project}/merge_requests/{}/merge",
            mr.id
        );

        let payload = serde_json::json!({
            "merge_when_pipeline_succeeds": true,
            "squash": true,
        });

        ureq::put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to enable auto-merge on MR !{}", mr.id))?;

        Ok(())
    }

    fn mr_noun(&self) -> &'static str {
        "MR"
    }

    fn release_noun(&self) -> &'static str {
        "GitLab Release"
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
        };
        assert_eq!(forge.encoded_project_id(), "owner%2Frepo");
    }

    #[test]
    fn encoded_project_id_subgroup() {
        let forge = GitLabForge {
            token: String::new(),
            slug: "group/subgroup/repo".to_string(),
        };
        assert_eq!(forge.encoded_project_id(), "group%2Fsubgroup%2Frepo");
    }
}
