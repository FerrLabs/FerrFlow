pub mod github;
pub mod gitlab;

use anyhow::Result;

pub use crate::config::ForgeKind;

pub struct MergeRequestResult {
    pub id: u64,
    pub auto_merge_key: String,
}

pub trait Forge {
    fn create_release(&self, tag: &str, body: &str, prerelease: bool, draft: bool) -> Result<()>;
    fn find_draft_release(&self, tag: &str) -> Result<Option<u64>>;
    fn publish_release(&self, release_id: u64) -> Result<()>;
    fn create_merge_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<MergeRequestResult>;
    fn enable_auto_merge(&self, mr: &MergeRequestResult) -> Result<()>;
    fn mr_noun(&self) -> &'static str;
    fn release_noun(&self) -> &'static str;
}

pub fn detect_forge_from_url(url: &str) -> Option<ForgeKind> {
    if url.contains("github.com") {
        Some(ForgeKind::Github)
    } else if url.contains("gitlab.com") {
        Some(ForgeKind::Gitlab)
    } else {
        None
    }
}

pub fn extract_repo_slug(url: &str) -> Option<String> {
    for host in ["github.com", "gitlab.com"] {
        let after = if url.contains(&format!("{host}/")) {
            url.split(&format!("{host}/")).nth(1)
        } else if url.contains(&format!("{host}:")) {
            url.split(&format!("{host}:")).nth(1)
        } else {
            None
        };
        if let Some(slug) = after {
            return Some(slug.trim_end_matches(".git").to_string());
        }
    }

    // Fallback for custom domains: extract path after host.
    // Handles both https://custom.host/owner/repo and git@custom.host:owner/repo
    let path = if let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    {
        rest.split_once('/').map(|x| x.1)
    } else if url.contains('@') && url.contains(':') {
        url.split_once(':').map(|x| x.1)
    } else {
        None
    };
    path.map(|p| p.trim_end_matches(".git").to_string())
        .filter(|s| s.contains('/') && !s.is_empty())
}

pub fn resolve_token(kind: ForgeKind) -> Option<String> {
    if let Ok(token) = std::env::var("FERRFLOW_TOKEN")
        && !token.is_empty()
    {
        return Some(token);
    }
    match kind {
        ForgeKind::Github => std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()),
        ForgeKind::Gitlab => std::env::var("GITLAB_TOKEN").ok().filter(|t| !t.is_empty()),
        ForgeKind::Auto => None,
    }
}

