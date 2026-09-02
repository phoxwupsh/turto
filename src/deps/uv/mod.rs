use crate::{
    deps::{DepsError, extract_targz_to, extract_to, fetch_github_latest},
    models::config::YtdlpConfig,
    utils::get_http_client,
};
use reqwest::header::USER_AGENT;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
    sync::OnceLock,
    time::Duration,
};
use tokio::io::AsyncWriteExt;

mod os;
use os::{get_archive_name, get_exec_name, get_managed_python_exe, get_venv_python};

/// Python version requested from uv's managed interpreters for the sidecar venv.
const PYTHON_VERSION: &str = "3.13";

/// Ceiling on one uv invocation. Deliberately generous -- a cold `uv pip install`
/// legitimately takes minutes on a slow link -- because the point is only that a
/// wedged uv cannot hang startup before the Discord client exists, or hold
/// `sidecar::update`'s lock for the rest of the process.
const UV_TIMEOUT: Duration = Duration::from_secs(10 * 60);

static UV_PYTHON: OnceLock<PathBuf> = OnceLock::new();
static UV_EXEC: OnceLock<PathBuf> = OnceLock::new();
static UV_CACHE: OnceLock<PathBuf> = OnceLock::new();

/// Absolute path to the Python interpreter of the uv-managed sidecar venv.
/// Panics if [`setup_uv`] has not run -- a wiring bug, not a runtime condition.
pub fn get_uv_python() -> &'static Path {
    UV_PYTHON.get().expect("setup_uv must run first")
}

/// Absolute path to the resolved uv binary (system or vendored), stored at
/// [`setup_uv`] so later upgrades reuse it without re-downloading. Panics if
/// [`setup_uv`] has not run.
pub fn get_uv_exec() -> &'static Path {
    UV_EXEC.get().expect("setup_uv must run first")
}

/// uv's cache dir (`<uv_dir>/cache`), captured at [`setup_uv`]. Owned here so
/// post-setup operations ([`upgrade_ytdlp`], [`installed_ytdlp_version`]) locate
/// it themselves instead of a caller threading the uv dir back in. Panics if
/// [`setup_uv`] has not run.
fn uv_cache() -> &'static Path {
    UV_CACHE.get().expect("setup_uv must run first")
}

/// Ensure a uv binary is available, then create (or refresh) a self-contained
/// venv holding the latest `yt-dlp` plus the FastAPI sidecar stack. Stores the
/// venv interpreter path for the sidecar to run. Mirrors
/// [`crate::deps::bun::setup_bun`].
pub async fn setup_uv(config: &YtdlpConfig, uv_dir: impl AsRef<Path>) -> Result<(), DepsError> {
    let uv_dir = uv_dir.as_ref();
    if !uv_dir.is_dir() {
        std::fs::create_dir_all(uv_dir)?;
    }

    let uv_exec = ensure_uv_binary(config, uv_dir).await?;
    let python = ensure_runtime(&uv_exec, uv_dir).await?;

    tracing::info!(python = %python.display(), "uv sidecar runtime ready");
    UV_PYTHON.set(python).ok();
    UV_EXEC.set(uv_exec).ok();
    UV_CACHE.set(uv_dir.join("cache")).ok();
    Ok(())
}

