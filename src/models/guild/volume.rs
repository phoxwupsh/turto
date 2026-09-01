use crate::{models::error::CommandError, utils::misc::ToEmoji};
use serde::{Deserialize, Deserializer, Serialize};
use std::ops::Deref;

/// A playback volume, always within `0.0..=1.0`.
///
/// The range is the invariant: every way of building one -- [`TryFrom`] for command
/// input, [`Deserialize`] for stored data -- goes through a check, and nothing hands
/// out a `&mut f32` that could put an out-of-range value back in.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GuildVolume(f32);

impl Default for GuildVolume {
    fn default() -> Self {
        GuildVolume(1.0_f32)
    }
}

/// Read-only: no `DerefMut`, since `&mut f32` would let any caller store a value
/// outside `0.0..=1.0` and bypass the [`TryFrom`] range check that is this
/// newtype's whole purpose.
impl Deref for GuildVolume {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Read a stored volume, **clamping** anything outside `0.0..=1.0` instead of failing.
///
/// Deliberately lenient, unlike [`TryFrom<f32>`]: a hand-edited or corrupted volume in
/// `guilds.json` would otherwise fail the whole file's deserialization, and the loader
/// treats that as "no data at all" -- every guild's queue, ban list and settings
/// discarded, then written back over the file as empty at shutdown. One bad number is
/// not worth the database.
impl<'de> Deserialize<'de> for GuildVolume {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = f32::deserialize(deserializer)?;
        // `clamp` propagates NaN (and panics on a NaN bound), so it cannot answer for
        // a value that is not a number at all; that falls back to the default.
        let value = if raw.is_nan() {
            *Self::default()
        } else {
            raw.clamp(0.0_f32, 1.0_f32)
        };
        // `!=` is true for NaN too, so a NaN is reported rather than passing silently.
        if value != raw {
            tracing::warn!(raw, value, "stored volume is out of range; clamped");
        }
        Ok(Self(value))
    }
}

/// Strict, unlike [`Deserialize`]: this is the command path, where an out-of-range
/// request is the user's mistake to be told about rather than data to be salvaged.
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

#[cfg(test)]
mod test {
    use super::GuildVolume;

    fn de(json: &str) -> GuildVolume {
        serde_json::from_str(json).expect("a numeric volume must always deserialize")
    }

    #[test]
    fn a_stored_volume_in_range_is_kept() {
        assert_eq!(*de("0.0"), 0.0);
        assert_eq!(*de("0.42"), 0.42);
        assert_eq!(*de("1.0"), 1.0);
    }

    /// The case that motivated this: a hand-edited `guilds.json` must not reach
    /// songbird with a volume it never validated.
    #[test]
    fn a_stored_volume_out_of_range_is_clamped() {
        assert_eq!(*de("500.0"), 1.0);
        assert_eq!(*de("-0.5"), 0.0);
    }

    /// A clamp cannot answer for NaN, so it falls back to the default rather than
    /// propagating one into playback. Fed straight to the impl, since JSON has no
    /// way to spell NaN.
    #[test]
    fn a_stored_nan_falls_back_to_the_default() {
        use serde::{Deserialize, de::IntoDeserializer, de::value::Error};

        let nan = GuildVolume::deserialize(IntoDeserializer::<Error>::into_deserializer(f32::NAN))
            .expect("a NaN must be salvaged, not rejected");
        assert_eq!(*nan, *GuildVolume::default());
    }

    /// Whatever a clamp let through must still satisfy the newtype's own check.
    #[test]
    fn every_deserialized_volume_satisfies_try_from() {
        for json in ["-1e9", "-0.5", "0.0", "0.5", "1.0", "1.5", "1e9"] {
            let volume = de(json);
            assert!(
                GuildVolume::try_from(*volume).is_ok(),
                "deserializing {json} produced {volume:?}, which TryFrom rejects"
            );
        }
    }
}
