use anyhow::{Context, Result};

pub fn create_github_release(token: &str, repo: &str, tag: &str, body: &str) -> Result<()> {
    let url = format!("https://api.github.com/repos/{repo}/releases");

    let payload = serde_json::json!({
        "tag_name": tag,
        "name": tag,
        "body": body,
        "draft": false,
        "prerelease": false,
    });

    ureq::post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "ferrflow")
        .send_json(payload)
        .with_context(|| format!("Failed to create GitHub release for {tag}"))?;

    Ok(())
}

pub struct PullRequest {
    pub number: u64,
    pub node_id: String,
}

pub fn create_github_pr(
    token: &str,
    repo: &str,
    head: &str,
    base: &str,
    title: &str,
    body: &str,
) -> Result<PullRequest> {
    let url = format!("https://api.github.com/repos/{repo}/pulls");

    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "head": head,
        "base": base,
    });

    let response: serde_json::Value = ureq::post(&url)
        .header("Authorization", &format!("Bearer {token}"))
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

    Ok(PullRequest { number, node_id })
}

pub fn enable_auto_merge(token: &str, pr_node_id: &str, pr_number: u64) -> Result<()> {
    let query = serde_json::json!({
        "query": "mutation($prId: ID!) { enablePullRequestAutoMerge(input: { pullRequestId: $prId, mergeMethod: SQUASH }) { pullRequest { number } } }",
        "variables": { "prId": pr_node_id },
    });

    let response: serde_json::Value = ureq::post("https://api.github.com/graphql")
        .header("Authorization", &format!("Bearer {token}"))
        .header("User-Agent", "ferrflow")
        .send_json(query)
        .with_context(|| format!("Failed to enable auto-merge on PR #{pr_number}"))?
        .body_mut()
        .read_json()
        .with_context(|| "Failed to parse GraphQL response")?;

    if let Some(errors) = response.get("errors") {
        let msg = errors[0]["message"]
            .as_str()
            .unwrap_or("unknown GraphQL error");
        anyhow::bail!("Auto-merge failed on PR #{pr_number}: {msg}");
    }

    Ok(())
}
