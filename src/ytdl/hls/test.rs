use super::*;
use crate::ytdl::tail;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A running [`serve`] instance.
struct HlsServer {
    /// The playlist URL to open, which is always `/index.m3u8`.
    url: String,
    /// Every path requested so far, in order, so a test can assert both *what* was
    /// fetched and *how often*.
    hits: Arc<Mutex<Vec<String>>>,
}

impl HlsServer {
    /// Requests answered for `path`.
    fn hits(&self, path: &str) -> usize {
        self.hits
            .lock()
            .unwrap()
            .iter()
            .filter(|p| *p == path)
            .count()
    }

    fn request(&self) -> HlsRequest {
        HlsRequest::new(reqwest::Client::new(), self.url.clone(), HeaderMap::new())
    }

    /// Open a walk with a read-ahead window small enough to be observable on
    /// kilobyte-sized segments.
    async fn open(&self, cancel: Arc<Cancel>) -> Result<(HlsFetch, PaceReporter), HlsError> {
        self.open_tuned(cancel, Tuning::LIVE).await
    }

    /// [`Self::open`] against explicit tuning, with the small window kept.
    async fn open_tuned(
        &self,
        cancel: Arc<Cancel>,
        tuning: Tuning,
    ) -> Result<(HlsFetch, PaceReporter), HlsError> {
        let tuning = Tuning {
            read_ahead: 1024,
            ..tuning
        };
        HlsFetch::open_with(self.request(), cancel, tuning).await
    }
}

/// How a fixture answers a `Range` request.
#[derive(Clone, Copy)]
enum Serves {
    /// Correctly: `206`, exactly the bytes asked for.
    Ranged,
    /// Ignoring the header: `200`, and the whole resource.
    WholeResource,
    /// `206`, then more than was asked for.
    Overlong,
    /// `206`, then half of what was asked for.
    Short,
}

/// Serve `routes` over loopback. Each path maps to the bodies successive requests for
/// it receive, the last repeating forever -- which is how a playlist that grows between
/// polls is modelled. A `Range` header is honoured, so `#EXT-X-BYTERANGE` works.
async fn serve(routes: Vec<(&'static str, Vec<Vec<u8>>)>) -> HlsServer {
    serve_as(routes, Serves::Ranged).await
}

/// [`serve`], answering every ranged request `how`.
async fn serve_as(routes: Vec<(&'static str, Vec<Vec<u8>>)>, how: Serves) -> HlsServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/index.m3u8", listener.local_addr().unwrap());
    let routes: Arc<HashMap<String, Vec<Vec<u8>>>> = Arc::new(
        routes
            .into_iter()
            .map(|(path, bodies)| (path.to_owned(), bodies))
            .collect(),
    );
    let hits = Arc::new(Mutex::new(Vec::new()));
    let log = hits.clone();

    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let routes = routes.clone();
            let log = log.clone();
            tokio::spawn(async move {
                let Some(req) = read_request(&mut sock).await else {
                    return;
                };
                let path = req
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_owned();
                // Which body this path is due, counted before the hit is logged.
                let nth = {
                    let mut log = log.lock().unwrap();
                    let nth = log.iter().filter(|p| **p == path).count();
                    log.push(path.clone());
                    nth
                };
                let Some(bodies) = routes.get(&path) else {
                    let _ = sock
                        .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                        .await;
                    return;
                };
                let body = &bodies[nth.min(bodies.len() - 1)];

                let (status, slice) = match parse_range(&req) {
                    None => ("200 OK", &body[..]),
                    Some((start, _)) if start >= body.len() => {
                        ("416 Range Not Satisfiable", &body[..0])
                    }
                    Some((start, end)) => {
                        let len = end.min(body.len() - 1) - start + 1;
                        match how {
                            Serves::Ranged => ("206 Partial Content", &body[start..start + len]),
                            Serves::WholeResource => ("200 OK", &body[..]),
                            Serves::Overlong => ("206 Partial Content", &body[start..]),
                            Serves::Short => ("206 Partial Content", &body[start..start + len / 2]),
                        }
                    }
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    slice.len()
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(slice).await;
                let _ = sock.flush().await;
            });
        }
    });

    HlsServer { url, hits }
}

