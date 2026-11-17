//! The yt-dlp cookie jar: parse the operator's Netscape cookies file, merge the
//! refreshes yt-dlp makes during extraction, and persist them back to disk.
//!
//! Cookies are credentials. No type here that holds cookie data derives
//! `Debug`; the manual impls print counts only. Logs carry counts, sizes, paths
//! and io errors, never cookie names, values, lines or base64.
//!
//! The flow: every `/extract` request carries the base64 of a [`Snapshot`]; the
//! sidecar answers with the jar yt-dlp wrote back on close, and
//! [`CookieStore::submit`] hands the pair to a single updater task, which
//! diffs it against the request's own base (so out-of-order completions merge
//! instead of overwriting each other), filters it (domain allowlist, size caps),
//! applies it to the current jar and schedules a debounced save.

use arc_swap::ArcSwap;
use base64::{Engine, prelude::BASE64_STANDARD};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    fs::{Metadata, OpenOptions},
    future::Future,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tempfile::NamedTempFile;
use tokio::sync::{mpsc, oneshot};

/// Prefix browsers' exporters put on http-only cookie lines. Accepted and
/// stripped on parse, never written back: yt-dlp's loader strips it and its
/// saver never writes it, so keeping it would make every http-only cookie look
/// changed in yt-dlp's answer (see [`parse`]).
const HTTPONLY_PREFIX: &str = "#HttpOnly_";

/// What [`Jar::serialize`] writes first: the magic first line yt-dlp's loader
/// requires, then a comment telling the operator turto owns the file while it
/// runs. The parser skips comments, so files with only the magic line (or any
/// other comments) still load.
const MANAGED_HEADER: &str = "# Netscape HTTP Cookie File\n\
    # This file is managed by turto. Stop turto before editing or replacing it.\n";

/// An upsert whose serialized line (without the LF) is longer than this is dropped.
pub(crate) const MAX_COOKIE_LINE: usize = 4 * 1024;

/// A delta may not grow the jar past this many cookies (or its current count, if larger).
pub(crate) const MAX_COOKIES: usize = 1000;

/// A delta may not grow the serialized jar past this many bytes (or its current
/// size, if larger).
pub(crate) const MAX_JAR_BYTES: usize = 512 * 1024;

// ---------------------------------------------------------------------------
// Jar
// ---------------------------------------------------------------------------

/// A cookie's identity: lowercased domain (without `#HttpOnly_`, leading dot
/// kept), path and name. Holds a cookie name, so no `Debug`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    domain: String,
    path: String,
    name: String,
}

/// Everything about a cookie that is not its [`Key`]. Holds the value, so no `Debug`.
#[derive(Clone, PartialEq, Eq)]
struct Cookie {
    /// Always `key.domain.starts_with('.')`, as yt-dlp writes it.
    include_subdomains: bool,
    secure: bool,
    /// One or more ASCII digits, as yt-dlp writes it back: see
    /// [`canonical_expiry`].
    expiry: String,
    value: String,
}

/// A parsed Netscape cookie jar.
#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct Jar {
    cookies: BTreeMap<Key, Cookie>,
}

impl fmt::Debug for Jar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Jar")
            .field("cookies", &self.cookies.len())
            .finish()
    }
}

/// What changed between two jars: cookies added or changed, and cookies gone.
#[derive(Default)]
pub(crate) struct Delta {
    upserts: BTreeMap<Key, Cookie>,
    removals: BTreeSet<Key>,
}

impl fmt::Debug for Delta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Delta")
            .field("upserts", &self.upserts.len())
            .field("removals", &self.removals.len())
            .finish()
    }
}

impl Delta {
    pub(crate) fn is_empty(&self) -> bool {
        self.upserts.is_empty() && self.removals.is_empty()
    }
}

enum Line {
    /// Blank line or comment.
    Ignored,
    Cookie(Key, Cookie),
    Malformed,
}

