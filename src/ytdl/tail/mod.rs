//! "Read while writing" scaffolding for the live tail paths: a background producer
//! drains a download into a temp file at network speed while playback tails the
//! same file at playback rate. This decouples the fetch from playback, so a slow
//! or paused consumer never holds the source connection open.
//!
//! One tail file has two ends. A [`TailWriter`] the producer fills, and a
//! [`TailHandle`] -- the shared, keep-alive end -- from which a [`TailReader`] can
//! be minted **at any point**: while the download is still running, after it has
//! completed, or again later for a replay. That is what lets one download serve both a
//! prefetch nobody is listening to yet and the playback that arrives later.
//! [`spawn_tail`] is the entry point, taking the producer for whichever byte path is in
//! play: [`spawn_sidecar_tail`] and [`spawn_hls_tail`] wrap the two thin ones, and
//! [`super::direct`] supplies its own.
//!
//! Stopping the producer is *not* a reader's decision (readers come and go, and
//! songbird disposes a finished one asynchronously, possibly after its replacement
//! has attached) -- it belongs to the track's lifetime. See
//! [`Cancel`].

use super::{cancel::Cancel, chunked};
use async_trait::async_trait;
use futures::task::AtomicWaker;
use songbird::input::{AsyncAdapterStream, AsyncMediaSource, AudioStream, Input, LiveInput};
use std::{
    future::Future,
    io::{Read, SeekFrom},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll},
};
use symphonia::core::io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions};
use tempfile::NamedTempFile;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWriteExt, ReadBuf};
use tracing::Instrument;

#[cfg(test)]
mod test;

/// Ring-buffer size for the async→sync bridge feeding tail downloads into
/// playback. Matches songbird's own `HttpRequest`/`HlsRequest` sources.
const ADAPTER_BUF: usize = 64 * 1024;

/// Start the sidecar `/download` tail: a background task drains the response into a
/// temp file at full speed while readers tail it.
pub(super) fn spawn_sidecar_tail(
    cancel: Arc<Cancel>,
    resp: reqwest::Response,
) -> std::io::Result<TailHandle> {
    spawn_tail(cancel, None, move |tail| drain_response(resp, tail))
}

/// Start the HLS tail. songbird's `HlsRequest` only hands out a *sync* `MediaSource`
/// (its async half is private), so this one is bridged off a blocking thread.
pub(super) fn spawn_hls_tail(
    cancel: Arc<Cancel>,
    source: Box<dyn MediaSource>,
) -> std::io::Result<TailHandle> {
    spawn_tail(cancel, None, move |tail| drain_blocking(source, tail))
}

/// Spawn `producer` to fill a fresh tail file, returning the handle readers are
/// minted from. `producer` returns `Ok(())` on a clean EOF or `Err` on failure --
/// surfaced to readers as a read error, never a truncated EOF, so a partial download
/// is never played.
///
/// The direct-HTTP path supplies its own producer ([`super::direct`]), since it is the
/// one with a recovery loop; the two here are thin enough to live beside the
/// machinery.
pub(super) fn spawn_tail<P, Fut>(
    cancel: Arc<Cancel>,
    pace: Option<chunked::PaceReporter>,
    producer: P,
) -> std::io::Result<TailHandle>
where
    P: FnOnce(TailWriter) -> Fut + Send + 'static,
    Fut: Future<Output = std::io::Result<()>> + Send + 'static,
{
    let (handle, writer) = TailHandle::new(cancel.clone(), pace)?;

    // The producer holds nothing of the track but its `Cancel`, so the track is free
    // to drop mid-download -- which is what fires that cancel.
    // `in_current_span` carries the caller's `play_track` span (with the url) into the
    // detached task, so resume/chunk/finish events stay attributed.
    let finish = handle.clone();
    tokio::spawn(
        async move {
            let result = producer(writer).await;
            finish.finish(result, &cancel);
        }
        .in_current_span(),
    );

    Ok(handle)
}