/// Read request headers up to the blank line.
async fn read_request(sock: &mut tokio::net::TcpStream) -> Option<String> {
    let mut req = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        match sock.read(&mut tmp).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => req.extend_from_slice(&tmp[..n]),
        }
        if req.windows(4).any(|w| w == b"\r\n\r\n") {
            return Some(String::from_utf8_lossy(&req).into_owned());
        }
    }
}

/// The inclusive bounds of a `Range: bytes=a-b` header, if the request carried one.
fn parse_range(req: &str) -> Option<(usize, usize)> {
    let spec = req
        .to_ascii_lowercase()
        .lines()
        .find_map(|line| line.trim().strip_prefix("range: bytes=").map(str::to_owned))?;
    let (start, end) = spec.split_once('-')?;
    Some((start.trim().parse().ok()?, end.trim().parse().ok()?))
}

/// `len` bytes distinguishable from every other segment's, so a reassembly assertion
/// catches an out-of-order or duplicated fetch rather than just a wrong length.
fn segment(seed: u8, len: usize) -> Vec<u8> {
    (0..len).map(|i| (i as u8).wrapping_add(seed)).collect()
}

/// Drain a walk to its end, returning the tail's bytes.
async fn walk(server: &HlsServer) -> std::io::Result<Vec<u8>> {
    let (handle, drained) = walk_handle(server).await;
    drained?;
    let mut out = Vec::new();
    handle.reader()?.read_to_end(&mut out).await?;
    Ok(out)
}

/// [`walk`] keeping the tail handle. A producer's own error never reaches a reader, so
/// what a *failed* walk left behind is all a test can inspect.
async fn walk_handle(server: &HlsServer) -> (tail::TailHandle, std::io::Result<()>) {
    let cancel: Arc<Cancel> = Arc::default();
    let (fetch, reporter) = server.open(cancel.clone()).await.expect("open the walk");
    let handle =
        tail::spawn_tail(cancel, Some(reporter), move |tail| fetch.run(tail)).expect("spawn");
    let drained = handle.warm().await;
    (handle, drained)
}

/// A finished playlist is walked in one pass: every segment in order, and no second
/// playlist fetch, since `#EXT-X-ENDLIST` says there is nothing left to learn.
#[tokio::test]
async fn a_finished_playlist_reassembles_in_one_pass() {
    let (a, b, c) = (segment(1, 3000), segment(2, 3000), segment(3, 1500));
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n",
        "#EXTINF:1.0,\nseg1.ts\n",
        "#EXTINF:1.0,\nseg2.ts\n",
        "#EXTINF:1.0,\nseg3.ts\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        ("/index.m3u8", vec![playlist.as_bytes().to_vec()]),
        ("/seg1.ts", vec![a.clone()]),
        ("/seg2.ts", vec![b.clone()]),
        ("/seg3.ts", vec![c.clone()]),
    ])
    .await;

    let out = walk(&server).await.expect("the walk must complete");

    assert_eq!(
        out,
        [a, b, c].concat(),
        "segments must land in playlist order"
    );
    assert_eq!(
        server.hits("/index.m3u8"),
        1,
        "an ended playlist must not be re-polled"
    );
}