/// Parse a Netscape cookies file strictly, the way yt-dlp's loader reads it.
///
/// Splits on `\n` and strips only one trailing `\r`; never trims leading
/// whitespace, so a line starting with a space or tab (a folded header, say) is
/// malformed rather than a cookie. Returns the jar and how many lines were
/// skipped as malformed or non-UTF-8.
///
/// Every jar holds the form yt-dlp's load/save round trip produces: no
/// `#HttpOnly_` marker, the expiry as Python's `int(float(x))` (`0` for an
/// empty one), and the subdomain flag derived from the domain's leading dot. Otherwise an unchanged cookie in yt-dlp's
/// answer would differ from the base, become an upsert of the base value, and
/// could revert a newer rotation merged from another request.
pub(crate) fn parse(input: &[u8]) -> (Jar, usize) {
    let mut jar = Jar::default();
    let mut skipped = 0;
    for raw in input.split(|&b| b == b'\n') {
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        if raw.is_empty() {
            continue;
        }
        let Ok(line) = std::str::from_utf8(raw) else {
            skipped += 1;
            continue;
        };
        match parse_line(line) {
            Line::Ignored => {}
            Line::Cookie(key, cookie) => {
                jar.cookies.insert(key, cookie);
            }
            Line::Malformed => skipped += 1,
        }
    }
    (jar, skipped)
}

fn parse_line(line: &str) -> Line {
    let rest = match line.strip_prefix(HTTPONLY_PREFIX) {
        Some(rest) => rest,
        None if line.starts_with('#') => return Line::Ignored,
        None => line,
    };
    let fields: Vec<&str> = rest.split('\t').collect();
    let [
        domain,
        include_subdomains,
        path,
        secure,
        expiry,
        name,
        value,
    ] = fields[..]
    else {
        return Line::Malformed;
    };
    if !valid_domain(domain) {
        return Line::Malformed;
    }
    // The subdomain flag must be well-formed, but its value is re-derived below.
    let (Some(_), Some(secure)) = (flag(include_subdomains), flag(secure)) else {
        return Line::Malformed;
    };
    let Some(expiry) = canonical_expiry(expiry) else {
        return Line::Malformed;
    };
    if [path, name, value]
        .iter()
        .any(|field| field.chars().any(char::is_control))
    {
        return Line::Malformed;
    }
    Line::Cookie(
        Key {
            domain: domain.to_ascii_lowercase(),
            path: path.to_owned(),
            name: name.to_owned(),
        },
        Cookie {
            include_subdomains: domain.starts_with('.'),
            secure,
            expiry,
            value: value.to_owned(),
        },
    )
}

/// An expiry field in the form yt-dlp's load -> save round trip writes it, or
/// [`None`] if yt-dlp's loader would not accept it.
///
/// yt-dlp accepts an empty field or `[0-9]+(\.[0-9]+)?` (ASCII digits only), and
/// Python's `Cookie` stores it as `int(float(x))`, so it saves `0` for an empty
/// field, drops leading zeros and any fraction, and rounds long digit strings
/// through an `f64` (`99999999999999999999` comes back as
/// `100000000000000000000`). Rust's `f64` parsing is correctly rounded like
/// Python's, and `{:.0}` of an integral `f64` prints it exactly, as `int()`
/// does. A value that overflows to infinity is rejected; Python would raise.
fn canonical_expiry(field: &str) -> Option<String> {
    if field.is_empty() {
        return Some("0".to_owned());
    }
    let (int, frac) = match field.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (field, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(int) || !frac.is_none_or(digits) {
        return None;
    }
    let value: f64 = field.parse().ok()?;
    value.is_finite().then(|| format!("{:.0}", value.trunc()))
}

/// `^\.?[A-Za-z0-9.-]+$`, except that a lone `.` is rejected too.
fn valid_domain(domain: &str) -> bool {
    let rest = domain.strip_prefix('.').unwrap_or(domain);
    !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
}

fn flag(field: &str) -> Option<bool> {
    match field {
        "TRUE" => Some(true),
        "FALSE" => Some(false),
        _ => None,
    }
}

fn flag_str(flag: bool) -> &'static str {
    if flag { "TRUE" } else { "FALSE" }
}

/// One cookie as a Netscape line (no LF), or [`None`] if any field holds a
/// character that would break the line structure.
fn serialize_line(key: &Key, cookie: &Cookie) -> Option<String> {
    let fields = [
        &key.domain,
        &key.path,
        &key.name,
        &cookie.expiry,
        &cookie.value,
    ];
    if fields
        .iter()
        .any(|field| field.contains(['\t', '\r', '\n']))
    {
        return None;
    }
    Some(format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        key.domain,
        flag_str(cookie.include_subdomains),
        key.path,
        flag_str(cookie.secure),
        cookie.expiry,
        key.name,
        cookie.value,
    ))
}

