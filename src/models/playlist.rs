use super::playlist_item::PlaylistItem;
use serde::{Deserialize, Serialize};
use std::{
    collections::{vec_deque::IntoIter, VecDeque},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist(VecDeque<PlaylistItem>);

impl Playlist {
    pub fn new() -> Self {
        Playlist(VecDeque::<PlaylistItem>::new())
    }
}

impl Deref for Playlist {
    type Target = VecDeque<PlaylistItem>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Playlist {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Playlist {
    type Item = PlaylistItem;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
