use anyhow::{Context, Result};
use ferrflow_api::{build_router, config::Config, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = Config::from_env().context("loading configuration")?;
    let state = AppState::from_config(&config);

    if state.github_app().is_some() {
        tracing::info!("FerrFlow bot token service: enabled");
    } else {
        tracing::warn!(
            "FerrFlow bot token service: disabled (no FERRFLOW_APP_CLIENT_ID / FERRFLOW_APP_PRIVATE_KEY) — /v1/ferrflow/token will answer 503"
        );
    }

    let app = build_router(state, &config.allowed_origins);
    let bind = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding {bind}"))?;

    tracing::info!(%bind, version = env!("CARGO_PKG_VERSION"), "starting FerrFlow API");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving")
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).json())
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
