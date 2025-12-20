use crate::{models::config::YtdlpConfig, ytdl::YouTubeDl};
use serde::{Deserialize, Serialize};
use std::{
    collections::{VecDeque, vec_deque::IntoIter},
    ops::{Deref, RangeBounds},
    sync::Arc,
};

const PAGE_SIZE: usize = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist(VecDeque<YouTubeDl>);

impl Playlist {
    pub fn new() -> Self {
        Playlist(VecDeque::<YouTubeDl>::new())
    }

    pub fn total_pages(&self) -> usize {
        self.0.len().div_ceil(PAGE_SIZE)
    }

    pub fn page_with_indices(&self, index: usize) -> Option<Vec<(usize, &YouTubeDl)>> {
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

    fn prefetch_first(&self, ytdlp_config: Arc<YtdlpConfig>) {
        if let Some(first) = self.0.front() {
            tokio::spawn(prefetch(first.clone(), ytdlp_config));
        }
    }

    pub fn pop_front_prefetch(&mut self, ytdlp_config: Arc<YtdlpConfig>) -> Option<YouTubeDl> {
        let front = self.0.pop_front()?;
        self.prefetch_first(ytdlp_config);
        Some(front)
    }

    pub fn pop_back_prefetch(&mut self, ytdlp_config: Arc<YtdlpConfig>) -> Option<YouTubeDl> {
        let back = self.0.pop_back()?;
        self.prefetch_first(ytdlp_config);
        Some(back)
    }

    pub fn push_front_prefetch(&mut self, value: YouTubeDl, ytdlp_config: Arc<YtdlpConfig>) {
        self.0.push_front(value);
        self.prefetch_first(ytdlp_config);
    }

    pub fn push_back_prefetch(&mut self, value: YouTubeDl, ytdlp_config: Arc<YtdlpConfig>) {
        self.0.push_back(value);
        self.prefetch_first(ytdlp_config);
    }

    pub fn extend_prefetch<I>(&mut self, iter: I, ytdlp_config: Arc<YtdlpConfig>)
    where
        I: IntoIterator<Item = YouTubeDl>,
    {
        self.0.extend(iter);
        self.prefetch_first(ytdlp_config);
    }

    pub fn make_contiguous(&mut self) -> &mut [YouTubeDl] {
        self.0.make_contiguous()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn remove_prefetch(
        &mut self,
        index: usize,
        ytdlp_config: Arc<YtdlpConfig>,
    ) -> Option<YouTubeDl> {
        let removed = self.0.remove(index);
        self.prefetch_first(ytdlp_config);
        removed
    }

    pub fn drain_prefetch<R>(&mut self, range: R, ytdlp_config: Arc<YtdlpConfig>) -> Vec<YouTubeDl>
    where
        R: RangeBounds<usize>,
    {
        let drain = self.0.drain(range).collect();
        self.prefetch_first(ytdlp_config);
        drain
    }
}

impl Deref for Playlist {
    type Target = VecDeque<YouTubeDl>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn prefetch(next: YouTubeDl, ytdlp_config: Arc<YtdlpConfig>) {
    tracing::info!(url = next.url(), "start prefetch next track");
    if let Err(err) = next.fetch_file(ytdlp_config).await {
        tracing::warn!(error = ?err, url = next.url(), "prefetch next track failed");
    } else {
        tracing::info!(url = next.url(), "prefetch next track success");
    }
}
// impl DerefMut for Playlist {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

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
