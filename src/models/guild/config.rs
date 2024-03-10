use crate::models::{autoleave::AutoleaveType, guild::volume::GuildVolume};
use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildConfig {
    pub auto_leave: AutoleaveType,
    pub repeat: bool,
    pub volume: GuildVolume,
    pub banned: HashSet<UserId>,
}

impl Default for GuildConfig {
    fn default() -> Self {
        GuildConfig {
            auto_leave: AutoleaveType::On,
            repeat: false,
            volume: GuildVolume::default(),
            banned: HashSet::default(),
        }
    }
}
