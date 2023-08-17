use std::time::Duration;

use serde::{Serialize,Deserialize
};
use songbird::input::Metadata;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaylistItem{
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: Duration,
    pub thumbnail: String
}

impl From<Metadata> for PlaylistItem {
    fn from(value: Metadata) -> Self {
        PlaylistItem { 
            url: value.source_url.unwrap_or_default(), 
            title: value.title.unwrap_or_default(), 
            channel: value.channel.unwrap_or_default(), 
            duration: value.duration.unwrap_or_default(), 
            thumbnail: value.thumbnail.unwrap_or_default() 
        }
    }
}
