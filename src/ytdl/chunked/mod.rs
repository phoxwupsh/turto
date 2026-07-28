//! Chunked HTTP byte source for the direct-play path.
//!
//! googlevideo throttles any single request larger than ~10 MB, so a whole-file
//! range request on a long track crawls. [`ChunkedHttpRequest`] fetches the media
//! as sequential closed byte-ranges of at most [`HTTP_CHUNK`] and concatenates
//! them into one stream. Because the chunks are adjacent ranges of the same file,
//! the decoder sees byte-identical output, with no seam at a boundary.
//!
//! - The first chunk is fetched eagerly, so an expired-URL 403 surfaces at open
//!   time for the caller's URL-expiry retry; later chunks are fetched lazily.
//! - A mid-play read error recovers through [`AsyncMediaSource::try_resume`],
//!   which rebuilds the stream from the delivered offset.
//! - On the live path the fetcher drains into a tail file in the background, paced
//!   by a [`PaceGate`]/[`PaceReporter`] pair to stay ~one chunk ahead of playback;
//!   prefetch ([`Compose::create_async`]) is unpaced.

use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt, stream};
use reqwest::{
    Client,
    header::{HeaderMap, RANGE},
};
use songbird::input::{
    AsyncAdapterStream, AsyncMediaSource, AudioStream, AudioStreamError, Compose,
};
use std::{
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use symphonia::core::io::MediaSource;
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};
use tokio::sync::Notify;
use tokio_util::io::StreamReader;
use tracing::Instrument;

#[cfg(test)]
mod test;

/// Maximum bytes per range request. googlevideo throttles single requests larger
/// than ~10 MB; this matches yt-dlp's own `http_chunk_size`.
pub(super) const HTTP_CHUNK: u64 = 10 * 1024 * 1024;

/// Ring buffer for the chunked source's async→sync bridge. At ~16 KB/s opus this
/// is ~60 s of read-ahead, so the range-request latency at a chunk boundary can
/// never drain it before the next chunk's bytes arrive.
const CHUNKED_ADAPTER_BUF: usize = 1024 * 1024;

/// How many times to rebuild the stream at the *same* offset before giving up. A
/// resume that makes progress resets the count, so this bounds only a genuine
/// stall (a dead URL, a persistently failing range) -- never a long track that
/// hits occasional transient blips.
const MAX_STALLED_RESUMES: u32 = 5;

/// Attempts to rebuild the stream within a single resume, and the pause between
/// them. The ring buffer's read-ahead hides the couple of seconds of retrying.
const RESUME_OPEN_ATTEMPTS: u32 = 3;
const RESUME_OPEN_BACKOFF: Duration = Duration::from_millis(500);

/// Shared state behind the [`PaceGate`]/[`PaceReporter`] pair. Split into two
/// typed halves (via [`pace_channel`]) so the fetcher can only *wait* and the
/// reader can only *report*.
struct PaceInner {
    /// Absolute byte offset playback has consumed so far.
    consumed: AtomicU64,
    /// Signalled by the reader on each advance and on cancellation; the fetcher
    /// parks on it when too far ahead.
    signal: Notify,
    /// Set when playback is gone, so the fetcher stops instead of finishing a
    /// skipped track.
    cancelled: AtomicBool,
    /// Max bytes the fetcher may stay ahead of `consumed`.
    window: u64,
}

