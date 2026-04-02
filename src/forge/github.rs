use anyhow::{Context, Result};

use super::{Forge, MergeRequestResult};

pub struct GitHubForge {
    pub token: String,
    pub slug: String,
}

impl Forge for GitHubForge {
    fn create_release(&self, tag: &str, body: &str, prerelease: bool) -> Result<()> {
        let url = format!("https://api.github.com/repos/{}/releases", self.slug);

        let payload = serde_json::json!({
            "tag_name": tag,
            "name": tag,
            "body": body,
            "draft": false,
            "prerelease": prerelease,
        });

        ureq::post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "ferrflow")
            .send_json(payload)
            .with_context(|| format!("Failed to create GitHub release for {tag}"))?;

        Ok(())
    }

    fn create_merge_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<MergeRequestResult> {
        let url = format!("https://api.github.com/repos/{}/pulls", self.slug);

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
            .with_context(|| format!("Failed to create PR from {head} to {base}"))?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse PR response")?;

        let number = response["number"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("PR response missing number field"))?;

        let node_id = response["node_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("PR response missing node_id field"))?
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

        let response: serde_json::Value = ureq::post("https://api.github.com/graphql")
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "ferrflow")
            .send_json(query)
            .with_context(|| format!("Failed to enable auto-merge on PR #{}", mr.id))?
            .body_mut()
            .read_json()
            .with_context(|| "Failed to parse GraphQL response")?;

        if let Some(errors) = response.get("errors") {
            let msg = errors[0]["message"]
                .as_str()
                .unwrap_or("unknown GraphQL error");
            anyhow::bail!("Auto-merge failed on PR #{}: {msg}", mr.id);
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
