//! The HLS byte producer: walk a media playlist and append its segments to the
//! track's tail file.
//!
//! Segments are appended raw and in order, each preceded by its `#EXT-X-MAP`
//! initialization section when the playlist names one. Ordered pieces with a pace gate
//! at their boundaries, so the structure mirrors [`super::direct`].
//!
//! A growing playlist is re-polled at its own `target_duration`, never faster than
//! [`MIN_POLL_INTERVAL`], until `#EXT-X-ENDLIST` or [`STALE_AFTER`] without a new
//! segment.
//!
//! Nothing here recovers: any failure ends the track, and the next play extracts afresh.

use super::{
    cancel::Cancel,
    chunked::{self, PaceGate, PaceReporter},
    tail::TailWriter,
};
use hls_m3u8::{
    MasterPlaylist, MediaPlaylist,
    tags::VariantStream,
    types::{ByteRange, MediaType},
};
use reqwest::{
    Client, StatusCode,
    header::{AUTHORIZATION, COOKIE, HeaderMap, RANGE},
};
use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::timeout;
use url::Url;

#[cfg(test)]
mod test;

/// How far ahead of consumption the walker may run: roughly two minutes of opus.
const READ_AHEAD: u64 = 2 * 1024 * 1024;

/// Ceiling on one request, and on the gap between two body chunks of the same response.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a playlist may add nothing before the stream is declared over. A playlist
/// that never publishes `#EXT-X-ENDLIST` has no other end marker.
const STALE_AFTER: Duration = Duration::from_secs(60);

/// Floor under the re-poll interval, since a playlist may declare a `target_duration` of
/// zero, or omit it, which would otherwise spin.
const MIN_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Slack allowed between an `#EXTINF` and the playlist's own `#EXT-X-TARGETDURATION`
/// before the parser rejects the playlist. Real-world playlists overshoot their target.
const ALLOWABLE_EXCESS: Duration = Duration::from_secs(10);

/// The tag that distinguishes a master playlist from a media playlist.
const VARIANT_TAG: &str = "#EXT-X-STREAM-INF";

