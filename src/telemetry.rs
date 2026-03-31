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
    package_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_version: Option<String>,
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
    let hash = sha2::Sha256::digest(url.as_bytes());
    Some(hex::encode(hash))
}

pub fn send_event(
    event_type: EventType,
    package_name: Option<&str>,
    package_version: Option<&str>,
    metadata: Option<serde_json::Value>,
    commits_count: Option<i32>,
) {
    if !is_enabled() {
        return;
    }

    let payload = EventPayload {
        event_type,
        package_name: package_name.map(String::from),
        package_version: package_version.map(String::from),
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
            package_name: None,
            package_version: None,
            metadata: None,
            repo_hash: None,
            commits_count: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(!json.as_object().unwrap().contains_key("package_name"));
        assert!(!json.as_object().unwrap().contains_key("package_version"));
        assert!(!json.as_object().unwrap().contains_key("metadata"));
        assert!(!json.as_object().unwrap().contains_key("repo_hash"));
        assert!(!json.as_object().unwrap().contains_key("commits_count"));
    }

    #[test]
    fn payload_includes_present_fields() {
        let payload = EventPayload {
            event_type: EventType::Release,
            package_name: Some("my-pkg".into()),
            package_version: Some("1.0.0".into()),
            metadata: None,
            repo_hash: Some("abc123".into()),
            commits_count: Some(42),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["event_type"], "release");
        assert_eq!(json["package_name"], "my-pkg");
        assert_eq!(json["package_version"], "1.0.0");
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
}