async fn ensure_uv_binary(config: &YtdlpConfig, uv_dir: &Path) -> Result<PathBuf, DepsError> {
    if config.use_system_uv {
        let path =
            which::which("uv").map_err(|_| std::io::Error::from(std::io::ErrorKind::NotFound))?;
        tracing::info!(path = %path.display(), "system uv found");
        return Ok(path);
    }

    if let Some(local) = locate_uv_exec(uv_dir)
        && uv_works(&local)
    {
        tracing::info!(path = %local.display(), "found local uv");
        return Ok(local);
    }

    tracing::warn!("local uv not found");
    let tag = fetch_github_latest("astral-sh/uv").await?;
    tracing::info!(version = tag, "found latest uv");

    let archive_name = get_archive_name();
    let url = format!(
        "https://github.com/astral-sh/uv/releases/download/{}/{}",
        tag, archive_name
    );
    let client = get_http_client();
    let mut resp = client
        .get(&url)
        .header(USER_AGENT, "phoxwupsh/turto")
        .send()
        .await?
        .error_for_status()?;

    let archive_path = uv_dir.join(archive_name);
    let mut archive = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&archive_path)
        .await?;

    tracing::info!(url, "fetching uv");
    while let Some(chunk) = resp.chunk().await? {
        archive.write_all(&chunk).await?;
    }
    archive.flush().await?;
    drop(archive);

    if archive_name.ends_with(".zip") {
        extract_to(&archive_path, uv_dir)?;
    } else {
        extract_targz_to(&archive_path, uv_dir)?;
    }
    std::fs::remove_file(&archive_path)?;

    let exec = locate_uv_exec(uv_dir).ok_or(DepsError::UvNotFound)?;
    make_executable(&exec)?;
    Ok(exec)
}

/// Install a managed CPython + create the venv (both under `uv_dir`, so every
/// runtime dep lives in the bot's working dir and nothing pollutes the system)
/// and install/upgrade the sidecar packages. All locations are passed as CLI
/// flags (`--install-dir`, `--cache-dir`, explicit `--python`); no env vars.
/// Returns the absolute path to the venv's Python interpreter.
async fn ensure_runtime(uv_exec: &Path, uv_dir: &Path) -> Result<PathBuf, DepsError> {
    let cache_dir = uv_dir.join("cache");
    let py_dir = uv_dir.join("python");
    let venv_dir = uv_dir.join("venv");
    let venv_python = get_venv_python(&venv_dir);
    let fresh = !venv_python.is_file();

    // 1. Install a managed CPython into the project-local python dir. Idempotent
    //    and cheap when already present; needs network only for a new patch.
    tracing::info!("ensuring uv-managed CPython {PYTHON_VERSION}");
    if let Err(err) = run_uv(
        uv_exec,
        [
            OsStr::new("python"),
            OsStr::new("install"),
            OsStr::new("--install-dir"),
            py_dir.as_os_str(),
            OsStr::new("--cache-dir"),
            cache_dir.as_os_str(),
            OsStr::new(PYTHON_VERSION),
        ],
    )
    .await
    {
        // Tolerate failure only if a managed interpreter is already present.
        if locate_managed_python(&py_dir).is_none() {
            return Err(err);
        }
        tracing::warn!(error = %err, "python install failed; using existing managed CPython");
    }

    let managed_python = locate_managed_python(&py_dir).ok_or_else(|| {
        DepsError::Uv(format!(
            "no managed CPython {PYTHON_VERSION} found under {}",
            py_dir.display()
        ))
    })?;

    // 2. Create the venv on that explicit interpreter (no managed lookup, no env).
    if fresh {
        tracing::info!("creating sidecar venv");
        run_uv(
            uv_exec,
            [
                OsStr::new("venv"),
                venv_dir.as_os_str(),
                OsStr::new("--python"),
                managed_python.as_os_str(),
                OsStr::new("--cache-dir"),
                cache_dir.as_os_str(),
            ],
        )
        .await?;
    }

    // 3. Install/upgrade the sidecar packages into the venv.
    tracing::info!("installing/upgrading sidecar venv packages");
    let install = run_uv(
        uv_exec,
        [
            OsStr::new("pip"),
            OsStr::new("install"),
            OsStr::new("--python"),
            venv_python.as_os_str(),
            OsStr::new("--cache-dir"),
            cache_dir.as_os_str(),
            OsStr::new("-U"),
            OsStr::new("yt-dlp"),
            OsStr::new("fastapi"),
            OsStr::new("uvicorn"),
        ],
    )
    .await;

    if let Err(err) = install {
        // A fresh venv with no packages is unusable; an existing one can keep
        // what it had (e.g. transient network failure on boot).
        if fresh {
            return Err(err);
        }
        tracing::warn!(error = %err, "package upgrade failed; using previously installed versions");
    }

    // Return an absolute interpreter path that still routes THROUGH the venv
    // directory. We canonicalize the venv dir (a real directory) but must NOT
    // canonicalize `bin/python` itself: that symlink points at the managed
    // interpreter, and invoking it directly would lose the venv's site-packages.
    let venv_abs = std::fs::canonicalize(&venv_dir).unwrap_or(venv_dir);
    Ok(get_venv_python(&venv_abs))
}

