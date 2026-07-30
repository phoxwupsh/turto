use crate::ytdl::playlist::{YouTubeDlPlaylistOutput, YouTubePlaylist};
use serde::Deserialize;
use songbird::input::Input;
use std::{collections::HashMap, sync::Arc};
use url::Url;

mod cancel;
mod chunked;
mod direct;
pub mod playlist;
pub mod sidecar;
mod source;
mod tail;

#[cfg(test)]
mod test_support;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct YouTubeDl {
    #[serde(flatten)]
    inner: Arc<YoutubeDlFileInner>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct YouTubeDlMetadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub channel: Option<String>,
    pub duration: Option<f64>,
    pub filesize: Option<u64>,
    pub http_headers: Option<HashMap<String, String>>,
    pub release_date: Option<String>,
    pub thumbnail: Option<String>,
    pub title: Option<String>,
    pub track: Option<String>,
    pub upload_date: Option<String>,
    pub uploader: Option<String>,
    pub url: String,
    pub webpage_url: Option<String>,
    pub protocol: Option<String>,
    pub timestamp: Option<i64>,
    pub uploader_url: Option<String>,
    pub channel_url: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct YoutubeDlFileInner {
    url: String,
    #[serde(
        serialize_with = "serialize_oncecell_arc",
        deserialize_with = "deserialize_oncecell_arc"
    )]
    metadata: tokio::sync::OnceCell<std::sync::Arc<YouTubeDlMetadata>>,
    /// Full yt-dlp info dict from `/extract`, cached for this session
    /// only so the `/download` fallback can reuse it.
    #[serde(skip)]
    info: tokio::sync::OnceCell<std::sync::Arc<serde_json::Value>>,
    /// This track's stop signal, handed to every byte producer it starts. Session
    /// state, so a deserialized queue entry gets a fresh one.
    #[serde(skip)]
    cancel: Arc<cancel::Cancel>,
    /// The tail this track's bytes land in, once something has asked for them -- also the
    /// replay cache, since a completed tail keeps its file.
    ///
    /// Held across the extract and the first-byte open, which is what makes "one producer
    /// per track" true. It cannot deadlock against [`Drop`](Self::drop): holding the lock
    /// requires holding an `Arc`, so the refcount cannot reach zero.
    #[serde(skip)]
    bytes: tokio::sync::Mutex<Option<tail::TailHandle>>,
}

