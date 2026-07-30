"""turto yt-dlp extraction sidecar.

A long-lived process that imports yt-dlp once and serves extraction over HTTP
on loopback via FastAPI/uvicorn. It does extraction ONLY -- audio bytes stay
in Rust.

Protocol:
  - On startup, prints ``PORT <n>`` as the first stdout line, then serves.
  - ``GET  /health``   -> 200 {"ok": true}                (unauthenticated)
  - ``POST /extract``  -> 200 <yt-dlp info dict as JSON>  (X-Turto-Secret gated)
        request body: {"url": str, "flat_playlist": bool, "cookies_b64": str|null}
        errors come back as {"error": str, "type"?: str} with a non-2xx status.
  - ``POST /download`` -> 200 streamed media bytes        (X-Turto-Secret gated)
        request body: {"info": <yt-dlp info dict from /extract>, "cookies_b64": str|null}
        Used only for formats Rust can't fetch directly (DASH segments, SABR /
        no-url): yt-dlp downloads the track in-process via --load-info-json (no
        second extraction) and the bytes are tail-streamed back as written.

``cookies_b64`` is the base64-encoded *content* of a Netscape cookies.txt (the
client sends bytes, not a path, so the sidecar never touches the caller's
filesystem). It is decoded into a per-request throwaway file for yt-dlp.

Configuration is passed entirely on the command line (no environment):
  --secret <s>           shared secret required on /extract and /download
  --bun <arg>            the bun runtime arg ("bun" or "bun:/path/to/bun")
  --max-concurrency <n>  max simultaneous extractions/downloads (default 8)
"""

import argparse
import asyncio
import base64
import contextlib
import json
import os
import shutil
import socket
import sys
import tempfile
import threading
from collections.abc import AsyncIterator, Callable, Iterator
from typing import Any

import uvicorn
import yt_dlp
from fastapi import FastAPI, Header
from fastapi.concurrency import run_in_threadpool
from fastapi.responses import JSONResponse, StreamingResponse
from pydantic import BaseModel
from yt_dlp.utils import DownloadCancelled

FORMAT = "ba[abr>0][vcodec=none]/best"


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="turto-ytdlp-sidecar")
    parser.add_argument("--secret", required=True)
    parser.add_argument("--bun", default="")
    parser.add_argument("--max-concurrency", type=int, default=8)
    return parser.parse_args(argv)


def build_js_runtimes(bun_arg: str) -> dict[str, dict[str, str | None]]:
    """Mirror the CLI default (``deno``) plus turto's configured bun runtime.

    The CLI flag ``--js-runtimes bun:<path>`` resolves to the YoutubeDL param
    ``{'deno': {'path': None}, 'bun': {'path': '<path>'}}``.
    """
    runtimes = {"deno": {"path": None}}
    if bun_arg:
        name, _, path = bun_arg.partition(":")
        runtimes[name.lower()] = {"path": path or None}
    return runtimes


ARGS = parse_args(sys.argv[1:])
SECRET = ARGS.secret
JS_RUNTIMES = build_js_runtimes(ARGS.bun)
# Bound concurrent yt-dlp work so a burst of guilds can't exhaust resources.
# Extraction (the hot path) and download (the rare, long-lived SABR fallback)
# get SEPARATE limiters, so a batch of slow downloads can never starve
# extraction. Each unit of work still gets its own YoutubeDL, run in a thread.
EXTRACT_SEM = asyncio.Semaphore(max(1, ARGS.max_concurrency))
DOWNLOAD_SEM = asyncio.Semaphore(max(1, ARGS.max_concurrency))
# Live cleanup tasks, held so a detached one cannot be garbage-collected mid-wait
# (see reap_download).
DOWNLOAD_REAPERS: set[asyncio.Task[None]] = set()

app = FastAPI()

# Server-agnostic "shutdown requested" signal. The /shutdown handler raises it;
# serve() (which owns the uvicorn.Server) acts on it, so the ASGI app never
# refers to the server it happens to run under.
SHUTDOWN = asyncio.Event()


class ExtractRequest(BaseModel):
    url: str
    flat_playlist: bool = False
    # base64-encoded Netscape cookies.txt content (see module docstring).
    cookies_b64: str | None = None


