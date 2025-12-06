use crate::{
    models::{config::YtdlpConfig, playlist::Playlist},
    ytdl::{YouTubeDl, YouTubeDlMetadata},
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct YouTubePlaylist {
    pub id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    pub entries: Vec<YouTubeDlMetadata>,
    pub(super) config: Arc<YtdlpConfig>,
}

impl YouTubePlaylist {
    pub fn to_playlist(self) -> Playlist {
        Playlist::from(self.to_vec())
    }

    pub fn to_vec(self) -> Vec<YouTubeDl> {
        self.entries
            .into_iter()
            .map(|metadata| {
                YouTubeDl::new_with(
                    metadata.webpage_url.clone().unwrap_or_default(),
                    self.config.clone(),
                    None,
                    metadata,
                )
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct YouTubeDlPlaylistOutput {
    pub id: Option<String>,
    pub title: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub channel: Option<String>,
    pub uploader: Option<String>,
    pub channel_url: Option<String>,
    pub uploader_url: Option<String>,
    pub entries: Vec<YouTubeDlMetadata>,
    pub webpage_url: Option<String>,
    pub original_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Thumbnail {
    pub url: Option<String>,
    pub height: Option<f64>,
    pub width: Option<f64>,
}
