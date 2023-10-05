use crate::models::playlist::Playlist;
use super::config::GuildConfig;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GuildData {
    pub config: GuildConfig,
    pub playlist: Playlist
}