//! Byte-source acquisition for a resolved yt-dlp format: classify the format into
//! the byte path that serves it ([`classify_source`]), then open it
//! ([`open_byte_stream`] for the paths that go through a songbird `Compose`). Every
//! byte the bot fetches lands in a tail file owned by [`super::tail`]; this module
//! is only the classification and the raw open.

use super::{YouTubeDlError, YouTubeDlMetadata, chunked};
use crate::utils::get_http_client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use songbird::input::{Compose, HlsRequest};
use std::collections::HashMap;
use symphonia::core::io::MediaSource;

/// The byte path chosen for a resolved format.
pub(super) enum ByteSource {
    /// Direct http(s): byte-range chunks ([`chunked`]), driven by
    /// [`super::direct`]'s recovering producer.
    Http(chunked::ChunkRequest),
    /// HLS: songbird's segment-based [`HlsRequest`].
    Hls(Box<dyn Compose + Send>),
    /// No single fetchable URL (DASH segments, SABR/no-url): the sidecar
    /// `/download` fallback.
    Sidecar,
}

impl ByteSource {
    /// A short static label for the chosen path, for logging.
    fn label(&self) -> &'static str {
        match self {
            ByteSource::Http(_) => "http",
            ByteSource::Hls(_) => "hls",
            ByteSource::Sidecar => "sidecar",
        }
    }
}

/// Is this format served by plain byte-range HTTP -- [`ByteSource::Http`]?
///
/// Its own question, separate from [`classify_source`], because two callers need the
/// answer without wanting the source: [`direct_url`] (is a *re*-extracted format still
/// resumable in place?) and prefetch (this is the only path whose read-ahead can be
/// bounded, since it is the only one with chunk boundaries to pause at).
pub(super) fn is_direct(meta: &YouTubeDlMetadata) -> bool {
    matches!(meta.protocol.as_deref(), Some("https" | "http")) && !meta.url.is_empty()
}

/// Classify the resolved format into the byte path that serves it.
pub(super) fn classify_source(meta: &YouTubeDlMetadata) -> ByteSource {
    let headers = build_header_map(&meta.http_headers);
    let client = get_http_client();
    let source = match meta.protocol.as_deref() {
        Some("m3u8_native" | "m3u8") => ByteSource::Hls(Box::new(
            HlsRequest::new_with_headers(client, meta.url.clone(), headers),
        )),
        // Direct http(s): the chunked source dodges googlevideo's >10 MB throttle;
        // `filesize` lets it stop exactly at EOF.
        _ if is_direct(meta) => ByteSource::Http(chunked::ChunkRequest::new(
            client,
            meta.url.clone(),
            headers,
            meta.filesize,
        )),
        _ => ByteSource::Sidecar,
    };
    // Never logs `meta.url` (the signed URL); the watch URL comes from the span.
    tracing::debug!(
        protocol = meta.protocol.as_deref().unwrap_or("none"),
        source = source.label(),
        "classified byte source"
    );
    source
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

/// The fresh signed URL + headers for a format that is *still* direct http(s).
///
/// `None` once the format has moved to a path that cannot be resumed in place (HLS,
/// or SABR/no-url), which is why [`super::direct`]'s refresh can fail: a mid-download
/// flip to SABR means the remaining bytes are simply not fetchable by range request,
/// and the track has to end.
pub(super) fn direct_url(meta: &YouTubeDlMetadata) -> Option<(String, HeaderMap)> {
    is_direct(meta).then(|| (meta.url.clone(), build_header_map(&meta.http_headers)))
}
