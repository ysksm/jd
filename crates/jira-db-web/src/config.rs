//! Configuration module for jira-db-web server.
//!
//! Supports loading configuration from TOML file.

use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Server settings
    #[serde(default)]
    pub server: ServerConfig,

    /// Application settings
    #[serde(default)]
    pub app: AppConfig,
}

/// Server bind settings
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Host address to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to bind to
    #[serde(default = "default_port")]
    pub port: u16,
}

/// Application settings
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    /// Path to settings.json
    #[serde(default = "default_settings_path")]
    pub settings_path: String,

    /// Path to static files directory
    #[serde(default = "default_static_dir")]
    pub static_dir: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_settings_path() -> String {
    "./data/settings.json".to_string()
}

fn default_static_dir() -> String {
    "./static/browser".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            settings_path: default_settings_path(),
            static_dir: default_static_dir(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            app: AppConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::ReadError {
            path: path.as_ref().display().to_string(),
            source: e,
        })?;

        toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            path: path.as_ref().display().to_string(),
            source: e,
        })
    }

    /// Load configuration from a file if it exists, otherwise return default
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        if path.as_ref().exists() {
            match Self::load(&path) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!(
                        "Failed to load config from {}: {}",
                        path.as_ref().display(),
                        e
                    );
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file '{path}': {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse config file '{path}': {source}")]
    ParseError {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}