/// A playlist still growing must be re-polled, and each segment fetched exactly once
/// even though a live playlist re-lists the segments it already published.
#[tokio::test]
async fn a_growing_playlist_fetches_each_segment_once_and_ends_on_the_end_list() {
    let (a, b, c) = (segment(1, 2000), segment(2, 2000), segment(3, 2000));
    let first = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n",
        "#EXTINF:1.0,\nseg1.ts\n",
        "#EXTINF:1.0,\nseg2.ts\n",
    );
    // The same two segments again, plus one more and the end marker.
    let grown = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n",
        "#EXTINF:1.0,\nseg1.ts\n",
        "#EXTINF:1.0,\nseg2.ts\n",
        "#EXTINF:1.0,\nseg3.ts\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        (
            "/index.m3u8",
            vec![first.as_bytes().to_vec(), grown.as_bytes().to_vec()],
        ),
        ("/seg1.ts", vec![a.clone()]),
        ("/seg2.ts", vec![b.clone()]),
        ("/seg3.ts", vec![c.clone()]),
    ])
    .await;

    let out = walk(&server).await.expect("the walk must complete");

    assert_eq!(
        out,
        [a, b, c].concat(),
        "the third segment must be appended"
    );
    assert_eq!(
        server.hits("/index.m3u8"),
        2,
        "the walk must have re-polled"
    );
    for seg in ["/seg1.ts", "/seg2.ts", "/seg3.ts"] {
        assert_eq!(server.hits(seg), 1, "{seg} must be fetched exactly once");
    }
}

/// A playlist that never publishes `#EXT-X-ENDLIST` is ended by the stale budget alone:
/// cleanly, since a live stream that stopped is over rather than broken, and no sooner
/// than the budget, since ending early publishes a truncated capture as a whole track.
///
/// `TARGETDURATION:0` also puts the poll floor under test, a zero target with no floor
/// spending the budget in hundreds of requests instead of a handful.
#[tokio::test]
async fn a_playlist_that_stops_growing_ends_the_stream_after_the_stale_budget() {
    const BUDGET: Duration = Duration::from_millis(400);
    const FLOOR: Duration = Duration::from_millis(50);

    let seg = segment(1, 2000);
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:0\n#EXT-X-MEDIA-SEQUENCE:0\n",
        "#EXTINF:1.0,\nseg1.ts\n",
    );
    let server = serve(vec![
        ("/index.m3u8", vec![playlist.as_bytes().to_vec()]),
        ("/seg1.ts", vec![seg.clone()]),
    ])
    .await;

    let cancel: Arc<Cancel> = Arc::default();
    let tuning = Tuning {
        stale_after: BUDGET,
        min_poll: FLOOR,
        ..Tuning::LIVE
    };
    let (fetch, reporter) = server
        .open_tuned(cancel.clone(), tuning)
        .await
        .expect("open the walk");
    let started = Instant::now();
    let handle = tail::spawn_tail(cancel, Some(reporter), move |tail| fetch.run(tail)).unwrap();

    tokio::time::timeout(Duration::from_secs(5), handle.warm())
        .await
        .expect("the walk must give up on a playlist that stopped growing")
        .expect("a stream that ended is a clean end, not a failure");
    let took = started.elapsed();

    assert!(
        !handle.failed(),
        "a stopped live stream is playable, not broken"
    );
    assert_eq!(handle.written(), seg.len() as u64);
    assert!(
        took >= BUDGET,
        "gave up after {took:?}, before the {BUDGET:?} budget was spent"
    );
    let polls = server.hits("/index.m3u8");
    assert!(
        polls > 1,
        "the walk must have kept polling, not given up at once"
    );
    assert!(
        polls <= (BUDGET.as_millis() / FLOOR.as_millis()) as usize + 3,
        "{polls} polls in {took:?} means the interval floor is not holding"
    );
}

/// The live window can slide past a walker that was parked or slow. The segments in
/// between are gone from the server, so the walk must resume at the playlist's new
/// start rather than stalling on a sequence number that will never come back.
#[tokio::test]
async fn a_jumped_media_sequence_resumes_at_the_new_window() {
    let (a, b) = (segment(1, 2000), segment(2, 2000));
    let first = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n",
        "#EXTINF:1.0,\nseg0.ts\n",
    );
    // Sequence 0 is gone; the window now starts at 5.
    let slid = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:5\n",
        "#EXTINF:1.0,\nseg5.ts\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        (
            "/index.m3u8",
            vec![first.as_bytes().to_vec(), slid.as_bytes().to_vec()],
        ),
        ("/seg0.ts", vec![a.clone()]),
        ("/seg5.ts", vec![b.clone()]),
    ])
    .await;

    let out = walk(&server).await.expect("the walk must complete");

    assert_eq!(
        out,
        [a, b].concat(),
        "the walk must pick up at the new window instead of stopping"
    );
    assert_eq!(server.hits("/seg5.ts"), 1);
}

