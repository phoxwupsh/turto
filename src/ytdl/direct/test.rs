use super::{Attempts, DirectFetch, MAX_REFRESHES, MAX_RETRIES, Recovery, same_format};
use crate::ytdl::cancel::Cancel;
use crate::ytdl::chunked::FetchError;
use crate::ytdl::tail;
use crate::ytdl::test_support::{Fault, request, serve_ranges};
use reqwest::StatusCode;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;

fn rejected() -> FetchError {
    FetchError::Rejected(StatusCode::FORBIDDEN)
}

fn transport() -> FetchError {
    FetchError::Transport(io::Error::other("connection reset"))
}

/// A dropped connection is worth another go at the same URL -- but only so many
/// times, or a permanently broken fetch would spin forever.
#[test]
fn transport_failures_retry_up_to_the_budget() {
    let mut attempts = Attempts::new(0);
    for _ in 0..MAX_RETRIES {
        assert_eq!(attempts.next(0, &transport()), Recovery::Retry);
    }
    assert_eq!(
        attempts.next(0, &transport()),
        Recovery::Fail,
        "a fetch stuck at one offset must give up"
    );
}

/// A rejection means the signed URL is dead, so the remedy is a fresh extract -- on
/// the *first* failure, not after burning the retry budget on a URL that cannot work.
#[test]
fn rejections_refresh_immediately() {
    let mut attempts = Attempts::new(0);
    assert_eq!(attempts.next(0, &rejected()), Recovery::Refresh);
}

/// Out of refreshes, a rejection is terminal. It must *not* fall through to the retry
/// budget: the URL cannot come back, so retrying only delays the inevitable.
#[test]
fn rejections_do_not_spend_the_retry_budget() {
    let mut attempts = Attempts::new(0);
    for _ in 0..MAX_REFRESHES {
        assert_eq!(attempts.next(0, &rejected()), Recovery::Refresh);
    }
    assert_eq!(attempts.next(0, &rejected()), Recovery::Fail);
    assert_eq!(
        attempts.retries, 0,
        "a rejection must never consume a retry"
    );
}

/// The one rule that makes the budgets safe for long tracks: getting further resets
/// everything. A two-hour track hits any number of isolated blips, and none of them
/// should count against a later, unrelated one.
#[test]
fn progress_resets_the_budget() {
    let mut attempts = Attempts::new(0);
    for _ in 0..MAX_RETRIES {
        assert_eq!(attempts.next(0, &transport()), Recovery::Retry);
    }
    // Exhausted at offset 0 -- but the next failure is further along.
    assert_eq!(
        attempts.next(4096, &transport()),
        Recovery::Retry,
        "progress must restore the retry budget"
    );
    assert_eq!(attempts.stuck_at, 4096);
    assert_eq!(attempts.retries, 1);
}

/// Refreshes reset with progress too, so a track that legitimately outlives several
/// signed URLs keeps playing.
#[test]
fn progress_resets_refreshes() {
    let mut attempts = Attempts::new(0);
    for _ in 0..MAX_REFRESHES {
        assert_eq!(attempts.next(0, &rejected()), Recovery::Refresh);
    }
    assert_eq!(attempts.next(0, &rejected()), Recovery::Fail);
    assert_eq!(
        attempts.next(1_000_000, &rejected()),
        Recovery::Refresh,
        "a later expiry is a fresh problem, not a continuation"
    );
}

/// Truncation is the one failure with no remedy, so it must fail at once *and* leave both
/// budgets untouched -- spending them only delays the restart that is the sole fix.
#[test]
fn truncation_fails_immediately_without_spending_a_budget() {
    let mut attempts = Attempts::new(4096);
    let truncated = FetchError::Truncated {
        at: 4096,
        total: 25_000,
        declared: Some(4096),
    };
    assert_eq!(attempts.next(4096, &truncated), Recovery::Fail);
    assert_eq!(attempts.retries, 0, "truncation must not consume a retry");
    assert_eq!(
        attempts.refreshes, 0,
        "truncation must not consume a refresh"
    );
}

/// The guard on resuming in place: appending a different encoding at the current offset
/// splices two formats into one silently-corrupt track. An unverifiable length must count
/// as a mismatch too -- the point is to resume only when a splice is ruled *out*.
#[test]
fn only_an_identical_declared_length_may_resume_in_place() {
    assert!(same_format(Some(9_000), Some(9_000)));
    assert!(
        !same_format(Some(9_000), Some(9_001)),
        "a different length is proof of a different encoding"
    );
    assert!(
        !same_format(None, None),
        "two unknown lengths are not a match; nothing has been ruled out"
    );
    assert!(!same_format(Some(9_000), None));
    assert!(!same_format(None, Some(9_000)));
}