class DownloadRequest(BaseModel):
    # The yt-dlp info dict previously returned by /extract.
    info: dict[str, Any]
    cookies_b64: str | None = None


@contextlib.contextmanager
def cookiefile_from_b64(cookies_b64: str | None) -> Iterator[str | None]:
    """Materialize base64 cookie content into a private, throwaway cookie file.

    The client sends the cookies file's *content*, so the sidecar never reaches
    into the caller's filesystem. Just as important: yt-dlp rewrites its
    ``cookiefile`` on every ``YoutubeDL.close()`` (``save_cookies`` -> a
    non-atomic truncate + rewrite). Handing each request its own copy means those
    write-backs can never race and corrupt a shared file under concurrency. The
    file is written and closed before being yielded (so yt-dlp can open it on
    every platform, incl. Windows) and removed on exit.
    """
    if not cookies_b64:
        yield None
        return
    data = base64.b64decode(cookies_b64)
    fd, path = tempfile.mkstemp(prefix="turto-ck-", suffix=".txt")
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
        yield path
    finally:
        with contextlib.suppress(OSError):
            os.remove(path)


def do_extract(url: str, flat: bool, cookies_b64: str | None) -> dict[str, Any]:
    opts = {
        "quiet": True,
        "no_warnings": True,
        "js_runtimes": JS_RUNTIMES,
    }
    if flat:
        opts["extract_flat"] = "in_playlist"
    else:
        opts["noplaylist"] = True
        opts["format"] = FORMAT

    with cookiefile_from_b64(cookies_b64) as cookiefile:
        if cookiefile:
            opts["cookiefile"] = cookiefile
        with yt_dlp.YoutubeDL(opts) as ydl:
            info = ydl.extract_info(url, download=False)
            return ydl.sanitize_info(info)


@app.get("/health")
async def health() -> dict[str, Any]:
    # The version lets the blue/green updater decide whether a recycle is needed.
    return {"ok": True, "yt_dlp": yt_dlp.version.__version__}


@app.post("/shutdown")
async def shutdown(x_turto_secret: str = Header(default="")) -> JSONResponse:
    if not SECRET or x_turto_secret != SECRET:
        return JSONResponse(status_code=403, content={"error": "forbidden"})
    # Raise the shutdown signal only; serve() turns it into uvicorn's graceful
    # drain. The blue/green updater calls this on the OLD instance after cutting
    # new traffic over to the new one.
    SHUTDOWN.set()
    return JSONResponse(content={"ok": True})


@app.post("/extract")
async def extract(
    req: ExtractRequest, x_turto_secret: str = Header(default="")
) -> JSONResponse:
    if not SECRET or x_turto_secret != SECRET:
        return JSONResponse(status_code=403, content={"error": "forbidden"})

    try:
        async with EXTRACT_SEM:
            info = await run_in_threadpool(
                do_extract, req.url, req.flat_playlist, req.cookies_b64
            )
    except Exception as exc:  # noqa: BLE001
        return JSONResponse(
            status_code=502,
            content={"error": str(exc), "type": type(exc).__name__},
        )
    return JSONResponse(content=info)


