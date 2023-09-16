use std::{sync::OnceLock, fs};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TurtoConfig {
    pub command_prefix: String,
    pub allow_seek: bool,
    pub allow_backward_seek: bool,
    pub seek_limit: u64,
    pub command_delay: u64
}

impl TurtoConfig {
    pub fn get_config() -> &'static Self {
        static CONFIG: OnceLock<TurtoConfig> = OnceLock::new();
        CONFIG.get_or_init(|| {
            let config_toml = fs::read_to_string("config.toml").expect("Error loading helps.json");
            toml::from_str::<TurtoConfig>(&config_toml).expect("Error parsing helps.json")
        })
    }
}