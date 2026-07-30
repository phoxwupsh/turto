use super::*;
use crate::ytdl::test_support::{Fault, request, serve_ranges};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Drain the byte stream to its end. Nothing recovers here -- [`ChunkRequest`] only
/// fetches, and reports why it stopped; recovery is [`super::super::direct`]'s job.
async fn read_all(req: ChunkRequest) -> Result<Vec<u8>, FetchError> {
    let mut stream = req.open(0).await?;
    let mut out = Vec::new();
    while let Some(bytes) = stream.next().await.transpose()? {
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// A server that answers every request with `status` and an empty body.
async fn serve_status(status: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut tmp = [0u8; 1024];
                let _ = sock.read(&mut tmp).await;
                let head =
                    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });
    url
}

/// The reassembled bytes must equal the original file, and the fetch must issue
/// exactly `ceil(len / chunk)` range requests (never one oversized request that
/// googlevideo would throttle).
#[tokio::test]
async fn chunks_reassemble_in_order_with_expected_request_count() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body.clone(), Fault::None).await;

    // -> chunks of 10k, 10k, 5k == 3 requests
    let out = read_all(request(srv.url, len, 10_000)).await.expect("read to end");

    assert_eq!(out, body, "reassembled bytes must match the original file");
    assert_eq!(
        srv.requests.load(Ordering::SeqCst),
        3,
        "expected ceil(25000/10000) = 3 range requests"
    );
}

/// A connection dropped mid-chunk must surface as a `Transport` failure -- something
/// worth another attempt at the same URL -- and never as a short clean end, which
/// would cache a truncated track.
#[tokio::test]
async fn mid_stream_drop_is_a_transport_failure() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body.clone(), Fault::DropMidBodyOnce).await;

    let err = read_all(request(srv.url, len, 10_000))
        .await
        .expect_err("a truncated body must not read as a clean end");

    assert!(
        srv.faulted.load(Ordering::SeqCst),
        "the server must have injected a mid-stream drop"
    );
    assert!(matches!(err, FetchError::Transport(_)), "got {err:?}");
}

/// A well-formed 206 that under-fills its requested range (consistent framing, so no
/// transport error) must not leave a gap: the next range starts where delivery
/// actually stopped, and the bytes still reassemble exactly. With requested-end
/// bookkeeping this would instead skip bytes and end as a *clean* (cacheable!) EOF.
#[tokio::test]
async fn short_range_response_realigns_without_gap() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body.clone(), Fault::ShortBodyOnce).await;

    let out = read_all(request(srv.url, len, 10_000)).await.expect("read to end");

    assert!(
        srv.faulted.load(Ordering::SeqCst),
        "the server must have injected a short response"
    );
    assert_eq!(
        out, body,
        "reassembled bytes must match the original despite the short response"
    );
}

/// A server answering with well-formed *empty* bodies makes no progress. That has to
/// be an error rather than a clean truncated EOF, and it must not refetch the same
/// range forever waiting for bytes that will never come.
#[tokio::test]
async fn zero_progress_chunks_error_out_instead_of_looping() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body, Fault::EmptyBodyAlways).await;

    let err = read_all(request(srv.url, len, 10_000))
        .await
        .expect_err("zero progress must be an error, not a clean EOF");

    assert!(matches!(err, FetchError::Transport(_)), "got {err:?}");
    assert!(
        srv.requests.load(Ordering::SeqCst) <= 4,
        "the no-progress guard must fire immediately, not after a refetch loop; got {}",
        srv.requests.load(Ordering::SeqCst)
    );
}

/// A `416` inside the declared length means the server's object is shorter than the
/// format said, so the bytes already fetched are not part of what it will serve. That
/// must be an error: treating it as the end (which is what a `416` means when no length
/// is known) would publish a short tail as the whole track and cache it.
#[tokio::test]
async fn unsatisfiable_range_inside_the_declared_length_is_truncation() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body, Fault::UnsatisfiableInsideBody).await;

    let err = read_all(request(srv.url, len, 10_000))
        .await
        .expect_err("a 416 inside the declared length must not read as a clean end");

    assert!(
        srv.faulted.load(Ordering::SeqCst),
        "the server must have refused a range inside the body"
    );
    // `declared` proves the `Content-Range: bytes */N` was read: the server's view of
    // the length, which is what distinguishes this from a one-off bad range.
    assert!(
        matches!(
            err,
            FetchError::Truncated {
                at: 10_000,
                total: 25_000,
                declared: Some(12_500),
            }
        ),
        "got {err:?}"
    );
}

/// The flip side, and the reason the check above is conditional: with no declared length
/// a `416` is the *only* way to learn where the file ends, so walking off the end must
/// still be a clean end and the track must come out whole. Breaking this would fail
/// every track whose format reports no `filesize`.
#[tokio::test]
async fn unsatisfiable_range_ends_cleanly_when_the_length_is_unknown() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body.clone(), Fault::None).await;

    // `None` total: the fetch cannot know where to stop.
    let mut req = ChunkRequest::new(reqwest::Client::new(), srv.url, HeaderMap::new(), None);
    req.set_chunk(10_000);
    let out = read_all(req).await.expect("read to end");

    assert_eq!(out, body, "the whole file must be reassembled");
    assert_eq!(
        srv.requests.load(Ordering::SeqCst),
        4,
        "3 ranges of body plus the 416 that discovers the end"
    );
}