/// Progress shared between a download producer and every [`TailReader`] minted
/// over the life of one tail file: the producer advances the fields, the readers
/// read them. Purely one-directional -- the reverse channel (stop fetching) is the
/// track's [`Cancel`], not part of this state.
#[derive(Debug, Default)]
struct TailState {
    /// Bytes flushed to the OS so far, so a reader never outruns durable data.
    written: AtomicU64,
    /// Set once the producer has finished, successfully or not.
    done: AtomicBool,
    /// Set before `done` on failure, so a reader that observes `done` also sees
    /// the failure and never mistakes a truncated file for a clean EOF.
    failed: AtomicBool,
    /// Wakes a reader that parked after catching up to the writer. An
    /// [`AtomicWaker`] survives across polls, avoiding the self-referential future
    /// a `Notify` would create.
    waker: AtomicWaker,
}

/// The write half of the tail file: appends bytes and publishes progress for the
/// paired [`TailReader`]s. Owns the write handle, the byte counter, and the flush
/// discipline.
pub(super) struct TailWriter {
    file: tokio::fs::File,
    state: Arc<TailState>,
    /// The track's stop signal, polled by producers between chunks.
    cancel: Arc<Cancel>,
    /// Bytes appended so far -- also the offset a recovery reopens at.
    written: u64,
}

impl TailWriter {
    fn new(file: tokio::fs::File, state: Arc<TailState>, cancel: Arc<Cancel>) -> Self {
        Self {
            file,
            state,
            cancel,
            written: 0,
        }
    }

    /// Append `bytes`, flush so the reader's separate handle sees them, then
    /// publish the new length and wake a parked reader. Flush-before-publish is the
    /// invariant that keeps the reader from outrunning durable data (tokio's `File`
    /// buffers otherwise).
    pub(super) async fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.file.write_all(bytes).await?;
        self.file.flush().await?;
        self.written += bytes.len() as u64;
        self.state.written.store(self.written, Ordering::Release);
        self.state.waker.wake();
        Ok(())
    }

    /// Total bytes appended so far -- what a recovery resumes from, and the only
    /// offset that is safe to resume from (anything later would leave a gap).
    pub(super) fn written(&self) -> u64 {
        self.written
    }

    /// Has the track been dropped (a skip, a removal)? Producers poll this to stop
    /// early.
    pub(super) fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// An [`AsyncMediaSource`] over a temp file being (or already) written by a
/// background download producer (the sidecar `/download` or the direct-HTTP chunked
/// path). Minted by [`TailHandle::reader`], at any point in the download.
///
/// When the reader catches up to the writer:
///
/// - It parks -- returns `Poll::Pending` after registering its waker rather than
///   polling; the producer wakes it on each write and on completion.
/// - An empty region mid-download is never EOF: completion is signalled only via
///   [`TailState::done`], so playback can never be truncated.
/// - A producer failure surfaces as a read error, not a clean EOF, so songbird's
///   adapter propagates it and a truncated download is never played or cached.
pub(super) struct TailReader {
    file: std::fs::File,
    pos: u64,
    state: Arc<TailState>,
    /// The chunked path's pacer: reports consumption so the fetcher can stay ~one
    /// chunk ahead. `None` on the sidecar path.
    pace: Option<chunked::PaceReporter>,
}

impl AsyncRead for TailReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            let written = this.state.written.load(Ordering::Acquire);
            if this.pos < written {
                let want = (written - this.pos).min(buf.remaining() as u64) as usize;
                if want == 0 {
                    return Poll::Ready(Ok(())); // caller's buffer is full
                }
                // Local temp-file read from the page cache; effectively instant.
                let dst = buf.initialize_unfilled_to(want);
                let n = this.file.read(dst)?;
                if n == 0 {
                    // The writer reported these bytes but the file is short:
                    // fail rather than signal a false EOF (silent truncation).
                    return Poll::Ready(Err(std::io::Error::other(
                        "tail download file shorter than reported progress",
                    )));
                }
                buf.advance(n);
                this.pos += n as u64;
                // Report consumption so the fetcher's pace gate can stay ~one
                // chunk ahead of us (no-op on the sidecar path).
                if let Some(pace) = this.pace.as_ref() {
                    pace.advance(this.pos);
                }
                return Poll::Ready(Ok(()));
            }
            if this.state.done.load(Ordering::Acquire) {
                if this.state.failed.load(Ordering::Acquire) {
                    return Poll::Ready(Err(std::io::Error::other(
                        "tail download failed before completion",
                    )));
                }
                return Poll::Ready(Ok(())); // leave buf empty -> clean EOF
            }
            // Caught up to an in-progress download: park until the producer wakes
            // us. Re-check after registering so an update landing in the gap
            // between the loads above and the register is not lost.
            this.state.waker.register(cx.waker());
            if this.pos < this.state.written.load(Ordering::Acquire)
                || this.state.done.load(Ordering::Acquire)
            {
                continue;
            }
            return Poll::Pending;
        }
    }
}