impl Drop for YoutubeDlFileInner {
    /// The last handle to this track is gone -- removed from the queue, or replaced in the
    /// playing slot -- so nobody can read its bytes again: stop fetching them. Producers
    /// hold nothing of this struct, so this really does fire mid-download.
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

fn serialize_oncecell_arc<S, T>(
    cell: &tokio::sync::OnceCell<Arc<T>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
    T: serde::ser::Serialize,
{
    let opt = cell.get().map(Arc::as_ref);
    <Option<&T> as serde::Serialize>::serialize(&opt, serializer)
}

fn deserialize_oncecell_arc<'de, D, T>(
    deserializer: D,
) -> Result<tokio::sync::OnceCell<Arc<T>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: serde::de::Deserialize<'de>,
{
    let res = match <Option<T> as serde::Deserialize>::deserialize(deserializer)? {
        Some(value) => tokio::sync::OnceCell::new_with(Some(Arc::new(value))),
        None => tokio::sync::OnceCell::new(),
    };
    Ok(res)
}

impl YouTubeDl {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(YoutubeDlFileInner {
                url: url.into(),
                metadata: tokio::sync::OnceCell::new(),
                info: tokio::sync::OnceCell::new(),
                cancel: Arc::default(),
                bytes: tokio::sync::Mutex::new(None),
            }),
        }
    }

    /// A track whose metadata is already known (a playlist entry), so the first
    /// display does not need an extract.
    pub fn new_with(url: impl Into<String>, metadata: YouTubeDlMetadata) -> Self {
        Self {
            inner: Arc::new(YoutubeDlFileInner {
                url: url.into(),
                metadata: tokio::sync::OnceCell::new_with(Some(Arc::new(metadata))),
                info: tokio::sync::OnceCell::new(),
                cancel: Arc::default(),
                bytes: tokio::sync::Mutex::new(None),
            }),
        }
    }

    pub fn has_yt_playlist(&self) -> bool {
        match Url::parse(&self.inner.url) {
            Ok(url) => match url.host_str() {
                Some("www.youtube.com")
                | Some("youtube.com")
                | Some("youtu.be")
                | Some("music.youtube.com") => url.query_pairs().any(|(k, _)| k == "list"),
                _ => false,
            },
            Err(_) => false,
        }
    }

    pub async fn fetch_yt_playlist(&self) -> Result<YouTubePlaylist, YouTubeDlError> {
        let info = sidecar::extract(self.inner.url.as_str(), true).await?;
        let output = serde_json::from_value::<YouTubeDlPlaylistOutput>(info)?;
        let yt_playlist = YouTubePlaylist {
            id: output.id,
            title: output.title,
            author: output.channel.or(output.uploader),
            url: output.webpage_url.or(output.original_url),
            entries: output.entries,
        };
        Ok(yt_playlist)
    }

    pub fn title(&self) -> Option<&str> {
        self.inner.metadata.get()?.title.as_deref()
    }

    pub fn url(&self) -> &str {
        self.inner.url.as_str()
    }

    /// Extract (once) the full yt-dlp info dict, caching it for the session so the
    /// `/download` fallback can reuse it.
    async fn fetch_info(&self) -> Result<Arc<serde_json::Value>, YouTubeDlError> {
        let val = self
            .inner
            .info
            .get_or_try_init(|| async {
                let info = sidecar::extract(self.inner.url.as_str(), false).await?;
                Ok::<_, YouTubeDlError>(Arc::new(info))
            })
            .await?;
        Ok(val.clone())
    }

    pub async fn fetch_metadata(&self) -> Result<Arc<YouTubeDlMetadata>, YouTubeDlError> {
        let val = self
            .inner
            .metadata
            .get_or_try_init(|| async {
                let info = self.fetch_info().await?;
                let ytdlp_data = YouTubeDlMetadata::deserialize(&*info)?;
                Ok::<_, YouTubeDlError>(Arc::new(ytdlp_data))
            })
            .await?;
        Ok(val.clone())
    }

    /// Start playing this track: its metadata, and an `Input` reading the bytes.
    ///
    /// Never waits for the whole download -- the returned `Input` tails the tail file,
    /// so playback starts as soon as the first bytes land (instantly if the track was
    /// already warmed or played once).
    pub async fn play(&self) -> Result<(Arc<YouTubeDlMetadata>, Input), YouTubeDlError> {
        let handle = self.tail().await?;
        let meta = self.fetch_metadata().await?;
        Ok((meta, handle.input()?))
    }

    /// Prime this track for its turn, without downloading it: the `/extract` always --
    /// multi-second, and where the failures live -- plus one chunk of bytes on the
    /// direct-HTTP path, whose producer then parks at the boundary.
    ///
    /// HLS and the sidecar `/download` have no boundary to park at, so they get the
    /// extract alone rather than a whole download of a track that may never play.
    ///
    /// Returns once the first bytes are moving, not once they have landed;
    /// [`Self::play`] attaches to the same download either way.
    pub async fn prefetch(&self) -> Result<(), YouTubeDlError> {
        let meta = self.fetch_metadata().await?;
        if !source::is_direct(&meta) {
            tracing::debug!("byte path cannot be bounded; primed the extract only");
            return Ok(());
        }
        self.tail().await?;
        Ok(())
    }

    /// Fetch this track's bytes in full, resolving once they are all local. Not what the
    /// queue does (see [`Self::prefetch`]) -- it is how the drain-to-the-end path is
    /// exercised without a voice connection.
    pub async fn warm(&self) -> Result<(), YouTubeDlError> {
        let handle = self.tail().await?;
        handle.warm().await?;
        Ok(())
    }

    /// The single way bytes are acquired -- and the point is that the first case is the
    /// common one, so a prefetch and the playback after it are the *same* download:
    ///
    /// - a live or completed tail: attach, fetching nothing.
    /// - no tail: extract, open the byte source, spawn a producer.
    /// - a *failed* tail: discard it and start over from a fresh extract, since it can
    ///   only ever hand its readers an error.
    async fn tail(&self) -> Result<tail::TailHandle, YouTubeDlError> {
        let mut slot = self.inner.bytes.lock().await;
        match slot.as_ref() {
            Some(handle) if !handle.failed() => return Ok(handle.clone()),
            Some(_) => tracing::info!("previous byte fetch failed; starting a fresh one"),
            None => {}
        }
        let handle = self.start_tail().await?;
        *slot = Some(handle.clone());
        Ok(handle)
    }

    /// Open this track's byte source and spawn the producer that drains it into a fresh
    /// tail. The eager first fetch means an unplayable track fails *here*, where a command
    /// can report it, and [`Self::start_tail_fresh`] gets one shot at a re-extract --
    /// separate from keeping a *working* fetch going, which is [`direct`]'s job.
    async fn start_tail(&self) -> Result<tail::TailHandle, YouTubeDlError> {
        let meta = self.fetch_metadata().await?;
        match self.open_tail(source::classify_source(&meta)).await {
            Ok(handle) => Ok(handle),
            Err(err) => {
                tracing::warn!(error = %err, "byte open failed; re-extracting for a fresh url");
                self.start_tail_fresh().await
            }
        }
    }

    /// The URL-expiry retry, tried once: re-extract past the session caches, then open
    /// whatever the fresh metadata names -- possibly a different byte path entirely.
    async fn start_tail_fresh(&self) -> Result<tail::TailHandle, YouTubeDlError> {
        let (meta, _) = self.re_extract().await?;
        self.open_tail(source::classify_source(&meta)).await
    }

    /// Spawn the producer for one classified byte source. Only the direct-HTTP one has a
    /// recovery loop: it alone holds a URL that expires *and* knows where to resume.
    async fn open_tail(&self, source: source::ByteSource) -> Result<tail::TailHandle, YouTubeDlError> {
        let cancel = self.inner.cancel.clone();
        match source {
            source::ByteSource::Http(req) => {
                let (fetch, reporter) =
                    direct::DirectFetch::open(self.inner.url.clone(), req, cancel.clone())
                        .await
                        .map_err(|err| YouTubeDlError::Stream(err.into()))?;
                Ok(tail::spawn_tail(cancel, Some(reporter), move |tail| {
                    fetch.run(tail)
                })?)
            }
            source::ByteSource::Hls(compose) => {
                let raw = source::open_byte_stream(compose).await?;
                Ok(tail::spawn_hls_tail(cancel, raw)?)
            }
            // No URL-expiry guard needed: the sidecar re-extracts from `webpage_url`
            // itself. Reuse the cached info dict.
            source::ByteSource::Sidecar => {
                let info = self.fetch_info().await?;
                let resp = sidecar::download(&info).await?;
                Ok(tail::spawn_sidecar_tail(cancel, resp)?)
            }
        }
    }

    /// Re-extract the info dict + typed metadata straight from the sidecar,
    /// bypassing the (possibly-expired) session caches.
    async fn re_extract(
        &self,
    ) -> Result<(Arc<YouTubeDlMetadata>, Arc<serde_json::Value>), YouTubeDlError> {
        let info = sidecar::extract(self.inner.url.as_str(), false).await?;
        let meta = YouTubeDlMetadata::deserialize(&info)?;
        Ok((Arc::new(meta), Arc::new(info)))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum YouTubeDlError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("sidecar error: {0}")]
    Sidecar(#[from] sidecar::SidecarError),
    #[error("audio stream error: {0}")]
    Stream(Box<dyn std::error::Error + Send + Sync>),
}
