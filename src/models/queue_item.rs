use std::sync::Arc;

use crate::{
    models::config::YtdlpConfig,
    ytdl::{YouTubeDl, playlist::YouTubePlaylist},
};
use anyhow::Result;
use url::Url;

pub struct QueueItem {
    url: Url,
    ytdlp_config: Arc<YtdlpConfig>,
}

pub enum QueueItemKind {
    Single(YouTubeDl),
    Playlist(YouTubePlaylist),
}

impl QueueItem {
    pub fn new(url: Url, ytdlp_config: Arc<YtdlpConfig>) -> Self {
        Self { url, ytdlp_config }
    }

    pub async fn query(self) -> Result<QueueItemKind> {
        let ytdl = YouTubeDl::new(self.url.as_str(), self.ytdlp_config);

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
