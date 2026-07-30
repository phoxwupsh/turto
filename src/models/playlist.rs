use crate::ytdl::YouTubeDl;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::{VecDeque, vec_deque::IntoIter},
    ops::{Deref, RangeBounds},
};
use tracing::{Instrument, instrument};

/// The guild's play queue.
///
/// Every mutation that can change the front primes it in the background, so the
/// next track is ready to play the moment it is popped. That holds only while
/// *every* mutation goes through these methods, hence no `DerefMut` and nothing
/// handing out a `&mut` into the wrapped [`VecDeque`].
///
/// Deserializing a saved queue deliberately does not prime: nothing plays at
/// startup, so it would cost an extract per guild for nothing.
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

    /// Shuffle the queue in place, priming whatever lands at the front.
    pub fn shuffle(&mut self) {
        self.0.make_contiguous().shuffle(&mut thread_rng());
        self.prefetch_front();
    }

    /// The one mutation with nothing to prime: it leaves no front.
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

    /// Prime the current front so it is ready to play when popped. Spawns a
    /// background [`prefetch`] under the current span (the triggering event), so
    /// the fetch is traced under its cause; a no-op on an empty queue.
    fn prefetch_front(&self) {
        if let Some(front) = self.0.front() {
            tokio::spawn(prefetch(front.clone()).in_current_span());
        }
    }
}

/// Prime a track ahead of its turn: its extract, and a bounded head start on its bytes
/// where the byte path allows one. The playback that follows attaches to this same
/// download, so an unfinished prefetch is a head start, never duplicated work.
///
/// "Primed" is weaker than "downloaded" — this resolves once the first bytes are moving.
/// The producer is spawned from here, so how the rest of the download ends still reports
/// under this span.
#[instrument(name = "prefetch", skip_all, fields(url = %next.url()))]
async fn prefetch(next: YouTubeDl) {
    tracing::info!("prefetch started");
    match next.prefetch().await {
        Ok(()) => tracing::info!("prefetch primed"),
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