def run_download(
    info: dict[str, Any],
    cookies_b64: str | None,
    tmpdir: str,
    on_filename: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    """Download the audio for an already-extracted info dict into `tmpdir`
    (blocking; raises on failure, including a cancellation).

    Uses ``download_with_info_file`` (the library form of ``--load-info-json``)
    so the expensive extraction done by ``/extract`` is NOT repeated. yt-dlp's
    own downloader is used, so SABR / DASH-segment formats with no single
    fetchable URL still work; if the saved info has gone stale it transparently
    re-extracts from ``webpage_url``. ``nopart=True`` writes straight to the
    final file so the streamer can tail it; ``on_filename`` reports that path
    (from the progress hook) as soon as it is known.

    The progress hook is also where a stop lands: yt-dlp has no other way in
    mid-download, and it re-raises ``DownloadCancelled`` untouched rather than
    turning it into a ``DownloadError`` (which ``download_with_info_file`` would
    answer by re-extracting and starting over).
    """

    def hook(d: dict[str, Any]) -> None:
        if cancel.is_set():
            raise DownloadCancelled("consumer went away")
        filename = d.get("filename")
        if filename:
            on_filename(filename)

    # A uniquely named file (not a fixed "info.json") so nothing can collide.
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".info.json", dir=tmpdir, delete=False, encoding="utf-8"
    ) as handle:
        json.dump(info, handle)
        info_path = handle.name

    opts = {
        "quiet": True,
        "no_warnings": True,
        "js_runtimes": JS_RUNTIMES,  # used only if a stale-info re-extract is needed
        "noplaylist": True,
        "format": FORMAT,
        "nopart": True,
        "outtmpl": os.path.join(tmpdir, "audio.%(ext)s"),
        "progress_hooks": [hook],
    }

    with cookiefile_from_b64(cookies_b64) as cookiefile:
        if cookiefile:
            opts["cookiefile"] = cookiefile
        with yt_dlp.YoutubeDL(opts) as ydl:
            ydl.download_with_info_file(info_path)


class DownloadState:
    """Loop-confined state for one in-progress download.

    Only the event loop thread ever touches these fields: the worker thread
    reports updates exclusively via ``loop.call_soon_threadsafe(state.set_*,
    ...)``. So no field is read and written from two threads at once and
    correctness does not depend on the GIL.
    """

    def __init__(self) -> None:
        self.filename: str | None = None
        self.error: str | None = None

    def set_filename(self, filename: str) -> None:
        self.filename = filename

    def set_error(self, error: str) -> None:
        self.error = error


def reap_download(running: asyncio.Future[None], tmpdir: str) -> None:
    """Free one download's resources once its worker thread has actually stopped.

    Both belong to the worker until then: ``tmpdir`` is what it writes into, and the
    permit is what makes ``--max-concurrency`` a limit. Releasing either while the
    worker still runs leaves it downloading at full speed into a deleted directory,
    off the books.

    Detached rather than awaited because the caller's ``finally`` may already be
    running under a cancelled task (a client disconnect), where every ``await``
    returns at once -- the wait has to outlive it.
    """

    async def reap() -> None:
        try:
            await running
        finally:
            shutil.rmtree(tmpdir, ignore_errors=True)
            DOWNLOAD_SEM.release()

    task = asyncio.create_task(reap())
    DOWNLOAD_REAPERS.add(task)
    task.add_done_callback(DOWNLOAD_REAPERS.discard)


