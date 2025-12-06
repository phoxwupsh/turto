use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;
use std::{path::Path, sync::Arc};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub struct TurtoConfig {
    pub allow_seek: bool,
    pub allow_backward_seek: bool,
    pub seek_limit: u64,
    pub command_delay: u64,
    pub owner: Option<UserId>,
    pub auto_save: bool,
    pub auto_save_interval: u64,

    #[serde(default)]
    pub ytdlp: Arc<YtdlpConfig>,
}

impl TurtoConfig {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config_str = std::fs::read_to_string(path.as_ref())?;
        let config = toml::from_str::<Self>(&config_str)?;
        if config.owner.is_none() {
            warn!("The owner of this bot hasn't been set");
        }
        Ok(config)
    }

    pub fn is_owner(&self, user: &UserId) -> bool {
        if let Some(owner) = &self.owner {
            return owner == user;
        }
        false
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YtdlpConfig {
    pub use_system_ytdlp: bool,
    pub use_nightly: bool,
    pub use_system_deno: bool,
    pub cookies_path: Option<String>,
}

impl Default for YtdlpConfig {
    fn default() -> Self {
        Self {
            use_system_ytdlp: false,
            use_nightly: false,
            use_system_deno: false,
            cookies_path: None,
        }
    }
}
