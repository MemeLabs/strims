#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Loads every devtools storybook story in headless Chromium and fails the
ones that crash, record errors their allow lists don't cover, or render
nothing; then reports the catalog's coverage.

  python3 hack/storybook/check.py (--port N | --url URL) [--build] [-j N]
                                  [--only PREFIX]... [--theme dark|light]
                                  [--device pc|portrait|landscape]
                                  [--screenshots DIR] [--strict] [--list]
                                  [--settle MS] [--timeout S] [-v]

`--port N` starts hack/console/server.py on 127.0.0.1:N (no indexing) to
serve this checkout's target/console/dist and stops it afterwards; `--url`
uses a server already running (http://HOST:PORT, the page is
/devtools.html). `--build` runs `npm run console:build` first (npm from
PATH, else nvm's newest).

The storybook's catalog page (#/storybook) embeds its index as JSON in
<script id="storybook-index">: every story's route, every UI file under
src/components, src/pages and src/watch as covered, skipped or missing,
and catalog entries naming files that don't exist. Each story then loads
in a fresh page at #<route>?theme=..&device=.. and waits for its root
([data-story-root]) to leave data-story-state="loading", then for
`--settle` ms (default 1000) and the network to go quiet (up to 3 s more).
The page records the story's uncaught errors, unhandled rejections,
console.error calls and render errors, marks each one its allow lists
(the host's and the story's allowErrors) cover, and says them in
<script id="storybook-story">; a story fails on data-story-state="error"
(a crash or an error no allow list covers), on a root with nothing
visible in it, or on not loading within `--timeout` s (default 20).

`--only PREFIX` keeps the stories whose route (with or without its
/storybook/ prefix) starts with PREFIX. `--screenshots DIR` writes one
PNG per story, of its frame, named by its route (chat/Chat/Chat ->
chat__Chat__Chat.png). `--strict` also fails on catalog files neither
covered nor skipped, and on stale catalog entries. `--list` prints the
stories and the coverage without loading them. `-j N` runs N browsers
(default 4). `-v` prints every error of a failing story, not just its
first. Exits 1 when a story (or, with --strict, the catalog) fails.
"""

import argparse
import base64
import concurrent.futures
import glob
import json
import os
import queue
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VIEWPORT = (1600, 1100)
CHROMIUMS = ["/snap/bin/chromium", "chromium", "chromium-browser", "google-chrome"]
CHROMIUM_FLAGS = [
    "--headless",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "--hide-scrollbars",
    "--mute-audio",
    "--autoplay-policy=no-user-gesture-required",
    # A page in a browser of its own, but never treated as backgrounded.
    "--disable-background-timer-throttling",
    "--disable-renderer-backgrounding",
    "--disable-backgrounding-occluded-windows",
    "--remote-debugging-port=0",
]
INDEX_TIMEOUT_S = 60
QUIET_S = 0.25
EXTRA_SETTLE_S = 3.0


# --- a minimal WebSocket client (RFC 6455, text messages) ---


class WebSocket:
    def __init__(self, url, timeout=10):
        u = urllib.parse.urlsplit(url)
        self.sock = socket.create_connection((u.hostname, u.port), timeout=timeout)
        key = base64.b64encode(os.urandom(16)).decode()
        path = u.path + ("?" + u.query if u.query else "")
        self.sock.sendall((
            f"GET {path} HTTP/1.1\r\nHost: {u.hostname}:{u.port}\r\nUpgrade: websocket\r\n"
            f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        ).encode())
        self.buf = bytearray()
        while b"\r\n\r\n" not in self.buf:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("the DevTools socket closed during its handshake")
            self.buf += chunk
        end = self.buf.index(b"\r\n\r\n")
        head = bytes(self.buf[:end])
        del self.buf[:end + 4]
        if not head.startswith(b"HTTP/1.1 101"):
            raise ConnectionError(f"DevTools handshake: {head.decode(errors='replace').splitlines()[0]}")
        self.fragments = []

    def send(self, text):
        payload = text.encode()
        n = len(payload)
        if n < 126:
            header = struct.pack(">BB", 0x81, 0x80 | n)
        elif n < 1 << 16:
            header = struct.pack(">BBH", 0x81, 0x80 | 126, n)
        else:
            header = struct.pack(">BBQ", 0x81, 0x80 | 127, n)
        mask = os.urandom(4)
        pad = (mask * (n // 4 + 1))[:n]
        masked = (int.from_bytes(payload, "big") ^ int.from_bytes(pad, "big")).to_bytes(n, "big") if n else b""
        self.sock.sendall(header + mask + masked)

    def _frame(self):
        """The next whole frame in the buffer (consumed), else None."""
        b = self.buf
        if len(b) < 2:
            return None
        fin, opcode, n, pos = b[0] & 0x80, b[0] & 0x0F, b[1] & 0x7F, 2
        if n == 126:
            if len(b) < 4:
                return None
            n, pos = struct.unpack(">H", b[2:4])[0], 4
        elif n == 127:
            if len(b) < 10:
                return None
            n, pos = struct.unpack(">Q", b[2:10])[0], 10
        if b[1] & 0x80:
            pos += 4  # servers don't mask; skip one anyway
        if len(b) < pos + n:
            return None
        payload = bytes(b[pos:pos + n])
        del b[:pos + n]
        return fin, opcode, payload

    def recv(self, timeout):
        """The next text message, or None when none comes within `timeout` s."""
        deadline = time.monotonic() + timeout
        while True:
            frame = self._frame()
            if frame is None:
                left = deadline - time.monotonic()
                if left <= 0:
                    return None
                self.sock.settimeout(left)
                try:
                    chunk = self.sock.recv(1 << 20)
                except socket.timeout:
                    return None
                if not chunk:
                    raise ConnectionError("the DevTools socket closed")
                self.buf += chunk
                continue
            fin, opcode, payload = frame
            if opcode == 0x8:
                raise ConnectionError("the DevTools socket closed")
            if opcode == 0x9:
                self.sock.sendall(struct.pack(">BB", 0x8A, 0x80 | len(payload)) + b"\0\0\0\0" + payload)
                continue
            if opcode in (0x1, 0x0):
                self.fragments.append(payload)
                if fin:
                    text = b"".join(self.fragments).decode()
                    self.fragments = []
                    return text

    def close(self):
        try:
            self.sock.close()
        except OSError:
            pass


# --- Chrome DevTools Protocol over it ---


class CDPError(Exception):
    pass


class Page:
    """One page target: commands, and the events the checks follow."""

    def __init__(self, ws):
        self.ws = ws
        self.next_id = 0
        self.inflight = set()
        self.exceptions = []

    def call(self, method, params=None, timeout=30):
        self.next_id += 1
        mid = self.next_id
        self.ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
        deadline = time.monotonic() + timeout
        while True:
            left = deadline - time.monotonic()
            if left <= 0:
                raise TimeoutError(f"{method} took over {timeout} s")
            msg = self.ws.recv(left)
            if msg is None:
                continue
            m = json.loads(msg)
            if m.get("id") == mid:
                if "error" in m:
                    raise CDPError(f"{method}: {m['error'].get('message')}")
                return m.get("result", {})
            self.event(m)

    def pump(self, seconds):
        deadline = time.monotonic() + seconds
        while (left := deadline - time.monotonic()) > 0:
            msg = self.ws.recv(left)
            if msg is not None:
                self.event(json.loads(msg))

    def event(self, m):
        method, p = m.get("method"), m.get("params", {})
        if method == "Network.requestWillBeSent":
            if not p.get("request", {}).get("url", "").startswith(("data:", "blob:")):
                self.inflight.add(p["requestId"])
        elif method in ("Network.loadingFinished", "Network.loadingFailed"):
            self.inflight.discard(p.get("requestId"))
        elif method == "Runtime.exceptionThrown":
            d = p.get("exceptionDetails", {})
            self.exceptions.append(d.get("exception", {}).get("description") or d.get("text", "exception"))

    def eval(self, expression, timeout=30):
        r = self.call("Runtime.evaluate", {"expression": expression, "returnByValue": True}, timeout)
        if "exceptionDetails" in r:
            raise CDPError(r["exceptionDetails"].get("text", "evaluation failed"))
        return r.get("result", {}).get("value")

    def try_eval(self, expression):
        """eval, or None while the page has no context to run it in (mid-navigation)."""
        try:
            return self.eval(expression)
        except CDPError:
            return None

    def navigate(self, url):
        self.call("Page.navigate", {"url": "about:blank"})
        self.inflight.clear()
        self.exceptions = []
        self.call("Page.navigate", {"url": url})


def find_chromium(name):
    for c in [name] if name else CHROMIUMS:
        if c and (Path(c).is_file() or shutil.which(c)):
            return c if Path(c).is_file() else shutil.which(c)
    sys.exit("check: no Chromium found (--chromium PATH)")


class Browser:
    """A headless Chromium of its own, with one page."""

    def __init__(self, exe, profiles, i):
        self.profile = Path(tempfile.mkdtemp(prefix=f"b{i}-", dir=profiles))
        self.log = open(self.profile.parent / f"{self.profile.name}.log", "wb")
        self.proc = subprocess.Popen(
            [exe, *CHROMIUM_FLAGS, f"--user-data-dir={self.profile}",
             f"--window-size={VIEWPORT[0]},{VIEWPORT[1]}", "about:blank"],
            stdin=subprocess.DEVNULL, stdout=self.log, stderr=self.log, start_new_session=True)
        port_file = self.profile / "DevToolsActivePort"
        deadline = time.monotonic() + 30
        while not port_file.is_file() or not port_file.read_text().strip():
            if self.proc.poll() is not None or time.monotonic() > deadline:
                self.close()
                raise RuntimeError(f"Chromium didn't start (its log: {self.log.name})")
            time.sleep(0.1)
        port = int(port_file.read_text().split()[0])
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/list", timeout=10) as res:
            targets = json.load(res)
        target = next(t for t in targets if t.get("type") == "page")
        self.page = Page(WebSocket(target["webSocketDebuggerUrl"]))
        self.page.call("Network.enable")
        self.page.call("Runtime.enable")
        self.page.call("Emulation.setDeviceMetricsOverride", {
            "width": VIEWPORT[0], "height": VIEWPORT[1], "deviceScaleFactor": 1, "mobile": False})

    def close(self):
        if getattr(self, "page", None):
            self.page.ws.close()
        try:
            os.killpg(self.proc.pid, signal.SIGTERM)
            self.proc.wait(10)
        except (ProcessLookupError, subprocess.TimeoutExpired):
            try:
                os.killpg(self.proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        self.log.close()
        shutil.rmtree(self.profile, ignore_errors=True)


# --- the checks ---


STORY_PROBE = """(() => {
  const root = document.querySelector("[data-story-root]");
  const json = document.getElementById("storybook-story")?.textContent;
  if (!root) return JSON.stringify({ root: false });
  let visible = false;
  for (const el of root.querySelectorAll("*")) {
    const r = el.getBoundingClientRect();
    if (r.width > 0 && r.height > 0) { visible = true; break; }
  }
  const r = root.getBoundingClientRect();
  return JSON.stringify({
    root: true,
    state: root.dataset.storyState,
    story: json ? JSON.parse(json) : null,
    visible,
    text: root.innerText.trim().length,
    rect: { x: r.x, y: r.y, width: r.width, height: r.height },
  });
})()"""


def first_line(text, width=110):
    line = (text or "").strip().splitlines()[0] if (text or "").strip() else ""
    return line if len(line) <= width else line[:width - 1] + "…"


def page_url(base, route, args):
    query = urllib.parse.urlencode({k: v for k, v in (("theme", args.theme), ("device", args.device))
                                    if v and v not in ("dark", "pc")})
    return f"{base}/devtools.html#{route}" + (f"?{query}" if query else "")


def load_index(browser, base, args):
    browser.page.navigate(page_url(base, "/storybook", args))
    deadline = time.monotonic() + INDEX_TIMEOUT_S
    while time.monotonic() < deadline:
        text = browser.page.try_eval('document.getElementById("storybook-index")?.textContent ?? null')
        if text:
            return json.loads(text)
        browser.page.pump(0.2)
    detail = "; ".join(browser.page.exceptions[:3])
    sys.exit(f"check: the catalog page gave no #storybook-index in {INDEX_TIMEOUT_S} s"
             + (f" ({detail})" if detail else ""))


def check_story(browser, base, story, args):
    route = story["route"]
    page = browser.page
    start = time.monotonic()
    result = {"route": route, "status": "ok", "error": "", "errors": [], "allowed": 0}
    page.navigate(page_url(base, route, args))

    while True:
        probe = json.loads(page.try_eval(STORY_PROBE) or "{}")
        if probe.get("state") in ("ok", "error"):
            break
        if time.monotonic() - start > args.timeout:
            result["status"] = "FAIL"
            state = probe.get("state") if probe.get("root") else "no story root"
            result["error"] = f"timeout after {args.timeout} s ({state})"
            result["errors"] = [result["error"], *page.exceptions]
            result["time"] = time.monotonic() - start
            return result
        page.pump(0.1)

    mounted = time.monotonic()
    quiet_since = None
    while True:
        now = time.monotonic()
        if page.inflight:
            quiet_since = None
        elif quiet_since is None:
            quiet_since = now
        settled = now - mounted >= args.settle / 1000 and quiet_since and now - quiet_since >= QUIET_S
        if settled or now - mounted >= args.settle / 1000 + EXTRA_SETTLE_S:
            break
        page.pump(0.05)

    probe = json.loads(page.eval(STORY_PROBE))
    errors = (probe.get("story") or {}).get("errors", [])
    failing = [e for e in errors if not e.get("allowedBy")]
    result["allowed"] = len(errors) - len(failing)
    result["errors"] = [f"[{e['kind']}] {e['text']}" for e in failing]
    if probe.get("state") == "error":
        result["status"] = "FAIL"
        result["error"] = first_line(result["errors"][0]) if result["errors"] else "error state"
    elif not probe.get("visible") and not probe.get("text"):
        result["status"] = "FAIL"
        result["error"] = "empty story root"
        result["errors"] = [result["error"]]

    if args.screenshots and probe.get("root"):
        r = probe["rect"]
        clip = {"x": max(0, r["x"]), "y": max(0, r["y"]), "width": max(1, r["width"]),
                "height": max(1, r["height"]), "scale": 1}
        shot = page.call("Page.captureScreenshot", {"format": "png", "clip": clip, "captureBeyondViewport": True})
        name = route.removeprefix("/storybook/").replace("/", "__") + ".png"
        (Path(args.screenshots) / name).write_bytes(base64.b64decode(shot["data"]))
    result["time"] = time.monotonic() - start
    return result


# --- the server and the build ---


def npm_command():
    npm = shutil.which("npm")
    if npm:
        return npm, os.environ.copy()
    found = sorted(glob.glob(os.path.expanduser("~/.nvm/versions/node/*/bin/npm")),
                   key=lambda p: [int(x) for x in Path(p).parts[-3].lstrip("v").split(".") if x.isdigit()])
    if not found:
        sys.exit("check: no npm on PATH or under ~/.nvm")
    env = os.environ.copy()
    env["PATH"] = str(Path(found[-1]).parent) + os.pathsep + env.get("PATH", "")
    return found[-1], env


def build():
    npm, env = npm_command()
    start = time.monotonic()
    res = subprocess.run([npm, "run", "console:build"], cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if res.returncode:
        print(res.stdout[-6000:], file=sys.stderr)
        sys.exit("check: npm run console:build failed")
    print(f"check: built in {time.monotonic() - start:.1f} s", file=sys.stderr)


def start_server(port, log_dir):
    log = open(log_dir / "server.log", "wb")
    proc = subprocess.Popen(
        [sys.executable, str(ROOT / "hack" / "console" / "server.py"), "--port", str(port),
         "--host", "127.0.0.1", "--no-index"],
        cwd=ROOT, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
    base = f"http://127.0.0.1:{port}"
    deadline = time.monotonic() + 60
    while True:
        if proc.poll() is not None:
            log.close()
            sys.exit(f"check: the console server exited: {(log_dir / 'server.log').read_text()[-2000:]}")
        try:
            with urllib.request.urlopen(f"{base}/devtools.html", timeout=2) as res:
                if res.status == 200:
                    return proc, log, base
        except OSError:
            pass
        if time.monotonic() > deadline:
            proc.terminate()
            sys.exit(f"check: the console server didn't answer on {base} in 60 s")
        time.sleep(0.2)


# --- output ---


def in_scope(path, files):
    return not files or any(path.startswith(f) for f in files)


def coverage_lines(index, files=()):
    c = index["counts"]
    done = c["covered"] + c["skipped"]
    pct = round(100 * done / c["total"]) if c["total"] else 100
    lines = [f"coverage: {c['covered']} covered, {c['skipped']} skipped, {c['missing']} missing of "
             f"{c['total']} files ({pct}% accounted for); {len(index['stories'])} stories in "
             f"{len(index['modules'])} modules"]
    for f in index["files"] if files else []:
        if in_scope(f["path"], files):
            note = f.get("reason") or f"{len(f.get('stories') or [])} stories"
            lines.append(f"  {f['status']:8} {f['path']}" + ("" if f["status"] == "missing" else f"  ({note})"))
    for s in index.get("stale", []):
        if not in_scope(s["path"], files):
            continue
        what = "skips a covered file" if s["kind"] == "skips-covered" else f"{s['kind']} a file outside the catalog"
        lines.append(f"  stale: {s['module']} {what}: {s['path']}")
    for m in index["modules"]:
        if m.get("error"):
            lines.append(f"  module error: {m['module']}: {first_line(m['error'])}")
    return lines


def print_table(results, verbose):
    width = max([len("story")] + [len(r["route"].removeprefix("/storybook/")) for r in results])
    print(f"{'story'.ljust(width)}  status  time   first error")
    for r in sorted(results, key=lambda r: r["route"]):
        status = r["status"] if r["status"] != "ok" or not r["allowed"] else f"ok+{r['allowed']}"
        print(f"{r['route'].removeprefix('/storybook/').ljust(width)}  {status.ljust(6)}  "
              f"{r['time']:4.1f}s  {r['error']}")
    for r in sorted(results, key=lambda r: r["route"]):
        if r["status"] == "ok":
            continue
        print(f"\n--- {r['route']}")
        for text in r["errors"] if verbose else r["errors"][:1]:
            print("\n".join(text.splitlines()[:40 if verbose else 20]))


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    where = p.add_mutually_exclusive_group(required=True)
    where.add_argument("--port", type=int, help="start hack/console/server.py on this port")
    where.add_argument("--url", help="a running server's base URL")
    p.add_argument("--build", action="store_true", help="npm run console:build first")
    p.add_argument("-j", "--jobs", type=int, default=4, help="browsers at once (default 4)")
    p.add_argument("--only", action="append", default=[], metavar="PREFIX", help="stories whose route starts so")
    p.add_argument("--theme", choices=["dark", "light"], default="dark")
    p.add_argument("--device", choices=["pc", "portrait", "landscape"], default="pc")
    p.add_argument("--screenshots", metavar="DIR", help="a PNG of each story's frame here")
    p.add_argument("--strict", action="store_true", help="fail on uncovered or stale catalog files too")
    p.add_argument("--list", action="store_true", help="print the stories and coverage only")
    p.add_argument("--files", action="append", default=[], metavar="PREFIX",
                   help="list the catalog files under this src/ path (e.g. components/Chat/); "
                        "--strict then judges only these")
    p.add_argument("--settle", type=int, default=1000, metavar="MS", help="wait after mount (default 1000)")
    p.add_argument("--timeout", type=float, default=20, metavar="S", help="per story (default 20)")
    p.add_argument("--chromium", help="the browser (default the snap's, else chromium on PATH)")
    p.add_argument("-v", "--verbose", action="store_true", help="every error of a failing story")
    args = p.parse_args(argv)

    exe = find_chromium(args.chromium)
    if args.build:
        build()
    if args.screenshots:
        Path(args.screenshots).mkdir(parents=True, exist_ok=True)

    # The snap's Chromium sees only its own corner of the home directory.
    profiles_root = (Path.home() / "snap" / "chromium" / "common") if "/snap/" in exe else Path(tempfile.gettempdir())
    profiles_root.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="storybook-check-", dir=profiles_root))

    server = None
    browsers = []
    started = time.monotonic()
    try:
        if args.port:
            server = start_server(args.port, work)
            base = server[2]
        else:
            base = args.url.rstrip("/")
            if base.endswith("/devtools.html"):
                base = base.removesuffix("/devtools.html")

        browsers.append(Browser(exe, work, 0))
        index = load_index(browsers[0], base, args)
        prefixes = [o if o.startswith("/") else f"/storybook/{o}" for o in args.only]
        stories = [s for s in index["stories"] if not prefixes or any(s["route"].startswith(x) for x in prefixes)]
        if not stories and prefixes:
            print(f"check: no story's route starts with {' or '.join(prefixes)}", file=sys.stderr)
            return 2

        if args.list:
            for s in stories:
                allow = f"  allowErrors {', '.join(s['allowErrors'])}" if s["allowErrors"] else ""
                print(f"{s['route']}{allow}")
            print("\n".join(coverage_lines(index, args.files)))
            return 0

        jobs = max(1, min(args.jobs, len(stories)))
        with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
            browsers += list(pool.map(lambda i: Browser(exe, work, i), range(1, jobs)))
        todo = queue.Queue()
        for s in stories:
            todo.put(s)
        results, lock = [], threading.Lock()

        def worker(browser):
            while True:
                try:
                    story = todo.get_nowait()
                except queue.Empty:
                    return
                try:
                    r = check_story(browser, base, story, args)
                except (CDPError, TimeoutError, ConnectionError, json.JSONDecodeError) as e:
                    r = {"route": story["route"], "status": "FAIL", "error": f"checker: {e}",
                         "errors": [f"checker: {e}"], "allowed": 0, "time": 0.0}
                with lock:
                    results.append(r)
                    print(f"[{len(results)}/{len(stories)}] {r['status']:4} {r['time']:4.1f}s {r['route']}",
                          file=sys.stderr)

        threads = [threading.Thread(target=worker, args=(b,), daemon=True) for b in browsers[:jobs]]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        print_table(results, args.verbose)
        print()
        failed = [r for r in results if r["status"] != "ok"]
        times = [r["time"] for r in results]
        print(f"stories: {len(results) - len(failed)} ok, {len(failed)} failed of {len(results)} "
              f"(theme {args.theme}, device {args.device}); {time.monotonic() - started:.1f} s total, "
              f"{(sum(times) / len(times)) if times else 0:.1f} s a story on {jobs} browsers")
        print("\n".join(coverage_lines(index, args.files)))
        module_errors = [m for m in index["modules"] if m.get("error")]
        catalog_failed = args.strict and (
            any(f["status"] == "missing" and in_scope(f["path"], args.files) for f in index["files"])
            or any(in_scope(x["path"], args.files) for x in index.get("stale", [])))
        if catalog_failed:
            print("strict: the catalog has missing or stale files")
        return 1 if failed or module_errors or catalog_failed else 0
    finally:
        for b in browsers:
            b.close()
        if server:
            proc, log, _ = server
            proc.terminate()
            try:
                proc.wait(15)
            except subprocess.TimeoutExpired:
                proc.kill()
            log.close()
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
