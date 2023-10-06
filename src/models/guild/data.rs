use super::config::GuildConfig;
use crate::models::playlist::Playlist;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GuildData {
    pub config: GuildConfig,
    pub playlist: Playlist,
}