impl Jar {
    pub(crate) fn len(&self) -> usize {
        self.cookies.len()
    }

    /// Serialize deterministically (key order, LF line endings, Netscape
    /// header plus turto's notice, see [`MANAGED_HEADER`]). Cookies with a
    /// tab, CR or LF in any field are refused; returns the bytes and how many
    /// cookies were refused.
    pub(crate) fn serialize(&self) -> (Vec<u8>, usize) {
        let mut out = MANAGED_HEADER.as_bytes().to_vec();
        let mut skipped = 0;
        for (key, cookie) in &self.cookies {
            match serialize_line(key, cookie) {
                Some(line) => {
                    out.extend_from_slice(line.as_bytes());
                    out.push(b'\n');
                }
                None => skipped += 1,
            }
        }
        (out, skipped)
    }

    /// What `result` changed relative to `self`: cookies new or different in
    /// `result` are upserts, cookies of `self` absent from `result` are removals.
    pub(crate) fn diff(&self, result: &Jar) -> Delta {
        let upserts = result
            .cookies
            .iter()
            .filter(|(key, cookie)| self.cookies.get(*key) != Some(*cookie))
            .map(|(key, cookie)| (key.clone(), cookie.clone()))
            .collect();
        let removals = self
            .cookies
            .keys()
            .filter(|key| !result.cookies.contains_key(*key))
            .cloned()
            .collect();
        Delta { upserts, removals }
    }

    /// Apply a delta in place. Removals only touch keys present; on a key that
    /// is both removed and upserted the upsert wins.
    pub(crate) fn apply(&mut self, delta: &Delta) {
        for key in &delta.removals {
            self.cookies.remove(key);
        }
        for (key, cookie) in &delta.upserts {
            self.cookies.insert(key.clone(), cookie.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Allowlist and caps
// ---------------------------------------------------------------------------

/// The domains present in the operator's file at startup (lowercased, one
/// leading dot stripped). Only these and their dot-boundary subdomains may be
/// refreshed.
pub(crate) struct Allowlist {
    domains: HashSet<String>,
}

impl fmt::Debug for Allowlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Allowlist")
            .field("domains", &self.domains.len())
            .finish()
    }
}

fn normalize_domain(domain: &str) -> &str {
    domain.strip_prefix('.').unwrap_or(domain)
}

impl Allowlist {
    pub(crate) fn from_jar(jar: &Jar) -> Self {
        let domains = jar
            .cookies
            .keys()
            .map(|key| normalize_domain(&key.domain))
            .filter(|domain| !domain.is_empty())
            .map(str::to_owned)
            .collect();
        Self { domains }
    }

    /// `domain` (already lowercased) equals, or is a dot-boundary subdomain
    /// of, an allowed domain.
    pub(crate) fn allows(&self, domain: &str) -> bool {
        let domain = normalize_domain(domain);
        if domain.is_empty() {
            return false;
        }
        self.domains.contains(domain)
            || domain.match_indices('.').any(|(i, _)| {
                let suffix = &domain[i + 1..];
                !suffix.is_empty() && self.domains.contains(suffix)
            })
    }
}

/// A delta after [`filter_delta`], with how many upserts were dropped and why.
pub(crate) struct Filtered {
    pub(crate) delta: Delta,
    pub(crate) foreign: usize,
    pub(crate) oversized: usize,
}

/// Drop upserts for domains outside the allowlist and upserts whose line
/// exceeds [`MAX_COOKIE_LINE`]. A dropped upsert is never turned into
/// a removal, so an existing cookie of that key stays as it is. Removals pass
/// through untouched: they can only ever touch keys already in the jar.
pub(crate) fn filter_delta(delta: Delta, allow: &Allowlist) -> Filtered {
    let Delta { upserts, removals } = delta;
    let mut kept = BTreeMap::new();
    let mut foreign = 0;
    let mut oversized = 0;
    for (key, cookie) in upserts {
        if !allow.allows(&key.domain) {
            foreign += 1;
            continue;
        }
        match serialize_line(&key, &cookie) {
            Some(line) if line.len() <= MAX_COOKIE_LINE => {
                kept.insert(key, cookie);
            }
            // Oversized, or (never from `parse`) unserializable.
            _ => oversized += 1,
        }
    }
    Filtered {
        delta: Delta {
            upserts: kept,
            removals,
        },
        foreign,
        oversized,
    }
}

/// A delta rejected whole by the total caps: the jar it would have made.
pub(crate) struct CapExceeded {
    pub(crate) cookies: usize,
    pub(crate) bytes: usize,
}

/// Apply `delta` to a copy of `current`, rejecting it whole if the result has
/// more cookies than `max(MAX_COOKIES, current count)` or serializes larger
/// than `max(MAX_JAR_BYTES, current size)`. So a jar already over the limits
/// can still be refreshed or shrink; it just cannot grow.
pub(crate) fn apply_capped(current: &Jar, delta: &Delta) -> Result<Jar, CapExceeded> {
    let current_bytes = current.serialize().0.len();
    let mut next = current.clone();
    next.apply(delta);
    let next_bytes = next.serialize().0.len();
    if next.len() > MAX_COOKIES.max(current.len()) || next_bytes > MAX_JAR_BYTES.max(current_bytes)
    {
        return Err(CapExceeded {
            cookies: next.len(),
            bytes: next_bytes,
        });
    }
    Ok(next)
}

// ---------------------------------------------------------------------------
// Store and updater
// ---------------------------------------------------------------------------

/// The jar requests are served from, plus the exact base64 sent to the sidecar.
pub(crate) struct Snapshot {
    pub(crate) jar: Jar,
    pub(crate) b64: String,
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("cookies", &self.jar.len())
            .field("b64_len", &self.b64.len())
            .finish()
    }
}

pub(crate) enum Msg {
    /// A successful extraction made from `base` came back with `result`.
    Update { base: Arc<Snapshot>, result: Jar },
    /// Save now if anything is unsaved, then acknowledge.
    Flush(oneshot::Sender<()>),
}

impl fmt::Debug for Msg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Msg::Update { base, result } => f
                .debug_struct("Update")
                .field("base_cookies", &base.jar.len())
                .field("result_cookies", &result.len())
                .finish(),
            Msg::Flush(_) => f.write_str("Flush"),
        }
    }
}