/// Create a coupled pair bounding the fetcher to `window` bytes of read-ahead:
/// the [`PaceGate`] waits, the [`PaceReporter`] reports.
pub(super) fn pace_channel(window: u64) -> (PaceGate, PaceReporter) {
    let inner = Arc::new(PaceInner {
        consumed: AtomicU64::new(0),
        signal: Notify::new(),
        cancelled: AtomicBool::new(false),
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
/// [`await_room`](Self::await_room). Cloning shares the state, so a resumed source
/// keeps pacing against the same reader.
#[derive(Clone)]
pub(super) struct PaceGate {
    inner: Arc<PaceInner>,
}

impl PaceGate {
    /// Resolve once it is OK to fetch the next chunk (playback is within `window`
    /// of `produced`), returning `true`; or `false` if the consumer is gone.
    /// Called only at a chunk boundary, so a park here never holds a response open.
    async fn await_room(&self, produced: u64) -> bool {
        loop {
            // Create the waiter *before* checking, so a signal arriving after the
            // check but before we park is still observed -- tokio's documented
            // lost-wakeup-free idiom. (`notify_one` also keeps one permit for a
            // not-yet-parked waiter as a backstop.) On a fast-path return the
            // unpolled future is just dropped: a no-op.
            let notified = self.inner.signal.notified();
            if self.inner.cancelled.load(Ordering::Acquire) {
                return false;
            }
            if produced.saturating_sub(self.inner.consumed.load(Ordering::Acquire))
                <= self.inner.window
            {
                return true;
            }
            notified.await;
        }
    }
}

/// Reader half of the pacer: it can only *report* — [`advance`](Self::advance) as
/// playback consumes bytes, [`cancel`](Self::cancel) when playback goes away.
/// Cloning shares the state.
#[derive(Clone)]
pub(super) struct PaceReporter {
    inner: Arc<PaceInner>,
}

impl PaceReporter {
    /// Report that playback has consumed up to `consumed` bytes.
    pub(super) fn advance(&self, consumed: u64) {
        self.inner.consumed.store(consumed, Ordering::Release);
        self.inner.signal.notify_one();
    }

    /// The consumer is gone; wake the fetcher so it stops.
    pub(super) fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        self.inner.signal.notify_one();
    }

    /// Test-only accessor to assert the pacer was cancelled.
    #[cfg(test)]
    pub(super) fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }
}

/// A resume-capable byte source paired with the [`PaceReporter`] coupled to its
/// background fetcher.
pub(super) struct PacedSource {
    pub(super) source: Box<dyn AsyncMediaSource>,
    pub(super) reporter: PaceReporter,
}

/// The immutable request parameters for one track, shared by the initial open
/// and every [`ChunkedSource::try_resume`]. Cloning is cheap: [`Client`] is
/// `Arc`-backed and the rest are small.
#[derive(Clone)]
struct ChunkSpec {
    client: Client,
    url: String,
    headers: HeaderMap,
    /// Total length if known (yt-dlp `filesize`). When present we stop exactly at
    /// the end; when absent we stop on the first `416` past EOF.
    total: Option<u64>,
    /// Max bytes per request; [`HTTP_CHUNK`] in production, small in tests.
    chunk: u64,
    /// The fetcher's [`PaceGate`] on the live path; `None` on prefetch (unpaced).
    pace: Option<PaceGate>,
}

/// A lazily-fetched, throttle-dodging HTTP byte source. See the module docs.
pub(super) struct ChunkedHttpRequest {
    spec: ChunkSpec,
}

impl ChunkedHttpRequest {
    pub(super) fn new(client: Client, url: String, headers: HeaderMap, total: Option<u64>) -> Self {
        Self {
            spec: ChunkSpec {
                client,
                url,
                headers,
                total,
                chunk: HTTP_CHUNK,
                pace: None,
            },
        }
    }

    /// Open the resume-capable byte source for the *live* path: wire in a one-chunk
    /// ([`HTTP_CHUNK`]) read-ahead pacer and eagerly fetch the first range so an
    /// expired-URL 403 surfaces here for the caller to re-extract. Unlike
    /// [`Compose::create_async`], the source is returned raw (no
    /// [`AsyncAdapterStream`]) so a background producer can drain it directly,
    /// alongside the [`PaceReporter`] coupling it to playback.
    pub(super) async fn open_source(&self) -> Result<PacedSource, AudioStreamError> {
        let (gate, reporter) = pace_channel(HTTP_CHUNK);
        let mut spec = self.spec.clone();
        spec.pace = Some(gate);
        Ok(PacedSource {
            source: Box::new(spec.open(0).await?),
            reporter,
        })
    }
}

#[cfg(test)]
impl ChunkedHttpRequest {
    /// Shrink the per-request chunk so tests exercise multiple ranges (and thus
    /// pacing) without multi-MB bodies.
    pub(super) fn set_chunk(&mut self, chunk: u64) {
        self.spec.chunk = chunk;
    }

    /// Open with an explicit [`PaceGate`] (or none) so tests can drive the
    /// fetch/resume paths under a controlled window.
    pub(super) async fn open_with_pace(
        &self,
        pace: Option<PaceGate>,
    ) -> Result<Box<dyn AsyncMediaSource>, AudioStreamError> {
        let mut spec = self.spec.clone();
        spec.pace = pace;
        Ok(Box::new(spec.open(0).await?))
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
    /// `416 Range Not Satisfiable`: the start is past EOF, i.e. a clean end.
    PastEnd,
}

/// Send one closed-range GET (`bytes=start-{end-1}`). A `416` is reported as
/// [`RangeResp::PastEnd`]; any other non-2xx is an error.
async fn fetch_range(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    start: u64,
    end: u64,
) -> Result<RangeResp, reqwest::Error> {
    // An empty range (start == end) is end-of-stream; treating it as such also
    // keeps `end - 1` below from underflowing on a zero-length file.
    if end <= start {
        return Ok(RangeResp::PastEnd);
    }
    let resp = client
        .get(url)
        .headers(headers.clone())
        .header(RANGE, format!("bytes={start}-{}", end - 1))
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        Ok(RangeResp::PastEnd)
    } else {
        let resp = resp.error_for_status()?;
        Ok(RangeResp::Body(resp))
    }
}

/// A chunk's byte stream with every delivered byte added to `delivered`.
fn count_bytes(
    resp: reqwest::Response,
    delivered: Arc<AtomicU64>,
) -> impl Stream<Item = io::Result<Bytes>> + Send + Sync {
    resp.bytes_stream()
        .map_err(io::Error::other)
        .inspect_ok(move |bytes| {
            // Relaxed: only ever read/written from the one task that drives the
            // stream (`try_flatten` drains a chunk before polling for the next).
            delivered.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
}

impl ChunkSpec {
    /// Build the byte source starting at `start`, eagerly fetching the first range
    /// so a dead URL or connection surfaces here (fail-fast). A `start` past EOF
    /// yields an empty (clean-EOF) source -- the correct end for a resume that
    /// lands exactly at the end.
    async fn open(self, start: u64) -> Result<ChunkedSource, AudioStreamError> {
        // Attribute the lazy chunk fetches to whoever opened the source. The span
        // has to ride on the *stream*, not the task: on the prefetch path
        // `Compose::create_async` hands the source to `AsyncAdapterStream`, which
        // spawns its own driver with no span propagation, so the later chunks are
        // polled from a task we never get to instrument.
        let span = tracing::Span::current();
        // Absolute offset of the next byte to fetch, advanced by the bytes each
        // chunk *actually delivers* -- not by the range it was asked for. A
        // well-formed 206 that under-fills its range then realigns seamlessly,
        // instead of leaving a silent gap the decoder would take as a clean
        // early end (promoting a truncated file into the replay cache).
        let delivered = Arc::new(AtomicU64::new(start));

        let first_end = chunk_end(start, self.chunk, self.total);
        tracing::debug!(start, end = first_end, "fetching first chunk");
        let first: BoxedByteStream =
            match fetch_range(&self.client, &self.url, &self.headers, start, first_end).await {
                Ok(RangeResp::Body(resp)) => Box::pin(count_bytes(resp, delivered.clone())),
                Ok(RangeResp::PastEnd) => {
                    if start == 0 {
                        // Nothing at all behind the URL: surface it at open time
                        // so the caller's URL-expiry retry can re-extract.
                        let msg: Box<dyn std::error::Error + Send + Sync> =
                            "empty media: range from 0 unsatisfiable".into();
                        return Err(AudioStreamError::Fail(msg));
                    }
                    Box::pin(stream::empty::<io::Result<Bytes>>())
                }
                Err(err) => return Err(AudioStreamError::Fail(err.into())),
            };

        // Remaining chunks, fetched one at a time as playback advances. The
        // unfold owns a clone of the spec; the source keeps the original so
        // try_resume can reissue ranges. Its state is the offset of the previous
        // fetch: `try_flatten` fully drains each chunk before polling for the
        // next, so `delivered` not moving past it means that chunk made no
        // progress -- an error (bounded by the resume stall guard), never an
        // infinite refetch of the same range.
        let spec = self.clone();
        let progress = delivered.clone();
        let rest = stream::try_unfold(None::<u64>, move |last_fetch| {
            let spec = spec.clone();
            let delivered = progress.clone();
            let span = span.clone();
            async move {
                let pos = delivered.load(Ordering::Relaxed);
                if spec.total.is_some_and(|t| pos >= t) {
                    return Ok(None);
                }
                if last_fetch == Some(pos) {
                    return Err(io::Error::other(format!(
                        "range from {pos} delivered no bytes"
                    )));
                }
                let end = chunk_end(pos, spec.chunk, spec.total);
                if end <= pos {
                    return Ok(None);
                }
                // Pace at the chunk boundary -- the one point where no response
                // body is open -- so a pause never leaves a connection draining
                // slowly. Waits until playback is within one window; stops (clean
                // end) if playback has gone away. `None` on prefetch: never waits.
                if let Some(pace) = &spec.pace
                    && !pace.await_room(pos).await
                {
                    return Ok(None);
                }
                tracing::debug!(offset = pos, end, "fetching chunk");
                match fetch_range(&spec.client, &spec.url, &spec.headers, pos, end).await {
                    Ok(RangeResp::Body(resp)) => {
                        Ok(Some((count_bytes(resp, delivered.clone()), Some(pos))))
                    }
                    Ok(RangeResp::PastEnd) => Ok(None),
                    Err(err) => Err(io::Error::other(err)),
                }
            }
            .instrument(span)
        })
        .try_flatten();

        let body: BoxedByteStream = Box::pin(first.chain(rest));
        Ok(ChunkedSource {
            reader: StreamReader::new(body),
            spec: self,
            start,
            stalled_resumes: 0,
        })
    }
}

#[async_trait::async_trait]
impl Compose for ChunkedHttpRequest {
    fn create(&mut self) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        // Async only: the eager first fetch has to run on the runtime.
        Err(AudioStreamError::Unsupported)
    }

    async fn create_async(
        &mut self,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        let source = self.spec.clone().open(0).await?;
        let stream = AsyncAdapterStream::new(Box::new(source), CHUNKED_ADAPTER_BUF);
        Ok(AudioStream {
            input: Box::new(stream),
        })
    }

    fn should_create_async(&self) -> bool {
        true
    }
}

type BoxedByteStream = Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send + Sync>>;

/// Adapts the concatenated chunk stream into an [`AsyncMediaSource`]. Not
/// seekable (the cached tempfile serves later seeks/replays), but it *does*
/// implement [`try_resume`](AsyncMediaSource::try_resume) so a mid-play
/// connection drop rebuilds the stream instead of truncating the track.
struct ChunkedSource {
    reader: StreamReader<BoxedByteStream, Bytes>,
    /// Request parameters, kept so `try_resume` can reissue ranges from a new
    /// offset.
    spec: ChunkSpec,
    /// Absolute byte offset this source started at. Lets `try_resume` tell real
    /// progress (a later offset) from a stall (the same offset again).
    start: u64,
    /// Consecutive resumes that made no progress. Reset once the stream advances,
    /// so only a stuck URL trips [`MAX_STALLED_RESUMES`].
    stalled_resumes: u32,
}

impl AsyncRead for ChunkedSource {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().reader).poll_read(cx, buf)
    }
}

