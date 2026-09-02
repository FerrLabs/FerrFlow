use std::sync::Arc;

use crate::bot::github_client::GithubClient;
use crate::config::{Config, GithubAppConfig};

/// The token service is the only stateful part of this API; every other route
/// is a pure function of its request. `None` means the deploy has no GitHub
/// App key, which is the normal state for a self-hosted or local run — the
/// token route answers 503 and the rest of the API is unaffected.
#[derive(Clone)]
pub struct AppState {
    github_app: Option<Arc<(GithubAppConfig, GithubClient)>>,
    marketplace_webhook_secret: Option<Arc<str>>,
}

impl AppState {
    pub fn new(app: Option<GithubAppConfig>) -> Self {
        Self {
            github_app: app.map(|cfg| {
                let client = GithubClient::new(cfg.api_base_url.clone());
                Arc::new((cfg, client))
            }),
            marketplace_webhook_secret: None,
        }
    }

    #[must_use]
    pub fn with_marketplace_webhook_secret(mut self, secret: Option<String>) -> Self {
        self.marketplace_webhook_secret = secret.map(Arc::from);
        self
    }

    pub fn from_config(config: &Config) -> Self {
        Self::new(config.github_app.clone())
            .with_marketplace_webhook_secret(config.marketplace_webhook_secret.clone())
    }

    pub fn marketplace_webhook_secret(&self) -> Option<&str> {
        self.marketplace_webhook_secret.as_deref()
    }

    pub fn github_app(&self) -> Option<(&GithubAppConfig, &GithubClient)> {
        self.github_app
            .as_deref()
            .map(|(cfg, client)| (cfg, client))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DEFAULT_AUDIENCE, DEFAULT_GITHUB_API};

    #[test]
    fn an_unconfigured_state_exposes_no_github_app() {
        assert!(AppState::new(None).github_app().is_none());
    }

    #[test]
    fn a_configured_state_carries_the_audience_through() {
        let state = AppState::new(Some(GithubAppConfig {
            client_id: "Iv23li".to_string(),
            private_key_pem: "pem".to_string(),
            expected_audience: DEFAULT_AUDIENCE.to_string(),
            api_base_url: DEFAULT_GITHUB_API.to_string(),
        }));
        let (cfg, _) = state.github_app().expect("configured");
        assert_eq!(cfg.expected_audience, DEFAULT_AUDIENCE);
    }
}
