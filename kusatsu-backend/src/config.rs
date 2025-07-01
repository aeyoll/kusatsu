use crate::error::{AppError, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub server_address: String,
    pub storage_dir: String,
    pub base_url: String,
    pub api_url: String,
    pub max_file_size: usize,
    pub cleanup_interval_hours: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://kusatsu.db".to_string()),

            server_address: env::var("SERVER_ADDRESS")
                .unwrap_or_else(|_| "127.0.0.1:3000".to_string()),

            storage_dir: env::var("STORAGE_DIR").unwrap_or_else(|_| "./storage".to_string()),

            base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()),

            api_url: env::var("API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),

            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "5000".to_string()) // Default 5GB
                .parse::<usize>()
                .map_err(|_| AppError::ConfigError("Invalid MAX_FILE_SIZE".to_string()))?
                * 1024
                * 1024, // Convert MB to bytes

            cleanup_interval_hours: env::var("CLEANUP_INTERVAL_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .map_err(|_| AppError::ConfigError("Invalid CLEANUP_INTERVAL_HOURS".to_string()))?,
        })
    }
}
