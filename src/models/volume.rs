use std::{ops::{Deref, DerefMut}, fmt::Display, error::Error};

use serde::{Serialize, Deserialize};

use crate::utils::misc::ToEmoji;

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

#[derive(Debug)]
pub enum VolumeError {
    OutOfRange
}

impl Display for VolumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VolumeError::OutOfRange => f.write_str("GuildVolume should be between 0.0 ~ 1.0")
        }
    }
}

impl Error for VolumeError {}