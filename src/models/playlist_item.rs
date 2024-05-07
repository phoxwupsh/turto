use serde::{Deserialize, Serialize};
use songbird::input::AuxMetadata;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistItem {
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: Duration,
    pub thumbnail: String,
}

impl From<AuxMetadata> for PlaylistItem {
    fn from(value: AuxMetadata) -> Self {
        PlaylistItem {
            url: value.source_url.unwrap_or_default(),
            title: value.title.unwrap_or_default(),
            channel: value.channel.unwrap_or_default(),
            duration: value.duration.unwrap_or_default(),
            thumbnail: value.thumbnail.unwrap_or_default(),
        }
    }
}