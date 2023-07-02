use super::volume::GuildVolume;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildSetting {
    pub volume: GuildVolume,
}

impl Default for GuildSetting {
    fn default() -> Self {
        GuildSetting {
            volume: GuildVolume::default(),
        }
    }
}