/// `#EXT-X-BYTERANGE` segments are sub-ranges of one resource, so the walk must issue
/// range requests and reassemble them -- including the second tag, whose offset is
/// implicit and only resolvable from the one before it.
#[tokio::test]
async fn byte_range_segments_reassemble() {
    let whole = segment(7, 6000);
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n",
        "#EXT-X-BYTERANGE:2500@0\n#EXTINF:1.0,\nall.ts\n",
        // No `@offset`: this range starts where the previous one ended.
        "#EXT-X-BYTERANGE:3500\n#EXTINF:1.0,\nall.ts\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        ("/index.m3u8", vec![playlist.as_bytes().to_vec()]),
        ("/all.ts", vec![whole.clone(), whole.clone()]),
    ])
    .await;

    let out = walk(&server).await.expect("the walk must complete");

    assert_eq!(
        out, whole,
        "the two ranges must tile the resource exactly once"
    );
    assert_eq!(
        server.hits("/all.ts"),
        2,
        "each sub-range is its own request"
    );
}

/// Two sub-ranges tiling one 6000-byte resource, for the range checks below.
const TILED: &str = concat!(
    "#EXTM3U\n#EXT-X-TARGETDURATION:1\n",
    "#EXT-X-BYTERANGE:2500@0\n#EXTINF:1.0,\nall.ts\n",
    "#EXT-X-BYTERANGE:3400\n#EXTINF:1.0,\nall.ts\n",
    "#EXT-X-ENDLIST\n",
);

/// A server that ignores `Range` answers every sub-range with the whole resource, which
/// taken at face value appends that resource once per segment. The walk must fail
/// instead: a track that plays as gibberish is never retried, where a failed one is.
#[tokio::test]
async fn a_range_answered_without_206_fails_the_walk() {
    let whole = segment(7, 6000);
    let server = serve_as(
        vec![
            ("/index.m3u8", vec![TILED.as_bytes().to_vec()]),
            ("/all.ts", vec![whole]),
        ],
        Serves::WholeResource,
    )
    .await;

    let (handle, drained) = walk_handle(&server).await;

    drained.expect_err("a server that ignored the range must fail the walk");
    assert!(
        handle.failed(),
        "and the tail must not read as a whole track"
    );
    assert_eq!(
        handle.written(),
        0,
        "the status is checked before the body, so none of it reaches the tail"
    );
}

/// A `206` that over-runs its range is bounded by what was asked for rather than by what
/// arrives, so the reassembly still tiles the resource exactly.
#[tokio::test]
async fn an_overlong_range_is_truncated_to_what_was_asked_for() {
    let whole = segment(7, 6000);
    let server = serve_as(
        vec![
            ("/index.m3u8", vec![TILED.as_bytes().to_vec()]),
            ("/all.ts", vec![whole.clone(), whole.clone()]),
        ],
        Serves::Overlong,
    )
    .await;

    let out = walk(&server).await.expect("an over-run is recoverable");

    assert_eq!(
        out,
        whole[..5900],
        "each range must contribute exactly its own bytes"
    );
}

/// A `206` that stops short leaves a hole, putting every later segment at the wrong
/// offset. Nothing downstream can detect that, so the walk has to.
#[tokio::test]
async fn a_short_range_fails_the_walk() {
    let whole = segment(7, 6000);
    let server = serve_as(
        vec![
            ("/index.m3u8", vec![TILED.as_bytes().to_vec()]),
            ("/all.ts", vec![whole]),
        ],
        Serves::Short,
    )
    .await;

    let (handle, drained) = walk_handle(&server).await;

    drained.expect_err("a short range must fail the walk");
    assert!(
        handle.failed(),
        "and the tail must not read as a whole track"
    );
    assert_eq!(
        server.hits("/all.ts"),
        1,
        "the walk must stop, not carry on placing segments at the wrong offset"
    );
}

