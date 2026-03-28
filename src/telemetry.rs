use serde::Serialize;

const DEFAULT_API_URL: &str = "https://api.ferrflow.com";

#[derive(Serialize)]
struct EventPayload {
    event_type: String,
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
    event_type: &str,
    package_name: Option<&str>,
    package_version: Option<&str>,
    metadata: Option<serde_json::Value>,
) {
    if !is_enabled() {
        return;
    }

    let payload = EventPayload {
        event_type: event_type.to_string(),
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