/// Handle to the live cookie jar. Reads are lock-free; all changes go through
/// the single updater task returned by [`CookieStore::new`].
pub(crate) struct CookieStore {
    current: Arc<ArcSwap<Snapshot>>,
    tx: mpsc::UnboundedSender<Msg>,
}

impl fmt::Debug for CookieStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CookieStore")
            .field("cookies", &self.current.load().jar.len())
            .finish()
    }
}

impl CookieStore {
    /// Build the store for the cookies file at `path`, whose content was
    /// `initial_bytes` and parsed to `jar`. Returns the store and the updater
    /// task, which the caller spawns; saves are debounced by `debounce`.
    pub(crate) fn new(
        path: PathBuf,
        jar: Jar,
        initial_bytes: Vec<u8>,
        debounce: Duration,
    ) -> (Self, impl Future<Output = ()> + Send + 'static) {
        let b64 = BASE64_STANDARD.encode(&initial_bytes);
        Self::with_sink(jar, b64, debounce, Persister::new(path))
    }

    fn with_sink<S: Sink>(
        jar: Jar,
        b64: String,
        debounce: Duration,
        sink: S,
    ) -> (Self, impl Future<Output = ()> + Send + 'static) {
        let allow = Allowlist::from_jar(&jar);
        let current = Arc::new(ArcSwap::from_pointee(Snapshot { jar, b64 }));
        let (tx, rx) = mpsc::unbounded_channel();
        let updater = Updater {
            current: current.clone(),
            allow,
            debounce,
            sink: Some(sink),
            dirty: false,
            deadline: None,
        };
        (Self { current, tx }, updater.run(rx))
    }

    /// The current snapshot, safe to hold across awaits.
    pub(crate) fn load(&self) -> Arc<Snapshot> {
        self.current.load_full()
    }

    /// Hand the jar an extraction from `base` came back with to the updater.
    /// Never fails: if the updater is gone the update is dropped.
    pub(crate) fn submit(&self, base: Arc<Snapshot>, result: Jar) {
        if self.tx.send(Msg::Update { base, result }).is_err() {
            tracing::debug!("cookie updater is gone; dropping a cookie update");
        }
    }

