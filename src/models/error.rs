use crate::ytdl::YouTubeDlError;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("serenity error: {0}")]
    Serenity(#[from] serenity::Error),

    #[error("songbird error: {0}")]
    Songbird(#[from] songbird::error::ControlError),

    #[error("join error: {0}")]
    Join(#[from] songbird::error::JoinError),

    #[error("ytdl error: {0}")]
    YouTubeDl(#[from] YouTubeDlError),

    #[error("invalid operation")]
    InvalidOperation { cause: &'static str },
}

impl CommandError {
    pub fn cause(&self) -> &'static str {
        match self {
            Self::Serenity(_) => "serenity",
            Self::Songbird(_) => "playing track",
            Self::Join(_) => "voice channel",
            Self::YouTubeDl(ytdl_err) => match ytdl_err {
                YouTubeDlError::Io(_) => "ytdl",
                YouTubeDlError::Json(_) => "ytdl json",
            },
            Self::InvalidOperation { cause } => cause,
        }
    }
}