/// Upgrade yt-dlp in the sidecar venv in place (`uv pip install -U yt-dlp`;
/// `--pre` selects the nightly channel). Only the on-disk package changes -- the
/// warm sidecar keeps running its already-imported yt-dlp until it is recycled.
pub async fn upgrade_ytdlp(nightly: bool) -> Result<(), DepsError> {
    let cache_dir = uv_cache();
    let venv_python = get_uv_python();
    let uv_exec = get_uv_exec();

    let mut args: Vec<&OsStr> = vec![
        OsStr::new("pip"),
        OsStr::new("install"),
        OsStr::new("--python"),
        venv_python.as_os_str(),
        OsStr::new("--cache-dir"),
        cache_dir.as_os_str(),
        OsStr::new("-U"),
    ];
    if nightly {
        args.push(OsStr::new("--pre"));
    }
    args.push(OsStr::new("yt-dlp"));
    run_uv(uv_exec, args).await
}

/// The yt-dlp version currently installed in the sidecar venv, read from
/// `uv pip show` (the `Version:` field). Compared against the running sidecar's
/// reported version to decide whether a recycle is warranted.
pub async fn installed_ytdlp_version() -> Result<String, DepsError> {
    let cache_dir = uv_cache();
    let venv_python = get_uv_python();
    let uv_exec = get_uv_exec();

    let stdout = run_uv_captured(
        uv_exec,
        [
            OsStr::new("pip"),
            OsStr::new("show"),
            OsStr::new("--python"),
            venv_python.as_os_str(),
            OsStr::new("--cache-dir"),
            cache_dir.as_os_str(),
            OsStr::new("yt-dlp"),
        ],
    )
    .await?;

    stdout
        .lines()
        .find_map(|line| line.strip_prefix("Version:").map(|v| v.trim().to_string()))
        .ok_or_else(|| DepsError::Uv("no Version field in `uv pip show yt-dlp`".to_string()))
}

/// Find the uv-managed CPython interpreter under `py_dir`. uv installs to
/// `<py_dir>/cpython-<ver>-<triple>/...`; there may be both a concrete
/// versioned dir and a `cpython-<minor>-<triple>` alias — either works, so we
/// take the newest patch whose interpreter exists.
fn locate_managed_python(py_dir: &Path) -> Option<PathBuf> {
    let prefix = format!("cpython-{PYTHON_VERSION}");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(py_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&prefix))
        })
        .collect();
    dirs.sort_by(|a, b| newest_patch_first(a, b));
    dirs.into_iter()
        .map(|d| get_managed_python_exe(&d))
        .find(|p| p.is_file())
        .and_then(|p| std::fs::canonicalize(&p).ok().or(Some(p)))
}

/// Order two install dirs newest-patch-first, the unversioned alias last, then by name
/// so the choice does not depend on `read_dir` order.
fn newest_patch_first(a: &Path, b: &Path) -> std::cmp::Ordering {
    managed_patch(b)
        .cmp(&managed_patch(a))
        .then_with(|| a.cmp(b))
}

