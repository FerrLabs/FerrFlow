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
