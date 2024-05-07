use super::{playlist::Playlist, playlist_item::PlaylistItem};
use serde::Deserialize;
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
    vec::IntoIter,
};

#[derive(Debug, Default)]
pub struct YouTubePlaylist {
    pub id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    playlist: Vec<PlaylistItem>,
}

impl YouTubePlaylist {
    pub fn into_playlist(self) -> Playlist {
        Playlist::from(self.playlist)
    }
}

impl Deref for YouTubePlaylist {
    type Target = Vec<PlaylistItem>;
    fn deref(&self) -> &Self::Target {
        &self.playlist
    }
}

impl DerefMut for YouTubePlaylist {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.playlist
    }
}

impl IntoIterator for YouTubePlaylist {
    type Item = PlaylistItem;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.playlist.into_iter()
    }
}

#[derive(Deserialize)]
pub struct Output {
    pub id: Option<String>,
    pub title: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub channel: Option<String>,
    pub uploader: Option<String>,
    pub channel_url: Option<String>,
    pub uploader_url: Option<String>,
    pub entries: Vec<Entry>,
    pub webpage_url: Option<String>,
    pub original_url: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct Thumbnail {
    pub url: Option<String>,
    pub height: Option<f64>,
    pub width: Option<f64>,
}

#[derive(Deserialize)]
pub struct Entry {
    pub id: Option<String>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub channel: Option<String>,
    pub channel_url: Option<String>,
    pub uploader: Option<String>,
    pub uploader_url: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

impl From<Entry> for PlaylistItem {
    fn from(value: Entry) -> Self {
        PlaylistItem {
            title: value.title.unwrap_or_default(),
            url: value.url.unwrap_or_default(),
            channel: value.channel.or(value.uploader).unwrap_or_default(),
            duration: value
                .duration
                .map(Duration::from_secs_f64)
                .unwrap_or_default(),
            thumbnail: value
                .thumbnails
                .last()
                .cloned()
                .and_then(|thumbnail| thumbnail.url)
                .unwrap_or_default(),
        }
    }
}

impl Output {
    pub fn to_playlist(self) -> YouTubePlaylist {
        YouTubePlaylist {
            id: self.id,
            title: self.title,
            author: self.channel.or(self.uploader),
            url: self.webpage_url.or(self.original_url),
            playlist: self.entries.into_iter().map(PlaylistItem::from).collect(),
        }
    }
}