/// Why an HLS walk could not start, or could not continue.
#[derive(Debug, thiserror::Error)]
pub(super) enum HlsError {
    #[error("hls url is unusable: {0}")]
    Url(#[from] url::ParseError),
    #[error("hls playlist fetch failed: {0}")]
    Fetch(#[source] io::Error),
    #[error("hls playlist cannot be walked: {0}")]
    Playlist(String),
    /// Refused rather than fetched; decryption is not implemented.
    #[error("hls segments are encrypted with {0}, which is not supported")]
    Encrypted(String),
}

impl HlsError {
    /// Whether this is a property of the stream itself, which no re-extract can change.
    pub(super) fn is_stream_property(&self) -> bool {
        matches!(self, Self::Encrypted(_) | Self::Playlist(_))
    }
}

impl From<HlsError> for io::Error {
    fn from(err: HlsError) -> Self {
        io::Error::other(err)
    }
}

/// Everything one track's HLS walk needs. Plain data: building it fetches nothing.
pub(super) struct HlsRequest {
    client: Client,
    /// The playlist URL, still text, because parsing it can fail.
    url: String,
    headers: HeaderMap,
}

impl HlsRequest {
    pub(super) fn new(client: Client, url: String, headers: HeaderMap) -> Self {
        Self {
            client,
            url,
            headers,
        }
    }
}

/// An `#EXT-X-MAP` initialization section: the header an fMP4 segment cannot be parsed
/// without. Written to the tail once, ahead of the first segment naming it.
#[derive(Clone, PartialEq, Eq)]
struct Init {
    url: Url,
    range: Option<(u64, u64)>,
}

/// One segment to fetch, owned so the [`MediaPlaylist`] borrowing the response body
/// can be dropped with it.
struct Segment {
    /// Absolute media-sequence number: the identity a live playlist re-lists a segment
    /// under, and so what makes "already fetched" decidable.
    seq: usize,
    url: Url,
    /// Inclusive `Range` bounds for an `#EXT-X-BYTERANGE` segment; `None` fetches the
    /// whole resource.
    range: Option<(u64, u64)>,
    init: Option<Init>,
}

/// One parsed playlist: what to fetch now, and what to do once it is fetched.
struct Snapshot {
    /// Segments numbered at or past the walker's cursor, in playlist order.
    fresh: Vec<Segment>,
    /// The playlist's own first sequence number. Past our cursor, it means the live
    /// window slid while we were not fetching.
    first_seq: usize,
    target_duration: Duration,
    /// `#EXT-X-ENDLIST`: the playlist is final.
    ended: bool,
}

/// The figures a walk runs on, so a test can dial them down.
#[derive(Clone, Copy)]
struct Tuning {
    read_ahead: u64,
    stale_after: Duration,
    min_poll: Duration,
}

impl Tuning {
    const LIVE: Self = Self {
        read_ahead: READ_AHEAD,
        stale_after: STALE_AFTER,
        min_poll: MIN_POLL_INTERVAL,
    };
}

/// Walks one track's playlist into its tail file.
pub(super) struct HlsFetch {
    client: Client,
    /// The media playlist's URL, which is also the base every segment URI resolves
    /// against.
    url: Url,
    headers: HeaderMap,
    /// Host the credentials in [`Self::headers`] were minted for: that of the URL yt-dlp
    /// handed over. See [`strip_foreign_credentials`].
    credentials_host: Option<String>,
    tuning: Tuning,
    pace: PaceGate,
    /// The track's stop signal, held so a skip cuts the re-poll wait short.
    cancel: Arc<Cancel>,
    /// Sequence number of the next segment to fetch.
    next_seq: usize,
    /// The initialization section already written, so a playlist that repeats one
    /// `#EXT-X-MAP` on every segment fetches it once.
    init: Option<Init>,
    /// The playlist [`Self::open`] already fetched. [`Self::run`] walks this one first
    /// and polls for its own after that.
    opened: Option<Snapshot>,
}

impl HlsFetch {
    /// Open the walk, eagerly, so a dead or unwalkable playlist fails *here* where a
    /// command can report it. Also returns the [`PaceReporter`] for the tail's readers.
    pub(super) async fn open(
        req: HlsRequest,
        cancel: Arc<Cancel>,
    ) -> Result<(Self, PaceReporter), HlsError> {
        Self::open_with(req, cancel, Tuning::LIVE).await
    }

    /// [`Self::open`] against explicit [`Tuning`].
    async fn open_with(
        req: HlsRequest,
        cancel: Arc<Cancel>,
        tuning: Tuning,
    ) -> Result<(Self, PaceReporter), HlsError> {
        let HlsRequest {
            client,
            url,
            headers,
        } = req;
        let handed = Url::parse(&url)?;
        let credentials_host = handed.host_str().map(str::to_owned);
        let (url, text) = resolve_media_playlist(&client, handed, &headers).await?;
        let opened = parse(&url, &text, 0)?;
        let (gate, reporter) = chunked::pace_channel(tuning.read_ahead, cancel.clone());
        Ok((
            Self {
                client,
                url,
                headers,
                credentials_host,
                tuning,
                pace: gate,
                cancel,
                // On a live stream this is the oldest segment still in the window.
                next_seq: opened.first_seq,
                init: None,
                opened: Some(opened),
            },
            reporter,
        ))
    }

    /// Walk the playlist into `tail` until the stream ends.
    ///
    /// `Ok(())` is a clean end -- `#EXT-X-ENDLIST`, a playlist that stopped growing, or
    /// a cancellation. `Err` means the track ends early, and reaches the reader as a
    /// read error rather than a short file.
    pub(super) async fn run(mut self, mut tail: TailWriter) -> io::Result<()> {
        // When the playlist stopped growing.
        let mut stale_since: Option<Instant> = None;
        loop {
            let snapshot = match self.opened.take() {
                Some(snapshot) => snapshot,
                None => self.poll().await?,
            };
            // Every segment in the new window is past our cursor already, so resuming
            // needs no help; the discontinuity it leaves in the tail is worth a line.
            if snapshot.first_seq > self.next_seq {
                tracing::warn!(
                    missed = snapshot.first_seq - self.next_seq,
                    "live window slid past the fetcher; resuming at its new start"
                );
            }
            let (target, ended) = (snapshot.target_duration, snapshot.ended);
            let grew = !snapshot.fresh.is_empty();

            for segment in snapshot.fresh {
                // Pace between segments, the one point where no response body is open.
                // `false` means the track was dropped while we waited.
                if !self.pace.await_room(tail.written()).await {
                    return Ok(());
                }
                self.write_segment(&segment, &mut tail).await?;
                // A cancel lands mid-segment, leaving a partial one in the tail; a
                // cancelled tail is published as failed, so it is never read as a track.
                if tail.is_cancelled() {
                    return Ok(());
                }
                self.next_seq = segment.seq + 1;
            }

            if ended {
                return Ok(());
            }
            if grew {
                stale_since = None;
            } else {
                let idle = stale_since.get_or_insert_with(Instant::now).elapsed();
                if idle >= self.tuning.stale_after {
                    tracing::info!(?idle, "playlist stopped growing; ending the stream");
                    return Ok(());
                }
            }
            // A growing playlist gains a segment roughly every target duration.
            if !self.wait_to_repoll(target).await {
                return Ok(());
            }
        }
    }

    /// Re-fetch the playlist and parse it against the walker's current cursor.
    async fn poll(&self) -> Result<Snapshot, HlsError> {
        let text = fetch_text(&self.client, &self.url, &self.headers_for(&self.url)).await?;
        parse(&self.url, &text, self.next_seq)
    }

    /// The headers to send to one URL, with anything credential-bearing dropped when
    /// that URL is off [`Self::credentials_host`].
    fn headers_for(&self, url: &Url) -> HeaderMap {
        strip_foreign_credentials(&self.headers, self.credentials_host.as_deref(), url)
    }

    /// Append one segment to the tail, preceded by its initialization section the
    /// first time that section is named.
    async fn write_segment(&mut self, segment: &Segment, tail: &mut TailWriter) -> io::Result<()> {
        if let Some(init) = &segment.init
            && self.init.as_ref() != Some(init)
        {
            self.fetch_into("initialization section", &init.url, init.range, tail)
                .await?;
            self.init = Some(init.clone());
        }
        tracing::debug!(seq = segment.seq, "fetching segment");
        self.fetch_into("segment", &segment.url, segment.range, tail)
            .await
    }

    /// Stream one resource -- a segment, or an initialization section -- into the tail.
    /// The cancel check is per body chunk, so a skip lands *inside* a segment.
    ///
    /// A `range` is held to exactly the bytes it asked for: an `#EXT-X-BYTERANGE`
    /// playlist names several segments inside one resource, so anything more or less
    /// corrupts the reassembly rather than failing it.
    async fn fetch_into(
        &self,
        what: &'static str,
        url: &Url,
        range: Option<(u64, u64)>,
        tail: &mut TailWriter,
    ) -> io::Result<()> {
        let mut request = self.client.get(url.clone()).headers(self.headers_for(url));
        if let Some((start, end)) = range {
            request = request.header(RANGE, format!("bytes={start}-{end}"));
        }
        let mut resp = timeout(REQUEST_TIMEOUT, request.send())
            .await
            .map_err(|_| timed_out(what, "request"))?
            .map_err(io::Error::other)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(io::Error::other(format!(
                "{what} request failed with status {status}"
            )));
        }
        let wanted = match range {
            // Only a 206 says the range was honoured; any other success is a body we
            // cannot place in the stream.
            Some((start, end)) if status != StatusCode::PARTIAL_CONTENT => {
                return Err(io::Error::other(format!(
                    "{what} request for bytes={start}-{end} was answered {status}, not 206"
                )));
            }
            Some((start, end)) => Some(end - start + 1),
            None => None,
        };

        let mut written = 0;
        while !tail.is_cancelled() {
            let chunk = timeout(REQUEST_TIMEOUT, resp.chunk())
                .await
                .map_err(|_| timed_out(what, "body"))?
                .map_err(io::Error::other)?;
            let Some(bytes) = chunk else { break };
            // Truncate rather than fail: the bytes up to the boundary are the ones the
            // playlist named, and they still tile the resource.
            let bytes = match wanted {
                Some(wanted) if written + bytes.len() as u64 > wanted => {
                    tracing::warn!(what, "server over-ran the requested range; truncating");
                    &bytes[..(wanted - written) as usize]
                }
                _ => &bytes[..],
            };
            written += bytes.len() as u64;
            tail.write(bytes).await?;
            if wanted == Some(written) {
                break;
            }
        }
        // A short body leaves a hole, which puts every later segment at the wrong
        // offset. Cancellation is the one expected short read.
        if let Some(wanted) = wanted
            && written < wanted
            && !tail.is_cancelled()
        {
            return Err(io::Error::other(format!(
                "{what} range request delivered {written} of {wanted} bytes"
            )));
        }
        Ok(())
    }

    /// Wait one floored target duration before re-polling, or return `false` at once
    /// when the track is dropped rather than leaving the walker asleep through the wait.
    async fn wait_to_repoll(&self, target: Duration) -> bool {
        // Build the waiter *before* the check, so a cancel landing in between is still
        // observed.
        let cancelled = self.cancel.notified();
        if self.cancel.is_cancelled() {
            return false;
        }
        tokio::select! {
            _ = cancelled => false,
            _ = tokio::time::sleep(target.max(self.tuning.min_poll)) => true,
        }
    }
}

/// `headers` as they should be sent to `url`: whole on the host they were minted for,
/// without `Cookie`/`Authorization` anywhere else. The rest -- `User-Agent`, `Referer` --
/// is what makes a multi-host CDN serve us at all, so it travels.
fn strip_foreign_credentials(
    headers: &HeaderMap,
    credentials_host: Option<&str>,
    url: &Url,
) -> HeaderMap {
    let mut headers = headers.clone();
    if url.host_str() != credentials_host {
        headers.remove(COOKIE);
        headers.remove(AUTHORIZATION);
    }
    headers
}

fn timed_out(what: &str, stage: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!("{what} {stage} timed out after {REQUEST_TIMEOUT:?}"),
    )
}

