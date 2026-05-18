use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::config::Config;
use crate::error_code::{self, ErrorCodeExt};

pub trait FileSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>>;
    fn path_exists(&self, path: &str) -> Result<bool>;
}

pub struct LocalSource {
    pub root: PathBuf,
}

impl FileSource for LocalSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let full = self.root.join(path);
        match std::fs::read(&full) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn path_exists(&self, path: &str) -> Result<bool> {
        Ok(self.root.join(path).exists())
    }
}

#[derive(Debug, PartialEq)]
pub enum RemoteProvider {
    GitHub,
    GitLab,
}

pub fn parse_repo_spec(spec: &str) -> Result<(RemoteProvider, String, String)> {
    let spec = spec
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let parts: Vec<&str> = spec.split('/').collect();
    match parts.len() {
        2 => Ok((
            RemoteProvider::GitHub,
            parts[0].to_string(),
            parts[1].to_string(),
        )),
        3 => {
            let host = parts[0].to_lowercase();
            let provider = if host.contains("gitlab") {
                RemoteProvider::GitLab
            } else {
                RemoteProvider::GitHub
            };
            Ok((provider, parts[1].to_string(), parts[2].to_string()))
        }
        _ => Err(anyhow::anyhow!(
            "Invalid repo spec: {spec}. Expected owner/repo or host/owner/repo"
        ))
        .error_code(error_code::VALIDATE_INVALID_REPO_SPEC)?,
    }
}

pub struct GitHubSource {
    pub owner: String,
    pub repo: String,
    pub git_ref: Option<String>,
    pub token: Option<String>,
}

impl FileSource for GitHubSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let mut url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            self.owner, self.repo, path
        );
        if let Some(ref r) = self.git_ref {
            url.push_str(&format!("?ref={r}"));
        }
        let mut req = ureq::get(&url).header("Accept", "application/vnd.github.v3.raw");
        if let Some(ref token) = self.token {
            req = req.header("Authorization", &format!("Bearer {token}"));
        }
        req = req.header("User-Agent", "ferrflow");
        match req.call() {
            Ok(mut resp) => {
                let body = resp.body_mut().read_to_vec()?;
                Ok(Some(body))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("GitHub API error for {path}: {e}"))
                .error_code(error_code::VALIDATE_GITHUB_API),
        }
    }

    fn path_exists(&self, path: &str) -> Result<bool> {
        Ok(self.read_file(path)?.is_some())
    }
}

pub struct GitLabSource {
    pub owner: String,
    pub repo: String,
    pub git_ref: Option<String>,
    pub token: Option<String>,
}

impl FileSource for GitLabSource {
    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let project_id = format!("{}/{}", self.owner, self.repo);
        let encoded_project = project_id.replace('/', "%2F");
        let encoded_path = path.replace('/', "%2F");
        let mut url = format!(
            "https://gitlab.com/api/v4/projects/{encoded_project}/repository/files/{encoded_path}/raw"
        );
        if let Some(ref r) = self.git_ref {
            url.push_str(&format!("?ref={r}"));
        } else {
            url.push_str("?ref=main");
        }
        let mut req = ureq::get(&url);
        if let Some(ref token) = self.token {
            req = req.header("PRIVATE-TOKEN", token);
        }
        req = req.header("User-Agent", "ferrflow");
        match req.call() {
            Ok(mut resp) => {
                let body = resp.body_mut().read_to_vec()?;
                Ok(Some(body))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("GitLab API error for {path}: {e}"))
                .error_code(error_code::VALIDATE_GITLAB_API),
        }
    }

    fn path_exists(&self, path: &str) -> Result<bool> {
        Ok(self.read_file(path)?.is_some())
    }
}

const CONFIG_FILENAMES: &[&str] = &[
    "ferrflow.json",
    "ferrflow.json5",
    "ferrflow.toml",
    ".ferrflow",
];

pub(super) fn parse_config_content(content: &[u8], filename: &str) -> Result<Config> {
    let text = std::str::from_utf8(content)
        .with_context(|| format!("Invalid UTF-8 in {filename}"))
        .error_code(error_code::VALIDATE_INVALID_UTF8)?;
    match filename {
        f if f.ends_with(".toml") => toml_edit::de::from_str(text)
            .with_context(|| format!("Failed to parse {filename}"))
            .error_code(error_code::VALIDATE_PARSE_FAILED),
        f if f.ends_with(".json5") => json5::from_str(text)
            .with_context(|| format!("Failed to parse {filename}"))
            .error_code(error_code::VALIDATE_PARSE_FAILED),
        _ => serde_json::from_str(text)
            .with_context(|| format!("Failed to parse {filename}"))
            .error_code(error_code::VALIDATE_PARSE_FAILED),
    }
}

pub fn load_config_from_source(
    source: &dyn FileSource,
    explicit_path: Option<&str>,
) -> Result<(Config, String)> {
    if let Some(path) = explicit_path {
        let content = source
            .read_file(path)?
            .ok_or_else(|| anyhow::anyhow!("Config file not found: {path}"))
            .error_code(error_code::VALIDATE_FILE_NOT_FOUND)?;
        let config = parse_config_content(&content, path)?;
        return Ok((config, path.to_string()));
    }
    for filename in CONFIG_FILENAMES {
        if let Some(content) = source.read_file(filename)? {
            let config = parse_config_content(&content, filename)?;
            return Ok((config, filename.to_string()));
        }
    }
    Err(anyhow::anyhow!(
        "No FerrFlow configuration file found. Looked for: {}",
        CONFIG_FILENAMES.join(", ")
    ))
    .error_code(error_code::VALIDATE_NO_CONFIG)?
}
