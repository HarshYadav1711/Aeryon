use std::path::PathBuf;
use std::process;

use aeryon_runtime::{AppConfig, Runtime, banner};
use tokio::sync::oneshot;

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
    let mut runtime = Runtime::boot(config)?;
    runtime.start()?;

    println!("{}", runtime.startup_summary());
    tracing::info!("press Ctrl+C to shutdown");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let mut shutdown_tx = Some(shutdown_tx);
    ctrlc::set_handler(move || {
        if let Some(tx) = shutdown_tx.take() {
            let _ = tx.send(());
        }
    })?;

    let _ = shutdown_rx.await;
    runtime.shutdown()?;

    Ok(())
}
