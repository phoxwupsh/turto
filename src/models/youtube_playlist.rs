use super::{playlist::Playlist, playlist_item::PlaylistItem};
use std::{
    collections::vec_deque::IntoIter,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct YouTubePlaylist {
    pub title: Option<String>,
    pub author: Option<String>,
    pub playlist_id: Option<String>,
    playlist: Playlist,
}

impl YouTubePlaylist {
    pub fn new() -> Self {
        Self {
            title: None,
            author: None,
            playlist_id: None,
            playlist: Playlist::new(),
        }
    }
}

impl Default for YouTubePlaylist {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for YouTubePlaylist {
    type Target = Playlist;
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