/// An fMP4 playlist's segments cannot be parsed without the `#EXT-X-MAP` header, so it
/// has to be written first -- and only once, however many segments name it.
#[tokio::test]
async fn the_initialization_section_is_written_once_before_the_segments() {
    let (init, a, b) = (segment(9, 500), segment(1, 2000), segment(2, 2000));
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-VERSION:6\n",
        "#EXT-X-MAP:URI=\"init.mp4\"\n",
        "#EXTINF:1.0,\nseg1.m4s\n",
        "#EXTINF:1.0,\nseg2.m4s\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        ("/index.m3u8", vec![playlist.as_bytes().to_vec()]),
        ("/init.mp4", vec![init.clone()]),
        ("/seg1.m4s", vec![a.clone()]),
        ("/seg2.m4s", vec![b.clone()]),
    ])
    .await;

    let out = walk(&server).await.expect("the walk must complete");

    assert_eq!(
        out,
        [init, a, b].concat(),
        "the init section must precede the first segment"
    );
    assert_eq!(
        server.hits("/init.mp4"),
        1,
        "one init section, however many segments carry the tag"
    );
}

/// A master playlist names variants rather than segments, so one has to be followed
/// before there is anything to walk.
#[tokio::test]
async fn a_master_playlist_is_followed_to_a_variant() {
    let audio = segment(4, 2500);
    let master = concat!(
        "#EXTM3U\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360,CODECS=\"avc1.4d401e,mp4a.40.2\"\nvideo.m3u8\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=128000,CODECS=\"mp4a.40.2\"\naudio.m3u8\n",
    );
    let variant = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n",
        "#EXTINF:1.0,\naudio1.ts\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        ("/index.m3u8", vec![master.as_bytes().to_vec()]),
        ("/audio.m3u8", vec![variant.as_bytes().to_vec()]),
        ("/audio1.ts", vec![audio.clone()]),
    ])
    .await;

    let out = walk(&server).await.expect("the walk must complete");

    assert_eq!(out, audio, "the audio-only variant must have been walked");
    assert_eq!(
        server.hits("/video.m3u8"),
        0,
        "the video rendition must not be touched"
    );
}

/// Nothing is reading, so the walk must stop after one segment instead of pulling a
/// whole stream that may never be played.
#[tokio::test]
async fn an_unattached_walk_parks_after_one_segment() {
    let seg = segment(1, 2000);
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n",
        "#EXTINF:1.0,\nseg1.ts\n",
        "#EXTINF:1.0,\nseg2.ts\n",
        "#EXTINF:1.0,\nseg3.ts\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        ("/index.m3u8", vec![playlist.as_bytes().to_vec()]),
        ("/seg1.ts", vec![seg.clone()]),
        ("/seg2.ts", vec![segment(2, 2000)]),
        ("/seg3.ts", vec![segment(3, 2000)]),
    ])
    .await;

    let cancel: Arc<Cancel> = Arc::default();
    let (fetch, reporter) = server.open(cancel.clone()).await.expect("open the walk");
    // Nothing ever reports consumption, so the gate must hold at its first boundary.
    drop(reporter);
    let handle = tail::spawn_tail(cancel, None, move |tail| fetch.run(tail)).unwrap();

    for _ in 0..100 {
        if handle.written() >= seg.len() as u64 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // Long enough for a second segment to land if the gate were not holding.
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(
        handle.written(),
        seg.len() as u64,
        "an unattached walk must hold exactly one segment"
    );
    assert_eq!(server.hits("/seg2.ts"), 0, "and fetch no further segment");
}

