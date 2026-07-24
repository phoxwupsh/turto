use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Misbehavior [`serve_ranges`] injects into requests at a non-zero offset
/// (so the eager first chunk, and with it `create_async`, always succeeds).
#[derive(Clone, Copy)]
enum Fault {
    /// Serve every request in full.
    None,
    /// Once: announce the full range length, send half, then hang up -- a
    /// transport-level truncation the client sees as a body error.
    DropMidBodyOnce,
    /// Once: serve a *well-formed* response (consistent framing) carrying
    /// only the first half of the requested range -- no transport error.
    ShortBodyOnce,
    /// Always: serve a well-formed response with an empty body.
    EmptyBodyAlways,
}

/// A minimal HTTP/1.1 server that serves `body` honoring a single
/// `Range: bytes=a-b` header, one request per connection, optionally
/// injecting `fault`. Returns the media URL, a counter of answered
/// requests, and a flag set once a fault has been injected.
async fn serve_ranges(body: Vec<u8>, fault: Fault) -> (String, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let faulted = Arc::new(AtomicBool::new(false));
    let count_srv = count.clone();
    let faulted_srv = faulted.clone();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let body = body.clone();
            let count = count_srv.clone();
            let faulted = faulted_srv.clone();
            tokio::spawn(async move {
                // Read request headers (until CRLFCRLF).
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
                count.fetch_add(1, Ordering::SeqCst);

                // Parse `range: bytes=a-b` (header names are case-insensitive).
                let text = String::from_utf8_lossy(&req).to_ascii_lowercase();
                let spec = text
                    .lines()
                    .find_map(|l| l.trim().strip_prefix("range: bytes="))
                    .expect("test client always sends a closed Range");
                let (a, b) = spec.split_once('-').unwrap();
                let start: usize = a.trim().parse().unwrap();
                let end_incl: usize = b.trim().parse().unwrap();
                let slice = &body[start..=end_incl];

                let inject = start > 0
                    && match fault {
                        Fault::None => false,
                        Fault::EmptyBodyAlways => {
                            faulted.store(true, Ordering::SeqCst);
                            true
                        }
                        Fault::DropMidBodyOnce | Fault::ShortBodyOnce => {
                            !faulted.swap(true, Ordering::SeqCst)
                        }
                    };
                // How much the header announces vs. how much is actually sent;
                // a mismatch (DropMidBodyOnce) is a transport error client-side.
                let (announced, served) = match (inject, fault) {
                    (true, Fault::DropMidBodyOnce) => (slice.len(), slice.len() / 2),
                    (true, Fault::ShortBodyOnce) => (slice.len() / 2, slice.len() / 2),
                    (true, Fault::EmptyBodyAlways) => (0, 0),
                    _ => (slice.len(), slice.len()),
                };

                let head = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                    announced,
                    start,
                    start + announced.max(1) - 1,
                    body.len(),
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(&slice[..served]).await;
                let _ = sock.flush().await;
            });
        }
    });

    (url, count, faulted)
}

/// Open the chunked stream and read it to the end. The `MediaSource` is a
/// sync bridge, so it is read off the runtime, letting its async sink
/// (spawned by [`AsyncAdapterStream`]) make progress.
async fn read_all(mut req: ChunkedHttpRequest) -> io::Result<Vec<u8>> {
    let audio = req.create_async().await.expect("open chunked stream");
    tokio::task::spawn_blocking(move || {
        let mut input = audio.input;
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut input, &mut out)?;
        Ok(out)
    })
    .await
    .unwrap()
}

/// The reassembled bytes must equal the original file, and the source must
/// issue exactly `ceil(len / chunk)` range requests (never one oversized
/// request that googlevideo would throttle).
#[tokio::test]
async fn chunks_reassemble_in_order_with_expected_request_count() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let (url, count, _) = serve_ranges(body.clone(), Fault::None).await;

    let mut req = ChunkedHttpRequest::new(
        reqwest::Client::new(),
        url,
        HeaderMap::new(),
        Some(len as u64),
    );
    req.spec.chunk = 10_000; // -> chunks of 10k, 10k, 5k == 3 requests

    let out = read_all(req).await.expect("read to end");

    assert_eq!(out, body, "reassembled bytes must match the original file");
    assert_eq!(
        count.load(Ordering::SeqCst),
        3,
        "expected ceil(25000/10000) = 3 range requests"
    );
}

