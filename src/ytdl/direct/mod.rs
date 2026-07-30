//! The direct-HTTP byte producer: fetch a track's media by byte-range chunks into its
//! tail file, and keep it going.
//!
//! [`chunked`] fetches and reports *why* a range failed; what to do next lives here,
//! the layer that knows the track's watch URL and can re-extract. Recovery always
//! resumes from what is durable in the tail file, never past it -- a gap would read as
//! a clean early EOF and cache a truncated track.

use super::{
    YouTubeDlMetadata,
    cancel::Cancel,
    chunked::{self, ByteStream, ChunkRequest, FetchError, PaceReporter},
    sidecar, source,
    tail::TailWriter,
};
use futures::StreamExt;
use serde::Deserialize;
use std::{io, sync::Arc, time::Duration};

#[cfg(test)]
mod test;

/// How far ahead of consumption the fetcher may run. One chunk is ~10 min of opus:
/// always in hand before it is needed, and little wasted if the track is skipped.
const READ_AHEAD: u64 = chunked::HTTP_CHUNK;

/// Attempts against the same URL before a stuck fetch is declared dead.
const MAX_RETRIES: u32 = 5;
/// Re-extracts before a stuck fetch is declared dead. More than one covers a track
/// long enough to outlive two signed URLs; unbounded would hammer the sidecar.
const MAX_REFRESHES: u32 = 2;
/// Pause between attempts against the same URL. The tail file's read-ahead hides it.
const RETRY_BACKOFF: Duration = Duration::from_millis(500);

/// What to do about a failed fetch.
#[derive(Debug, PartialEq, Eq)]
enum Recovery {
    /// Reopen at the durable offset, same URL, after a backoff.
    Retry,
    /// Re-extract for a fresh signed URL, then reopen at the durable offset.
    Refresh,
    /// Out of budget, or unrecoverable: the track ends here.
    Fail,
}

/// The recovery budget for one download. **Progress resets it**, so a multi-hour track
/// rides out any number of isolated blips while one stuck at an offset fails promptly.
#[derive(Debug)]
struct Attempts {
    /// Offset the current streak of failures started at. Anything past it is
    /// progress.
    stuck_at: u64,
    retries: u32,
    refreshes: u32,
}

impl Attempts {
    fn new(offset: u64) -> Self {
        Self {
            stuck_at: offset,
            retries: 0,
            refreshes: 0,
        }
    }

    /// Decide what to do about `err`, having got as far as `offset`.
    fn next(&mut self, offset: u64, err: &FetchError) -> Recovery {
        if offset > self.stuck_at {
            *self = Self::new(offset);
        }
        match err {
            // Neither remedy applies: the tail holds bytes this URL does not serve, so
            // there is nothing to resume from. Failing discards the tail, and the next
            // play starts clean.
            FetchError::Truncated { .. } => Recovery::Fail,
            FetchError::Rejected(_) if self.refreshes < MAX_REFRESHES => {
                self.refreshes += 1;
                Recovery::Refresh
            }
            // A dead URL cannot come back, so retrying it only delays the inevitable.
            FetchError::Rejected(_) => Recovery::Fail,
            _ if self.retries < MAX_RETRIES => {
                self.retries += 1;
                Recovery::Retry
            }
            _ => Recovery::Fail,
        }
    }
}

/// Why one attempt stopped. Only a fetch failure is worth recovering from: a
/// tail-file write failure means the local disk is the problem, and refetching the
/// same bytes cannot fix it.
enum Stopped {
    Fetch(FetchError),
    Write(io::Error),
}

/// Fetches one track's media into its tail file, recovering as it goes.
pub(super) struct DirectFetch {
    /// The **watch** URL, which never expires -- so re-extracting needs no handle on the
    /// track, and this producer cannot keep it alive.
    webpage_url: String,
    req: ChunkRequest,
    attempts: Attempts,
    /// The stream [`Self::open`] already opened. [`Self::run`] drains this first and
    /// opens its own after each recovery.
    opened: Option<ByteStream>,
}

impl DirectFetch {
    /// Open the fetch, eagerly, so a dead track fails *here* where a command can report
    /// it. Also returns the [`PaceReporter`] the tail hands to every reader it mints.
    pub(super) async fn open(
        webpage_url: String,
        req: ChunkRequest,
        cancel: Arc<Cancel>,
    ) -> Result<(Self, PaceReporter), FetchError> {
        Self::open_with_window(webpage_url, req, cancel, READ_AHEAD).await
    }