    /// Ask the updater to save any unsaved change now, waiting up to `timeout`.
    pub(crate) async fn flush(&self, timeout: Duration) {
        let (ack, done) = oneshot::channel();
        if self.tx.send(Msg::Flush(ack)).is_err() {
            tracing::debug!("cookie updater is gone; nothing to flush");
            return;
        }
        match tokio::time::timeout(timeout, done).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => tracing::debug!("cookie updater stopped before flushing"),
            Err(_) => tracing::warn!(
                timeout_secs = timeout.as_secs_f32(),
                "timed out saving cookies to disk"
            ),
        }
    }
}

/// Where the updater saves a serialized jar. [`Persister`] in production; a
/// seam so the debounce logic is testable without a filesystem.
pub(crate) trait Sink: Send + 'static {
    fn save(&mut self, bytes: &[u8]) -> SaveOutcome;
}

pub(crate) enum SaveOutcome {
    Saved,
    /// Persisting is off for good (the file or its filesystem is read-only).
    Stopped,
    /// This save failed; retry at the next deadline.
    Failed,
}

struct Updater<S> {
    current: Arc<ArcSwap<Snapshot>>,
    allow: Allowlist,
    debounce: Duration,
    /// [`None`] once persisting has stopped.
    sink: Option<S>,
    dirty: bool,
    deadline: Option<tokio::time::Instant>,
}

