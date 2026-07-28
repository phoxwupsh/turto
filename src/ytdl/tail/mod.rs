//! "Read while writing" scaffolding for the live tail paths: a background producer
//! drains a download into a temp file at network speed while playback tails the
//! same file at playback rate. This decouples the fetch from playback, so a slow
//! or paused consumer never holds the source connection open. One [`TailChannel`]
//! ties three independent handles to one tempfile together -- a [`TailWriter`] the
//! producer fills, a [`TailReader`] playback pulls, and a [`TailFinalizer`] that
//! promotes the completed file to the replay cache (or discards it); [`sidecar_tail`]
//! and [`http_tail`] are the entry points.

use super::{YoutubeDlFileInner, chunked};
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
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use tempfile::NamedTempFile;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncWriteExt, ReadBuf};
use tracing::Instrument;

#[cfg(test)]
mod test;

/// Ring-buffer size for the async→sync bridge feeding tail downloads into
/// playback. Matches songbird's own `HttpRequest`/`HlsRequest` sources.
const ADAPTER_BUF: usize = 64 * 1024;

/// Live input for the sidecar `/download` path: a background task drains the
/// response into a temp file at full speed while playback tails it.
pub(super) fn sidecar_tail(
    cache: Arc<YoutubeDlFileInner>,
    resp: reqwest::Response,
) -> std::io::Result<Input> {
    spawn_tail_input(cache, None, move |tail| drain_response(resp, tail))
}

/// Live input for the direct-HTTP chunked path: a background task drains the
/// paced source into a temp file while playback tails it. The pacer keeps the
/// fetcher ~one chunk ahead and stops it if playback goes away.
pub(super) fn http_tail(
    cache: Arc<YoutubeDlFileInner>,
    paced: chunked::PacedSource,
) -> std::io::Result<Input> {
    let chunked::PacedSource { source, reporter } = paced;
    spawn_tail_input(cache, Some(reporter), move |tail| drain_resuming(source, tail))
}

/// Spawn `producer` to fill a tail file while playback tails it, returning the
/// reader wrapped as a live [`Input`]. `producer` returns `Ok(())` on a clean EOF
/// or `Err` on failure -- surfaced to the reader as a read error, never a truncated
/// EOF, so a partial download is never played or cached.
fn spawn_tail_input<P, Fut>(
    cache: Arc<YoutubeDlFileInner>,
    pace: Option<chunked::PaceReporter>,
    producer: P,
) -> std::io::Result<Input>
where
    P: FnOnce(TailWriter) -> Fut + Send + 'static,
    Fut: Future<Output = std::io::Result<()>> + Send + 'static,
{
    let TailChannel {
        writer,
        reader,
        finalizer,
    } = TailChannel::new(pace)?;

    // The producer fills the tail file; the finalizer decides its fate once the
    // producer returns. songbird's `AsyncAdapterStream` spawns its own driver
    // that pulls the reader (waker-driven, no polling) into a sync MediaSource.
    // `in_current_span` carries the caller's `play_track` span (with the url) into
    // the detached task, so resume/chunk/finalize events stay attributed.
    tokio::spawn(
        async move {
            let result = producer(writer).await;
            finalizer.finish(result, cache);
        }
        .in_current_span(),
    );

    let adapter = AsyncAdapterStream::new(Box::new(reader), ADAPTER_BUF);
    Ok(Input::Live(
        LiveInput::Wrapped(AudioStream {
            input: MediaSourceStream::new(Box::new(adapter), MediaSourceStreamOptions::default()),
        }),
        None,
    ))
}

/// Progress shared between a download producer and the playback [`TailReader`]:
/// the producer advances the fields, the reader reads them, and `cancelled` flows
/// back (reader → producer) to stop the download early.
#[derive(Default)]
struct TailState {
    /// Bytes flushed to the OS so far, so the reader never outruns durable data.
    written: AtomicU64,
    /// Set once the producer has finished, successfully or not.
    done: AtomicBool,
    /// Set before `done` on failure, so a reader that observes `done` also sees
    /// the failure and never mistakes a truncated file for a clean EOF.
    failed: AtomicBool,
    /// Set by the reader on drop (a skip) so the producer stops early. The
    /// finalizer also reads it to skip caching the partial file.
    cancelled: AtomicBool,
    /// Wakes a reader that parked after catching up to the writer. An
    /// [`AtomicWaker`] survives across polls, avoiding the self-referential future
    /// a `Notify` would create.
    waker: AtomicWaker,
}

/// The write half of the tail file: appends bytes and publishes progress for the
/// paired [`TailReader`]. Owns the write handle, the byte counter, and the flush
/// discipline.
struct TailWriter {
    file: tokio::fs::File,
    state: Arc<TailState>,
    /// Bytes appended so far -- also the offset a chunked resume restarts from.
    written: u64,
}

impl TailWriter {
    fn new(file: tokio::fs::File, state: Arc<TailState>) -> Self {
        Self {
            file,
            state,
            written: 0,
        }
    }

    /// Append `bytes`, flush so the reader's separate handle sees them, then
    /// publish the new length and wake a parked reader. Flush-before-publish is the
    /// invariant that keeps the reader from outrunning durable data (tokio's `File`
    /// buffers otherwise).
    async fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.file.write_all(bytes).await?;
        self.file.flush().await?;
        self.written += bytes.len() as u64;
        self.state.written.store(self.written, Ordering::Release);
        self.state.waker.wake();
        Ok(())
    }

    /// Total bytes appended so far.
    fn written(&self) -> u64 {
        self.written
    }

    /// Has playback gone away (a skip)? Producers poll this to stop early.
    fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::Acquire)
    }
}