/// The patch number of a `cpython-3.13.9-<triple>` install dir; `None` for uv's
/// unversioned `cpython-3.13-<triple>` alias. Parsed rather than compared as text
/// because lexicographically `3.13.10` sorts *below* `3.13.9`.
fn managed_patch(dir: &Path) -> Option<u32> {
    dir.file_name()?
        .to_str()?
        .strip_prefix("cpython-")?
        .split('-')
        .next()?
        .split('.')
        .nth(2)?
        .parse()
        .ok()
}

/// Run a uv subcommand, discarding its stdout. See [`run_uv_captured`].
async fn run_uv<I, S>(uv_exec: &Path, args: I) -> Result<(), DepsError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_uv_captured(uv_exec, args).await.map(|_| ())
}

/// Run a uv subcommand and return its captured stdout, inheriting turto's environment
/// unchanged. uv owns its own configuration (cache dir, managed-Python dir, ...) via its
/// many env vars and defaults; turto deliberately does not set any of them.
///
/// Bounded by [`UV_TIMEOUT`], and `kill_on_drop` so the timeout actually ends the
/// process: an orphaned uv would keep its lock on the venv and stall every later
/// invocation.
async fn run_uv_captured<I, S>(uv_exec: &Path, args: I) -> Result<String, DepsError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let child = tokio::process::Command::new(uv_exec)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let output = tokio::time::timeout(UV_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| DepsError::UvTimeout(UV_TIMEOUT))??;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DepsError::Uv(stderr.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Walk `uv_dir` (one level deep is enough for uv's archives) to find the uv
/// executable, since the archive layout differs between platforms.
fn locate_uv_exec(uv_dir: &Path) -> Option<PathBuf> {
    let name = get_exec_name();
    let direct = uv_dir.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let entries = std::fs::read_dir(uv_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn uv_works(path: &Path) -> bool {
    std::process::Command::new(path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map(|o| o.stdout.trim_ascii_start().starts_with(b"uv"))
        .unwrap_or(false)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{locate_managed_python, managed_patch, newest_patch_first};
    use std::path::{Path, PathBuf};

    /// Install dir names in the order [`locate_managed_python`] considers them.
    fn sorted(names: &[&str]) -> Vec<String> {
        let mut dirs: Vec<PathBuf> = names.iter().map(PathBuf::from).collect();
        dirs.sort_by(|a, b| newest_patch_first(a, b));
        dirs.iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    /// Patch numbers must be compared numerically: `3.13.10` is newer than `3.13.9`,
    /// though it sorts below it as text.
    #[test]
    fn the_newest_patch_wins_over_the_longer_string() {
        assert_eq!(
            managed_patch(Path::new("cpython-3.13.9-linux-x86_64-gnu")),
            Some(9)
        );
        assert_eq!(
            managed_patch(Path::new("cpython-3.13.10-linux-x86_64-gnu")),
            Some(10)
        );
        assert_eq!(
            sorted(&[
                "cpython-3.13.9-linux-x86_64-gnu",
                "cpython-3.13.10-linux-x86_64-gnu",
            ])[0],
            "cpython-3.13.10-linux-x86_64-gnu"
        );
    }

    /// uv's unversioned alias dir has no patch to compare, so it is the last resort
    /// rather than an unknown that outranks a real install.
    #[test]
    fn the_unversioned_alias_sorts_last() {
        assert_eq!(
            managed_patch(Path::new("cpython-3.13-linux-x86_64-gnu")),
            None
        );
        assert_eq!(
            sorted(&[
                "cpython-3.13-linux-x86_64-gnu",
                "cpython-3.13.9-linux-x86_64-gnu",
            ]),
            vec![
                "cpython-3.13.9-linux-x86_64-gnu",
                "cpython-3.13-linux-x86_64-gnu",
            ]
        );
    }

    /// A missing python dir is a normal state (nothing installed yet), not a panic.
    #[test]
    fn a_missing_python_dir_is_none() {
        assert!(locate_managed_python(Path::new("/nonexistent/turto-uv-python")).is_none());
    }
}
