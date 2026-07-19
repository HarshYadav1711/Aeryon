//! HTTP and WebSocket route handlers.

mod events;
mod health;
mod plugins;
mod runtime;
mod sensors;

use std::time::Duration;

use aeryon_runtime::ApiConfig;
use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::error::ApiError;
use super::state::AppState;

/// Builds the Axum router for the live API surface.
pub fn build_router(state: AppState, api: &ApiConfig) -> Router {
    let cors = build_cors_layer(api);

    Router::new()
        .route("/health", get(health::health_handler))
        .route("/api/v1/runtime", get(runtime::runtime_handler))
        .route("/api/v1/plugins", get(plugins::plugins_handler))
        .route(
            "/api/v1/sensors/synthetic",
            get(sensors::synthetic_sensor_handler),
        )
        .route("/api/v1/events/ws", get(events::events_ws_handler))
        .fallback(api_not_found)
        .layer(cors)
        .with_state(state)
}

async fn api_not_found() -> ApiError {
    ApiError::not_found("endpoint not found")
}

/// Binds and serves the API until `shutdown` becomes true.
pub async fn serve(
    state: AppState,
    api: ApiConfig,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = api.socket_addr()?;
    tracing::info!(%addr, "binding HTTP API");

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%addr, %error, "HTTP API bind failed");
            return Err(error.into());
        }
    };

    let bound = listener.local_addr().unwrap_or(addr);
    tracing::info!(%bound, "HTTP API listening");

    let app = build_router(state, &api);
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        loop {
            if *shutdown.borrow() {
                break;
            }
            if shutdown.changed().await.is_err() {
                break;
            }
        }
        tracing::info!("HTTP API graceful shutdown starting");
    });

    server.await?;
    tracing::info!("HTTP API stopped");
    Ok(())
}

fn build_cors_layer(api: &ApiConfig) -> CorsLayer {
    let origins: Vec<_> = api
        .cors_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    if origins.is_empty() {
        CorsLayer::new()
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([axum::http::Method::GET, axum::http::Method::OPTIONS])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::ACCEPT])
            .max_age(Duration::from_secs(600))
    }
}

/// Helper used by tests to bind an ephemeral listener.
#[cfg(test)]
pub async fn bind_ephemeral(
    state: AppState,
    api: ApiConfig,
) -> Result<(std::net::SocketAddr, watch::Sender<bool>), Box<dyn std::error::Error + Send + Sync>> {
    let mut api = api;
    api.host = "127.0.0.1".to_owned();
    api.port = 0;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let app = build_router(state, &api);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let mut shutdown_rx = shutdown_rx;
                loop {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                    if shutdown_rx.changed().await.is_err() {
                        break;
                    }
                }
            })
            .await;
    });

    Ok((addr, shutdown_tx))
}

/// Convenience constructor for tests.
#[cfg(test)]
pub fn test_state(runtime: aeryon_runtime::Runtime) -> AppState {
    AppState::new(std::sync::Arc::new(tokio::sync::RwLock::new(runtime)))
}
