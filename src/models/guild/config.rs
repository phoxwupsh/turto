use std::collections::HashSet;

use super::volume::GuildVolume;
use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GuildConfig {
    pub auto_leave: bool,
    pub volume: GuildVolume,
    pub banned: HashSet<UserId>
}

impl Default for GuildConfig {
    fn default() -> Self {
        GuildConfig {
            auto_leave: true,
            volume: GuildVolume::default(),
            banned: HashSet::default()
        }
    }
}