/// Dropping the track must stop the walk promptly, and what it wrote must not pass for
/// a whole track: a partial capture is not something to play or cache.
#[tokio::test]
async fn cancel_stops_the_walk_and_the_tail_reads_as_failed() {
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n",
        "#EXTINF:1.0,\nseg1.ts\n",
        "#EXTINF:1.0,\nseg2.ts\n",
        "#EXT-X-ENDLIST\n",
    );
    let server = serve(vec![
        ("/index.m3u8", vec![playlist.as_bytes().to_vec()]),
        ("/seg1.ts", vec![segment(1, 2000)]),
        ("/seg2.ts", vec![segment(2, 2000)]),
    ])
    .await;

    let cancel: Arc<Cancel> = Arc::default();
    let (fetch, reporter) = server.open(cancel.clone()).await.expect("open the walk");
    drop(reporter);
    let handle = tail::spawn_tail(cancel.clone(), None, move |tail| fetch.run(tail)).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    cancel.cancel();

    tokio::time::timeout(Duration::from_secs(2), handle.warm())
        .await
        .expect("a cancelled walk must end promptly")
        .expect_err("a cancelled tail is incomplete and must not read as complete");
    assert!(handle.failed(), "the owner must see the tail as unusable");
}

/// A dead playlist URL must fail at open, where a command can still report it, rather
/// than spawning a producer that immediately fails the tail.
#[tokio::test]
async fn a_missing_playlist_fails_at_open() {
    let server = serve(vec![("/elsewhere.m3u8", vec![b"#EXTM3U\n".to_vec()])]).await;

    let err = server
        .open(Arc::default())
        .await
        .err()
        .expect("a 404 playlist must not open");

    assert!(matches!(err, HlsError::Fetch(_)), "got {err:?}");
}

/// Encrypted segments must be refused rather than written to the tail, where they would
/// reach the decoder as garbage. `METHOD=NONE` is not encryption, though -- it is how a
/// playlist turns encryption back *off* -- so it must still play.
#[test]
fn only_a_real_key_counts_as_encrypted() {
    let base = Url::parse("https://example.invalid/hls/index.m3u8").unwrap();
    let with_key = |key: &str| {
        format!("#EXTM3U\n#EXT-X-TARGETDURATION:1\n{key}\n#EXTINF:1.0,\nseg1.ts\n#EXT-X-ENDLIST\n")
    };

    let err = parse(
        &base,
        &with_key("#EXT-X-KEY:METHOD=AES-128,URI=\"k.key\""),
        0,
    )
    .err()
    .expect("an encrypted playlist must be refused");
    assert!(matches!(err, HlsError::Encrypted(_)), "got {err:?}");

    let snapshot = parse(&base, &with_key("#EXT-X-KEY:METHOD=NONE"), 0)
        .expect("METHOD=NONE is not encryption");
    assert_eq!(snapshot.fresh.len(), 1);
}

/// Relative segment URIs are resolved against the playlist, not concatenated onto it:
/// an absolute-path URI replaces the playlist's path, and `..` climbs out of it.
#[test]
fn segment_uris_resolve_against_the_playlist() {
    let base = Url::parse("https://cdn.invalid/hls/720p/index.m3u8").unwrap();
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:97\n",
        "#EXTINF:1.0,\nseg97.ts\n",
        "#EXTINF:1.0,\n../shared/seg98.ts\n",
        "#EXTINF:1.0,\n/root/seg99.ts\n",
        "#EXTINF:1.0,\nhttps://other.invalid/seg100.ts\n",
    );

    let snapshot = parse(&base, playlist, 0).expect("parse");

    let urls: Vec<_> = snapshot
        .fresh
        .iter()
        .map(|segment| (segment.seq, segment.url.as_str().to_owned()))
        .collect();
    assert_eq!(
        urls,
        vec![
            (97, "https://cdn.invalid/hls/720p/seg97.ts".to_owned()),
            (98, "https://cdn.invalid/hls/shared/seg98.ts".to_owned()),
            (99, "https://cdn.invalid/root/seg99.ts".to_owned()),
            (100, "https://other.invalid/seg100.ts".to_owned()),
        ],
        "sequence numbers must be absolute and URIs resolved against the playlist"
    );
    assert_eq!(
        snapshot.first_seq, 97,
        "the window's start is what a later poll is compared against"
    );
    assert!(!snapshot.ended, "no end list means keep polling");
}

