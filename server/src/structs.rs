use std::{fs::Permissions, os::unix::prelude::PermissionsExt, path::PathBuf};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub code: u128,
    pub port: u16,
}

const CONFIG_DIR: &str = "/etc/local-forwarder";
const CONFIG_FILE: &str = "config.json";

impl Config {
    pub async fn load() -> Result<Self> {
        if std::env::var("LF_ENV").is_ok() {
            return Self::load_from_env().await;
        }

        Self::ensure_dir().await?;
        let config_path = PathBuf::from(CONFIG_DIR).join(CONFIG_FILE);

        if config_path.exists() {
            let config = tokio::fs::read_to_string(&config_path).await?;
            let config: Config = serde_json::from_str(&config)?;

            Ok(config)
        } else {
            let config = Config {
                code: rand::random::<u128>(),
                port: 1337,
            };

            let config_str = serde_json::to_string_pretty(&config)?;
            tokio::fs::write(&config_path, config_str).await?;
            tokio::fs::set_permissions(config_path, Permissions::from_mode(0o600)).await?;

            Ok(config)
        }
    }

    async fn load_from_env() -> Result<Self> {
        Ok(Self {
            code: std::env::var("LF_CODE")?.parse()?,
            port: std::env::var("LF_PORT")?.parse()?,
        })
    }

    async fn ensure_dir() -> Result<()> {
        tokio::fs::create_dir_all(CONFIG_DIR).await?;
        Ok(())
    }
}