/// Fetch a playlist as text. One timeout covers request and body, a playlist being a
/// small document.
async fn fetch_text(client: &Client, url: &Url, headers: &HeaderMap) -> Result<String, HlsError> {
    let fetch = async {
        let resp = client
            .get(url.clone())
            .headers(headers.clone())
            .send()
            .await
            .map_err(io::Error::other)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(io::Error::other(format!(
                "playlist request failed with status {status}"
            )));
        }
        resp.text().await.map_err(io::Error::other)
    };
    timeout(REQUEST_TIMEOUT, fetch)
        .await
        .unwrap_or_else(|_| Err(timed_out("playlist", "fetch")))
        .map_err(HlsError::Fetch)
}

/// Fetch `url`, following a master playlist to one of its variants, so the walker
/// always starts from a media playlist and the URL its segment URIs resolve against.
///
/// One hop is enough: HLS does not allow a variant URI to name another master.
async fn resolve_media_playlist(
    client: &Client,
    url: Url,
    headers: &HeaderMap,
) -> Result<(Url, String), HlsError> {
    let text = fetch_text(client, &url, headers).await?;
    if !text.contains(VARIANT_TAG) {
        return Ok((url, text));
    }
    let variant = {
        let master = MasterPlaylist::try_from(text.as_str())
            .map_err(|err| HlsError::Playlist(err.to_string()))?;
        pick_variant(&master)
            .ok_or_else(|| {
                HlsError::Playlist("master playlist names no playable variant".to_owned())
            })?
            .to_owned()
    };
    let media = url.join(&variant)?;
    tracing::debug!("followed a master playlist to one of its variants");
    let headers = strip_foreign_credentials(headers, url.host_str(), &media);
    let text = fetch_text(client, &media, &headers).await?;
    Ok((media, text))
}