async def download_stream(
    info: dict[str, Any], cookies_b64: str | None
) -> AsyncIterator[bytes]:
    """Tail-stream the file yt-dlp writes, so playback can start before the
    download completes.

    Thread/async safety:

    * The blocking download runs in an executor thread; it does NOT mutate any
      state shared with this coroutine. It reports the output path and
      completion/failure back to the event loop via ``loop.call_soon_threadsafe``,
      so ``state`` and ``done`` are only ever touched on the loop thread -- no
      reliance on the GIL making shared mutable state thread-safe.
    * An empty read while ``done`` is unset is NOT end-of-stream; we wait and
      retry, finishing only once ``done`` is set, which the worker signals after
      the file is fully written and closed. So the final drain read sees the
      whole file and can never truncate, regardless of read vs. write speed (a
      stalled writer just makes the reader wait).
    * On failure the worker records ``state.error`` (ordered before ``done`` by
      FIFO callback scheduling) and we raise, aborting the response, so the
      client treats it as failed and never caches a truncated/empty file.
    * ``cancel`` is the one field both threads touch, hence a ``threading.Event``:
      set here when nobody is left to read the bytes, polled by the worker's
      progress hook.

    Assumes yt-dlp writes the output sequentially (append-only): true here
    (audio-only format, ``nopart=True``, no post-processing, no concurrent
    fragments).
    """
    await DOWNLOAD_SEM.acquire()
    tmpdir = None
    handle = None
    running = None
    cancel = threading.Event()
    try:
        # Inside the try so a failure here (e.g. mkdtemp) still hits the finally
        # that releases the semaphore -- acquiring before the try would leak a
        # permit on any such failure.
        tmpdir = tempfile.mkdtemp(prefix="turto-dl-")
        loop = asyncio.get_running_loop()
        done = asyncio.Event()
        # Touched only on the loop thread; the worker reports via call_soon_threadsafe.
        state = DownloadState()

        def report_filename(filename: str) -> None:  # called from the worker thread
            loop.call_soon_threadsafe(state.set_filename, filename)

        def worker() -> None:  # runs in the executor thread
            try:
                run_download(info, cookies_b64, tmpdir, report_filename, cancel)
            except Exception as exc:  # noqa: BLE001
                loop.call_soon_threadsafe(state.set_error, str(exc))
            finally:
                loop.call_soon_threadsafe(done.set)

        running = loop.run_in_executor(None, worker)

        streamed = 0
        while True:
            if handle is None:
                filename = state.filename
                if filename and os.path.exists(filename):
                    handle = open(filename, "rb")  # noqa: SIM115
                elif done.is_set():
                    # Finished before any file appeared: only valid as a failure
                    # (a genuine success always writes an output file).
                    raise RuntimeError(state.error or "download produced no output")
                else:
                    await asyncio.sleep(0.05)
                    continue

            chunk = handle.read(262144)
            if chunk:
                streamed += len(chunk)
                yield chunk
            elif done.is_set():
                # Writer finished; surface a failure by raising (aborts the
                # response) rather than ending cleanly with a partial file.
                if state.error:
                    raise RuntimeError(state.error)
                tail = handle.read()  # drain anything written since the last read
                if tail:
                    streamed += len(tail)
                    yield tail
                if streamed == 0:
                    # A "successful" empty download must not look like a valid,
                    # cacheable track; surface it as a failure instead.
                    raise RuntimeError("download produced no output")
                break
            else:
                await asyncio.sleep(0.05)
    finally:
        # Nobody is left to read the bytes -- the track was skipped, the client hung
        # up, or the download is simply over -- so stop yt-dlp instead of paying full
        # bandwidth for a file that is about to be deleted.
        cancel.set()
        if handle is not None:
            handle.close()
        if running is None:
            # Never got as far as a worker; nothing to wait for.
            if tmpdir is not None:
                shutil.rmtree(tmpdir, ignore_errors=True)
            DOWNLOAD_SEM.release()
        else:
            reap_download(running, tmpdir)


@app.post("/download", response_model=None)
async def download(
    req: DownloadRequest, x_turto_secret: str = Header(default="")
) -> StreamingResponse | JSONResponse:
    if not SECRET or x_turto_secret != SECRET:
        return JSONResponse(status_code=403, content={"error": "forbidden"})

    return StreamingResponse(
        download_stream(req.info, req.cookies_b64),
        media_type="application/octet-stream",
    )


async def serve(sock: socket.socket) -> None:
    """Composition root for the server: own the uvicorn.Server here and stop it
    when the shutdown signal fires, so the ASGI app never sees the server.

    Setting ``should_exit`` triggers uvicorn's graceful drain (stop accepting,
    let in-flight requests finish, exit). The Rust parent owns the hard-kill
    backstop.
    """
    config = uvicorn.Config(app, log_level="warning", access_log=False)
    server = uvicorn.Server(config)
    serve_task = asyncio.create_task(server.serve(sockets=[sock]))
    shutdown_task = asyncio.create_task(SHUTDOWN.wait())

    # Whichever comes first: a /shutdown request, or the server stopping on its
    # own (an error, or a signal uvicorn caught). Then let it drain and exit.
    await asyncio.wait({serve_task, shutdown_task}, return_when=asyncio.FIRST_COMPLETED)
    server.should_exit = True
    shutdown_task.cancel()
    await serve_task


def main() -> None:
    # Bind an ephemeral loopback port ourselves (port 0 -> the OS picks a free
    # one) so we can announce it before serving, then hand the *same* bound
    # socket to uvicorn. Keeping the one socket makes this race-free: nothing can
    # claim the port between discovery and listen.
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.bind(("127.0.0.1", 0))
    (_addr, port) = sock.getsockname()
    sys.stdout.write(f"PORT={port}\n")
    sys.stdout.flush()

    asyncio.run(serve(sock))


if __name__ == "__main__":
    main()
