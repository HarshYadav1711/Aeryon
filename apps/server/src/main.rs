//! Aeryon server process entry point.
//!
//! Startup order:
//! 1. load and validate configuration
//! 2. initialize tracing / runtime / plugin runtime
//! 3. start application services and synthetic sensor
//! 4. bind the HTTP API when enabled
//! 5. report startup status
//!
//! Shutdown drains the HTTP server, then stops plugins and the runtime.

mod api;

use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::Duration;

use aeryon_runtime::{AppConfig, Runtime, banner};
use tokio::sync::{RwLock, oneshot, watch};

use crate::api::{AppState, serve};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("server error: {error}");
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", banner());

    let config_path = std::env::var_os("AERYON_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/aeryon.toml"));

    let config = AppConfig::load_or_default(&config_path)?;
    let api_config = config.api.clone();

    let mut runtime = Runtime::boot(config)?;
    runtime.start()?;

    println!("{}", runtime.startup_summary());
    if api_config.enabled {
        println!(
            "API listening on http://{}:{} (local development)",
            api_config.host, api_config.port
        );
    }
    tracing::info!("press Ctrl+C to shutdown");

    let runtime = Arc::new(RwLock::new(runtime));
    let (signal_tx, signal_rx) = oneshot::channel();
    let mut signal_tx = Some(signal_tx);
    ctrlc::set_handler(move || {
        if let Some(tx) = signal_tx.take() {
            let _ = tx.send(());
        }
    })?;

    let (api_shutdown_tx, api_shutdown_rx) = watch::channel(false);
    let api_task = if api_config.enabled {
        let state = AppState::new(Arc::clone(&runtime));
        Some(tokio::spawn(async move {
            if let Err(error) = serve(state, api_config, api_shutdown_rx).await {
                tracing::error!(%error, "HTTP API exited with error");
            }
        }))
    } else {
        tracing::info!("HTTP API disabled by configuration");
        None
    };

    let _ = signal_rx.await;
    tracing::info!("shutdown signal received");

    let _ = api_shutdown_tx.send(true);
    if let Some(task) = api_task {
        match tokio::time::timeout(Duration::from_secs(5), task).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "HTTP API task join error"),
            Err(_) => tracing::warn!("HTTP API shutdown timed out"),
        }
    }

    {
        let mut runtime = runtime.write().await;
        runtime.shutdown()?;
    }

    Ok(())
}
