use crate::ytdl::playlist::{YouTubeDlPlaylistOutput, YouTubePlaylist};
use serde::Deserialize;
use songbird::input::{AudioStream, Compose, Input, LiveInput};
use std::{
    collections::HashMap,
    future::Future,
    io::{Seek, SeekFrom},
    pin::Pin,
    sync::Arc,
};
use symphonia::core::io::{
    MediaSource, MediaSourceStream, MediaSourceStreamOptions, ReadOnlySource,
};
use tempfile::tempfile;
use tracing::instrument;
use url::Url;

mod chunked;
pub mod playlist;
pub mod sidecar;
mod source;
mod tail;

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
    #[serde(skip)]
    file: tokio::sync::OnceCell<std::fs::File>,
    #[serde(
        serialize_with = "serialize_oncecell_arc",
        deserialize_with = "deserialize_oncecell_arc"
    )]
    metadata: tokio::sync::OnceCell<std::sync::Arc<YouTubeDlMetadata>>,
    /// Full yt-dlp info dict from `/extract`, cached for this session
    /// only so the `/download` fallback can reuse it.
    #[serde(skip)]
    info: tokio::sync::OnceCell<std::sync::Arc<serde_json::Value>>,
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
                file: tokio::sync::OnceCell::new(),
                metadata: tokio::sync::OnceCell::new(),
                info: tokio::sync::OnceCell::new(),
            }),
        }
    }

    pub fn new_with(
        url: impl Into<String>,
        file: Option<std::fs::File>,
        metadata: YouTubeDlMetadata,
    ) -> Self {
        Self {
            inner: Arc::new(YoutubeDlFileInner {
                url: url.into(),
                file: tokio::sync::OnceCell::new_with(file),
                metadata: tokio::sync::OnceCell::new_with(Some(Arc::new(metadata))),
                info: tokio::sync::OnceCell::new(),
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

    pub async fn fetch_file(&self) -> Result<Input, YouTubeDlError> {
        let file = self
            .inner
            .file
            .get_or_try_init(|| async {
                // Warm-extract, then download the whole track to a tempfile.
                let meta = self.fetch_metadata().await?;
                // A queued track's resolved URL can expire before prefetch reaches
                // it, so retry once with a fresh extract on the first byte-open
                // failure. Http and Hls both download through a `Compose`; only the
                // sidecar path differs, so it returns early and the rest is shared.
                let compose: Box<dyn Compose + Send> = match source::classify_source(&meta) {
                    source::ByteSource::Http(req) => Box::new(req),
                    source::ByteSource::Hls(compose) => compose,
                    source::ByteSource::Sidecar => {
                        let info = self.fetch_info().await?;
                        let resp = sidecar::download(&info).await?;
                        return source::sidecar_download_to_file(resp).await;
                    }
                };
                match source::download_to_file(compose).await {
                    Ok(file) => Ok(file),
                    Err(err) => {
                        tracing::warn!(error = %err, url = %self.inner.url, "prefetch byte open failed; re-extracting for a fresh url");
                        self.retry_download_to_file().await
                    }
                }
            })
            .await?;

        let mut res = file.try_clone()?;
        res.seek(SeekFrom::Start(0))?;

        let input = Input::Live(
            LiveInput::Wrapped(AudioStream {
                input: MediaSourceStream::new(Box::new(res), MediaSourceStreamOptions::default()),
            }),
            None,
        );

        Ok(input)
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

    #[instrument(skip_all, fields(url = self.inner.url))]
    pub async fn play(
        &self,
    ) -> Result<
        (
            Pin<Box<dyn Future<Output = Result<Arc<YouTubeDlMetadata>, YouTubeDlError>> + Send>>,
            Input,
        ),
        YouTubeDlError,
    > {
        if let Some(file) = self.inner.file.get() {
            let mut file = file.try_clone()?;
            file.seek(SeekFrom::Start(0))?;

            let input = Input::Live(
                LiveInput::Wrapped(AudioStream {
                    input: MediaSourceStream::new(
                        Box::new(file),
                        MediaSourceStreamOptions::default(),
                    ),
                }),
                None,
            );
            let self_inner = self.clone();
            let meta_fut = async move { self_inner.fetch_metadata().await };
            return Ok((Box::pin(meta_fut), input));
        }

        // The googlevideo URL is time-limited (~6 h) and a track can sit in the
        // queue past that, so the eager first fetch surfaces an expired URL (403)
        // here and the retry re-extracts a fresh one.
        let meta = self.fetch_metadata().await?;
        let input = match source::classify_source(&meta) {
            // Direct HTTP: tail a paced background download.
            source::ByteSource::Http(req) => match req.open_source().await {
                Ok(paced) => self.http_live_input(paced)?,
                Err(err) => {
                    tracing::warn!(error = %err, url = %self.inner.url, "byte open failed; re-extracting for a fresh url");
                    self.retry_live_input().await?
                }
            },
            // HLS: tee songbird's adapter into the cache tempfile.
            source::ByteSource::Hls(compose) => match source::open_byte_stream(compose).await {
                Ok(raw) => self.tee_live_input(raw)?,
                Err(err) => {
                    tracing::warn!(error = %err, url = %self.inner.url, "byte open failed; re-extracting for a fresh url");
                    self.retry_live_input().await?
                }
            },
            // No URL-expiry guard needed: the sidecar re-extracts from `webpage_url`
            // itself. Reuse the cached info dict.
            source::ByteSource::Sidecar => {
                let info = self.fetch_info().await?;
                let resp = sidecar::download(&info).await?;
                self.sidecar_live_input(resp)?
            }
        };

        let meta_fut = async move { Ok::<_, YouTubeDlError>(meta) };
        Ok((Box::pin(meta_fut), input))
    }

    /// Wrap a freshly opened byte stream as a live `Input`, teeing bytes into a
    /// tempfile that is promoted to the replay cache on clean EOF.
    fn tee_live_input(&self, raw: Box<dyn MediaSource>) -> Result<Input, YouTubeDlError> {
        let tee = source::TeeToCache::new(raw, tempfile()?, self.inner.clone());
        Ok(Input::Live(
            LiveInput::Wrapped(AudioStream {
                input: MediaSourceStream::new(
                    Box::new(ReadOnlySource::new(tee)),
                    MediaSourceStreamOptions::default(),
                ),
            }),
            None,
        ))
    }

    /// Sidecar `/download` live path: tail a background drain of the response.
    fn sidecar_live_input(&self, resp: reqwest::Response) -> Result<Input, YouTubeDlError> {
        Ok(tail::sidecar_tail(self.inner.clone(), resp)?)
    }

    /// Direct-HTTP chunked live path: tail a background drain of the paced source.
    fn http_live_input(&self, paced: chunked::PacedSource) -> Result<Input, YouTubeDlError> {
        Ok(tail::http_tail(self.inner.clone(), paced)?)
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

    /// URL-expiry retry for [`Self::play`]: re-extract fresh, then build a live
    /// input. A format that has flipped to SABR/no-url on the fresh extract falls
    /// through to the sidecar `/download` path.
    async fn retry_live_input(&self) -> Result<Input, YouTubeDlError> {
        let (meta, info) = self.re_extract().await?;
        match source::classify_source(&meta) {
            source::ByteSource::Http(req) => {
                let paced = req
                    .open_source()
                    .await
                    .map_err(|err| YouTubeDlError::Stream(err.into()))?;
                self.http_live_input(paced)
            }
            source::ByteSource::Hls(compose) => {
                let raw = source::open_byte_stream(compose).await?;
                self.tee_live_input(raw)
            }
            source::ByteSource::Sidecar => {
                let resp = sidecar::download(&info).await?;
                self.sidecar_live_input(resp)
            }
        }
    }

    /// URL-expiry retry for [`Self::fetch_file`]: re-extract fresh, then download
    /// the whole track. Mirrors [`Self::retry_live_input`] but yields a rewound
    /// tempfile for the replay cache.
    async fn retry_download_to_file(&self) -> Result<std::fs::File, YouTubeDlError> {
        let (meta, info) = self.re_extract().await?;
        match source::classify_source(&meta) {
            source::ByteSource::Http(req) => source::download_to_file(Box::new(req)).await,
            source::ByteSource::Hls(compose) => source::download_to_file(compose).await,
            source::ByteSource::Sidecar => {
                let resp = sidecar::download(&info).await?;
                source::sidecar_download_to_file(resp).await
            }
        }
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