/// The playlist to follow out of a master: the cheapest rendition, preferring one that
/// declares no video, redirected to its separate audio playlist when it has one.
///
/// The bot decodes the audio and discards the rest, and `RESOLUTION` is the spec's own
/// signal that a rendition carries video at all.
fn pick_variant<'a>(master: &'a MasterPlaylist<'a>) -> Option<&'a str> {
    let (.., variant, uri) = master
        .variant_streams
        .iter()
        .filter_map(|variant| match variant {
            // An I-frame rendition is a thumbnail track; it carries no audio.
            VariantStream::ExtXIFrame { .. } => None,
            VariantStream::ExtXStreamInf {
                uri, stream_data, ..
            } => Some((
                stream_data.resolution().is_some(),
                stream_data.bandwidth(),
                variant,
                uri.as_ref(),
            )),
        })
        .min_by_key(|(has_video, bandwidth, ..)| (*has_video, *bandwidth))?;
    Some(demuxed_audio(master, variant).unwrap_or(uri))
}

/// The playlist holding a variant's audio, when the variant does not hold it itself.
///
/// A `TYPE=AUDIO` tag with no `URI` describes audio already inside the variant, so there
/// is nothing to follow. Where a group offers several renditions, `DEFAULT=YES` is the
/// author's own answer to which to play.
fn demuxed_audio<'a>(
    master: &'a MasterPlaylist<'a>,
    variant: &'a VariantStream<'a>,
) -> Option<&'a str> {
    master
        .associated_with(variant)
        .filter(|media| media.media_type == MediaType::Audio && media.uri().is_some())
        .min_by_key(|media| !media.is_default)
        .and_then(|media| media.uri())
        .map(|uri| uri.as_ref())
}

