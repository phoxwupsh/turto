use crate::ytdl::YouTubeDl;
use serde::{Deserialize, Serialize};
use std::{
    collections::{VecDeque, vec_deque::IntoIter},
    ops::{Deref, RangeBounds},
};
use tracing::{Instrument, instrument};

/// The guild's play queue.
///
/// Every mutation that can change the front warms it in the background, so the
/// next track is ready to play the moment it is popped.
/// Coupling the prefetch to the mutation makes it impossible to forget and models
/// it as what it is: a side effect of the queue changing. The spawned prefetch
/// inherits the span of whatever event triggered the mutation (a command, a track
/// end), so it is traced under its cause.
#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist(VecDeque<YouTubeDl>);

impl Playlist {
    pub fn new() -> Self {
        Playlist(VecDeque::<YouTubeDl>::new())
    }

    pub fn pop_front(&mut self) -> Option<YouTubeDl> {
        let front = self.0.pop_front();
        self.prefetch_front();
        front
    }

    pub fn pop_back(&mut self) -> Option<YouTubeDl> {
        let back = self.0.pop_back();
        self.prefetch_front();
        back
    }

    pub fn push_front(&mut self, value: YouTubeDl) {
        self.0.push_front(value);
        self.prefetch_front();
    }

    pub fn push_back(&mut self, value: YouTubeDl) {
        self.0.push_back(value);
        self.prefetch_front();
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = YouTubeDl>,
    {
        self.0.extend(iter);
        self.prefetch_front();
    }

    pub fn make_contiguous(&mut self) -> &mut [YouTubeDl] {
        self.0.make_contiguous()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn remove(&mut self, index: usize) -> Option<YouTubeDl> {
        let removed = self.0.remove(index);
        self.prefetch_front();
        removed
    }

    pub fn drain<R>(&mut self, range: R) -> Vec<YouTubeDl>
    where
        R: RangeBounds<usize>,
    {
        let drained = self.0.drain(range).collect();
        self.prefetch_front();
        drained
    }

    /// Warm the current front so it is ready to play when popped. Spawns a
    /// background [`prefetch`] under the current span (the triggering event), so
    /// the fetch is traced under its cause; a no-op on an empty queue.
    fn prefetch_front(&self) {
        if let Some(front) = self.0.front() {
            tokio::spawn(prefetch(front.clone()).in_current_span());
        }
    }
}

/// Fetch a track to its replay tempfile ahead of playback.
#[instrument(name = "prefetch", skip_all, fields(url = %next.url()))]
async fn prefetch(next: YouTubeDl) {
    tracing::info!("prefetch started");
    match next.fetch_file().await {
        Ok(_) => tracing::info!("prefetch complete"),
        Err(err) => tracing::warn!(error = %err, "prefetch failed"),
    }
}

impl Deref for Playlist {
    type Target = VecDeque<YouTubeDl>;
    fn deref(&self) -> &Self::Target {
        &self.0
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
