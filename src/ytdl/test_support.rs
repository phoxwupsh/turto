//! Shared fixtures for the byte-path tests: a range-honouring HTTP server with
//! injectable misbehaviour, needed by both [`chunked`](super::chunked) and
//! [`direct`](super::direct). One copy, because two drifted -- only one of them learned
//! that a range overhanging EOF must clamp rather than panic.

use super::chunked::ChunkRequest;
use reqwest::header::HeaderMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Misbehaviour [`serve_ranges`] injects, always at a non-zero offset so the eager
/// first range -- and with it the open -- still succeeds.
#[derive(Clone, Copy)]
pub(super) enum Fault {
    /// Serve every request in full.
    None,
    /// Once: announce the full range length, send half, then hang up -- a
    /// transport-level truncation the client sees as a body error.
    DropMidBodyOnce,
    /// Once: serve a *well-formed* response (consistent framing) carrying only the
    /// first half of the requested range -- no transport error.
    ShortBodyOnce,
    /// Always: serve a well-formed response with an empty body.
    EmptyBodyAlways,
    /// Always: answer any range starting past byte 0 with `416`, declaring a total of
    /// half the real body. Models the server's object being *shorter* than the length
    /// the format declared -- the only way a 416 can land inside a known total.
    UnsatisfiableInsideBody,
}

/// A running [`serve_ranges`] instance.
pub(super) struct RangeServer {
    /// The media URL to fetch.
    pub(super) url: String,
    /// Requests answered so far.
    pub(super) requests: Arc<AtomicUsize>,
    /// Set once a fault has actually been injected, so a test can assert it provoked
    /// the thing it claims to test rather than passing for an unrelated reason.
    pub(super) faulted: Arc<AtomicBool>,
}

/// A minimal HTTP/1.1 server serving `body` and honouring one `Range: bytes=a-b` per
/// connection, optionally injecting `fault`.
pub(super) async fn serve_ranges(body: Vec<u8>, fault: Fault) -> RangeServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let requests = Arc::new(AtomicUsize::new(0));
    let faulted = Arc::new(AtomicBool::new(false));
    let count_srv = requests.clone();
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

                // Range semantics, as a real server implements them: a start at or past
                // EOF is unsatisfiable, while an end merely *overhanging* EOF is
                // satisfiable and clamps. A client that does not know the length
                // discovers it by walking off the end into the former.
                let refuse = matches!(fault, Fault::UnsatisfiableInsideBody) && start > 0;
                if start >= body.len() || refuse {
                    // Only ever *set* `faulted`: a legitimate past-EOF 416 must not
                    // clear a fault already injected, since the same flag doubles as the
                    // once-only latch for the `...Once` faults.
                    if refuse {
                        faulted.store(true, Ordering::SeqCst);
                    }
                    // A 416 should report the server's own total. Under the fault that
                    // is deliberately half the body, so the client can tell that the
                    // server disagrees about the object rather than about one range.
                    let declared = if refuse { body.len() / 2 } else { body.len() };
                    let head = format!(
                        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{declared}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.flush().await;
                    return;
                }
                let end_incl = end_incl.min(body.len() - 1);
                let slice = &body[start..=end_incl];

                let inject = start > 0
                    && match fault {
                        Fault::None | Fault::UnsatisfiableInsideBody => false,
                        Fault::EmptyBodyAlways => {
                            faulted.store(true, Ordering::SeqCst);
                            true
                        }
                        Fault::DropMidBodyOnce | Fault::ShortBodyOnce => {
                            !faulted.swap(true, Ordering::SeqCst)
                        }
                    };
                // How much the header announces vs. how much is actually sent; a
                // mismatch (DropMidBodyOnce) is a transport error client-side.
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

    RangeServer {
        url,
        requests,
        faulted,
    }
}

/// A request for a `len`-byte object served in `chunk`-sized ranges.
pub(super) fn request(url: String, len: usize, chunk: u64) -> ChunkRequest {
    let mut req = ChunkRequest::new(
        reqwest::Client::new(),
        url,
        HeaderMap::new(),
        Some(len as u64),
    );
    req.set_chunk(chunk);
    req
}