/// Already-fetched segments must be dropped by sequence number, so a re-poll of a
/// playlist that has not changed yields nothing to do.
#[test]
fn a_repoll_yields_only_what_is_new() {
    let base = Url::parse("https://cdn.invalid/hls/index.m3u8").unwrap();
    let playlist = concat!(
        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:10\n",
        "#EXTINF:1.0,\nseg10.ts\n",
        "#EXTINF:1.0,\nseg11.ts\n",
    );

    assert_eq!(parse(&base, playlist, 0).unwrap().fresh.len(), 2);
    assert_eq!(parse(&base, playlist, 11).unwrap().fresh.len(), 1);
    assert!(
        parse(&base, playlist, 12).unwrap().fresh.is_empty(),
        "an unchanged playlist must be a stale poll, not a refetch"
    );
}

/// The rendition to follow is the cheapest one, and a rendition declaring no
/// `RESOLUTION` beats a cheaper one that carries video: the bot decodes the audio and
/// discards the rest.
#[test]
fn the_variant_without_video_wins_and_price_breaks_the_tie() {
    let pick = |master: &str| {
        pick_variant(&MasterPlaylist::try_from(master).expect("parse the master"))
            .map(str::to_owned)
    };

    assert_eq!(
        pick(concat!(
            "#EXTM3U\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=426x240\nlow.m3u8\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=900000,CODECS=\"mp4a.40.2\"\naudio.m3u8\n",
        )),
        Some("audio.m3u8".to_owned()),
        "an audio-only rendition wins even when it is the more expensive one"
    );
    assert_eq!(
        pick(concat!(
            "#EXTM3U\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1920x1080\nhigh.m3u8\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=426x240\nlow.m3u8\n",
        )),
        Some("low.m3u8".to_owned()),
        "with only mixed renditions, the extra bandwidth would all be video"
    );
}

/// A master that keeps its audio in its own playlist gives *every* variant a
/// `RESOLUTION`, so the preference for a rendition without video finds nothing to prefer
/// and the cheapest variant is video-only. The associated `TYPE=AUDIO` rendition is what
/// gets followed instead, `DEFAULT=YES` first where there are several.
#[test]
fn a_demuxed_master_is_followed_to_its_audio_rendition() {
    let pick = |master: &str| {
        pick_variant(&MasterPlaylist::try_from(master).expect("parse the master"))
            .map(str::to_owned)
    };

    assert_eq!(
        pick(concat!(
            "#EXTM3U\n",
            "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"French\",URI=\"fr/audio.m3u8\"\n",
            "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"English\",DEFAULT=YES,\
             URI=\"en/audio.m3u8\"\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=426x240,AUDIO=\"aud\"\nlow.m3u8\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1920x1080,AUDIO=\"aud\"\nhigh.m3u8\n",
        )),
        Some("en/audio.m3u8".to_owned()),
        "the default audio rendition must be walked, not the cheapest video one"
    );
    assert_eq!(
        pick(concat!(
            "#EXTM3U\n",
            // No `URI`: this describes audio already muxed into the variants.
            "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"English\",DEFAULT=YES\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=426x240,AUDIO=\"aud\"\nlow.m3u8\n",
        )),
        Some("low.m3u8".to_owned()),
        "a rendition with nothing to follow must leave the variant alone"
    );
    assert_eq!(
        pick(concat!(
            "#EXTM3U\n",
            // Two groups, one per ladder rung. Only the picked variant's own is ours.
            "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"hi\",NAME=\"High\",URI=\"hi/audio.m3u8\"\n",
            "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"lo\",NAME=\"Low\",URI=\"lo/audio.m3u8\"\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=426x240,AUDIO=\"lo\"\nlow.m3u8\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1920x1080,AUDIO=\"hi\"\nhigh.m3u8\n",
        )),
        Some("lo/audio.m3u8".to_owned()),
        "only the variant's own audio group may be followed"
    );
}

