//! Chunked HTTP byte fetching for the direct-play path.
//!
//! googlevideo throttles any single request larger than ~10 MB, so a whole-file
//! range request on a long track crawls. [`ChunkRequest`] fetches the media as
//! sequential closed byte-ranges of at most [`HTTP_CHUNK`] and presents their
//! concatenation as one stream. Because the chunks are adjacent ranges of the same
//! file, the decoder sees byte-identical output, with no seam at a boundary.
//!
//! This module is *only* the fetch. It has no opinion on what to do when one fails:
//! failures come out typed as [`FetchError`], and the recovery policy lives with the
//! code that can actually act on them ([`super::direct`]) -- only that layer knows
//! the track's watch URL and how to re-extract it.
//!
//! - The first range is fetched eagerly, so a dead URL surfaces at open time instead
//!   of mid-playback; later ranges are fetched lazily as the consumer advances.
//! - The fetcher is held to a bounded read-ahead by a [`PaceGate`]/[`PaceReporter`]
//!   pair, gating only *between* chunks -- never mid-response, which would trickle a
//!   connection at playback rate and invite googlevideo's connection reaper. With
//!   nothing reading yet -- a queued track's prefetch -- the read-ahead is *nothing*:
//!   one chunk in hand, and park.

use super::cancel::Cancel;
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt, stream};
use reqwest::{
    Client, StatusCode,
    header::{CONTENT_RANGE, HeaderMap, RANGE},
};
use std::{
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio::sync::Notify;

#[cfg(test)]
mod test;

/// Maximum bytes per range request. googlevideo throttles single requests larger
/// than ~10 MB; this matches yt-dlp's own `http_chunk_size`.
pub(super) const HTTP_CHUNK: u64 = 10 * 1024 * 1024;

/// The media as a stream of byte chunks, in order and gap-free.
pub(super) type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, FetchError>> + Send>>;

/// Why a range fetch failed. The distinction is the whole point of this type: each case
/// needs a different remedy, and only the caller can apply any of them.
#[derive(Debug, thiserror::Error)]
pub(super) enum FetchError {
    /// The server rejected the request in a way only a *fresh URL* can fix -- how
    /// googlevideo answers an expired or IP-rebound signature. Retrying the same URL
    /// is guaranteed to fail again.
    #[error("range request rejected with status {0}")]
    Rejected(StatusCode),
    /// A transport error, a 5xx, or a range that delivered no bytes: worth another
    /// attempt against the same URL.
    #[error("range fetch failed: {0}")]
    Transport(#[from] io::Error),
    /// A `416` at an offset the format says is inside the file: the server's object is
    /// *shorter* than we were told, which can only happen if the URL and the length
    /// describe different representations. Nothing to retry and nothing to re-extract --
    /// resuming in place is what is impossible. Reported rather than treated as a clean
    /// end, which would publish a short tail as the whole track and cache it.
    #[error("range from {at} refused but the format declares {total} bytes (server: {declared:?})")]
    Truncated {
        at: u64,
        total: u64,
        /// The server's own view of the length, from the `Content-Range: bytes */N` a
        /// 416 should carry. `None` if it sent none.
        declared: Option<u64>,
    },
}

impl From<FetchError> for io::Error {
    fn from(err: FetchError) -> Self {
        io::Error::other(err)
    }
}

/// Shared state behind the [`PaceGate`]/[`PaceReporter`] pair. Split into two
/// typed halves (via [`pace_channel`]) so the fetcher can only *wait* and the
/// reader can only *report*.
#[derive(Debug)]
struct PaceInner {
    /// Absolute byte offset playback has consumed so far.
    consumed: AtomicU64,
    /// Has anything actually read a byte yet? Set by the first
    /// [`advance`](PaceReporter::advance) and never cleared: a download has one
    /// beginning, and later readers of the same tail are not a fresh start.
    started: AtomicBool,
    /// Signalled by the reader on each advance; the fetcher parks on it when too
    /// far ahead.
    signal: Notify,
    /// The track's cancel signal. Stopping is a property of the track's lifetime,
    /// not of the pacer, so the gate only *reads* it -- see [`Cancel`].
    cancel: Arc<Cancel>,
    /// Max bytes the fetcher may stay ahead of `consumed`.
    window: u64,
}

/// Create a coupled pair bounding the fetcher to `window` bytes of read-ahead --
/// or, until something reads, to none at all: the [`PaceGate`] waits, the
/// [`PaceReporter`] reports. `cancel` is the owning track's signal, which releases a
/// parked gate for good.
pub(super) fn pace_channel(window: u64, cancel: Arc<Cancel>) -> (PaceGate, PaceReporter) {
    let inner = Arc::new(PaceInner {
        consumed: AtomicU64::new(0),
        started: AtomicBool::new(false),
        signal: Notify::new(),
        cancel,
        window,
    });
    (
        PaceGate {
            inner: inner.clone(),
        },
        PaceReporter { inner },
    )
}

/// Fetcher half of the pacer: it can only *wait*, via
/// [`await_room`](Self::await_room). Cloning shares the state, so a reopened request
/// keeps pacing against the same reader.
#[derive(Clone)]
pub(super) struct PaceGate {
    inner: Arc<PaceInner>,
}

impl PaceGate {
    /// Resolve once it is OK to fetch the next piece (playback is within the
    /// effective window of `produced`), returning `true`; or `false` once the track is
    /// cancelled. Called only at a piece boundary, so a park never holds a response
    /// open.
    pub(super) async fn await_room(&self, produced: u64) -> bool {
        loop {
            // Until something reads, the window is *closed*: the chunk already in
            // hand is the whole read-ahead a queued track wants. Once a reader
            // consumes its first byte the window opens to its full size, so a
            // playing track keeps one whole chunk beyond the one being consumed.
            let window = if self.inner.started.load(Ordering::Acquire) {
                self.inner.window
            } else {
                0
            };
            // Create both waiters *before* checking, so a signal arriving after the
            // check but before we park is still observed -- tokio's documented
            // lost-wakeup-free idiom. (`notify_one` also keeps one permit for a
            // not-yet-parked waiter as a backstop.) On a fast-path return the
            // unpolled futures are just dropped: a no-op.
            let advanced = self.inner.signal.notified();
            let cancelled = self.inner.cancel.notified();
            if self.inner.cancel.is_cancelled() {
                return false;
            }
            if produced.saturating_sub(self.inner.consumed.load(Ordering::Acquire)) <= window {
                return true;
            }
            // Either signal just re-runs the checks above, so a spurious wake is
            // harmless.
            tokio::select! {
                _ = advanced => {}
                _ = cancelled => {}
            }
        }
    }
}

/// Reader half of the pacer: it can only *report* — [`advance`](Self::advance) as
/// playback consumes bytes. Stopping the fetcher is not its job; that is the
/// track's [`Cancel`]. Cloning shares the state.
#[derive(Clone, Debug)]
pub(super) struct PaceReporter {
    inner: Arc<PaceInner>,
}

impl PaceReporter {
    /// Report that playback has consumed up to `consumed` bytes. The first call also
    /// opens the window (see [`PaceGate::await_room`]) -- consumption, not the
    /// existence of a reader, is what turns a parked prefetch back into a download.
    pub(super) fn advance(&self, consumed: u64) {
        self.inner.consumed.store(consumed, Ordering::Release);
        self.inner.started.store(true, Ordering::Release);
        // Both stores precede the notify, so the woken gate cannot miss either. In
        // the interleavings where it reads one and not the other it merely parks
        // again, and this notify has already reserved its permit.
        self.inner.signal.notify_one();
    }
}

/// Everything one track's range requests need.
///
/// `url` and `headers` are the *signed* media URL, which expires; they are swapped in
/// place by [`rebind`](Self::rebind) when the owner re-extracts. Cloning is cheap:
/// [`Client`] is `Arc`-backed and the rest are small.
#[derive(Clone)]
pub(super) struct ChunkRequest {
    client: Client,
    url: String,
    headers: HeaderMap,
    /// Total length if known (yt-dlp `filesize`). When present we stop exactly at
    /// the end; when absent we stop on the first `416` past EOF.
    total: Option<u64>,
    /// Max bytes per request; [`HTTP_CHUNK`] in production, small in tests.
    chunk: u64,
    /// Read-ahead gate. `None` leaves the fetcher unpaced.
    pace: Option<PaceGate>,
}

impl ChunkRequest {
    pub(super) fn new(client: Client, url: String, headers: HeaderMap, total: Option<u64>) -> Self {
        Self {
            client,
            url,
            headers,
            total,
            chunk: HTTP_CHUNK,
            pace: None,
        }
    }

    /// Hold the fetcher to `gate`'s read-ahead window.
    pub(super) fn set_pace(&mut self, gate: PaceGate) {
        self.pace = Some(gate);
    }

    /// The declared length this fetch was built around, if the format gave one. The
    /// owner needs it to check that a re-extracted format is still the *same* bytes
    /// before resuming in place -- see [`super::direct`].
    pub(super) fn total(&self) -> Option<u64> {
        self.total
    }

    /// Point the same fetch at a freshly extracted signed URL. Everything else --
    /// offsets, chunk size, pacing -- is deliberately preserved, so a rebind resumes
    /// exactly where the dead URL left off.
    pub(super) fn rebind(&mut self, url: String, headers: HeaderMap) {
        self.url = url;
        self.headers = headers;
    }

    /// Stream the media from `offset` to its end.
    ///
    /// The first range is fetched eagerly, so a dead URL or connection fails *here*
    /// rather than on the first poll. An `offset` past the end yields an empty
    /// (clean-EOF) stream -- the right answer for a resume that lands exactly at the
    /// end.
    pub(super) async fn open(&self, offset: u64) -> Result<ByteStream, FetchError> {
        // Absolute offset of the next byte to fetch, advanced by the bytes each
        // chunk *actually delivers* -- not by the range it was asked for. A
        // well-formed 206 that under-fills its range then realigns seamlessly,
        // instead of leaving a silent gap the consumer would take as a clean early
        // end (and cache as a truncated track).
        let delivered = Arc::new(AtomicU64::new(offset));

        let first_end = chunk_end(offset, self.chunk, self.total);
        tracing::debug!(offset, end = first_end, "fetching first chunk");
        let first: ByteStream =
            match fetch_range(&self.client, &self.url, &self.headers, offset, first_end).await? {
                RangeResp::Body(resp) => Box::pin(count_bytes(resp, delivered.clone())),
                RangeResp::PastEnd { declared } => {
                    if offset == 0 {
                        // Nothing at all behind the URL. Not a rejection, so a fresh
                        // extract is unlikely to help, but the caller still gets to try.
                        return Err(FetchError::Transport(io::Error::other(
                            "empty media: range from 0 unsatisfiable",
                        )));
                    }
                    // Reopening inside the declared length: see [`FetchError::Truncated`].
                    if let Some(total) = self.total.filter(|total| offset < *total) {
                        return Err(FetchError::Truncated {
                            at: offset,
                            total,
                            declared,
                        });
                    }
                    // Length unknown, or genuinely at the end: a resume landing exactly
                    // on EOF is a clean, empty stream.
                    Box::pin(stream::empty())
                }
            };

        // Remaining chunks, one at a time as the consumer advances. The unfold's
        // state is the offset of the previous fetch: `try_flatten` fully drains each
        // chunk before polling for the next, so `delivered` not moving past it means
        // that chunk made no progress -- an error, never an infinite refetch of the
        // same range.
        let req = self.clone();
        let progress = delivered.clone();
        let rest = stream::try_unfold(None::<u64>, move |last_fetch| {
            let req = req.clone();
            let delivered = progress.clone();
            async move {
                let pos = delivered.load(Ordering::Relaxed);
                if req.total.is_some_and(|t| pos >= t) {
                    return Ok(None);
                }
                if last_fetch == Some(pos) {
                    return Err(FetchError::Transport(io::Error::other(format!(
                        "range from {pos} delivered no bytes"
                    ))));
                }
                let end = chunk_end(pos, req.chunk, req.total);
                if end <= pos {
                    return Ok(None);
                }
                // Pace at the chunk boundary -- the one point where no response body
                // is open -- so a pause never leaves a connection draining slowly.
                // Waits until the consumer is within one window; stops (clean end)
                // once the track is cancelled. `None` never waits.
                if let Some(pace) = &req.pace
                    && !pace.await_room(pos).await
                {
                    return Ok(None);
                }
                tracing::debug!(offset = pos, end, "fetching chunk");
                match fetch_range(&req.client, &req.url, &req.headers, pos, end).await? {
                    RangeResp::Body(resp) => {
                        Ok(Some((count_bytes(resp, delivered.clone()), Some(pos))))
                    }
                    // Inside the declared length this is a contradiction, not an end (see
                    // [`FetchError::Truncated`]); with no declared length it *is* the end.
                    RangeResp::PastEnd { declared } => match req.total.filter(|t| pos < *t) {
                        Some(total) => Err(FetchError::Truncated {
                            at: pos,
                            total,
                            declared,
                        }),
                        None => Ok(None),
                    },
                }
            }
        })
        .try_flatten();

        Ok(Box::pin(first.chain(rest)))
    }
}

#[cfg(test)]
impl ChunkRequest {
    /// Shrink the per-request chunk so tests exercise multiple ranges (and thus
    /// pacing) without multi-MB bodies.
    pub(super) fn set_chunk(&mut self, chunk: u64) {
        self.chunk = chunk;
    }
}

/// Exclusive end offset of the chunk beginning at `start`.
fn chunk_end(start: u64, chunk: u64, total: Option<u64>) -> u64 {
    match total {
        Some(t) => (start + chunk).min(t),
        None => start + chunk,
    }
}

/// Outcome of a single range request.
enum RangeResp {
    /// A 2xx response carrying (part of) the requested range.
    Body(reqwest::Response),
    /// `416 Range Not Satisfiable`: the start is at or past the end of the representation
    /// the server selected. A range merely *overhanging* the end is satisfiable and comes
    /// back clamped, so this means "your start is past my EOF" and nothing else.
    PastEnd {
        /// The server's own total length, from `Content-Range: bytes */N`.
        declared: Option<u64>,
    },
}

/// The total length a 416 reports in `Content-Range: bytes */N`, which a server
/// answering a byte-range request should send. Missing or unparseable is not an error:
/// it only costs the detail in the log.
fn unsatisfied_length(resp: &reqwest::Response) -> Option<u64> {
    resp.headers()
        .get(CONTENT_RANGE)?
        .to_str()
        .ok()?
        .rsplit_once('/')?
        .1
        .trim()
        .parse()
        .ok()
}

/// Send one closed-range GET (`bytes=start-{end-1}`), classifying the answer.
async fn fetch_range(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    start: u64,
    end: u64,
) -> Result<RangeResp, FetchError> {
    // An empty range (start == end) is end-of-stream; treating it as such also
    // keeps `end - 1` below from underflowing on a zero-length file.
    if end <= start {
        return Ok(RangeResp::PastEnd { declared: None });
    }
    let resp = client
        .get(url)
        .headers(headers.clone())
        .header(RANGE, format!("bytes={start}-{}", end - 1))
        .send()
        .await
        .map_err(|err| FetchError::Transport(io::Error::other(err)))?;

    let status = resp.status();
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        Ok(RangeResp::PastEnd {
            declared: unsatisfied_length(&resp),
        })
    } else if status.is_success() {
        Ok(RangeResp::Body(resp))
    } else if matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::GONE
    ) {
        // How googlevideo answers an expired signature or a rebound IP. Reported
        // separately because retrying is futile and re-extracting is not.
        Err(FetchError::Rejected(status))
    } else {
        Err(FetchError::Transport(io::Error::other(format!(
            "range request failed with status {status}"
        ))))
    }
}

/// A chunk's byte stream with every delivered byte added to `delivered`.
fn count_bytes(
    resp: reqwest::Response,
    delivered: Arc<AtomicU64>,
) -> impl Stream<Item = Result<Bytes, FetchError>> + Send {
    resp.bytes_stream()
        .map_err(|err| FetchError::Transport(io::Error::other(err)))
        .inspect_ok(move |bytes| {
            // Relaxed: only ever read/written from the one task that drives the
            // stream (`try_flatten` drains a chunk before polling for the next).
            delivered.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
}
