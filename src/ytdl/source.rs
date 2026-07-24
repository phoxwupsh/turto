//! Byte-source acquisition for a resolved yt-dlp format: classify the format into
//! the byte path that serves it ([`classify_source`]), then realize that path --
//! open a raw stream ([`open_byte_stream`]), download the whole track to a tempfile
//! ([`download_to_file`]/[`sidecar_download_to_file`]), or tee a live stream into
//! the replay cache ([`TeeToCache`]). The paced/tailed live paths live in
//! [`super::tail`]; this module is the non-tail byte plumbing.

use super::{YouTubeDlError, YouTubeDlMetadata, YoutubeDlFileInner, chunked};
use crate::utils::get_http_client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use songbird::input::{Compose, HlsRequest};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
    sync::Arc,
};
use symphonia::core::io::MediaSource;
use tempfile::tempfile;
use tokio::io::AsyncWriteExt;

/// The byte path chosen for a resolved format.
pub(super) enum ByteSource {
    /// Direct http(s): the chunked, resume-capable source ([`chunked`]).
    Http(chunked::ChunkedHttpRequest),
    /// HLS: songbird's segment-based [`HlsRequest`].
    Hls(Box<dyn Compose + Send>),
    /// No single fetchable URL (DASH segments, SABR/no-url): the sidecar
    /// `/download` fallback.
    Sidecar,
}

/// Classify the resolved format into the byte path that serves it.
pub(super) fn classify_source(meta: &YouTubeDlMetadata) -> ByteSource {
    let headers = build_header_map(&meta.http_headers);
    let client = get_http_client();
    match meta.protocol.as_deref() {
        Some("m3u8_native") | Some("m3u8") => ByteSource::Hls(Box::new(
            HlsRequest::new_with_headers(client, meta.url.clone(), headers),
        )),
        // Direct http(s): the chunked source dodges googlevideo's >10 MB throttle;
        // `filesize` lets it stop exactly at EOF.
        Some("https") | Some("http") if !meta.url.is_empty() => ByteSource::Http(
            chunked::ChunkedHttpRequest::new(client, meta.url.clone(), headers, meta.filesize),
        ),
        _ => ByteSource::Sidecar,
    }
}

fn build_header_map(headers: &Option<HashMap<String, String>>) -> HeaderMap {
    let mut map = HeaderMap::new();
    if let Some(headers) = headers {
        for (key, value) in headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                map.insert(name, val);
            }
        }
    }
    map
}

/// Open a songbird `Compose` into its raw byte stream.
pub(super) async fn open_byte_stream(
    mut compose: Box<dyn Compose + Send>,
) -> Result<Box<dyn MediaSource>, YouTubeDlError> {
    let audio = compose
        .create_async()
        .await
        .map_err(|err| YouTubeDlError::Stream(err.into()))?;
    Ok(audio.input)
}

/// Download a whole byte stream into a (rewound) tempfile.
pub(super) async fn download_to_file(
    compose: Box<dyn Compose + Send>,
) -> Result<std::fs::File, YouTubeDlError> {
    let mut raw = open_byte_stream(compose).await?;
    let file = tokio::task::spawn_blocking(move || -> std::io::Result<std::fs::File> {
        let mut tmp = tempfile()?;
        std::io::copy(&mut raw, &mut tmp)?;
        tmp.seek(SeekFrom::Start(0))?;
        Ok(tmp)
    })
    .await
    .map_err(std::io::Error::other)??;
    Ok(file)
}

/// Download the whole sidecar `/download` body into a (rewound) tempfile.
pub(super) async fn sidecar_download_to_file(
    mut resp: reqwest::Response,
) -> Result<std::fs::File, YouTubeDlError> {
    let tmp = tokio::fs::File::from_std(tempfile()?);
    let mut writer = tokio::io::BufWriter::new(tmp);
    while let Some(bytes) = resp
        .chunk()
        .await
        .map_err(|err| YouTubeDlError::Stream(err.into()))?
    {
        writer.write_all(&bytes).await?;
    }
    writer.flush().await?;

    let mut file = writer.into_inner().into_std().await;
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}

/// A `Read` adapter that tees every byte it yields into a tempfile; on clean
/// EOF the completed file is promoted into the replay cache.
pub(super) struct TeeToCache {
    inner: Box<dyn MediaSource>,
    writer: Option<std::io::BufWriter<std::fs::File>>,
    cache: Arc<YoutubeDlFileInner>,
    done: bool,
}

impl TeeToCache {
    pub(super) fn new(
        inner: Box<dyn MediaSource>,
        file: std::fs::File,
        cache: Arc<YoutubeDlFileInner>,
    ) -> Self {
        Self {
            inner,
            writer: Some(std::io::BufWriter::new(file)),
            cache,
            done: false,
        }
    }

    fn finalize(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        let Some(mut writer) = self.writer.take() else {
            return;
        };
        let finished = writer
            .flush()
            .and_then(|()| {
                writer
                    .into_inner()
                    .map_err(std::io::IntoInnerError::into_error)
            })
            .and_then(|mut file| file.seek(SeekFrom::Start(0)).map(|_| file));
        match finished {
            Ok(file) => {
                let _ = self.cache.file.set(file);
                tracing::info!("byte cache complete");
            }
            Err(err) => tracing::warn!(error = %err, "failed to finalize byte cache"),
        }
    }
}

impl Read for TeeToCache {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 {
            self.finalize();
        } else if let Some(writer) = self.writer.as_mut()
            && writer.write_all(&buf[..n]).is_err()
        {
            // Stop caching on a write error, but keep playback going.
            self.writer = None;
        }
        Ok(n)
    }
}