/// A playlist may point its segments at another host, which has no business holding a
/// `Cookie` minted for the first one. Everything else must still travel: a multi-host CDN
/// can turn a missing `Referer` into a 403.
#[test]
fn credentials_do_not_follow_a_segment_off_its_host() {
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, "session=secret".parse().unwrap());
    headers.insert(AUTHORIZATION, "Bearer token".parse().unwrap());
    headers.insert(
        reqwest::header::REFERER,
        "https://cdn.invalid/".parse().unwrap(),
    );
    let sent = |url: &str| {
        strip_foreign_credentials(&headers, Some("cdn.invalid"), &Url::parse(url).unwrap())
    };

    let same = sent("https://cdn.invalid/hls/seg1.ts");
    assert_eq!(same.len(), 3, "the playlist's own host gets the whole set");

    let other = sent("https://edge.elsewhere.invalid/hls/seg1.ts");
    assert!(
        !other.contains_key(COOKIE) && !other.contains_key(AUTHORIZATION),
        "another host must not be handed our credentials"
    );
    assert!(
        other.contains_key(reqwest::header::REFERER),
        "the headers a CDN needs to serve us at all must still travel"
    );
}

/// The walk against a real CDN, which no loopback fixture stands in for: a master
/// playlist as its author wrote it, 180 segments from a real edge, and relative URIs
/// resolved against a path we did not choose.
///
/// The reference is the same stream fetched the dumbest possible way -- the audio
/// rendition's playlist by hand, every `.aac` line in order -- so the assertion assumes
/// nothing about the container. It also pins master resolution: these bytes only appear
/// if the walk followed the audio-only rendition rather than one of the four video ones.
///
/// Ignored by default -- it needs the network, and it moves ~3 MB.
#[tokio::test]
#[ignore = "walks a real HLS stream; needs network"]
async fn a_real_stream_matches_a_naive_sequential_fetch() {
    const MASTER: &str = "https://devstreaming-cdn.apple.com/videos/streaming/examples/\
                          bipbop_4x3/bipbop_4x3_variant.m3u8";
    const AUDIO: &str = "https://devstreaming-cdn.apple.com/videos/streaming/examples/\
                         bipbop_4x3/gear0/prog_index.m3u8";

    let client = reqwest::Client::new();
    let req = HlsRequest::new(client.clone(), MASTER.to_owned(), HeaderMap::new());
    let cancel: Arc<Cancel> = Arc::default();
    let (fetch, reporter) = HlsFetch::open(req, cancel.clone()).await.expect("open");
    let handle = tail::spawn_tail(cancel, Some(reporter), move |tail| fetch.run(tail)).unwrap();
    tokio::time::timeout(Duration::from_secs(300), handle.warm())
        .await
        .expect("the walk must finish")
        .expect("the walk must succeed");
    let mut walked = Vec::new();
    handle
        .reader()
        .unwrap()
        .read_to_end(&mut walked)
        .await
        .unwrap();

    let base = Url::parse(AUDIO).unwrap();
    let listing = client
        .get(AUDIO)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let mut reference = Vec::new();
    let mut segments = 0;
    for line in listing.lines().filter(|line| line.ends_with(".aac")) {
        let url = base.join(line).unwrap();
        let bytes = client.get(url).send().await.unwrap().bytes().await.unwrap();
        reference.extend_from_slice(&bytes);
        segments += 1;
    }

    println!("segments={segments} walked={} ", walked.len());
    assert!(segments > 100, "the fixture stream should be ~180 segments");
    assert_eq!(
        walked.len(),
        reference.len(),
        "the walk must produce every segment's bytes and no others"
    );
    assert!(walked == reference, "the walk must reassemble in order");
}
