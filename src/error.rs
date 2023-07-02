use std::{error::Error, fmt::Display};

use songbird::{
    input::error::Error as InputError,
    tracks::TrackError
};

#[derive(Debug)]
pub enum TurtoError {
    EmptyPlaylist,
    TrackError(TrackError),
    InputError(InputError),
    InternalError(&'static str),
    VolumeError
}

impl Display for TurtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPlaylist => f.write_str("The playlist is empty"),
            Self::TrackError(e) => f.write_str(e.to_string().as_str()),
            Self::InputError(e) => f.write_str(e.to_string().as_str()),
            Self::InternalError(s) => f.write_str(s),
            Self::VolumeError => f.write_str("Volume out of range")
        }
    }
}

impl Error for TurtoError {}
