//! Lifecycle & client for the yt-dlp sidecar.
//!
//! The sidecar is a long-lived Python process (run on the uv-managed venv)
//! that imports yt-dlp once and serves over HTTP. This module spawns it,
//! learns its ephemeral port, health-checks it, and exposes a typed [`extract`]
//! client used by [`crate::ytdl`].

use crate::{
    deps::{bun::get_bun_arg, uv::get_uv_python},
    models::config::YtdlpConfig,
};
use arc_swap::ArcSwap;
use base64::{Engine, prelude::BASE64_STANDARD};
use std::{
    process::Stdio,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
};
use tracing::instrument;
use uuid::Uuid;

/// The embedded sidecar source.
const SIDECAR_PY: &[u8] = include_bytes!("sidecar.py");

/// How long to wait for the sidecar to announce its port and pass health check.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Per-request timeout for an extraction.
const EXTRACT_TIMEOUT: Duration = Duration::from_secs(60);

/// How long to let an old sidecar drain in-flight requests after a blue/green
/// swap before hard-killing it.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(300);

struct Sidecar {
    client: reqwest::Client,
    base: String,
    secret: String,
}

/// The live sidecar handle, hot-swapped on a blue/green update.
static CURRENT: OnceLock<ArcSwap<Sidecar>> = OnceLock::new();

/// The live sidecar's child process, replaced on swap.
static CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

/// Serializes [`update`] so two overlapping runs can't both spawn/swap.
static UPDATE_LOCK: Mutex<()> = Mutex::const_new(());

/// base64-encoded cookies.txt content, loaded once at startup.
/// [`None`] = no cookies configured.
static COOKIES_B64: OnceLock<Option<String>> = OnceLock::new();

/// Configured cap on the sidecar's simultaneous extractions and downloads, kept for
/// the respawn in [`update`] as well as the first spawn.
static MAX_CONCURRENCY: OnceLock<u32> = OnceLock::new();

/// Load the current sidecar handle as a full `Arc` (safe to hold across awaits,
/// unlike an `ArcSwap` guard).
fn current() -> Result<Arc<Sidecar>, SidecarError> {
    Ok(CURRENT.get().ok_or(SidecarError::NotStarted)?.load_full())
}