    /// [`Self::open`] with an explicit read-ahead window, so tests can exercise pacing
    /// without multi-megabyte bodies.
    async fn open_with_window(
        webpage_url: String,
        mut req: ChunkRequest,
        cancel: Arc<Cancel>,
        window: u64,
    ) -> Result<(Self, PaceReporter), FetchError> {
        let (gate, reporter) = chunked::pace_channel(window, cancel);
        req.set_pace(gate);
        let opened = req.open(0).await?;
        Ok((
            Self {
                webpage_url,
                req,
                attempts: Attempts::new(0),
                opened: Some(opened),
            },
            reporter,
        ))
    }

    /// Drain the media into `tail` until it ends, recovering per [`Attempts`].
    ///
    /// `Ok(())` is a clean end -- the whole track, or a cancellation. `Err` means the
    /// track ends early, and reaches the reader as a read error rather than a short
    /// file.
    pub(super) async fn run(mut self, mut tail: TailWriter) -> io::Result<()> {
        loop {
            let err = match self.attempt(&mut tail).await {
                Ok(()) => return Ok(()),
                Err(Stopped::Write(err)) => return Err(err),
                Err(Stopped::Fetch(err)) => err,
            };
            // The track may have been dropped while that attempt was failing. Recovery
            // is not free -- `Refresh` spends a whole sidecar extract -- and `attempt`
            // only notices a cancel after reopening a range.
            if tail.is_cancelled() {
                return Ok(());
            }
            let offset = tail.written();
            match self.attempts.next(offset, &err) {
                Recovery::Retry => {
                    tracing::info!(offset, error = %err, "fetch failed; retrying");
                    tokio::time::sleep(RETRY_BACKOFF).await;
                }
                Recovery::Refresh => {
                    tracing::info!(offset, error = %err, "signed url is dead; re-extracting");
                    self.refresh().await?;
                }
                Recovery::Fail => {
                    tracing::warn!(offset, error = %err, "fetch unrecoverable; track will end early");
                    return Err(err.into());
                }
            }
        }
    }

    /// One attempt: take the already-open stream, or open one at whatever is durable,
    /// then feed the tail until the media ends, the track is cancelled, or something
    /// fails.
    async fn attempt(&mut self, tail: &mut TailWriter) -> Result<(), Stopped> {
        let mut stream = match self.opened.take() {
            Some(stream) => stream,
            None => self
                .req
                .open(tail.written())
                .await
                .map_err(Stopped::Fetch)?,
        };
        // Checking per chunk lands a cancel within one response piece rather than one
        // 10 MB range; the pace gate catches a *parked* fetcher separately.
        while !tail.is_cancelled() {
            let Some(bytes) = stream.next().await.transpose().map_err(Stopped::Fetch)? else {
                break;
            };
            tail.write(&bytes).await.map_err(Stopped::Write)?;
        }
        Ok(())
    }

    /// Re-extract from the watch URL and point the fetch at the fresh signed URL.
    ///
    /// The fresh format must still be byte-range HTTP *and* [`same_format`] as the one
    /// already in the tail file -- resuming at the current offset means nothing
    /// otherwise. Either failure ends the track, which is recoverable: a failed tail is
    /// discarded, so the next play starts clean.
    async fn refresh(&mut self) -> io::Result<()> {
        let info = sidecar::extract(&self.webpage_url, false)
            .await
            .map_err(io::Error::other)?;
        let meta = YouTubeDlMetadata::deserialize(&info).map_err(io::Error::other)?;
        let (url, headers) = source::direct_url(&meta)
            .ok_or_else(|| io::Error::other("format is no longer a direct http download"))?;
        if !same_format(self.req.total(), meta.filesize) {
            return Err(io::Error::other(format!(
                "re-extract changed the format ({:?} -> {:?} bytes); cannot resume in place",
                self.req.total(),
                meta.filesize,
            )));
        }
        self.req.rebind(url, headers);
        Ok(())
    }
}

/// Is a re-extracted format byte-identical to the one already in the tail file?
///
/// Declared length is the proxy. yt-dlp's format selection is not a promise, and
/// appending a different encoding at the current offset corrupts the file from the seam
/// on -- silently, since both halves are valid HTTP. So an *unverifiable* length counts
/// as a mismatch: being wrong this way costs one restart, the other way costs the track.
fn same_format(before: Option<u64>, after: Option<u64>) -> bool {
    matches!((before, after), (Some(before), Some(after)) if before == after)
}
