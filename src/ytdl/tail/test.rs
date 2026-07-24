use crate::ytdl::chunked::{ChunkedHttpRequest, pace_channel};
use super::{TailReader, TailState, TailWriter, drain_response, drain_resuming};
use reqwest::header::HeaderMap;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// End-to-end for the direct-HTTP tail path: a range server that drops the
/// body mid-way on its first response (announcing the full length, so the
/// client sees a truncation error, not a clean EOF). `drain_resuming` must
/// resume from where it stopped and still write the whole file, which the
/// tailing [`TailReader`] then reads back intact.
#[tokio::test]
async fn http_tail_producer_resumes_mid_stream_drop() {
    let len = 40_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let dropped = Arc::new(AtomicBool::new(false));

    let srv_body = body.clone();
    let dropped_srv = dropped.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let body = srv_body.clone();
            let dropped = dropped_srv.clone();
            tokio::spawn(async move {
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
                let text = String::from_utf8_lossy(&req).to_ascii_lowercase();
                let spec = text
                    .lines()
                    .find_map(|l| l.trim().strip_prefix("range: bytes="))
                    .expect("client always sends a closed Range");
                let (a, b) = spec.split_once('-').unwrap();
                let start: usize = a.trim().parse().unwrap();
                let end_incl: usize = b.trim().parse().unwrap();
                let slice = &body[start..=end_incl];

                // Always announce the full range length.
                let head = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                    slice.len(),
                    start,
                    end_incl,
                    body.len(),
                );
                let _ = sock.write_all(head.as_bytes()).await;

                // First response: send half the body, then hang up.
                if !dropped.swap(true, Ordering::SeqCst) {
                    let _ = sock.write_all(&slice[..slice.len() / 2]).await;
                    let _ = sock.flush().await;
                    return;
                }
                let _ = sock.write_all(slice).await;
                let _ = sock.flush().await;
            });
        }
    });

    let req = ChunkedHttpRequest::new(
        reqwest::Client::new(),
        url,
        HeaderMap::new(),
        Some(len as u64),
    );
    let src = req.open_with_pace(None).await.expect("open source");

    let backing = NamedTempFile::new().unwrap();
    let read_handle = backing.reopen().unwrap();
    let write_handle = tokio::fs::File::from_std(backing.reopen().unwrap());
    let state = Arc::new(TailState::default());

    let tail = TailWriter::new(write_handle, state.clone());
    drain_resuming(src, tail)
        .await
        .expect("producer must finish cleanly after resuming");
    assert!(
        dropped.load(Ordering::SeqCst),
        "server must have injected a mid-stream drop"
    );
    // Finalize as `spawn_tail_input` would, so the tail reader sees a clean EOF
    // rather than parking forever at end-of-data.
    state.done.store(true, Ordering::Release);
    state.waker.wake();

    // Read back what the producer wrote via the tail reader.
    let mut reader = TailReader {
        file: read_handle,
        pos: 0,
        state,
        pace: None,
    };
    let mut out = Vec::new();
    reader.read_to_end(&mut out).await.unwrap();
    assert_eq!(
        out, body,
        "tailed bytes must equal the original despite the mid-stream drop"
    );
}