impl AsyncSeek for ChunkedSource {
    fn start_seek(self: Pin<&mut Self>, _pos: io::SeekFrom) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Poll::Ready(Err(io::ErrorKind::Unsupported.into()))
    }
}

#[async_trait::async_trait]
impl AsyncMediaSource for ChunkedSource {
    fn is_seekable(&self) -> bool {
        false
    }

    async fn byte_len(&self) -> Option<u64> {
        self.spec.total
    }

    /// Rebuild the chunked stream from `offset` after a mid-play read error, so a
    /// dropped connection resumes instead of truncating the track. `offset` is the
    /// total bytes read so far -- the next byte to fetch.
    async fn try_resume(
        &mut self,
        offset: u64,
    ) -> Result<Box<dyn AsyncMediaSource>, AudioStreamError> {
        // Advancing past where this source opened means a genuine transient blip,
        // so forgive earlier attempts. No advance means we're stuck at one offset.
        let stalled = if offset > self.start {
            0
        } else {
            self.stalled_resumes + 1
        };
        if stalled > MAX_STALLED_RESUMES {
            // Giving up: the track ends here -- worth a `warn`.
            tracing::warn!(
                offset,
                total = ?self.spec.total,
                "chunked stream stuck; giving up, track will end early"
            );
            let msg: Box<dyn std::error::Error + Send + Sync> = format!(
                "chunked stream stuck at offset {offset} after {MAX_STALLED_RESUMES} resume attempts"
            )
            .into();
            return Err(AudioStreamError::Fail(msg));
        }
        // A resume is routine, not a fault; logged at `info`.
        tracing::info!(
            offset,
            total = ?self.spec.total,
            stalled,
            "chunked stream connection dropped; resuming"
        );
        // open() fetches eagerly, so a dead URL fails fast; a few paced attempts
        // ride out a transient one (the ring buffer's read-ahead hides the pauses).
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.spec.clone().open(offset).await {
                Ok(mut resumed) => {
                    resumed.stalled_resumes = stalled;
                    return Ok(Box::new(resumed));
                }
                Err(err) if attempt < RESUME_OPEN_ATTEMPTS => {
                    tracing::debug!(offset, attempt, error = %err, "resume open failed; retrying");
                    tokio::time::sleep(RESUME_OPEN_BACKOFF).await;
                }
                Err(err) => {
                    // Attempts exhausted: the track ends here -- worth a `warn`.
                    tracing::warn!(
                        offset,
                        attempts = RESUME_OPEN_ATTEMPTS,
                        error = %err,
                        "chunked stream resume failed, track will end early"
                    );
                    return Err(err);
                }
            }
        }
    }
}
