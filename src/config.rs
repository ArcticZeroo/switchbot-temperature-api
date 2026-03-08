use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingVar(String),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub switchbot_token: String,
    pub switchbot_secret: String,
    pub switchbot_device_id: Option<String>,
    pub database_path: String,
    pub listen_addr: String,
}

impl Config {
    /// Load config for the `serve` subcommand. All vars required.
    pub fn load_for_serve() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv(); // ignore if .env doesn't exist

        let switchbot_token = require_env("SWITCHBOT_TOKEN")?;
        let switchbot_secret = require_env("SWITCHBOT_SECRET")?;
        let switchbot_device_id = require_env("SWITCHBOT_DEVICE_ID")?;
        let database_path =
            env::var("DATABASE_PATH").unwrap_or_else(|_| "climate.db".to_string());
        let listen_addr =
            env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        Ok(Config {
            switchbot_token,
            switchbot_secret,
            switchbot_device_id: Some(switchbot_device_id),
            database_path,
            listen_addr,
        })
    }

    /// Load config for the `discover` subcommand. Only token+secret required.
    pub fn load_for_discover() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();

        let switchbot_token = require_env("SWITCHBOT_TOKEN")?;
        let switchbot_secret = require_env("SWITCHBOT_SECRET")?;
        let database_path =
            env::var("DATABASE_PATH").unwrap_or_else(|_| "climate.db".to_string());
        let listen_addr =
            env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        Ok(Config {
            switchbot_token,
            switchbot_secret,
            switchbot_device_id: None,
            database_path,
            listen_addr,
        })
    }
}

fn require_env(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingVar(key.to_string()))
}