impl AsyncSeek for TailReader {
    fn start_seek(self: Pin<&mut Self>, _position: SeekFrom) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "tail stream is not seekable",
        ))
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Poll::Ready(Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "tail stream is not seekable",
        )))
    }
}

#[async_trait]
impl AsyncMediaSource for TailReader {
    fn is_seekable(&self) -> bool {
        false
    }

    async fn byte_len(&self) -> Option<u64> {
        None
    }
}

/// The shared, keep-alive end of one tail file: it owns the backing tempfile and
/// the progress state, so it can mint a [`TailReader`] whenever one is wanted and
/// settle the file's fate when the producer returns. Cloning shares both (each
/// reader gets its own OS handle, and therefore its own cursor).
///
/// The tempfile stays *linked* for as long as any handle lives, because a new
/// reader needs a fresh [`NamedTempFile::reopen`] -- something an unlinked file
/// cannot give. `try_clone()` is not an option: cloned handles share a file offset,
/// which would corrupt concurrent read/write.
#[derive(Clone, Debug)]
pub(super) struct TailHandle {
    backing: Arc<NamedTempFile>,
    state: Arc<TailState>,
    /// The fetcher's read-ahead gate, shared by every reader minted here so a late
    /// one keeps driving the same producer (`None` on the unpaced paths). Successive
    /// readers each start at 0, so `consumed` can step backwards -- which is why the
    /// reporter *stores* rather than taking a maximum: a replay must be paced against the
    /// reader actually playing, and a high-water mark would let the fetcher outrun it.
    pace: Option<chunked::PaceReporter>,
}

impl TailHandle {
    /// One fresh tail file, as a handle plus the [`TailWriter`] its producer fills.
    fn new(
        cancel: Arc<Cancel>,
        pace: Option<chunked::PaceReporter>,
    ) -> std::io::Result<(Self, TailWriter)> {
        let backing = Arc::new(NamedTempFile::new()?);
        let write_handle = tokio::fs::File::from_std(backing.reopen()?);
        let state = Arc::new(TailState::default());
        Ok((
            Self {
                backing,
                state: state.clone(),
                pace,
            },
            TailWriter::new(write_handle, state, cancel),
        ))
    }

    /// Mint a reader positioned at the start of the file. Valid at any point: it
    /// serves what is durable, parks for the rest, and ends where the producer did
    /// (a clean EOF, or a read error if the producer failed).
    pub(super) fn reader(&self) -> std::io::Result<TailReader> {
        Ok(TailReader {
            file: self.backing.reopen()?,
            pos: 0,
            state: self.state.clone(),
            pace: self.pace.clone(),
        })
    }

    /// A fresh reader wrapped for songbird. `AsyncAdapterStream` spawns its own
    /// driver to pull the reader (waker-driven, no polling) into a sync
    /// `MediaSource`; the tail file is the read-ahead buffer, so the ring only has to
    /// cover a local file read.
    pub(super) fn input(&self) -> std::io::Result<Input> {
        let adapter = AsyncAdapterStream::new(Box::new(self.reader()?), ADAPTER_BUF);
        Ok(Input::Live(
            LiveInput::Wrapped(AudioStream {
                input: MediaSourceStream::new(
                    Box::new(adapter),
                    MediaSourceStreamOptions::default(),
                ),
            }),
            None,
        ))
    }