/// No progress means *strictly* greater: re-failing at the same offset is a stall,
/// even after a successful recovery that fetched nothing.
#[test]
fn same_offset_is_not_progress() {
    let mut attempts = Attempts::new(100);
    assert_eq!(attempts.next(100, &transport()), Recovery::Retry);
    assert_eq!(attempts.retries, 1, "the streak must continue at 100");
    assert_eq!(attempts.next(99, &transport()), Recovery::Retry);
    assert_eq!(
        attempts.retries, 2,
        "an earlier offset is not progress either"
    );
}

/// End to end: a dropped connection mid-track must be recovered from, resuming at
/// exactly what is durable, so the track comes out byte-identical. A gap here would
/// read as a clean early EOF and cache a truncated track.
#[tokio::test]
async fn transport_failure_recovers_and_keeps_the_track_intact() {
    let len = 40_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body.clone(), Fault::DropMidBodyOnce).await;

    let cancel: Arc<Cancel> = Arc::default();
    let (fetch, reporter) = DirectFetch::open(
        "https://example.invalid/watch".to_owned(),
        request(srv.url, len, 10_000),
        cancel.clone(),
    )
    .await
    .expect("the eager first range must succeed");

    let handle = tail::spawn_tail(cancel, Some(reporter), move |tail| fetch.run(tail))
        .expect("spawn the producer");
    handle.warm().await.expect("the producer must recover");

    let mut out = Vec::new();
    handle
        .reader()
        .unwrap()
        .read_to_end(&mut out)
        .await
        .unwrap();
    assert_eq!(out, body, "the recovered track must be byte-identical");
}

/// The bound, end to end: an unattached prefetch stops at exactly one chunk, and the
/// playback arriving later *continues that same download* rather than starting its own.
/// Reading is what reopens the window.
#[tokio::test]
async fn attaching_a_reader_resumes_a_parked_prefetch() {
    let len = 400_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body.clone(), Fault::None).await;

    let cancel: Arc<Cancel> = Arc::default();
    let (fetch, reporter) = DirectFetch::open_with_window(
        "https://example.invalid/watch".to_owned(),
        request(srv.url, len, 10_000),
        cancel.clone(),
        10_000,
    )
    .await
    .expect("open");

    // The reporter goes to the tail, so every reader minted from it drives this gate.
    let handle = tail::spawn_tail(cancel, Some(reporter), move |tail| fetch.run(tail)).unwrap();

    // Nothing is reading yet. Wait for the eager chunk to land, then leave the fetcher
    // time to overrun if it is going to.
    for _ in 0..100 {
        if handle.written() >= 10_000 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        handle.written(),
        10_000,
        "an unattached prefetch must hold exactly one chunk"
    );

    // Attach: reading reports consumption, which opens the window and restarts the
    // fetch from where it parked.
    let mut out = Vec::new();
    handle
        .reader()
        .unwrap()
        .read_to_end(&mut out)
        .await
        .unwrap();
    assert_eq!(out, body, "the reader must resume the parked download to its end");
}

/// The paced fetcher must not race ahead of consumption: with nothing reading, it
/// parks after ~one window instead of pulling the whole track. And cancelling the
/// *track* is what stops it -- readers come and go over one download.
#[tokio::test]
async fn paced_fetcher_stays_bounded_and_stops_on_track_cancel() {
    let len = 400_000usize;
    let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let srv = serve_ranges(body, Fault::None).await;

    // One chunk (10 KB) of read-ahead, so pacing is observable on a small body.
    let cancel: Arc<Cancel> = Arc::default();
    let (fetch, reporter) = DirectFetch::open_with_window(
        "https://example.invalid/watch".to_owned(),
        request(srv.url, len, 10_000),
        cancel.clone(),
        10_000,
    )
    .await
    .expect("open");
    // Nothing ever reads, so `consumed` stays at 0 and the gate must hold.
    drop(reporter);

    let handle = tail::spawn_tail(cancel.clone(), None, move |tail| fetch.run(tail)).unwrap();

    tokio::time::sleep(Duration::from_millis(150)).await;
    let ahead = handle.written();
    assert!(ahead > 0, "the eager first chunk must have been drained");
    assert!(
        (ahead as usize) < len,
        "the fetcher must park at its gate, not pull the whole track; got {ahead}"
    );

    cancel.cancel();
    // Stop promptly, and do not leave behind a partial file that passes for a whole track.
    tokio::time::timeout(Duration::from_secs(2), handle.warm())
        .await
        .expect("a cancelled fetcher must end promptly")
        .expect_err("a cancelled tail is incomplete and must not read as complete");
    assert!(handle.failed(), "the owner must see the tail as unusable");
}
