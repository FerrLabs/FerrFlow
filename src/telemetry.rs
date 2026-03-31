use serde::Serialize;

const DEFAULT_API_URL: &str = "https://api.ferrflow.com";

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
}

fn is_enabled() -> bool {
    match std::env::var("FERRFLOW_TELEMETRY") {
        Ok(val) => !matches!(val.to_lowercase().as_str(), "false" | "0" | "off" | "no"),
        Err(_) => true,
    }
}

fn api_url() -> String {
    std::env::var("FERRFLOW_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

pub fn send_event(
    event_type: EventType,
    package_name: Option<&str>,
    package_version: Option<&str>,
    metadata: Option<serde_json::Value>,
) {
    if !is_enabled() {
        return;
    }

    let payload = EventPayload {
        event_type,
        package_name: package_name.map(String::from),
        package_version: package_version.map(String::from),
        metadata,
    };

    let url = format!("{}/events", api_url());

    std::thread::spawn(move || {
        let agent = ureq::Agent::new_with_defaults();
        let _ = agent
            .post(&url)
            .header("Content-Type", "application/json")
            .send_json(&payload);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn set_env(key: &str, val: &str) {
        unsafe { std::env::set_var(key, val) }
    }

    unsafe fn unset_env(key: &str) {
        unsafe { std::env::remove_var(key) }
    }

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
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(!json.as_object().unwrap().contains_key("package_name"));
        assert!(!json.as_object().unwrap().contains_key("package_version"));
        assert!(!json.as_object().unwrap().contains_key("metadata"));
    }

    #[test]
    fn payload_includes_present_fields() {
        let payload = EventPayload {
            event_type: EventType::Release,
            package_name: Some("my-pkg".into()),
            package_version: Some("1.0.0".into()),
            metadata: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["event_type"], "release");
        assert_eq!(json["package_name"], "my-pkg");
        assert_eq!(json["package_version"], "1.0.0");
    }

    #[test]
    fn disabled_by_env_var() {
        unsafe {
            for val in &["false", "0", "off", "no", "FALSE", "Off", "NO"] {
                set_env("FERRFLOW_TELEMETRY", val);
                assert!(!is_enabled(), "should be disabled for {val}");
            }
            unset_env("FERRFLOW_TELEMETRY");
        }
    }

    #[test]
    fn enabled_by_default() {
        unsafe { unset_env("FERRFLOW_TELEMETRY") };
        assert!(is_enabled());
    }

    #[test]
    fn enabled_when_set_to_true() {
        unsafe {
            set_env("FERRFLOW_TELEMETRY", "true");
            assert!(is_enabled());
            unset_env("FERRFLOW_TELEMETRY");
        }
    }

    #[test]
    fn api_url_defaults() {
        unsafe { unset_env("FERRFLOW_API_URL") };
        assert_eq!(api_url(), "https://api.ferrflow.com");
    }

    #[test]
    fn api_url_from_env() {
        unsafe {
            set_env("FERRFLOW_API_URL", "http://localhost:3000");
            assert_eq!(api_url(), "http://localhost:3000");
            unset_env("FERRFLOW_API_URL");
        }
    }

    #[test]
    fn send_event_noop_when_disabled() {
        unsafe {
            set_env("FERRFLOW_TELEMETRY", "false");
            send_event(EventType::Check, None, None, None);
            unset_env("FERRFLOW_TELEMETRY");
        }
    }
}
