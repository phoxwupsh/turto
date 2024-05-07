use super::{playlist_item::PlaylistItem, youtube_playlist::YouTubePlaylist};
use crate::utils::{get_http_client, url::UrlExt, ytdl::ytdl_playlist};
use songbird::input::{Compose, YoutubeDl};
use anyhow::Result;
use url::Url;

pub struct QueueItem {
    url: Url,
}

pub enum QueueItemKind {
    Single(PlaylistItem),
    Playlist(YouTubePlaylist),
}

impl QueueItem {
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    pub async fn query(self) -> Result<QueueItemKind> {
        if self.url.is_yt_playlist() {
            Ok(ytdl_playlist(&self.url)
                .await
                .map(QueueItemKind::Playlist)?)
        } else {
            Ok(YoutubeDl::new(get_http_client(), self.url.to_string())
                .aux_metadata()
                .await
                .map(PlaylistItem::from)
                .map(QueueItemKind::Single)?)
        }
    }
}