impl<S: Sink> Updater<S> {
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Msg>) {
        loop {
            let msg = match self.deadline {
                Some(at) => tokio::select! {
                    msg = rx.recv() => msg,
                    () = tokio::time::sleep_until(at) => {
                        self.save().await;
                        continue;
                    }
                },
                None => rx.recv().await,
            };
            let Some(msg) = msg else { break };
            match msg {
                Msg::Update { base, result } => {
                    if self.merge(&base, &result) && self.sink.is_some() && self.deadline.is_none()
                    {
                        self.deadline = Some(tokio::time::Instant::now() + self.debounce);
                    }
                }
                Msg::Flush(ack) => {
                    if self.dirty {
                        self.save().await;
                    }
                    let _ = ack.send(());
                }
            }
        }
    }

    /// diff -> domain filter -> per-cookie cap -> apply -> total cap. Returns
    /// whether the current jar changed.
    fn merge(&mut self, base: &Snapshot, result: &Jar) -> bool {
        let delta = base.jar.diff(result);
        if delta.is_empty() {
            return false;
        }
        let Filtered {
            delta,
            foreign,
            oversized,
        } = filter_delta(delta, &self.allow);
        if foreign > 0 || oversized > 0 {
            tracing::debug!(foreign, oversized, "dropped cookie updates");
        }
        if delta.is_empty() {
            return false;
        }
        let current = self.current.load_full();
        let next = match apply_capped(&current.jar, &delta) {
            Ok(next) => next,
            Err(CapExceeded { cookies, bytes }) => {
                tracing::warn!(
                    cookies,
                    bytes,
                    max_cookies = MAX_COOKIES,
                    max_bytes = MAX_JAR_BYTES,
                    "rejected a cookie update that would grow the jar past its limits"
                );
                return false;
            }
        };
        if next == current.jar {
            return false;
        }
        let (bytes, skipped) = next.serialize();
        if skipped > 0 {
            tracing::debug!(skipped, "refused unserializable cookies");
        }
        tracing::debug!(
            upserts = delta.upserts.len(),
            removals = delta.removals.len(),
            cookies = next.len(),
            "applied cookie refresh"
        );
        self.current.store(Arc::new(Snapshot {
            jar: next,
            b64: BASE64_STANDARD.encode(bytes),
        }));
        if self.sink.is_some() {
            self.dirty = true;
        }
        true
    }

    async fn save(&mut self) {
        self.deadline = None;
        let Some(mut sink) = self.sink.take() else {
            self.dirty = false;
            return;
        };
        let (bytes, _) = self.current.load().jar.serialize();
        let joined = tokio::task::spawn_blocking(move || {
            let outcome = sink.save(&bytes);
            (sink, outcome)
        })
        .await;
        match joined {
            Ok((sink, SaveOutcome::Saved)) => {
                self.sink = Some(sink);
                self.dirty = false;
            }
            Ok((sink, SaveOutcome::Failed)) => {
                // Stays dirty; retried at the next deadline.
                self.sink = Some(sink);
                self.deadline = Some(tokio::time::Instant::now() + self.debounce);
            }
            Ok((_, SaveOutcome::Stopped)) => self.dirty = false,
            Err(err) => {
                tracing::warn!(error = %err, "cookie save task failed; no longer saving cookies until restart");
                self.dirty = false;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// How the atomic path writes the serialized jar into its temp file. A seam so
/// tests can force a write failure.
type TempWriter = Box<dyn FnMut(&mut NamedTempFile, &[u8]) -> io::Result<()> + Send>;

/// Writes the jar back to the operator's cookies file. turto owns the file
/// while it runs (last writer wins): a file changed on disk is overwritten and
/// a deleted one is recreated. It never writes onto a read-only file and never
/// leaves the file empty on a write error.
pub(crate) struct Persister {
    configured: PathBuf,
    /// A rename failed with EBUSY/EXDEV (bind mount, other device): go in place.
    atomic_disabled: bool,
    /// The previous save failed (warn on the first failure only).
    failing: bool,
    temp_writer: TempWriter,
}

impl fmt::Debug for Persister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Persister")
            .field("path", &self.configured)
            .field("atomic_disabled", &self.atomic_disabled)
            .finish()
    }
}

enum Attempt {
    Saved,
    Stop,
    Fail(io::Error),
}

enum AtomicError {
    /// Not eligible / rename refused: write in place instead.
    Fallback,
    /// A failed save; must not fall back (the target is untouched).
    Failed(io::Error),
}

impl Persister {
    pub(crate) fn new(configured: PathBuf) -> Self {
        Self {
            configured,
            atomic_disabled: false,
            failing: false,
            temp_writer: Box::new(|tmp, bytes| tmp.write_all(bytes)),
        }
    }

    fn try_save(&mut self, bytes: &[u8]) -> Attempt {
        // Resolved on every write, so a symlink swapped later is followed.
        // A missing file does not resolve; the configured path is used then.
        let target =
            std::fs::canonicalize(&self.configured).unwrap_or_else(|_| self.configured.clone());
        let path = target.display();

        let meta = match std::fs::metadata(&target) {
            Ok(meta) => meta,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Deleted while turto runs: recreate it (atomic path, 0600).
                return match self.create_atomic(&target, bytes) {
                    Ok(()) => {
                        tracing::info!(path = %path, "cookies file was missing; recreated it");
                        Attempt::Saved
                    }
                    Err(err) => Attempt::Fail(err),
                };
            }
            Err(err) => return Attempt::Fail(err),
        };
        if meta.permissions().readonly() {
            tracing::info!(
                path = %path,
                "cookies file is read-only; cookie refreshes are kept in memory only"
            );
            return Attempt::Stop;
        }

        if self.atomic_eligible(&meta) {
            match self.write_atomic(&target, &meta, bytes) {
                Ok(()) => return Attempt::Saved,
                Err(AtomicError::Fallback) => {}
                Err(AtomicError::Failed(err)) => return Attempt::Fail(err),
            }
        }

        match write_in_place(&target, bytes) {
            Ok(()) => Attempt::Saved,
            Err(err) if err.kind() == io::ErrorKind::ReadOnlyFilesystem => {
                tracing::info!(
                    path = %path,
                    "cookies file is on a read-only filesystem; cookie refreshes are kept in memory only"
                );
                Attempt::Stop
            }
            Err(err) => Attempt::Fail(err),
        }
    }

    fn atomic_eligible(&self, meta: &Metadata) -> bool {
        if self.atomic_disabled {
            return false;
        }
        // A rename would split a hard link from the file it shares.
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.nlink() != 1 {
                return false;
            }
        }
        #[cfg(not(unix))]
        let _ = meta;
        true
    }

    /// Create a missing target: write a sibling temp file (0600 on unix, as
    /// `tempfile` creates it) and rename it into place. There is no existing
    /// file, so no hard-link, owner or mode handling and no in-place fallback;
    /// any error is a failed save.
    fn create_atomic(&mut self, target: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut tmp = tempfile::Builder::new()
            .prefix(&temp_prefix(target))
            .tempfile_in(temp_dir(target))?;
        (self.temp_writer)(&mut tmp, bytes)?;
        tmp.as_file().sync_all()?;
        // `persist` renames over whatever is at `target` by then (last writer
        // wins); on error the temp file is removed when dropped.
        tmp.persist(target).map(drop).map_err(|err| err.error)
    }

    /// Write a sibling temp file and rename it over the target.
    fn write_atomic(
        &mut self,
        target: &Path,
        meta: &Metadata,
        bytes: &[u8],
    ) -> Result<(), AtomicError> {
        let mut tmp = match tempfile::Builder::new()
            .prefix(&temp_prefix(target))
            .tempfile_in(temp_dir(target))
        {
            Ok(tmp) => tmp,
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem
                ) =>
            {
                tracing::debug!(error = %err, "cannot create a temp file beside the cookies file; writing in place");
                return Err(AtomicError::Fallback);
            }
            Err(err) => return Err(AtomicError::Failed(err)),
        };

        // A rename would hand the file to whoever we run as.
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let tmp_uid = tmp.as_file().metadata().map_err(AtomicError::Failed)?.uid();
            if tmp_uid != meta.uid() {
                return Err(AtomicError::Fallback);
            }
        }

        // A write/sync failure (ENOSPC, EIO, ...) is a failed save: the target
        // is untouched, and writing in place could leave it half-written.
        (self.temp_writer)(&mut tmp, bytes).map_err(AtomicError::Failed)?;
        tmp.as_file().sync_all().map_err(AtomicError::Failed)?;

        // Keep the operator's mode; the temp file starts at 0600.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode() & 0o777;
            tmp.as_file()
                .set_permissions(std::fs::Permissions::from_mode(mode))
                .map_err(AtomicError::Failed)?;
        }
        #[cfg(not(unix))]
        let _ = meta;

        match tmp.persist(target) {
            Ok(_) => Ok(()),
            Err(err) => {
                // `err.file` drops here, removing the temp file.
                let err = err.error;
                if matches!(
                    err.kind(),
                    io::ErrorKind::ResourceBusy | io::ErrorKind::CrossesDevices
                ) {
                    tracing::debug!(error = %err, "cannot rename over the cookies file; writing it in place from now on");
                    self.atomic_disabled = true;
                    Err(AtomicError::Fallback)
                } else if rename_refused(&err) {
                    tracing::debug!(error = %err, "cannot rename over the cookies file; writing in place");
                    Err(AtomicError::Fallback)
                } else {
                    Err(AtomicError::Failed(err))
                }
            }
        }
    }
}

