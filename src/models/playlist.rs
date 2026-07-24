use crate::ytdl::YouTubeDl;
use serde::{Deserialize, Serialize};
use std::{
    collections::{VecDeque, vec_deque::IntoIter},
    ops::{Deref, RangeBounds},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist(VecDeque<YouTubeDl>);

impl Playlist {
    pub fn new() -> Self {
        Playlist(VecDeque::<YouTubeDl>::new())
    }

    fn prefetch_first(&self) {
        if let Some(first) = self.0.front() {
            tokio::spawn(prefetch(first.clone()));
        }
    }

    pub fn pop_front_prefetch(&mut self) -> Option<YouTubeDl> {
        let front = self.0.pop_front()?;
        self.prefetch_first();
        Some(front)
    }

    pub fn pop_back_prefetch(&mut self) -> Option<YouTubeDl> {
        let back = self.0.pop_back()?;
        self.prefetch_first();
        Some(back)
    }

    pub fn push_front_prefetch(&mut self, value: YouTubeDl) {
        self.0.push_front(value);
        self.prefetch_first();
    }

    pub fn push_back_prefetch(&mut self, value: YouTubeDl) {
        self.0.push_back(value);
        self.prefetch_first();
    }

    pub fn extend_prefetch<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = YouTubeDl>,
    {
        self.0.extend(iter);
        self.prefetch_first();
    }

    pub fn make_contiguous(&mut self) -> &mut [YouTubeDl] {
        self.0.make_contiguous()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn remove_prefetch(&mut self, index: usize) -> Option<YouTubeDl> {
        let removed = self.0.remove(index);
        self.prefetch_first();
        removed
    }

    pub fn drain_prefetch<R>(&mut self, range: R) -> Vec<YouTubeDl>
    where
        R: RangeBounds<usize>,
    {
        let drain = self.0.drain(range).collect();
        self.prefetch_first();
        drain
    }
}

impl Deref for Playlist {
    type Target = VecDeque<YouTubeDl>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn prefetch(next: YouTubeDl) {
    tracing::info!(url = next.url(), "start prefetch next track");
    if let Err(err) = next.fetch_file().await {
        tracing::warn!(error = ?err, url = next.url(), "prefetch next track failed");
    } else {
        tracing::info!(url = next.url(), "prefetch next track success");
    }
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Playlist {
    type Item = YouTubeDl;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<YouTubeDl>> for Playlist {
    fn from(value: Vec<YouTubeDl>) -> Self {
        Self(VecDeque::from(value))
    }
}
