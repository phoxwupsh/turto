use super::{
    Cancel, MediaSource, TailHandle, TailReader, TailState, TailWriter, drain_blocking,
    drain_response,
};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// The tail source must return every byte in order and must never signal EOF
/// while the download is in progress (which would truncate playback),
/// regardless of read vs. write pacing. Exercises the park/wake path: the
/// reader repeatedly catches up and is woken by the producer.
#[tokio::test]
async fn tail_reader_reads_all_bytes_without_early_eof() {
    let backing = NamedTempFile::new().unwrap();
    let read_handle = backing.reopen().unwrap();
    let mut write_handle = backing.reopen().unwrap();
    let state = Arc::new(TailState::default());

    let data: Vec<u8> = (0..100_000u32).map(|i| i as u8).collect();
    let producer_state = state.clone();
    let producer_data = data.clone();
    // Write slowly so the reader repeatedly parks and must be woken.
    let producer = std::thread::spawn(move || {
        let mut written = 0u64;
        for chunk in producer_data.chunks(4096) {
            write_handle.write_all(chunk).unwrap();
            write_handle.flush().unwrap();
            written += chunk.len() as u64;
            producer_state.written.store(written, Ordering::Release);
            producer_state.waker.wake();
            std::thread::sleep(Duration::from_millis(1));
        }
        producer_state.done.store(true, Ordering::Release);
        producer_state.waker.wake();
    });

    let mut source = TailReader {
        file: read_handle,
        pos: 0,
        state,
        pace: None,
    };
    let mut out = Vec::new();
    source.read_to_end(&mut out).await.unwrap();
    producer.join().unwrap();

    assert_eq!(out, data, "tailed bytes must match what was written");
}

/// A failed download must surface as a read error once the buffer drains,
/// never a clean EOF -- otherwise a truncated file would be cached/played.
#[tokio::test]
async fn tail_reader_surfaces_failure_as_error() {
    let backing = NamedTempFile::new().unwrap();
    let read_handle = backing.reopen().unwrap();
    let mut write_handle = backing.reopen().unwrap();
    let state = Arc::new(TailState::default());

    write_handle.write_all(b"partial").unwrap();
    write_handle.flush().unwrap();
    state.written.store(7, Ordering::Release);
    // Publish failed before done, as the producer does.
    state.failed.store(true, Ordering::Release);
    state.done.store(true, Ordering::Release);

    let mut source = TailReader {
        file: read_handle,
        pos: 0,
        state,
        pace: None,
    };
    let mut out = Vec::new();
    let err = source.read_to_end(&mut out).await.unwrap_err();

    assert_eq!(out, b"partial", "buffered bytes are served before failing");
    assert_eq!(err.kind(), std::io::ErrorKind::Other);
}

/// The sidecar `/download` tail has no pacer, but cancelling the track must still
/// stop it: `drain_response` polls the shared signal and returns at the next chunk
/// boundary instead of draining a whole (possibly hour-long) response.
#[tokio::test]
async fn sidecar_producer_stops_on_track_cancel() {
    let piece = vec![7u8; 4096];
    let pieces = 50usize;
    let len = piece.len() * pieces;

    // A server that streams the body slowly, piece by piece, so the client is
    // still draining when the reader is dropped mid-way.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let piece_srv = piece.clone();
    tokio::spawn(async move {
        let Ok((mut sock, _)) = listener.accept().await else {
            return;
        };
        let mut req = Vec::new();
        let mut tmp = [0u8; 1024];
        loop {
            match sock.read(&mut tmp).await {
                Ok(0) => return,
                Ok(n) => req.extend_from_slice(&tmp[..n]),
                Err(_) => return,
            }
            if req.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let head =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n");
        if sock.write_all(head.as_bytes()).await.is_err() {
            return;
        }
        for _ in 0..pieces {
            // A write failure means the client hung up (the response was dropped
            // once the producer stopped) -- so stop sending.
            if sock.write_all(&piece_srv).await.is_err() {
                return;
            }
            let _ = sock.flush().await;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });

    let resp = reqwest::Client::new().get(&url).send().await.unwrap();

    let backing = NamedTempFile::new().unwrap();
    let write_handle = tokio::fs::File::from_std(backing.reopen().unwrap());
    let state = Arc::new(TailState::default());
    let cancel: Arc<Cancel> = Arc::default();

    // Sidecar path: no pacer, so the producer only ever observes the signal between
    // body chunks -- no reader is involved in stopping it.
    let tail = TailWriter::new(write_handle, state.clone(), cancel.clone());
    let producer = tokio::spawn(async move { drain_response(resp, tail).await });

    // Let it drain a few pieces, then drop the track.
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !producer.is_finished(),
        "producer must still be draining the slow response"
    );
    cancel.cancel();

    tokio::time::timeout(Duration::from_secs(2), producer)
        .await
        .expect("a cancelled sidecar producer must return promptly")
        .unwrap()
        .expect("a cancelled sidecar producer ends cleanly");
    assert!(
        (state.written.load(Ordering::Acquire) as usize) < len,
        "cancel must stop the download before the whole body is drained"
    );
}

/// The point of [`TailHandle`]: one download, readers minted whenever they are
/// wanted. A reader attached mid-download serves what is durable and parks for the
/// rest, and a reader minted after completion replays the whole file -- which is what
/// lets a prefetch nobody is listening to yet become the playback that arrives later.
#[tokio::test]
async fn tail_handle_mints_readers_at_any_point() {
    let cancel: Arc<Cancel> = Arc::default();
    let (handle, mut writer) = TailHandle::new(cancel.clone(), None).unwrap();
    let head: &[u8] = b"first-half-";
    let rest: &[u8] = b"second-half";

    writer.write(head).await.unwrap();

    // Attach mid-download.
    let mut mid = handle.reader().unwrap();
    let mut buf = vec![0u8; head.len()];
    mid.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, head, "a late reader starts at the beginning of the file");

    writer.write(rest).await.unwrap();
    handle.finish(Ok(()), &cancel);

    // The mid-download reader carries on to a clean EOF.
    let mut tail_bytes = Vec::new();
    mid.read_to_end(&mut tail_bytes).await.unwrap();
    assert_eq!(tail_bytes, rest);

    // A reader minted after completion replays from the start.
    let mut late = handle.reader().unwrap();
    let mut all = Vec::new();
    late.read_to_end(&mut all).await.unwrap();
    assert_eq!(all, [head, rest].concat(), "a replay reader sees everything");
}

