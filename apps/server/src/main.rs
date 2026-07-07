use std::path::PathBuf;
use std::process;
use std::sync::mpsc;

use aeryon_runtime::{AppConfig, Runtime, banner};

fn main() {
    if let Err(error) = run() {
        eprintln!("server error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", banner());

    let config_path = std::env::var_os("AERYON_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/aeryon.toml"));

    let config = AppConfig::load_or_default(&config_path)?;
    let mut runtime = Runtime::boot(config)?;
    runtime.start()?;

    println!("{}", runtime.startup_summary());
    println!("Press Ctrl+C to shutdown");

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = shutdown_tx.send(());
    })?;

    shutdown_rx.recv()?;
    runtime.shutdown()?;

    Ok(())
}
