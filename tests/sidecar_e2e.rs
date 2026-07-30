//! End-to-end smoke test for the warm yt-dlp sidecar and the Rust byte path.
//!
//! Ignored by default: it downloads the uv binary, a managed CPython, and
//! yt-dlp into a temp dir, then hits live YouTube. Run on demand with:
//!
//! ```sh
//! cargo test --test sidecar_e2e -- --ignored --nocapture
//! ```
//!
//! Requires network access and a local `bun/` (the repo's vendored bun, found
//! by `setup_bun` in the crate root) or `bun` on PATH.

use std::sync::Arc;
use turto::{
    deps::{bun::setup_bun, uv::setup_uv},
    models::config::YtdlpConfig,
    ytdl::{YouTubeDl, YouTubeDlMetadata, sidecar},
};

const SINGLE_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const PLAYLIST_URL: &str =
    "https://www.youtube.com/playlist?list=PLFgquLnL59alCl_2TQvOiD5Vgm1hCaGSI";

#[tokio::test]
#[ignore = "downloads uv + managed python + yt-dlp; needs network"]
async fn sidecar_end_to_end() {
    let config = Arc::new(YtdlpConfig::default());
    // Stable dir so the vendored uv binary + managed Python + venv persist
    // across reruns (avoids re-downloading and re-hitting the GitHub API).
    let uv_dir = std::env::temp_dir().join("turto-uv-e2e");

    // bun is the JS runtime the sidecar uses; setup_bun finds the repo's
    // vendored bun in ./bun (or downloads it).
    setup_bun(&config, "bun").await.expect("setup_bun");
    setup_uv(&config, &uv_dir).await.expect("setup_uv");

    // Isolation: every runtime dep must live under uv_dir (the bot's cwd in
    // production), not in the user's home / system dirs.
    assert!(uv_dir.join("venv").is_dir(), "venv under uv_dir");
    assert!(uv_dir.join("cache").is_dir(), "uv cache under uv_dir");
    assert!(
        std::fs::read_dir(uv_dir.join("python"))
            .map(|mut d| d.next().is_some())
            .unwrap_or(false),
        "managed CPython under uv_dir/python"
    );

    // Cookies load once at startup as base64 *content*; in
    // production sidecar::init does this. A minimal valid Netscape cookies.txt
    // makes the whole suite run "with cookies" (0 cookies == harmless for public
    // videos), and lets us assert at the end that the sidecar never wrote back to
    // OUR file: yt-dlp rewrites its cookiefile on close, so it must target a
    // private per-request copy, never the caller's.
    let cookie_path = std::env::temp_dir().join("turto-cookies-e2e.txt");
    let cookie_content: &[u8] = b"# Netscape HTTP Cookie File\n";
    std::fs::write(&cookie_path, cookie_content).expect("write cookies file");

    // init brings up the sidecar process and loads the cookies file in one step.
    sidecar::init(Some(cookie_path.to_str().expect("utf8 cookie path")))
        .await
        .expect("sidecar init");

    // Single video: must select an audio-only format with a usable URL.
    let info = sidecar::extract(SINGLE_URL, false)
        .await
        .expect("extract single");
    let meta: YouTubeDlMetadata =
        serde_json::from_value(info).expect("deserialize YouTubeDlMetadata");
    assert!(!meta.url.is_empty(), "expected a resolved media url");
    assert_eq!(meta.protocol.as_deref(), Some("https"));
    assert!(
        meta.http_headers
            .as_ref()
            .map(|h| h.contains_key("User-Agent"))
            .unwrap_or(false),
        "expected http_headers with a User-Agent"
    );

    // Flat playlist: must return entries.
    let pl = sidecar::extract(PLAYLIST_URL, true)
        .await
        .expect("extract playlist");
    let entries = pl
        .get("entries")
        .and_then(|e| e.as_array())
        .expect("playlist entries");
    assert!(!entries.is_empty(), "expected non-empty playlist entries");

    // play() must open the resolved http(s) stream *with the format's headers*:
    // googlevideo answers a header-less request with a 403, so a successful open here
    // is the guard on them being carried through.
    let ytdl = YouTubeDl::new(SINGLE_URL);
    let (played, input) = ytdl.play().await.expect("play opens byte stream");
    assert_eq!(played.protocol.as_deref(), Some("https"));
    assert!(!played.url.is_empty());
    drop(input);

    // warm() must fetch the whole track through the byte path into its tail file.
    // Not what the queue does (see prefetch() below), but it is the guarantee the
    // tail rests on: a completed tail is the replay cache, so playing it again mints
    // a reader over the same bytes rather than downloading them twice.
    let warmed = YouTubeDl::new(SINGLE_URL);
    warmed.warm().await.expect("warm fetches the whole track");
    let (_, input) = warmed
        .play()
        .await
        .expect("play attaches to the completed tail");
    drop(input);

    // What the queue actually does: prefetch() takes the extract plus a *bounded* head
    // start on the bytes, returning long before the track is local, and the playback
    // that follows must attach to that same parked download rather than starting its own.
    let primed = YouTubeDl::new(SINGLE_URL);
    primed.prefetch().await.expect("prefetch primes the track");
    let (_, input) = primed
        .play()
        .await
        .expect("play attaches to the primed tail");
    drop(input);

    // URL-expiry retry guard. A resolved googlevideo URL is time-/IP-
    // bound and can expire while a track waits deep in the queue. Simulate that
    // stale cache with `new_with`: the resolved metadata points at a dead local
    // endpoint (instant connection-refused) so the first byte-open fails, while
    // the webpage URL is real -- the guard must re-extract and recover. Two
    // fresh objects because a success keeps the tail (which would shortcut the
    // second call). warm() completing at all proves the retry's fresh URL fetched
    // the whole track; play() returning Ok proves it opened the stream (the first,
    // stale attempt can only reach Ok via the retry).
    let stale = YouTubeDl::new_with(SINGLE_URL, stale_media_metadata());
    stale
        .warm()
        .await
        .expect("retry guard recovers a prefetch from an expired url");

    let stale_play = YouTubeDl::new_with(SINGLE_URL, stale_media_metadata());
    let (_meta, input) = stale_play
        .play()
        .await
        .expect("retry guard recovers play() from an expired url");
    drop(input);

    // The sidecar /download endpoint downloads from an already-extracted
    // info dict (--load-info-json, no second extraction) and tail-streams media
    // bytes. Read a little and disconnect to also exercise mid-stream cleanup.
    let dl_info = sidecar::extract(SINGLE_URL, false)
        .await
        .expect("extract for /download");
    let mut resp = sidecar::download(&dl_info)
        .await
        .expect("sidecar /download opens");
    let mut total = 0usize;
    while let Some(chunk) = resp.chunk().await.expect("download chunk") {
        total += chunk.len();
        if total > 64 * 1024 {
            break;
        }
    }
    assert!(total > 0, "expected streamed download bytes");
    drop(resp);

    // Failure path: a download that errors must NOT look like a clean,
    // non-empty stream (otherwise Rust would cache a truncated/empty file).
    // An unusable info dict makes download_with_info_file fail, so the sidecar
    // generator raises and the chunked response aborts.
    let bad_info = serde_json::json!({ "id": "0000000000", "extractor": "generic" });
    if let Ok(mut bad) = sidecar::download(&bad_info).await {
        let mut errored = false;
        let mut bad_total = 0usize;
        loop {
            match bad.chunk().await {
                Ok(Some(chunk)) => bad_total += chunk.len(),
                Ok(None) => break,
                Err(_) => {
                    errored = true;
                    break;
                }
            }
        }
        assert!(
            errored || bad_total == 0,
            "failed download must not appear as a clean, non-empty stream"
        );
    }

    // Blue/green update. Upgrading to the nightly channel almost
    // always yields a newer version than the running stable build, forcing a
    // real recycle: spawn green -> health-check -> atomic swap -> drain the old
    // instance via /shutdown. Whether or not it recycles, extraction must keep
    // working afterward, proving the (possibly hot-swapped) handle still serves.
    let recycled = sidecar::update(true).await.expect("update runs");
    println!("blue/green update recycled = {recycled}");
    let after = sidecar::extract(SINGLE_URL, false)
        .await
        .expect("extract after update");
    let meta_after: YouTubeDlMetadata =
        serde_json::from_value(after).expect("deserialize after update");
    assert!(
        !meta_after.url.is_empty(),
        "sidecar still resolves media after a blue/green update"
    );

    // Cookies write-back isolation: after a whole suite of extractions/downloads
    // (each of which had yt-dlp rewrite its per-request cookiefile copy on close),
    // the caller's cookies file must be byte-for-byte untouched.
    assert_eq!(
        std::fs::read(&cookie_path).expect("reread cookies file"),
        cookie_content,
        "the sidecar must not write back to the caller's cookies file"
    );
    std::fs::remove_file(&cookie_path).ok();

    sidecar::shutdown().await;
    // uv_dir is intentionally kept as a reusable cache for fast reruns.
}

/// A resolved-format metadata whose media URL points at a dead loopback port,
/// standing in for an expired googlevideo URL: `classify_source` routes it to the
/// direct byte-range path, whose first request is refused instantly. All other
/// fields default to `None`.
fn stale_media_metadata() -> YouTubeDlMetadata {
    serde_json::from_value(serde_json::json!({
        "url": "http://127.0.0.1:1/videoplayback",
        "protocol": "http",
    }))
    .expect("build stale metadata")
}
