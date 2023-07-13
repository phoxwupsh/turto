use std::ops::{Deref, DerefMut};

use serde::{Serialize, Deserialize};

use crate::error::TurtoError;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct  GuildVolume(f32);

impl Default for GuildVolume {
    fn default() -> Self {
        GuildVolume(1.0_f32)
    }
}

impl Deref for GuildVolume {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GuildVolume {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<f32> for GuildVolume {
    type Error = TurtoError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !(0.0_f32..=1.0_f32).contains(&value) {
            return Err(TurtoError::VolumeError);
        }
        Ok(GuildVolume(value))
    }
}

impl TryFrom<u32> for GuildVolume {
    type Error = TurtoError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let vf = (value as f32) / 100.0_f32;
        Self::try_from(vf)
    }
}

impl From<GuildVolume> for i32 {
    fn from(val: GuildVolume) -> Self {
        (val.0 * 100.0_f32) as i32
    }
}
