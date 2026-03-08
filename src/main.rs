mod api;
mod config;
mod db;
mod models;
mod poller;
mod switchbot;

use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "switchbot-api", about = "SwitchBot climate data proxy API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the HTTP server and hourly polling scheduler
    Serve,
    /// Discover SwitchBot devices with temperature/humidity sensors
    Discover,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("switchbot_api=info")
        }))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve => run_server().await?,
        Commands::Discover => run_discover().await?,
    }

    Ok(())
}

async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::Config::load_for_serve()?;

    let db = db::Database::open(&config.database_path)?;
    let db = Arc::new(db);

    let switchbot_client =
        switchbot::SwitchBotClient::new(config.switchbot_token, config.switchbot_secret);

    let device_id = config.switchbot_device_id.expect("device_id required for serve");

    // Start the background poller
    poller::start_poller(
        Arc::clone(&db),
        switchbot_client.clone(),
        device_id.clone(),
    )
    .await?;

    let state = Arc::new(api::AppState {
        db,
        switchbot: switchbot_client,
        device_id,
    });

    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    tracing::info!("listening on {}", config.listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn run_discover() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::Config::load_for_discover()?;
    let client = switchbot::SwitchBotClient::new(config.switchbot_token, config.switchbot_secret);

    println!("Discovering SwitchBot climate devices...\n");

    let devices = client.discover_climate_devices().await?;

    if devices.is_empty() {
        println!("No temperature/humidity devices found.");
        println!("Make sure Cloud Service is enabled in the SwitchBot app for your device.");
    } else {
        println!(
            "{:<20} {:<25} {}",
            "DEVICE ID", "NAME", "TYPE"
        );
        println!("{}", "-".repeat(65));
        for device in &devices {
            println!(
                "{:<20} {:<25} {}",
                device.device_id, device.device_name, device.device_type
            );
        }
        println!(
            "\nSet SWITCHBOT_DEVICE_ID to one of the device IDs above in your .env file."
        );
    }

    Ok(())
}

