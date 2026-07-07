use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use aeryon_runtime::{banner, AppConfig, RuntimeHealth, version};

#[derive(Parser)]
#[command(name = "aeryon-cli", version, about = "Aeryon command-line interface")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the platform version.
    Version,
    /// Print application configuration summary.
    Info,
    /// Print runtime health status.
    Health,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cli error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("Aeryon {}", version());
        }
        Commands::Info => {
            let config = load_config()?;
            println!("{}", banner());
            println!("version: {}", version());
            println!("application: {}", config.application.name);
            println!("environment: {}", config.application.environment);
            println!(
                "plugins: {}",
                if config.plugins.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!("log level: {}", config.logging.level);
        }
        Commands::Health => {
            let config = load_config()?;
            println!("runtime: {}", RuntimeHealth::Stopped);
            println!("server: not running");
            println!(
                "plugins: {}",
                if config.plugins.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
    }

    Ok(())
}

fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let config_path = std::env::var_os("AERYON_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/aeryon.toml"));

    Ok(AppConfig::load_or_default(&config_path)?)
}