/// A rename refused for permission/sharing reasons, which an in-place write may
/// still get past.
fn rename_refused(err: &io::Error) -> bool {
    if matches!(
        err.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem
    ) {
        return true;
    }
    // ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION.
    #[cfg(windows)]
    if matches!(err.raw_os_error(), Some(5 | 32 | 33)) {
        return true;
    }
    false
}

/// The directory the temp file goes in: the target's own, so the rename stays
/// on one filesystem.
fn temp_dir(target: &Path) -> &Path {
    match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// `.<file name>.turto-tmp-`, so a leftover temp file is recognizable.
fn temp_prefix(target: &Path) -> String {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "cookies".into());
    format!(".{name}.turto-tmp-")
}

/// Overwrite the target from offset 0 without truncating first, then cut it to
/// length: a failed write leaves old bytes behind, never an empty file.
fn write_in_place(target: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(target)?;
    file.write_all(bytes)?;
    file.set_len(bytes.len() as u64)?;
    file.sync_all()
}

impl Sink for Persister {
    fn save(&mut self, bytes: &[u8]) -> SaveOutcome {
        match self.try_save(bytes) {
            Attempt::Saved => {
                if self.failing {
                    tracing::info!(path = %self.configured.display(), "saved cookies file again");
                    self.failing = false;
                }
                tracing::debug!(path = %self.configured.display(), bytes = bytes.len(), "saved cookies file");
                SaveOutcome::Saved
            }
            Attempt::Stop => SaveOutcome::Stopped,
            Attempt::Fail(err) => {
                if self.failing {
                    tracing::debug!(path = %self.configured.display(), kind = ?err.kind(), error = %err, "failed to save cookies file again");
                } else {
                    tracing::warn!(path = %self.configured.display(), kind = ?err.kind(), error = %err, "failed to save cookies file; will retry");
                }
                self.failing = true;
                SaveOutcome::Failed
            }
        }
    }
}

#[cfg(test)]
mod test;
