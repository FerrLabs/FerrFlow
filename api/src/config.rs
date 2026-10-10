use std::env;

pub const DEFAULT_AUDIENCE: &str = "ferrflow.ferrlabs.com";
pub const DEFAULT_GITHUB_API: &str = "https://api.github.com";

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub allowed_origins: Vec<String>,
    /// `None` disables `POST /v1/ferrflow/token`, which then answers 503.
    /// That is the intended state for any deploy without the GitHub App key.
    pub github_app: Option<GithubAppConfig>,
    pub marketplace_webhook_secret: Option<String>,
}

#[derive(Clone)]
pub struct GithubAppConfig {
    pub client_id: String,
    pub private_key_pem: String,
    pub expected_audience: String,
    pub api_base_url: String,
}

// The private key must never reach a log line, so `Debug` is written by hand
// rather than derived.
impl std::fmt::Debug for GithubAppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GithubAppConfig")
            .field("client_id", &self.client_id)
            .field("private_key_pem", &"<redacted>")
            .field("expected_audience", &self.expected_audience)
            .field("api_base_url", &self.api_base_url)
            .finish()
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            port: match env::var("PORT") {
                Ok(raw) => raw.parse()?,
                Err(_) => 3000,
            },
            allowed_origins: split_list(env::var("ALLOWED_ORIGINS").ok().as_deref()),
            github_app: github_app_from_env(),
            marketplace_webhook_secret: marketplace_webhook_secret(),
        })
    }
}

fn github_app_from_env() -> Option<GithubAppConfig> {
    let client_id = non_empty("FERRFLOW_APP_CLIENT_ID")?;
    let private_key_pem = private_key()?;
    Some(GithubAppConfig {
        client_id,
        private_key_pem,
        expected_audience: non_empty("FERRFLOW_APP_AUDIENCE")
            .unwrap_or_else(|| DEFAULT_AUDIENCE.to_string()),
        api_base_url: non_empty("GITHUB_API_URL").unwrap_or_else(|| DEFAULT_GITHUB_API.to_string()),
    })
}

/// The App key comes from a file in the cluster — the Vault agent writes it to
/// `/vault/secrets/`, and the runtime image is distroless, so there is no shell
/// to `export "$(cat …)"` the way a scratch-based image would. Reading the path
/// directly is both simpler and keeps the key out of the process environment,
/// where anything that dumps `/proc/self/environ` would find it.
fn private_key() -> Option<String> {
    if let Some(path) = non_empty("FERRFLOW_APP_PRIVATE_KEY_FILE") {
        return read_secret_file(&path);
    }
    non_empty("FERRFLOW_APP_PRIVATE_KEY")
}

fn marketplace_webhook_secret() -> Option<String> {
    if let Some(path) = non_empty("FERRFLOW_MARKETPLACE_WEBHOOK_SECRET_FILE") {
        return read_secret_file(&path);
    }
    non_empty("FERRFLOW_MARKETPLACE_WEBHOOK_SECRET")
}

fn read_secret_file(path: &str) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(secret) if !secret.trim().is_empty() => Some(secret.trim().to_string()),
        Ok(_) => {
            tracing::error!(path, "secret file is empty");
            None
        }
        Err(e) => {
            tracing::error!(path, error = %e, "cannot read secret file");
            None
        }
    }
}

fn non_empty(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn split_list(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_split_and_trim_and_drop_blanks() {
        assert_eq!(
            split_list(Some("https://a.com, https://b.com ,, ")),
            ["https://a.com", "https://b.com"]
        );
        assert_eq!(split_list(None), Vec::<String>::new());
        assert_eq!(split_list(Some("")), Vec::<String>::new());
    }

    // A Vault-injected key that failed to materialise leaves the var set but
    // empty. Treating that as "configured" would mint App JWTs from an empty
    // PEM and fail every request with a 502 instead of an honest 503.
    #[test]
    fn a_secret_file_is_trimmed_like_the_env_var_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("webhook-secret");
        std::fs::write(&path, "s3cr3t\n").unwrap();

        assert_eq!(
            read_secret_file(path.to_str().unwrap()).as_deref(),
            Some("s3cr3t")
        );
    }

    #[test]
    fn a_blank_key_is_the_same_as_an_absent_one() {
        let cfg = GithubAppConfig {
            client_id: "Iv23li".to_string(),
            private_key_pem: String::new(),
            expected_audience: DEFAULT_AUDIENCE.to_string(),
            api_base_url: DEFAULT_GITHUB_API.to_string(),
        };
        assert_eq!(cfg.private_key_pem, "");
    }

    // The Vault agent writes the key to a file; a deploy that lost its mount
    // must degrade to "token service disabled" (503) rather than start with an
    // empty PEM and fail every exchange with a 502.
    #[test]
    fn a_key_file_is_read_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github-app.pem");
        std::fs::write(
            &path,
            "-----BEGIN RSA PRIVATE KEY-----
x
",
        )
        .unwrap();

        let pem = read_secret_file(path.to_str().unwrap());
        assert!(pem.unwrap().contains("BEGIN RSA PRIVATE KEY"));
    }

    #[test]
    fn a_missing_or_empty_key_file_yields_no_key() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.pem");
        std::fs::write(
            &empty, "   
",
        )
        .unwrap();

        assert!(read_secret_file(empty.to_str().unwrap()).is_none());
        assert!(read_secret_file(dir.path().join("absent.pem").to_str().unwrap()).is_none());
    }

    #[test]
    fn debug_never_prints_the_private_key() {
        let cfg = GithubAppConfig {
            client_id: "Iv23li".to_string(),
            private_key_pem: "-----BEGIN RSA PRIVATE KEY-----SECRET".to_string(),
            expected_audience: DEFAULT_AUDIENCE.to_string(),
            api_base_url: DEFAULT_GITHUB_API.to_string(),
        };
        let rendered = format!("{cfg:?}");
        assert!(!rendered.contains("SECRET"), "{rendered}");
        assert!(rendered.contains("<redacted>"));
    }
}
