pub mod help;
pub mod message_template;

use crate::models::config::TurtoConfig;
use std::{fs, sync::OnceLock};

pub struct TurtoConfigProvider;

impl TurtoConfigProvider {
    pub fn get() -> &'static TurtoConfig {
        static CONFIG: OnceLock<TurtoConfig> = OnceLock::new();
        CONFIG.get_or_init(|| {
            fs::read_to_string("config.toml")
                .map_err(|err| panic!("Error loading config.toml: {err}"))
                .and_then(|config_toml| toml::from_str::<TurtoConfig>(&config_toml))
                .unwrap_or_else(|err| panic!("Error parsing config.toml: {err}"))
        })
    }
}
