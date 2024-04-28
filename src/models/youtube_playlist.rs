use super::{playlist::Playlist, playlist_item::PlaylistItem};
use std::{
    ops::{Deref, DerefMut},
    vec::IntoIter,
};

#[derive(Debug, Default)]
pub struct YouTubePlaylist {
    pub title: Option<String>,
    pub author: Option<String>,
    pub playlist_id: Option<String>,
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