/// An [`AsyncMediaSource`] over a temp file still being written by a background
/// download producer (the sidecar `/download` or the direct-HTTP chunked path).
///
/// When the reader catches up to the writer:
///
/// - It parks -- returns `Poll::Pending` after registering its waker rather than
///   polling; the producer wakes it on each write and on completion.
/// - An empty region mid-download is never EOF: completion is signalled only via
///   [`TailState::done`], so playback can never be truncated.
/// - A producer failure surfaces as a read error, not a clean EOF, so songbird's
///   adapter propagates it and a truncated download is never played or cached.
struct TailReader {
    file: std::fs::File,
    pos: u64,
    state: Arc<TailState>,
    /// The chunked path's pacer: reports consumption and cancels on drop. `None`
    /// on the sidecar path.
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

impl Drop for TailReader {
    fn drop(&mut self) {
        // Playback is gone (a skip): flag the producer to stop. `pace.cancel()`
        // also wakes a chunked fetcher parked at its gate.
        self.state.cancelled.store(true, Ordering::Release);
        if let Some(pace) = self.pace.as_ref() {
            pace.cancel();
        }
    }
}

/// The finalize half of the tail file: once the producer returns, it decides the
/// file's fate. Owns the backing tempfile, so it can promote a completed download
/// to the replay cache and then unlink.
struct TailFinalizer {
    backing: NamedTempFile,
    state: Arc<TailState>,
}

impl TailFinalizer {
    /// Called once the producer finishes. On a skip (playback gone) drop the
    /// partial file; on `Err` surface the failure to the reader (never a truncated
    /// EOF, so a partial download is never played or cached); on `Ok` promote a
    /// fresh handle to `cache`'s replay slot. Publishes `failed` before `done`,
    /// wakes the reader last, then unlinks.
    fn finish(self, result: std::io::Result<()>, cache: Arc<YoutubeDlFileInner>) {
        if self.state.cancelled.load(Ordering::Acquire) {
            // Playback went away mid-track (a skip): the partial file is neither a
            // failure to surface nor a complete track to cache -- just drop it.
            tracing::debug!("tail download cancelled; not caching");
        } else if let Err(err) = result {
            // Order matters: publish `failed` before `done` so a reader that
            // observes `done` (Acquire) is guaranteed to see `failed` too.
            self.state.failed.store(true, Ordering::Release);
            tracing::warn!(error = %err, "tail download failed");
        } else {
            // A fresh handle, positioned at the start, becomes the replay cache.
            // Taken before `backing` drops (which unlinks the path). Not
            // `try_clone()`: cloned handles share a file offset, which would
            // corrupt concurrent read/write.
            match self.backing.reopen() {
                Ok(handle) => {
                    let _ = cache.file.set(handle);
                    tracing::info!("tail download cached");
                }
                Err(err) => tracing::warn!(error = %err, "failed to cache tail download"),
            }
        }
        self.state.done.store(true, Ordering::Release);
        self.state.waker.wake();
        // Unlink now; the open read/cache handles keep the data alive.
        drop(self.backing);
    }
}

/// The three coupled ends of one tail file, built together by [`TailChannel::new`]
/// (mirroring `chunked`'s `pace_channel`): a [`TailWriter`] for the background
/// producer, a [`TailReader`] for playback, and a [`TailFinalizer`] to promote or
/// discard the file once the producer finishes. All three share one [`TailState`].
struct TailChannel {
    writer: TailWriter,
    reader: TailReader,
    finalizer: TailFinalizer,
}

impl TailChannel {
    /// One tempfile with three independent handles (own cursors) over one shared
    /// [`TailState`]. `pace` is the reader's optional pacer (chunked path only).
    fn new(pace: Option<chunked::PaceReporter>) -> std::io::Result<Self> {
        let backing = NamedTempFile::new()?;
        let read_handle = backing.reopen()?;
        let write_handle = tokio::fs::File::from_std(backing.reopen()?);
        let state = Arc::new(TailState::default());
        Ok(Self {
            writer: TailWriter::new(write_handle, state.clone()),
            reader: TailReader {
                file: read_handle,
                pos: 0,
                state: state.clone(),
                pace,
            },
            finalizer: TailFinalizer { backing, state },
        })
    }
}

/// Drain the sidecar `/download` response body into `tail` until a clean EOF or a
/// skip. A stream or tail-file write error propagates as [`std::io::Error`].
async fn drain_response(mut resp: reqwest::Response, mut tail: TailWriter) -> std::io::Result<()> {
    // A skip flips `is_cancelled`, ending the loop (not a failure).
    while !tail.is_cancelled() {
        match resp.chunk().await.map_err(std::io::Error::other)? {
            Some(bytes) => tail.write(&bytes).await?,
            None => break,
        }
    }
    Ok(())
}

/// Drain the resume-capable `src` into `tail`. On a read error it rebuilds `src`
/// from the bytes written so far via [`AsyncMediaSource::try_resume`] and
/// continues; only an exhausted resume (or a tail-file write error) returns `Err`.
/// A skip arrives through the pacer as a clean EOF.
async fn drain_resuming(
    mut src: Box<dyn AsyncMediaSource>,
    mut tail: TailWriter,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match src.read(&mut buf).await {
            Ok(0) => return Ok(()),
            Ok(n) => tail.write(&buf[..n]).await?,
            Err(read_err) => {
                src = src.try_resume(tail.written()).await.map_err(|resume_err| {
                    std::io::Error::other(format!(
                        "chunked resume failed (read error: {read_err}): {resume_err}"
                    ))
                })?;
            }
        }
    }
}