/// Error that may occur while spawning a sidecar process
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    /// The uv-managed interpreter could not be run.
    #[error("failed to launch the sidecar interpreter: {0}")]
    Launch(#[source] std::io::Error),

    /// Writing the embedded program to the child's stdin failed.
    #[error("failed to send the sidecar program over stdin: {0}")]
    WriteScript(#[source] std::io::Error),

    /// I/O error reading the child's stdout while waiting for its port line.
    #[error("failed to read the sidecar's port line: {0}")]
    ReadPort(#[source] std::io::Error),

    /// The child closed stdout before printing `PORT=<n>` -- it exited during
    /// startup (usually a failed `import yt_dlp`; see the captured stderr).
    #[error("sidecar exited before announcing its port")]
    NoPort,

    /// The first stdout line was not the expected `PORT=<n>`.
    #[error("expected a `PORT=<n>` line from the sidecar, got {line:?}")]
    BadPortLine { line: String },

    /// Building the loopback HTTP client failed.
    #[error("failed to build the sidecar http client: {0}")]
    Client(#[source] reqwest::Error),

    /// The child never announced its port / passed health check in the window.
    #[error("sidecar did not become ready within {}", .0.as_secs_f32())]
    Timeout(Duration),
}

impl SidecarError {
    /// Read whatever the dying child wrote to stderr (usually a Python traceback)
    /// and fold it into a [`SidecarError::Spawn`] beside the structured cause, so a
    /// spawn failure shows *why* the child died instead of a bare "exited".
    async fn spawn(source: SpawnError, stderr: tokio::process::ChildStderr) -> SidecarError {
        let mut buf = String::new();
        let read = async {
            let mut reader = BufReader::new(stderr);
            let _ = reader.read_to_string(&mut buf).await;
        };
        let _ = tokio::time::timeout(Duration::from_secs(2), read).await;
        let trimmed = buf.trim();
        SidecarError::Spawn {
            source,
            stderr: (!trimmed.is_empty()).then(|| trimmed.to_string()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("sidecar not started")]
    NotStarted,
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// A sidecar process failed to spawn. Carries the structured [`SpawnError`]
    /// cause plus any stderr the dying child emitted (usually a Python
    /// traceback) -- diagnostic text that has no more structured form.
    #[error("sidecar spawn failed: {source}{}", .stderr.as_deref().map(|s| format!("; sidecar stderr: {s}")).unwrap_or_default())]
    Spawn {
        source: SpawnError,
        stderr: Option<String>,
    },
    #[error("yt-dlp update failed: {0}")]
    Update(#[from] crate::deps::DepsError),
    #[error("sidecar /health response did not include a yt-dlp version")]
    MissingHealthVersion,
    /// A sidecar endpoint answered with a non-success status. Names the endpoint,
    /// since all three carry the same `{"error", "type"?}` body but fail for
    /// different reasons.
    #[error("sidecar {endpoint} failed ({status}{}): {error}", kind.as_deref().map(|k| format!(", {k}")).unwrap_or_default())]
    Endpoint {
        endpoint: &'static str,
        status: u16,
        error: String,
        kind: Option<String>,
    },
    #[error("failed to read cookies file {path}: {source}")]
    Cookies {
        path: String,
        source: std::io::Error,
    },
}

/// Bring the sidecar up for the whole process:
///
/// 1. Record the settings a later respawn needs too
/// 2. Load and validate the cookies file
/// 3. Spawn the sidecar process
/// 4. Learn the sidecar's port
/// 5. Wait until it's healthy.
pub async fn init(config: &YtdlpConfig) -> Result<(), SidecarError> {
    MAX_CONCURRENCY.set(config.max_concurrency).ok();
    init_cookies(config.cookies_path.as_deref()).await?;
    let (sidecar, child) = spawn_instance().await?;
    tracing::info!(base = %sidecar.base, "yt-dlp sidecar ready");
    CURRENT.set(ArcSwap::from_pointee(sidecar)).ok();
    CHILD.set(Mutex::new(Some(child))).ok();
    Ok(())
}

/// Spawn one sidecar process, learn its port, build its client, and block until
/// it passes a health check. Kills the process and errors on any failure.
async fn spawn_instance() -> Result<(Sidecar, Child), SidecarError> {
    let python = get_uv_python();
    let bun_arg = get_bun_arg();
    let secret = Uuid::new_v4().simple().to_string();
    let concurrency = MAX_CONCURRENCY
        .get()
        .expect("init must run before spawning a sidecar");

    // `-` makes Python read the program from stdin; config is passed as argv,
    let mut cmd = Command::new(python);
    cmd.arg("-")
        .arg("--secret")
        .arg(&secret)
        .arg("--bun")
        .arg(bun_arg)
        .arg("--max-concurrency")
        .arg(concurrency.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    // Detach from the terminal's foreground group so a console Ctrl+C
    // (SIGINT / CTRL_C_EVENT) isn't delivered to the sidecar. Rust is the sole
    // owner of its teardown via shutdown() + kill_on_drop.
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    #[cfg(windows)]
    {
        cmd.creation_flags(0x0000_0200);
    }

    let mut child = cmd.spawn().map_err(|err| SidecarError::Spawn {
        source: SpawnError::Launch(err),
        stderr: None,
    })?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    // Feed the embedded sidecar source over stdin
    // A write failure here means the interpreter died before reading the program,
    let mut stdin = child.stdin.take().expect("piped stdin");
    if let Err(err) = stdin.write_all(SIDECAR_PY).await {
        let _ = child.kill().await;
        return Err(SidecarError::spawn(SpawnError::WriteScript(err), stderr).await);
    }
    // `python -` reads the whole program until EOF before running it
    // EOF is delivered by closing the write fd, i.e. dropping `stdin`.
    drop(stdin);

    // One budget for the whole bring-up (port announcement + health check), so
    // startup is bounded by STARTUP_TIMEOUT rather than 2x it (the port read and
    // the health poll used to get a full timeout each).
    let deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;

    let port = match tokio::time::timeout_at(deadline, read_port(stdout)).await {
        Ok(Ok((port, reader))) => {
            drain_lines(reader, "sidecar stdout", false);
            port
        }
        Ok(Err(err)) => {
            let _ = child.kill().await;
            return Err(SidecarError::spawn(err, stderr).await);
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(SidecarError::spawn(SpawnError::Timeout(STARTUP_TIMEOUT), stderr).await);
        }
    };

    drain_lines(BufReader::new(stderr), "sidecar stderr", true);

    let base = format!("http://127.0.0.1:{}", port);
    // No global timeout: /download streams the whole track and may run for
    // minutes. Per-request timeouts are applied to /extract and /health.
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|err| SidecarError::Spawn {
            source: SpawnError::Client(err),
            stderr: None,
        })?;

    let sidecar = Sidecar {
        client,
        base,
        secret,
    };
    // stderr is already draining in a background task by now, so a health-check
    // timeout can't fold it in -- it stays a bare `Timeout`.
    if let Err(err) = wait_healthy(&sidecar, deadline).await {
        let _ = child.kill().await;
        return Err(SidecarError::Spawn {
            source: err,
            stderr: None,
        });
    }
    Ok((sidecar, child))
}

/// Run an extraction. `flat` requests a flat-playlist dump; otherwise a single
/// processed video info dict (with format selection applied). Returns the
/// yt-dlp info dict as JSON for the caller to deserialize.
#[instrument(skip_all, fields(url = %url, flat))]
pub async fn extract(url: &str, flat: bool) -> Result<serde_json::Value, SidecarError> {
    let sc = current()?;
    let body = serde_json::json!({
        "url": url,
        "flat_playlist": flat,
        "cookies_b64": cookies_b64(),
    });

    tracing::debug!("calling sidecar /extract");
    let started = std::time::Instant::now();
    let resp = sc
        .client
        .post(format!("{}/extract", sc.base))
        .header("X-Turto-Secret", &sc.secret)
        .timeout(EXTRACT_TIMEOUT)
        .json(&body)
        .send()
        .await?;

    // Timed after the body is read and parsed, not just after the headers: a
    // flat-playlist dump is multi-MB, and the download plus `serde_json` parse is
    // the non-trivial tail of the call.
    let status = resp.status();
    if status.is_success() {
        let info = resp.json::<serde_json::Value>().await?;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        tracing::info!(status = status.as_u16(), elapsed_ms, "extract succeeded");
        Ok(info)
    } else {
        let err = error_from_response("/extract", resp).await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        tracing::warn!(status = status.as_u16(), elapsed_ms, error = %err, "extract failed");
        Err(err)
    }
}

/// Stream a download from the sidecar's `/download` endpoint. Used as the
/// fallback for formats Rust cannot fetch directly (DASH segments, SABR /
/// no-url). `info` is the yt-dlp info dict already produced by `/extract`, so
/// the sidecar downloads via `--load-info-json` (no second extraction). Returns
/// the streaming [`reqwest::Response`]; the caller reads its body chunk by chunk.
#[instrument(skip_all)]
pub async fn download(info: &serde_json::Value) -> Result<reqwest::Response, SidecarError> {
    let sc = current()?;
    let body = serde_json::json!({ "info": info, "cookies_b64": cookies_b64() });

    tracing::debug!("calling sidecar /download");
    let resp = sc
        .client
        .post(format!("{}/download", sc.base))
        .header("X-Turto-Secret", &sc.secret)
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if status.is_success() {
        tracing::info!(status = status.as_u16(), content_length = ?resp.content_length(), "download started");
        Ok(resp)
    } else {
        let err = error_from_response("/download", resp).await;
        tracing::warn!(status = status.as_u16(), error = %err, "download failed");
        Err(err)
    }
}

/// Load, validate, and cache the cookies file for the whole process.
async fn init_cookies(cookies_path: Option<&str>) -> Result<(), SidecarError> {
    let encoded = match cookies_path {
        Some(path) => {
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|source| SidecarError::Cookies {
                    path: path.to_string(),
                    source,
                })?;
            tracing::info!(path = %path, bytes = bytes.len(), "loaded cookies file");
            Some(BASE64_STANDARD.encode(bytes))
        }
        None => None,
    };
    COOKIES_B64.set(encoded).ok();
    Ok(())
}

/// The cached base64 cookies (or [`None`] if unconfigured).
///
/// # Panics
///
/// Panics if [`init_cookies`] has not run, that is a wiring bug,
/// not a runtime state.
fn cookies_b64() -> Option<&'static str> {
    COOKIES_B64
        .get()
        .expect("init_cookies must run before serving requests")
        .as_deref()
}

/// Check for a newer yt-dlp and, if found, recycle the sidecar blue/green:
///
/// 1. Upgrade the venv on disk
/// 2. Spawn & health-check a fresh instance
/// 3. Atomically cut new requests over to it
/// 4. Drain the old one in the background.
///
/// Returns `Ok(true)` if a recycle happened, `Ok(false)` if already current.
pub async fn update(nightly: bool) -> Result<bool, SidecarError> {
    let _serialize = UPDATE_LOCK.lock().await;

    let running = health_version(current()?.as_ref()).await?;
    crate::deps::uv::upgrade_ytdlp(nightly).await?;
    let installed = crate::deps::uv::installed_ytdlp_version().await?;

    if same_version(&installed, &running) {
        tracing::info!(version = %installed, "yt-dlp already current; no recycle");
        return Ok(false);
    }
    tracing::info!(from = %running, to = %installed, "yt-dlp updated; recycling sidecar");

    // Green: spawn & health-check the new version BEFORE touching the live
    // handle. If it fails (e.g. a broken release), blue keeps serving.
    let (green, green_child) = spawn_instance().await?;

    // Atomic cutover: new requests hit green from here on.
    let old = CURRENT
        .get()
        .ok_or(SidecarError::NotStarted)?
        .swap(Arc::new(green));
    let old_child = CHILD
        .get()
        .ok_or(SidecarError::NotStarted)?
        .lock()
        .await
        .replace(green_child);

    if let Some(child) = old_child {
        tokio::spawn(drain_old(old, child));
    }
    Ok(true)
}

/// Whether two version strings name the same yt-dlp release
///
/// Comparing dot-separated components as numbers: yt-dlp's own `__version__`
/// keeps its release tag's zero padding (`2026.07.04`) where `uv pip show`
/// reports the PEP 440 normalization (`2026.7.4`). A version with a non-numeric
/// component is compared as text.
fn same_version(a: &str, b: &str) -> bool {
    fn numeric(v: &str) -> Result<Vec<u64>, std::num::ParseIntError> {
        v.split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
    }
    match (numeric(a), numeric(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Retire an old sidecar after a swap: ask it to drain gracefully via HTTP API,
/// wait for it to exit, and hard-kill it if it overruns [`DRAIN_TIMEOUT`].
///
/// Holds the old handle so its client stays alive for any in-flight requests
/// still draining.
async fn drain_old(old: Arc<Sidecar>, mut child: Child) {
    let signalled = old
        .client
        .post(format!("{}/shutdown", old.base))
        .header("X-Turto-Secret", &old.secret)
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    if let Err(err) = signalled {
        tracing::warn!(error = %err, "failed to signal old sidecar; will wait then kill");
    }

    match tokio::time::timeout(DRAIN_TIMEOUT, child.wait()).await {
        Ok(Ok(status)) => tracing::info!(?status, "old sidecar drained and exited"),
        Ok(Err(err)) => tracing::warn!(error = %err, "error awaiting old sidecar exit"),
        Err(_) => {
            tracing::warn!(timeout = ?DRAIN_TIMEOUT, "old sidecar drain timed out; killing");
            let _ = child.kill().await;
        }
    }
    // Keep the old handle alive until the process is fully retired.
    drop(old);
}

/// Ask a sidecar which yt-dlp version it currently has imported (from `/health`).
///
/// The status is checked before the body: an unhealthy sidecar makes [`update`] skip
/// that cycle's yt-dlp upgrade, so it must say *that* rather than report a decode
/// failure over an error page.
async fn health_version(sc: &Sidecar) -> Result<String, SidecarError> {
    #[derive(serde::Deserialize)]
    struct Health {
        yt_dlp: Option<String>,
    }
    let resp = sc
        .client
        .get(format!("{}/health", sc.base))
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(error_from_response("/health", resp).await);
    }
    resp.json::<Health>()
        .await?
        .yt_dlp
        .ok_or(SidecarError::MissingHealthVersion)
}

/// Build a [`SidecarError::Endpoint`] from a non-success response of `endpoint`, whose
/// body is the `{"error", "type"?}` JSON shape.
async fn error_from_response(endpoint: &'static str, resp: reqwest::Response) -> SidecarError {
    #[derive(serde::Deserialize, Default)]
    struct ErrorBody {
        error: Option<String>,
        #[serde(rename = "type")]
        kind: Option<String>,
    }
    let status = resp.status().as_u16();
    // A non-JSON or unexpected body falls back to defaults rather than masking
    // the original status.
    let body = resp.json::<ErrorBody>().await.unwrap_or_default();
    SidecarError::Endpoint {
        endpoint,
        status,
        error: body.error.unwrap_or_else(|| "unknown error".to_string()),
        kind: body.kind,
    }
}

/// Terminate the sidecar process. Safe to call when never started.
pub async fn shutdown() {
    if let Some(lock) = CHILD.get()
        && let Some(mut child) = lock.lock().await.take()
    {
        let _ = child.kill().await;
        tracing::info!("yt-dlp sidecar stopped");
    }
}

async fn read_port(
    stdout: tokio::process::ChildStdout,
) -> Result<(u16, BufReader<tokio::process::ChildStdout>), SpawnError> {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .await
        .map_err(SpawnError::ReadPort)?;
    if n == 0 {
        return Err(SpawnError::NoPort);
    }
    let port = line
        .trim()
        .strip_prefix("PORT=")
        .and_then(|p| p.parse::<u16>().ok())
        .ok_or_else(|| SpawnError::BadPortLine { line })?;
    Ok((port, reader))
}

/// Drain remaining lines from a child pipe into the tracing log.
fn drain_lines<R>(reader: BufReader<R>, what: &'static str, warn: bool)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                continue;
            }
            if warn {
                tracing::warn!(target: "ytdlp_sidecar", "{what}: {line}");
            } else {
                tracing::debug!(target: "ytdlp_sidecar", "{what}: {line}");
            }
        }
    });
}

async fn wait_healthy(sc: &Sidecar, deadline: tokio::time::Instant) -> Result<(), SpawnError> {
    let url = format!("{}/health", sc.base);
    let poll = async {
        loop {
            if let Ok(resp) = sc
                .client
                .get(&url)
                .timeout(Duration::from_secs(2))
                .send()
                .await
                && resp.status().is_success()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };
    // Shares the caller's startup deadline (port read + health check together),
    // so the two phases can't each consume a full STARTUP_TIMEOUT.
    tokio::time::timeout_at(deadline, poll)
        .await
        .map_err(|_| SpawnError::Timeout(STARTUP_TIMEOUT))
}

#[cfg(test)]
mod tests {
    use super::{SidecarError, init_cookies, same_version};

    /// A configured-but-unreadable cookies file is a misconfiguration: it must
    /// error rather than silently serving cookieless.
    /// Errors before `COOKIES_B64` is set, so it does not disturb the global
    /// for other tests.
    #[tokio::test]
    async fn init_cookies_errors_on_unreadable_path() {
        let err = init_cookies(Some("/nonexistent/turto-cookies-does-not-exist.txt"))
            .await
            .expect_err("a missing cookies file must be a hard error");
        assert!(matches!(err, SidecarError::Cookies { .. }));
    }

    /// The pair [`update`] actually compares: `/health` reports the padded release
    /// tag, `uv pip show` its PEP 440 normalization.
    #[test]
    fn a_padded_version_matches_its_normalized_form() {
        assert!(same_version("2026.07.04", "2026.7.4"));
        assert!(same_version("2026.07.04.232811", "2026.7.4.232811"));
    }

    #[test]
    fn different_releases_do_not_match() {
        assert!(!same_version("2026.07.04", "2026.07.05"));
        assert!(!same_version("2026.07.04", "2026.7.5"));
        // A nightly of the same day is a different build.
        assert!(!same_version("2026.07.04", "2026.7.4.232811"));
    }

    #[test]
    fn a_non_numeric_version_compares_as_text() {
        assert!(same_version("2026.07.04rc1", "2026.07.04rc1"));
        assert!(!same_version("2026.07.04rc1", "2026.7.4"));
    }
}
