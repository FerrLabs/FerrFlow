use anyhow::{Context, Result};
use ureq::RequestBuilder;
use ureq::typestate::{WithBody, WithoutBody};

use super::ReleaseResult;

const MAX_PAGES: u32 = 100;

#[derive(Clone, Copy)]
enum Unidentified {
    AnyBot,
    Nobody,
}

enum CommentOwner {
    Login(String),
    Unidentified(Unidentified),
}

impl CommentOwner {
    fn wrote(&self, comment: &serde_json::Value) -> bool {
        let author = &comment["user"];
        match self {
            Self::Login(login) => author["login"].as_str() == Some(login),
            Self::Unidentified(Unidentified::AnyBot) => author["type"].as_str() == Some("Bot"),
            Self::Unidentified(Unidentified::Nobody) => false,
        }
    }
}

pub(super) struct RestClient<'a> {
    agent: &'a ureq::Agent,
    repo_url: String,
    user_url: String,
    unidentified: Unidentified,
    auth: (&'static str, String),
    extra_headers: &'static [(&'static str, &'static str)],
    page_size_param: &'static str,
    page_size: u32,
}

impl<'a> RestClient<'a> {
    pub fn github(agent: &'a ureq::Agent, api_base: &str, slug: &str, token: &str) -> Self {
        Self {
            agent,
            repo_url: format!("{api_base}/repos/{slug}"),
            user_url: format!("{api_base}/user"),
            unidentified: Unidentified::AnyBot,
            auth: ("Authorization", format!("Bearer {token}")),
            extra_headers: &[
                ("Accept", "application/vnd.github+json"),
                ("X-GitHub-Api-Version", "2022-11-28"),
            ],
            page_size_param: "per_page",
            page_size: 100,
        }
    }

    pub fn gitea(agent: &'a ureq::Agent, api_base: &str, slug: &str, token: &str) -> Self {
        Self {
            agent,
            repo_url: format!("{api_base}/repos/{slug}"),
            user_url: format!("{api_base}/user"),
            unidentified: Unidentified::Nobody,
            auth: ("Authorization", format!("token {token}")),
            extra_headers: &[],
            page_size_param: "limit",
            page_size: 50,
        }
    }

    fn headers<B>(&self, mut request: RequestBuilder<B>) -> RequestBuilder<B> {
        request = request
            .header(self.auth.0, &self.auth.1)
            .header("User-Agent", "ferrflow");
        for (name, value) in self.extra_headers {
            request = request.header(*name, *value);
        }
        request
    }

    fn get(&self, url: &str) -> RequestBuilder<WithoutBody> {
        self.headers(self.agent.get(url))
    }

    fn post(&self, url: &str) -> RequestBuilder<WithBody> {
        self.headers(self.agent.post(url))
    }

    fn patch(&self, url: &str) -> RequestBuilder<WithBody> {
        self.headers(self.agent.patch(url))
    }

    pub fn paginated_json_array(
        &self,
        base_url: &str,
        what: &str,
    ) -> Result<Vec<serde_json::Value>> {
        paginate(
            || self.get(base_url),
            self.page_size_param,
            self.page_size,
            what,
        )
    }

    pub fn create_release(
        &self,
        tag: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
    ) -> Result<ReleaseResult> {
        let response: serde_json::Value = self
            .post(&format!("{}/releases", self.repo_url))
            .send_json(serde_json::json!({
                "tag_name": tag,
                "name": tag,
                "body": body,
                "draft": draft,
                "prerelease": prerelease,
            }))?
            .body_mut()
            .read_json()
            .unwrap_or(serde_json::Value::Null);
        Ok(ReleaseResult {
            id: response["id"].as_u64(),
            url: response["html_url"].as_str().map(str::to_string),
        })
    }

    pub fn find_draft_release(&self, tag: &str, what: &str) -> Result<Option<u64>> {
        let releases = self.paginated_json_array(&format!("{}/releases", self.repo_url), what)?;
        Ok(releases.iter().find_map(|release| {
            (release["draft"].as_bool() == Some(true) && release["tag_name"].as_str() == Some(tag))
                .then(|| release["id"].as_u64())
                .flatten()
        }))
    }

    pub fn publish_release(&self, release_id: u64) -> Result<()> {
        self.patch(&format!("{}/releases/{release_id}", self.repo_url))
            .send_json(serde_json::json!({ "draft": false }))?;
        Ok(())
    }

    pub fn find_comment(&self, pr_id: u64, marker: &str, what: &str) -> Result<Option<u64>> {
        let comments =
            self.paginated_json_array(&format!("{}/issues/{pr_id}/comments", self.repo_url), what)?;
        let mut marked = comments
            .iter()
            .rev()
            .filter(|comment| {
                comment["body"]
                    .as_str()
                    .is_some_and(|body| body.contains(marker))
            })
            .peekable();
        if marked.peek().is_none() {
            return Ok(None);
        }
        let owner = self.comment_owner()?;
        Ok(marked
            .find(|comment| owner.wrote(comment))
            .and_then(|comment| comment["id"].as_u64()))
    }

    fn comment_owner(&self) -> Result<CommentOwner> {
        match self.get(&self.user_url).call() {
            Ok(mut response) => {
                let user: serde_json::Value = response
                    .body_mut()
                    .read_json()
                    .context("Failed to parse the token's user")?;
                let login = user["login"]
                    .as_str()
                    .context("The token's user has no login")?;
                Ok(CommentOwner::Login(login.to_string()))
            }
            Err(ureq::Error::StatusCode(401 | 403)) => {
                if matches!(self.unidentified, Unidentified::Nobody) {
                    tracing::warn!(
                        "Warning: the token cannot read its own user, posting a new comment instead of updating one"
                    );
                }
                Ok(CommentOwner::Unidentified(self.unidentified))
            }
            Err(e) => Err(e).context("Failed to read the token's user"),
        }
    }

    pub fn create_comment(&self, pr_id: u64, body: &str) -> Result<()> {
        self.post(&format!("{}/issues/{pr_id}/comments", self.repo_url))
            .send_json(serde_json::json!({ "body": body }))?;
        Ok(())
    }

    pub fn update_comment(&self, comment_id: u64, body: &str) -> Result<()> {
        self.patch(&format!("{}/issues/comments/{comment_id}", self.repo_url))
            .send_json(serde_json::json!({ "body": body }))?;
        Ok(())
    }
}

pub(super) fn paginate(
    request: impl Fn() -> RequestBuilder<WithoutBody>,
    page_size_param: &str,
    page_size: u32,
    what: &str,
) -> Result<Vec<serde_json::Value>> {
    let mut all = Vec::new();
    for page in 1..=MAX_PAGES {
        let body: serde_json::Value = request()
            .query(page_size_param, page_size.to_string())
            .query("page", page.to_string())
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
        if (len as u32) < page_size {
            return Ok(all);
        }
    }
    Ok(all)
}