/// A connection dropped mid-chunk must not truncate playback: the source
/// resumes from where it left off (via `try_resume`) and the reassembled
/// bytes still equal the original file.
#[tokio::test]
async fn mid_stream_drop_resumes_without_truncation() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let (url, _, faulted) = serve_ranges(body.clone(), Fault::DropMidBodyOnce).await;

    let mut req = ChunkedHttpRequest::new(
        reqwest::Client::new(),
        url,
        HeaderMap::new(),
        Some(len as u64),
    );
    req.spec.chunk = 10_000;

    let out = read_all(req).await.expect("read to end");

    assert!(
        faulted.load(Ordering::SeqCst),
        "the server must have injected a mid-stream drop"
    );
    assert_eq!(
        out, body,
        "reassembled bytes must match the original despite the mid-stream drop"
    );
}

/// A well-formed 206 that under-fills its requested range (consistent
/// framing, so no transport error and no resume) must not leave a gap: the
/// next range starts where delivery actually stopped, and the bytes still
/// reassemble exactly. With requested-end bookkeeping this would instead
/// skip bytes and end the stream early as a *clean* (cacheable!) EOF.
#[tokio::test]
async fn short_range_response_realigns_without_gap() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let (url, _, faulted) = serve_ranges(body.clone(), Fault::ShortBodyOnce).await;

    let mut req = ChunkedHttpRequest::new(
        reqwest::Client::new(),
        url,
        HeaderMap::new(),
        Some(len as u64),
    );
    req.spec.chunk = 10_000;

    let out = read_all(req).await.expect("read to end");

    assert!(
        faulted.load(Ordering::SeqCst),
        "the server must have injected a short response"
    );
    assert_eq!(
        out, body,
        "reassembled bytes must match the original despite the short response"
    );
}

/// A server that keeps answering with well-formed empty bodies must surface
/// an error (no-progress guard, bounded by the resume stall cap) -- neither
/// a clean truncated EOF nor an unbounded refetch loop.
#[tokio::test]
async fn zero_progress_chunks_error_out_instead_of_looping() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let (url, count, _) = serve_ranges(body, Fault::EmptyBodyAlways).await;

    let mut req = ChunkedHttpRequest::new(
        reqwest::Client::new(),
        url,
        HeaderMap::new(),
        Some(len as u64),
    );
    req.spec.chunk = 10_000;

    let res = read_all(req).await;

    assert!(
        res.is_err(),
        "zero progress must be an error, not a clean EOF"
    );
    assert!(
        count.load(Ordering::SeqCst) <= 20,
        "request count must stay bounded by the stall guard, got {}",
        count.load(Ordering::SeqCst)
    );
}

/// Within the window there is room, so the fetcher proceeds without waiting.
#[tokio::test]
async fn pacer_proceeds_within_window() {
    let (gate, _reporter) = pace_channel(100);
    // 100 - 0 == window: the edge still counts as room.
    assert!(gate.await_room(100).await);
}

/// Too far ahead, the fetcher parks until the consumer advances into range.
#[tokio::test]
async fn pacer_blocks_until_consumer_advances() {
    let (gate, reporter) = pace_channel(100);
    // 300 - 0 = 300 > 100: must wait.
    let handle = tokio::spawn(async move { gate.await_room(300).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !handle.is_finished(),
        "fetcher must park while too far ahead"
    );

    reporter.advance(250); // 300 - 250 = 50 <= 100
    let proceeded = tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("advancing the consumer must release the fetcher")
        .unwrap();
    assert!(proceeded, "released with room, not cancelled");
}

/// Cancelling wakes a parked fetcher and reports the consumer as gone.
#[tokio::test]
async fn pacer_cancel_releases_the_fetcher() {
    let (gate, reporter) = pace_channel(100);
    let handle = tokio::spawn(async move { gate.await_room(300).await });
    tokio::time::sleep(Duration::from_millis(20)).await;

    reporter.cancel();
    let proceeded = tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("cancel must wake the fetcher")
        .unwrap();
    assert!(!proceeded, "cancel reports the consumer gone");
}
