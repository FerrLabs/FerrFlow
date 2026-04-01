use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use sha2::Digest;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<sha2::Sha256>;

const DEFAULT_API_URL: &str = "https://api.ferrflow.com";

fn hmac_secret() -> Option<&'static str> {
    option_env!("FERRFLOW_HMAC_SECRET")
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum EventType {
    Check,
    Release,
    VersionBump,
    Init,
    Error,
}

#[derive(Serialize)]
struct EventPayload {
    event_type: EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commits_count: Option<i32>,
}

fn is_enabled() -> bool {
    check_enabled(
        std::env::var("FERRFLOW_ANONYMOUS_TELEMETRY")
            .or_else(|_| std::env::var("FERRFLOW_TELEMETRY"))
            .ok()
            .as_deref(),
    )
}

fn check_enabled(val: Option<&str>) -> bool {
    match val {
        Some(v) => !matches!(v.to_lowercase().as_str(), "false" | "0" | "off" | "no"),
        None => true,
    }
}

fn api_url() -> String {
    std::env::var("FERRFLOW_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

fn normalize_remote_url(raw: &str) -> String {
    let url = raw.trim();

    // SSH: git@github.com:Owner/Repo.git -> github.com/Owner/Repo
    if let Some(rest) = url.strip_prefix("git@") {
        let normalized = rest.replace(':', "/");
        return normalized.trim_end_matches(".git").to_lowercase();
    }

    // HTTPS: strip scheme, credentials, and .git suffix
    // https://x-access-token:TOKEN@github.com/Owner/Repo.git -> github.com/Owner/Repo
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Strip credentials (user:pass@ or token@)
    let without_creds = match without_scheme.find('@') {
        Some(pos) => &without_scheme[pos + 1..],
        None => without_scheme,
    };

    without_creds
        .trim_end_matches(".git")
        .trim_end_matches('/')
        .to_lowercase()
}

fn hash_remote_url(url: &str) -> String {
    let normalized = normalize_remote_url(url);
    let hash = sha2::Sha256::digest(normalized.as_bytes());
    hex::encode(hash)
}

fn get_repo_hash() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if url.is_empty() {
        return None;
    }
    Some(hash_remote_url(&url))
}

pub fn send_event(
    event_type: EventType,
    metadata: Option<serde_json::Value>,
    commits_count: Option<i32>,
) {
    if !is_enabled() {
        return;
    }

    let payload = EventPayload {
        event_type,
        metadata,
        repo_hash: get_repo_hash(),
        commits_count,
    };

    let url = format!("{}/events", api_url());

    std::thread::spawn(move || {
        let body = match serde_json::to_string(&payload) {
            Ok(b) => b,
            Err(_) => return,
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let agent = ureq::Agent::new_with_defaults();
        let mut req = agent.post(&url).header("Content-Type", "application/json");

        if let Some(secret) = hmac_secret() {
            let message = format!("{timestamp}.{body}");
            let mut mac =
                HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
            mac.update(message.as_bytes());
            let signature = hex::encode(mac.finalize().into_bytes());
            req = req
                .header("X-Timestamp", &timestamp)
                .header("X-Signature", &signature);
        }

        let _ = req.send(body.as_bytes());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&EventType::Check).unwrap(),
            "\"check\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::Release).unwrap(),
            "\"release\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::VersionBump).unwrap(),
            "\"version_bump\""
        );
        assert_eq!(serde_json::to_string(&EventType::Init).unwrap(), "\"init\"");
        assert_eq!(
            serde_json::to_string(&EventType::Error).unwrap(),
            "\"error\""
        );
    }

    #[test]
    fn payload_skips_none_fields() {
        let payload = EventPayload {
            event_type: EventType::Check,
            metadata: None,
            repo_hash: None,
            commits_count: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(!json.as_object().unwrap().contains_key("metadata"));
        assert!(!json.as_object().unwrap().contains_key("repo_hash"));
        assert!(!json.as_object().unwrap().contains_key("commits_count"));
    }

    #[test]
    fn payload_includes_present_fields() {
        let payload = EventPayload {
            event_type: EventType::Release,
            metadata: None,
            repo_hash: Some("abc123".into()),
            commits_count: Some(42),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["event_type"], "release");
        assert_eq!(json["repo_hash"], "abc123");
        assert_eq!(json["commits_count"], 42);
    }

    #[test]
    fn check_enabled_disabled_values() {
        for val in ["false", "0", "off", "no", "FALSE", "Off", "NO"] {
            assert!(!check_enabled(Some(val)), "should be disabled for {val}");
        }
    }

    #[test]
    fn check_enabled_default() {
        assert!(check_enabled(None));
    }

    #[test]
    fn check_enabled_true_values() {
        for val in ["true", "1", "yes", "anything"] {
            assert!(check_enabled(Some(val)), "should be enabled for {val}");
        }
    }

    #[test]
    fn hash_remote_url_produces_consistent_hash() {
        let h1 = hash_remote_url("git@github.com:Org/Repo.git");
        let h2 = hash_remote_url("https://github.com/Org/Repo.git");
        let h3 = hash_remote_url("https://x-access-token:TOKEN@github.com/Org/Repo");
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn normalize_ssh_url() {
        assert_eq!(
            normalize_remote_url("git@github.com:FerrFlow-Org/FerrFlow.git"),
            "github.com/ferrflow-org/ferrflow"
        );
    }

    #[test]
    fn normalize_https_url() {
        assert_eq!(
            normalize_remote_url("https://github.com/FerrFlow-Org/FerrFlow.git"),
            "github.com/ferrflow-org/ferrflow"
        );
    }

    #[test]
    fn normalize_https_without_git_suffix() {
        assert_eq!(
            normalize_remote_url("https://github.com/FerrFlow-Org/FerrFlow"),
            "github.com/ferrflow-org/ferrflow"
        );
    }

    #[test]
    fn normalize_https_with_token() {
        assert_eq!(
            normalize_remote_url(
                "https://x-access-token:ghs_abc123@github.com/FerrFlow-Org/FerrFlow.git"
            ),
            "github.com/ferrflow-org/ferrflow"
        );
    }

    #[test]
    fn normalize_https_with_user_pass() {
        assert_eq!(
            normalize_remote_url("https://user:pass@github.com/Org/Repo.git"),
            "github.com/org/repo"
        );
    }

    #[test]
    fn normalize_ssh_and_https_produce_same_hash() {
        let ssh = normalize_remote_url("git@github.com:FerrFlow-Org/FerrFlow.git");
        let https = normalize_remote_url("https://github.com/FerrFlow-Org/FerrFlow.git");
        let https_no_git = normalize_remote_url("https://github.com/FerrFlow-Org/FerrFlow");
        let https_token = normalize_remote_url(
            "https://x-access-token:TOKEN@github.com/FerrFlow-Org/FerrFlow.git",
        );
        assert_eq!(ssh, https);
        assert_eq!(https, https_no_git);
        assert_eq!(https_no_git, https_token);
    }

    #[test]
    fn normalize_trailing_slash() {
        assert_eq!(
            normalize_remote_url("https://github.com/Org/Repo/"),
            "github.com/org/repo"
        );
    }

    #[test]
    fn normalize_http_url() {
        assert_eq!(
            normalize_remote_url("http://github.com/Org/Repo.git"),
            "github.com/org/repo"
        );
    }
}