/// Parse one playlist into what the walker needs, dropping every segment numbered before
/// `next_seq`. Segments are converted to owned data so the borrowed [`MediaPlaylist`] can
/// be dropped here.
fn parse(base: &Url, text: &str, next_seq: usize) -> Result<Snapshot, HlsError> {
    let playlist = MediaPlaylist::builder()
        .allowable_excess_duration(ALLOWABLE_EXCESS)
        .parse(text)
        .map_err(|err| HlsError::Playlist(err.to_string()))?;

    let mut fresh = Vec::new();
    for (_, segment) in playlist.segments.iter() {
        if segment.number() < next_seq {
            continue;
        }
        // `METHOD=NONE` parses to a *keyless* key -- how a playlist turns encryption back
        // off mid-stream -- so only a real key means these bytes are encrypted.
        if let Some(key) = segment.keys.iter().find_map(|key| key.as_ref()) {
            return Err(HlsError::Encrypted(key.method.to_string()));
        }
        let init = segment
            .map
            .as_ref()
            .map(|map| {
                Ok::<_, HlsError>(Init {
                    url: base.join(map.uri())?,
                    range: map.range().as_ref().and_then(bounds),
                })
            })
            .transpose()?;
        fresh.push(Segment {
            seq: segment.number(),
            // `join`, so an absolute-path URI replaces the playlist's path.
            url: base.join(segment.uri())?,
            range: segment.byte_range.as_ref().and_then(|range| bounds(range)),
            init,
        });
    }

    Ok(Snapshot {
        fresh,
        first_seq: playlist.media_sequence,
        target_duration: playlist.target_duration,
        ended: playlist.has_end_list,
    })
}

/// Inclusive `Range` bounds for one `#EXT-X-BYTERANGE`. The parser has already resolved
/// an omitted offset from the preceding segment, so a missing start means the first range
/// of a resource. A zero-length range counts as no range at all.
fn bounds(range: &ByteRange) -> Option<(u64, u64)> {
    let start = range.start().unwrap_or(0) as u64;
    let len = range.len() as u64;
    (len > 0).then(|| (start, start + len - 1))
}