    /// Drain the whole file and discard it, resolving once the download is complete.
    /// The way to *warm* a track without playing it: it goes through a real reader, so
    /// the pacer sees consumption and lets the fetcher run to the end.
    pub(super) async fn warm(&self) -> std::io::Result<()> {
        let mut reader = self.reader()?;
        tokio::io::copy(&mut reader, &mut tokio::io::sink()).await?;
        Ok(())
    }

    /// Did this download fail? A failed tail can never serve its track, so the owner
    /// throws it away and extracts afresh rather than handing readers an error.
    pub(super) fn failed(&self) -> bool {
        self.state.failed.load(Ordering::Acquire)
    }

    /// How far the producer has got. A reader learns this by reading; only tests
    /// need to ask.
    #[cfg(test)]
    pub(super) fn written(&self) -> u64 {
        self.state.written.load(Ordering::Acquire)
    }

    /// Called once the producer finishes. A tail that did not reach the end of its track
    /// -- failed or cancelled -- is published as `failed`, so a reader gets a read error
    /// rather than a clean EOF over a partial file and the owner never reuses it as the
    /// replay cache. Publishes `failed` before `done`, and wakes parked readers last.
    fn finish(&self, result: std::io::Result<()>, cancel: &Cancel) {
        if cancel.is_cancelled() {
            // Nothing observes this today: `Cancel` is raised only by the track's `Drop`,
            // so the route back to this handle is already gone. Stored anyway so
            // "complete means whole" holds here on its own.
            self.state.failed.store(true, Ordering::Release);
            tracing::debug!("tail download cancelled");
        } else if let Err(err) = result {
            // Order matters: publish `failed` before `done` so a reader that
            // observes `done` (Acquire) is guaranteed to see `failed` too.
            self.state.failed.store(true, Ordering::Release);
            tracing::warn!(error = %err, "tail download failed");
        } else {
            tracing::info!(
                bytes = self.state.written.load(Ordering::Acquire),
                "tail download complete"
            );
        }
        self.state.done.store(true, Ordering::Release);
        self.state.waker.wake();
    }
}

/// Drain the sidecar `/download` response body into `tail` until a clean EOF or a
/// skip. A stream or tail-file write error propagates as [`std::io::Error`].
async fn drain_response(mut resp: reqwest::Response, mut tail: TailWriter) -> std::io::Result<()> {
    // Dropping the track flips `is_cancelled`, ending the loop (not a failure).
    while !tail.is_cancelled() {
        match resp.chunk().await.map_err(std::io::Error::other)? {
            Some(bytes) => tail.write(&bytes).await?,
            None => break,
        }
    }
    Ok(())
}

/// Bytes per read on the blocking bridge. Matches the other producers' buffer.
const BLOCKING_CHUNK: usize = 64 * 1024;

/// Drain a *sync* [`MediaSource`] into `tail` from a blocking thread, bridged over a
/// small channel. songbird's `HlsRequest` keeps its async source private, so HLS
/// bytes can only be pulled by a blocking read; doing it on the runtime would stall a
/// worker for the length of a segment fetch.
///
/// The bridge is also what makes the path interruptible: the blocking side checks the
/// track's cancel between reads, so a stop lands within one read instead of at the end
/// of the track.
async fn drain_blocking(
    mut src: Box<dyn MediaSource>,
    mut tail: TailWriter,
) -> std::io::Result<()> {
    let cancel = tail.cancel.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<std::io::Result<Vec<u8>>>(4);

    tokio::task::spawn_blocking(move || {
        let mut buf = vec![0u8; BLOCKING_CHUNK];
        while !cancel.is_cancelled() {
            match src.read(&mut buf) {
                Ok(0) => break,
                // A closed receiver means the consumer went away: stop reading.
                Ok(n) => {
                    if tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = tx.blocking_send(Err(err));
                    break;
                }
            }
        }
    });

    while let Some(chunk) = rx.recv().await {
        tail.write(&chunk?).await?;
    }
    Ok(())
}
