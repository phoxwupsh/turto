use serde::{Deserialize, Serialize};
use std::{fs, sync::OnceLock};

#[derive(Debug, Serialize, Deserialize)]
pub struct TurtoConfig {
    pub command_prefix: String,
    pub allow_seek: bool,
    pub allow_backward_seek: bool,
    pub seek_limit: u64,
    pub command_delay: u64,
}

impl TurtoConfig {
    pub fn get_config() -> &'static Self {
        static CONFIG: OnceLock<TurtoConfig> = OnceLock::new();
        CONFIG.get_or_init(|| {
            fs::read_to_string("config.toml")
                .map_err(|err| panic!("Error loading config.toml: {err}"))
                .and_then(|config_toml| toml::from_str::<TurtoConfig>(&config_toml))
                .unwrap_or_else(|err| panic!("Error parsing config.toml: {err}"))
        })
    }
}