pub fn build_forge(kind: ForgeKind, token: String, slug: String) -> Box<dyn Forge> {
    match kind {
        ForgeKind::Github => Box::new(github::GitHubForge { token, slug }),
        ForgeKind::Gitlab => Box::new(gitlab::GitLabForge { token, slug }),
        ForgeKind::Auto => unreachable!("ForgeKind::Auto must be resolved before building"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn detect_github_https() {
        assert_eq!(
            detect_forge_from_url("https://github.com/owner/repo.git"),
            Some(ForgeKind::Github)
        );
    }

    #[test]
    fn detect_github_ssh() {
        assert_eq!(
            detect_forge_from_url("git@github.com:owner/repo.git"),
            Some(ForgeKind::Github)
        );
    }

    #[test]
    fn detect_gitlab_https() {
        assert_eq!(
            detect_forge_from_url("https://gitlab.com/owner/repo.git"),
            Some(ForgeKind::Gitlab)
        );
    }

    #[test]
    fn detect_gitlab_ssh() {
        assert_eq!(
            detect_forge_from_url("git@gitlab.com:owner/repo.git"),
            Some(ForgeKind::Gitlab)
        );
    }

    #[test]
    fn detect_unknown_host() {
        assert_eq!(
            detect_forge_from_url("https://bitbucket.org/owner/repo.git"),
            None
        );
    }

    #[test]
    fn slug_github_https() {
        assert_eq!(
            extract_repo_slug("https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn slug_github_ssh() {
        assert_eq!(
            extract_repo_slug("git@github.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn slug_gitlab_https() {
        assert_eq!(
            extract_repo_slug("https://gitlab.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn slug_gitlab_ssh() {
        assert_eq!(
            extract_repo_slug("git@gitlab.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn slug_gitlab_subgroup() {
        assert_eq!(
            extract_repo_slug("https://gitlab.com/group/subgroup/repo.git"),
            Some("group/subgroup/repo".to_string())
        );
    }

    #[test]
    fn slug_custom_domain_https() {
        assert_eq!(
            extract_repo_slug("https://git.company.com/team/project.git"),
            Some("team/project".to_string())
        );
    }

    #[test]
    fn slug_custom_domain_ssh() {
        assert_eq!(
            extract_repo_slug("git@git.company.com:team/project.git"),
            Some("team/project".to_string())
        );
    }

    #[test]
    fn forge_kind_deserialize_lowercase() {
        let kind: ForgeKind = serde_json::from_str("\"github\"").unwrap();
        assert_eq!(kind, ForgeKind::Github);
    }

    #[test]
    fn forge_kind_deserialize_default() {
        let kind: ForgeKind = serde_json::from_str("\"auto\"").unwrap();
        assert_eq!(kind, ForgeKind::Auto);
    }

    #[test]
    fn forge_kind_serialize() {
        assert_eq!(
            serde_json::to_string(&ForgeKind::Gitlab).unwrap(),
            "\"gitlab\""
        );
    }

    #[test]
    fn resolve_token_ferrflow_token_takes_precedence() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("FERRFLOW_TOKEN", "ferrflow-tok");
            std::env::set_var("GITHUB_TOKEN", "gh-tok");
        }
        let result = resolve_token(ForgeKind::Github);
        unsafe {
            std::env::remove_var("FERRFLOW_TOKEN");
            std::env::remove_var("GITHUB_TOKEN");
        }
        assert_eq!(result, Some("ferrflow-tok".to_string()));
    }

    #[test]
    fn resolve_token_falls_back_to_github_token() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::remove_var("FERRFLOW_TOKEN");
            std::env::set_var("GITHUB_TOKEN", "gh-tok");
        }
        let result = resolve_token(ForgeKind::Github);
        unsafe {
            std::env::remove_var("GITHUB_TOKEN");
        }
        assert_eq!(result, Some("gh-tok".to_string()));
    }

    #[test]
    fn resolve_token_falls_back_to_gitlab_token() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::remove_var("FERRFLOW_TOKEN");
            std::env::set_var("GITLAB_TOKEN", "gl-tok");
        }
        let result = resolve_token(ForgeKind::Gitlab);
        unsafe {
            std::env::remove_var("GITLAB_TOKEN");
        }
        assert_eq!(result, Some("gl-tok".to_string()));
    }

    #[test]
    fn resolve_token_empty_ferrflow_token_ignored() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("FERRFLOW_TOKEN", "");
            std::env::set_var("GITHUB_TOKEN", "gh-tok");
        }
        let result = resolve_token(ForgeKind::Github);
        unsafe {
            std::env::remove_var("FERRFLOW_TOKEN");
            std::env::remove_var("GITHUB_TOKEN");
        }
        assert_eq!(result, Some("gh-tok".to_string()));
    }

    #[test]
    fn resolve_token_auto_returns_none() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::remove_var("FERRFLOW_TOKEN");
        }
        assert_eq!(resolve_token(ForgeKind::Auto), None);
    }

    #[test]
    fn build_forge_github() {
        let forge = build_forge(ForgeKind::Github, "tok".into(), "owner/repo".into());
        assert_eq!(forge.mr_noun(), "PR");
        assert_eq!(forge.release_noun(), "GitHub Release");
    }

    #[test]
    fn build_forge_gitlab() {
        let forge = build_forge(ForgeKind::Gitlab, "tok".into(), "owner/repo".into());
        assert_eq!(forge.mr_noun(), "MR");
        assert_eq!(forge.release_noun(), "GitLab Release");
    }

    #[test]
    #[should_panic(expected = "unreachable")]
    fn build_forge_auto_panics() {
        build_forge(ForgeKind::Auto, "tok".into(), "owner/repo".into());
    }

    #[test]
    fn slug_no_suffix() {
        assert_eq!(
            extract_repo_slug("https://github.com/owner/repo"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn detect_forge_empty_string() {
        assert_eq!(detect_forge_from_url(""), None);
    }
}
