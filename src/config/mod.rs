pub mod help;
pub mod message_template;

use crate::models::config::TurtoConfig;
use anyhow::{Context, Result};
use std::{fs, path::Path, sync::OnceLock};
use tracing::warn;

static CONFIG: OnceLock<TurtoConfig> = OnceLock::new();

pub fn get_config() -> &'static TurtoConfig {
    CONFIG.get().unwrap()
}

pub fn load_config(config_path: impl AsRef<Path>) -> Result<()> {
    let config = fs::read_to_string(config_path.as_ref())
        .context(format!(
            "Failed to load config from {}",
            config_path.as_ref().display()
        ))
        .and_then(|config_toml| {
            toml::from_str::<TurtoConfig>(&config_toml).context("Failed to parse config")
        })?;
    if config.owner.is_none() {
        warn!("The owner of this bot hasn't been set");
    }

    CONFIG.set(config).unwrap();

    Ok(())
}