/// A reader minted after a *failed* download must surface the failure, not a clean
/// EOF over the partial file -- otherwise attaching late to a dead prefetch would
/// silently play a truncated track.
#[tokio::test]
async fn tail_handle_reader_after_failure_errors() {
    let cancel: Arc<Cancel> = Arc::default();
    let (handle, mut writer) = TailHandle::new(cancel.clone(), None).unwrap();

    writer.write(b"partial").await.unwrap();
    handle.finish(Err(std::io::Error::other("producer blew up")), &cancel);

    let mut reader = handle.reader().unwrap();
    assert!(handle.failed(), "the owner must see the tail as unusable");
    let mut out = Vec::new();
    let err = reader.read_to_end(&mut out).await.unwrap_err();

    assert_eq!(out, b"partial", "durable bytes are served before failing");
    assert_eq!(err.kind(), std::io::ErrorKind::Other);
}

/// A sync [`MediaSource`], which is all songbird's `HlsRequest` will hand out. Reads
/// are small and slow so the bridge has to make many hops and a cancel has somewhere
/// to land.
struct SlowSyncSource {
    data: std::io::Cursor<Vec<u8>>,
    per_read: usize,
}

impl std::io::Read for SlowSyncSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::thread::sleep(Duration::from_millis(2));
        let take = self.per_read.min(buf.len());
        std::io::Read::read(&mut self.data, &mut buf[..take])
    }
}

impl std::io::Seek for SlowSyncSource {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.data, pos)
    }
}

impl MediaSource for SlowSyncSource {
    fn is_seekable(&self) -> bool {
        false
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

/// The HLS path can only be pulled by a blocking read, so it is bridged off the
/// runtime. The bridge must still deliver every byte in order.
#[tokio::test]
async fn hls_bridge_writes_all_bytes() {
    let body: Vec<u8> = (0..40_000).map(|i| (i % 251) as u8).collect();
    let cancel: Arc<Cancel> = Arc::default();
    let (handle, writer) = TailHandle::new(cancel.clone(), None).unwrap();

    let src = Box::new(SlowSyncSource {
        data: std::io::Cursor::new(body.clone()),
        per_read: 4096,
    });
    drain_blocking(src, writer)
        .await
        .expect("the bridge must drain a sync source cleanly");
    handle.finish(Ok(()), &cancel);

    let mut out = Vec::new();
    handle.reader().unwrap().read_to_end(&mut out).await.unwrap();
    assert_eq!(out, body, "bridged bytes must match the source exactly");
}

/// Cancelling the track must stop the blocking side within one read, rather than
/// pulling a whole HLS track nobody will listen to.
#[tokio::test]
async fn hls_bridge_stops_on_track_cancel() {
    let len = 400_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let cancel: Arc<Cancel> = Arc::default();
    let (handle, writer) = TailHandle::new(cancel.clone(), None).unwrap();

    let src = Box::new(SlowSyncSource {
        data: std::io::Cursor::new(body),
        per_read: 4096,
    });
    let producer = tokio::spawn(async move { drain_blocking(src, writer).await });

    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !producer.is_finished(),
        "producer must still be pulling the slow source"
    );
    cancel.cancel();

    tokio::time::timeout(Duration::from_secs(2), producer)
        .await
        .expect("a cancelled bridge must return promptly")
        .unwrap()
        .expect("a cancelled bridge ends cleanly");
    assert!(
        (handle.written() as usize) < len,
        "cancel must stop the pull before the whole source is read"
    );
}
