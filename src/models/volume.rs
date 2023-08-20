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

impl TryFrom<u32> for GuildVolume {
    type Error = VolumeError;
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

impl ToEmoji for GuildVolume {
    fn to_emoji(&self) -> String {
        let num = i32::from(*self);
        let num_str = num.to_string();
        let mut emoji_str = String::new();

        if num < 0 {
            emoji_str.push('➖');
        }

        for ch in num_str.chars() {
            let emoji = match ch {
                '0' => "0️⃣",
                '1' => "1️⃣",
                '2' => "2️⃣",
                '3' => "3️⃣",
                '4' => "4️⃣",
                '5' => "5️⃣",
                '6' => "6️⃣",
                '7' => "7️⃣",
                '8' => "8️⃣",
                '9' => "9️⃣",
                _ => continue,
            };
            emoji_str.push_str(emoji);
        }
        emoji_str
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