/// The paced live path must not race ahead of playback: with a consumer that
/// never reads, the fetcher parks after ~one window of read-ahead instead of
/// downloading the whole file, and dropping the reader (a skip) cancels it.
#[tokio::test]
async fn paced_producer_stays_bounded_and_cancels_on_reader_drop() {
    let len = 100_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();

    // A range server that always serves the requested range in full.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/media", listener.local_addr().unwrap());
    let srv_body = body.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let body = srv_body.clone();
            tokio::spawn(async move {
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
                let text = String::from_utf8_lossy(&req).to_ascii_lowercase();
                let spec = text
                    .lines()
                    .find_map(|l| l.trim().strip_prefix("range: bytes="))
                    .expect("client always sends a closed Range");
                let (a, b) = spec.split_once('-').unwrap();
                let start: usize = a.trim().parse().unwrap();
                let end_incl: usize = b.trim().parse().unwrap();
                let slice = &body[start..=end_incl];
                let head = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                    slice.len(),
                    start,
                    end_incl,
                    body.len(),
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(slice).await;
                let _ = sock.flush().await;
            });
        }
    });

    // 10 KB chunks with a one-chunk (10 KB) read-ahead window.
    let mut req = ChunkedHttpRequest::new(
        reqwest::Client::new(),
        url,
        HeaderMap::new(),
        Some(len as u64),
    );
    req.set_chunk(10_000);
    let (gate, reporter) = pace_channel(10_000);
    let src = req
        .open_with_pace(Some(gate))
        .await
        .expect("open source");

    let backing = NamedTempFile::new().unwrap();
    let read_handle = backing.reopen().unwrap();
    let write_handle = tokio::fs::File::from_std(backing.reopen().unwrap());
    let state = Arc::new(TailState::default());

    // Playback that never reads: `consumed` stays at 0, so the fetcher must
    // park at the pace gate after buffering ~one window ahead.
    let reader = TailReader {
        file: read_handle,
        pos: 0,
        state: state.clone(),
        pace: Some(reporter.clone()),
    };

    let tail = TailWriter::new(write_handle, state.clone());
    let producer = tokio::spawn(async move { drain_resuming(src, tail).await });

    // Let the fetcher run as far as the window allows, then confirm it parked.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !producer.is_finished(),
        "fetcher must park at the pace gate, not run to completion"
    );
    let ahead = state.written.load(Ordering::Acquire);
    assert!(ahead > 0, "the eager first chunk must have been drained");
    assert!(
        ahead <= 30_000,
        "fetcher must stay near the window ahead of a stalled reader, got {ahead}"
    );
    assert!(
        (ahead as usize) < len,
        "fetcher must not have downloaded the whole file"
    );

    // A skip: dropping the reader cancels the fetcher, which then returns.
    drop(reader);
    tokio::time::timeout(Duration::from_secs(2), producer)
        .await
        .expect("fetcher must return promptly after cancel")
        .unwrap()
        .expect("a cancelled fetcher ends its stream cleanly");
    assert!(
        reporter.is_cancelled(),
        "dropping the reader must cancel the pacer"
    );
    assert!(
        (state.written.load(Ordering::Acquire) as usize) < len,
        "cancel must stop the fetcher before the whole file is downloaded"
    );
}

/// The sidecar `/download` tail has no pacer, but a skip must still stop it: the
/// reader's drop sets the shared cancel flag and `drain_response` returns at the
/// next chunk boundary instead of draining a whole (possibly hour-long) response.
#[tokio::test]
async fn sidecar_producer_cancels_on_reader_drop() {
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
    let read_handle = backing.reopen().unwrap();
    let write_handle = tokio::fs::File::from_std(backing.reopen().unwrap());
    let state = Arc::new(TailState::default());

    // Sidecar path: no pacer. The reader carries only the shared cancel flag.
    let reader = TailReader {
        file: read_handle,
        pos: 0,
        state: state.clone(),
        pace: None,
    };

    let tail = TailWriter::new(write_handle, state.clone());
    let producer = tokio::spawn(async move { drain_response(resp, tail).await });

    // Let it drain a few pieces, then skip.
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !producer.is_finished(),
        "producer must still be draining the slow response"
    );
    drop(reader);

    tokio::time::timeout(Duration::from_secs(2), producer)
        .await
        .expect("a cancelled sidecar producer must return promptly")
        .unwrap()
        .expect("a cancelled sidecar producer ends cleanly");
    assert!(
        state.cancelled.load(Ordering::Acquire),
        "dropping the reader must set the shared cancel flag"
    );
    assert!(
        (state.written.load(Ordering::Acquire) as usize) < len,
        "cancel must stop the download before the whole body is drained"
    );
}
