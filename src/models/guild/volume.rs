use crate::{models::error::CommandError, utils::misc::ToEmoji};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct GuildVolume(f32);

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
    type Error = VolumeError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !(0.0_f32..=1.0_f32).contains(&value) {
            return Err(VolumeError::OutOfRange);
        }
        Ok(GuildVolume(value))
    }
}

impl TryFrom<usize> for GuildVolume {
    type Error = VolumeError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if !(0_usize..=100_usize).contains(&value) {
            return Err(VolumeError::OutOfRange);
        }
        let vf = (value as f32) / 100.0_f32;
        Self::try_from(vf)
    }
}

impl From<GuildVolume> for usize {
    fn from(val: GuildVolume) -> Self {
        (val.0 * 100.0_f32) as usize
    }
}

impl ToEmoji for GuildVolume {
    fn to_emoji(&self) -> String {
        let num = usize::from(*self);
        num.to_emoji()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VolumeError {
    #[error("volume should be between 0.0 ~ 1.0")]
    OutOfRange,
}

impl From<VolumeError> for CommandError {
    fn from(value: VolumeError) -> Self {
        match value {
            VolumeError::OutOfRange => CommandError::InvalidOperation {
                cause: "volume should be between 0.0 ~ 1.0",
            },
        }
    }
}
