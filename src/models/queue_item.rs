use crate::ytdl::{YouTubeDl, playlist::YouTubePlaylist};
use anyhow::Result;
use url::Url;

pub struct QueueItem {
    url: Url,
}

pub enum QueueItemKind {
    Single(YouTubeDl),
    Playlist(YouTubePlaylist),
}

impl QueueItem {
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    pub async fn query(self) -> Result<QueueItemKind> {
        let ytdl = YouTubeDl::new(self.url.as_str());

        if ytdl.has_yt_playlist() {
            Ok(ytdl
                .fetch_yt_playlist()
                .await
                .map(QueueItemKind::Playlist)?)
        } else {
            Ok(QueueItemKind::Single(ytdl))
        }
    }
}