/// The classification that drives the whole recovery policy: how googlevideo answers
/// an expired signature has to come out as `Rejected`, because that is the only
/// failure a fresh extract can fix and the only one where retrying is futile.
#[tokio::test]
async fn expired_signature_is_reported_as_rejected() {
    for status in ["401 Unauthorized", "403 Forbidden", "410 Gone"] {
        let url = serve_status(status).await;
        let Err(err) = request(url, 25_000, 10_000).open(0).await else {
            panic!("{status}: a rejected range must fail the open");
        };
        assert!(
            matches!(err, FetchError::Rejected(_)),
            "{status} must be Rejected, got {err:?}"
        );
    }
}

/// Everything else is transient as far as this layer knows, so it must *not* be
/// reported as `Rejected` -- re-extracting would not help and would waste the budget.
#[tokio::test]
async fn server_errors_are_reported_as_transport() {
    for status in ["500 Internal Server Error", "429 Too Many Requests"] {
        let url = serve_status(status).await;
        let Err(err) = request(url, 25_000, 10_000).open(0).await else {
            panic!("{status}: a server error must fail the open");
        };
        assert!(
            matches!(err, FetchError::Transport(_)),
            "{status} must be Transport, got {err:?}"
        );
    }
}

/// Rebinding swaps the signed URL and nothing else, so a fetch reopened at the offset
/// it got to picks up exactly where the dead URL stopped -- no gap, no overlap.
#[tokio::test]
async fn rebind_resumes_at_the_same_offset_on_a_new_url() {
    let len = 25_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let dead = serve_ranges(body.clone(), Fault::None).await;
    let fresh = serve_ranges(body.clone(), Fault::None).await;

    let mut req = request(dead.url, len, 10_000);
    let mut stream = req.open(0).await.expect("open on the first url");
    let mut out = Vec::new();
    // Take exactly the first chunk, then pretend the url died.
    while out.len() < 10_000 {
        let bytes = stream.next().await.expect("first chunk").expect("no error");
        out.extend_from_slice(&bytes);
    }
    drop(stream);

    req.rebind(fresh.url, HeaderMap::new());
    let mut stream = req
        .open(out.len() as u64)
        .await
        .expect("reopen on the fresh url");
    while let Some(bytes) = stream.next().await.transpose().expect("no error") {
        out.extend_from_slice(&bytes);
    }

    assert_eq!(out, body, "the swap must leave the track byte-identical");
    assert_eq!(
        fresh.requests.load(Ordering::SeqCst),
        2,
        "the fresh url serves only the remaining 15k == 2 ranges"
    );
}

/// Within the window there is room, so the fetcher proceeds without waiting.
#[tokio::test]
async fn pacer_proceeds_within_window() {
    let (gate, reporter) = pace_channel(100, Arc::default());
    reporter.advance(100); // something is reading, so the window is open
    // 200 - 100 == window: the edge still counts as room.
    assert!(gate.await_room(200).await);
}

/// An *unattached* prefetch gets one chunk, not two: with nothing reading, the window
/// is closed, so the fetcher parks on the chunk it already has rather than filling the
/// read-ahead a playing track would get. Consuming the first byte is what opens it.
#[tokio::test]
async fn pacer_holds_an_unread_prefetch_to_one_chunk() {
    let (gate, reporter) = pace_channel(100, Arc::default());
    // 100 - 0 == window, so an attached reader would be waved through here.
    let handle = tokio::spawn(async move { gate.await_room(100).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !handle.is_finished(),
        "an unread prefetch must park at one chunk"
    );

    reporter.advance(1);
    let proceeded = tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("the first byte read must release the fetcher")
        .unwrap();
    assert!(proceeded, "released with room, not cancelled");
}

/// Too far ahead, the fetcher parks until the consumer advances into range.
#[tokio::test]
async fn pacer_blocks_until_consumer_advances() {
    let (gate, reporter) = pace_channel(100, Arc::default());
    reporter.advance(100); // open the window, so distance is the only thing tested
    // 300 - 100 = 200 > 100: must wait.
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

/// Cancelling the *track* wakes a parked fetcher and reports the consumer as gone.
#[tokio::test]
async fn pacer_cancel_releases_the_fetcher() {
    let cancel: Arc<Cancel> = Arc::default();
    let (gate, _reporter) = pace_channel(100, cancel.clone());
    let handle = tokio::spawn(async move { gate.await_room(300).await });
    tokio::time::sleep(Duration::from_millis(20)).await;

    cancel.cancel();
    let proceeded = tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("cancel must wake the fetcher")
        .unwrap();
    assert!(!proceeded, "cancel reports the consumer gone");
}
