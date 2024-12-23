use super::playlist_item::PlaylistItem;
use serde::{Deserialize, Serialize};
use std::{
    collections::{vec_deque::IntoIter, VecDeque},
    ops::{Deref, DerefMut},
};

const PAGE_SIZE: usize = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist(VecDeque<PlaylistItem>);

impl Playlist {
    pub fn new() -> Self {
        Playlist(VecDeque::<PlaylistItem>::new())
    }

    pub fn total_pages(&self) -> usize {
        self.0.len().div_ceil(PAGE_SIZE)
    }

    pub fn page_with_indices(&self, index: usize) -> Option<Vec<(usize, &PlaylistItem)>> {
        if index > self.total_pages() {
            return None;
        }
        let start = (index - 1) * PAGE_SIZE;
        let res = self
            .0
            .iter()
            .enumerate()
            .skip(start)
            .take(PAGE_SIZE)
            .collect();
        Some(res)
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

impl From<Vec<PlaylistItem>> for Playlist {
    fn from(value: Vec<PlaylistItem>) -> Self {
        Self(VecDeque::from(value))
    }
}