use super::{playlist_item::PlaylistItem, url::ParsedUrl, youtube_playlist::YouTubePlaylist};
use crate::utils::{get_http_client, ytdl::ytdl_playlist};
use songbird::input::{Compose, YoutubeDl};

pub struct QueueItem {
    url: ParsedUrl,
}

pub enum QueueItemKind {
    Single(PlaylistItem),
    Playlist(YouTubePlaylist),
}

impl QueueItem {
    pub fn new(url: ParsedUrl) -> Self {
        Self { url }
    }

    pub async fn query(self) -> Option<QueueItemKind> {
        match self.url {
            ParsedUrl::Youtube(yt_url) => {
                if yt_url.is_playlist() {
                    ytdl_playlist(&yt_url.to_string()).map(QueueItemKind::Playlist)
                } else {
                    query_single(yt_url.to_string()).await
                }
            }
            ParsedUrl::Other(url) => query_single(url).await,
        }
    }
}

async fn query_single(url: String) -> Option<QueueItemKind> {
    let mut source = YoutubeDl::new(get_http_client(), url);
    source
        .aux_metadata()
        .await
        .map(PlaylistItem::from)
        .map(QueueItemKind::Single)
        .ok()
}
