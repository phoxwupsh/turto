use super::volume::GuildVolume;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildSetting {
    pub auto_leave: bool,
    pub volume: GuildVolume,
}

impl Default for GuildSetting {
    fn default() -> Self {
        GuildSetting {
            auto_leave: true,
            volume: GuildVolume::default(),
        }
    }
}
