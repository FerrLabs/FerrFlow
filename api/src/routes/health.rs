use axum::Json;
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub time: String,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "ferrflow-api",
        version: env!("CARGO_PKG_VERSION"),
        time: Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_reports_ok_and_parseable_time() {
        let Json(body) = health().await;
        assert_eq!(body.status, "ok");
        assert_eq!(body.service, "ferrflow-api");
        assert!(body.time.parse::<chrono::DateTime<Utc>>().is_ok());
    }
}
