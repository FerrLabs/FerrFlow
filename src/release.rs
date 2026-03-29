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

pub fn create_github_pr(
    token: &str,
    repo: &str,
    head: &str,
    base: &str,
    title: &str,
    body: &str,
) -> Result<u64> {
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

    Ok(number)
}

pub fn enable_auto_merge(token: &str, repo: &str, pr_number: u64) -> Result<()> {
    let url = format!("https://api.github.com/repos/{repo}/pulls/{pr_number}/merge");

    ureq::put(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "ferrflow")
        .send_json(serde_json::json!({
            "merge_method": "squash",
        }))
        .with_context(|| format!("Failed to enable auto-merge on PR #{pr_number}"))?;

    Ok(())
}
