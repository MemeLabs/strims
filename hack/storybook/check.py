#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Loads every devtools storybook story in headless Chromium and fails the
ones that crash, record errors their allow lists don't cover, or render
nothing; then reports the catalog's coverage.

  python3 hack/storybook/check.py (--port N | --url URL) [--build] [-j N]
                                  [--only PREFIX]... [--theme dark|light]
                                  [--device pc|portrait|landscape|TABLET]
                                  [--screenshots DIR] [--strict] [--list]
                                  [--settle MS] [--timeout S] [-v]

`--port N` starts hack/console/server.py on 127.0.0.1:N (no indexing) to
serve this checkout's target/console/dist and stops it afterwards; `--url`
uses a server already running (http://HOST:PORT, the page is
/devtools.html). `--build` runs `pnpm run console:build` first (pnpm from
PATH, else the newest nvm node's).

The storybook's index document (#/storybook-index, the page's catalog
builds its index there) embeds the index as JSON in
<script id="storybook-index">: every story's route, every UI file under
src/components, src/pages and src/watch as covered, skipped or missing,
and catalog entries naming files that don't exist. Each story then loads
as its frame document, the one the storybook page shows in an iframe, at
#/storybook-frame/<route below /storybook/>?theme=..&device=.., in a fresh
page whose viewport is the device's (pc 1372x1017, about the page's
frame; portrait 390x844, landscape 844x390; the tablets ipad_portrait
820x1180, ipad_landscape 1180x820, tablet_portrait 800x1280 and
tablet_landscape 1280x800), and waits for its root
([data-story-root]) to leave data-story-state="loading", then for
`--settle` ms (default 1000) and the network to go quiet (up to 3 s more).
The page records the story's uncaught errors, unhandled rejections,
console.error calls and render errors, marks each one its allow lists
(the host's and the story's allowErrors) cover, and says them in
<script id="storybook-story">; a story fails on data-story-state="error"
(a crash or an error no allow list covers), on nothing visible in its
root or in what it portals into the document's body by `--timeout` s
(an empty root is polled until then), or on not loading within
`--timeout` s (default 20).

`--only PREFIX` keeps the stories whose route (with or without its
/storybook/ prefix) starts with PREFIX. `--screenshots DIR` writes one
PNG per story, of its frame, named by its route (chat/Chat/Chat ->
chat__Chat__Chat.png). `--strict` also fails on catalog files neither
covered nor skipped, and on stale catalog entries. `--list` prints the
stories and the coverage without loading them. `-j N` runs N browsers
(default JOBS: three in four of the cores, at most 12). `-v` prints
every error of a failing story, not just its first. Exits 1 when a story
(or, with --strict, the catalog) fails.

  python3 hack/storybook/check.py (--port N | --url URL --base-port M)
                                  --compare REF [--build] [-j N] [--only PREFIX]...
                                  [--theme dark|light] [--device pc|portrait|landscape|TABLET]
                                  [--all-modes] [--tolerance N] [--report DIR]
                                  [--fail-on-diff] [--rebuild-base] [--settle MS] [--timeout S]

`--compare REF` is visual regression: each story of either build, this
checkout's (head) and REF's (base), is shot in both, in the same browser
one after the other, and the two diffed. REF builds once per commit, in a
detached worktree at target/storybook/base/<sha>/src, kept as
target/storybook/base/<sha>/dist (the worktree removed; `--rebuild-base`
builds it again), and is served by hack/console/server.py --dist on
`--base-port` (default `--port` + 1). The worktree installs its own
node_modules, as its lockfile calls for: `pnpm install --frozen-lockfile`
for a pnpm-lock.yaml (hard links into pnpm's store, so it's cheap), `npm
ci` for a package-lock.json (a commit from before pnpm), and builds with
the same tool. A cached base build that doesn't say it was built so is
built again. The head's own node_modules must be what its pnpm-lock.yaml
installs (pnpm's own check, verify-deps-before-run, and the lockfile
pnpm's install left in node_modules/.pnpm the same), or the run is
refused (exit 2, before --build: `pnpm install --frozen-lockfile` is the
cure).
Each build's index says what it was made from (its commit and the time
its build began: GIT_HASH and BUILD_TIME). The head is refused (exit 2,
before any story runs) unless it is the tree's: HEAD's commit, and no
input of the build (HEAD_INPUTS) changed since it began, so a stale
target/console/dist (or one --url serves from elsewhere) is never compared
under HEAD's name; `--build` makes it the tree's. Its name is HEAD's
commit, with +dirty when those inputs have changes git sees (the build
holds them). A base whose index names another commit than REF's is
refused too (`--rebuild-base`). A
mode is a theme and a device: dark (pc), light (pc), portrait and
landscape (dark), and each tablet (dark); `--all-modes` runs all but the
iPad's two (ALL_MODES), else `--theme` and `--device` name one. A build
from before the per-story frames loads its stories in the storybook page
at 1600x1100, as it did then, shot by its story root.

A capture holds everything that moves still, so a build compared with
itself diffs to nothing: before its own scripts each document gets a
clock only the checker moves (Date, performance.now and Intl's default
date start at 2026-01-01T00:00:00Z; timeouts, intervals, animation
frames, idle callbacks and CSS animations and transitions run on it), a
seeded Math.random and crypto random, empty local and session storage,
no text caret and no smooth scrolling (CAPTURE_SCRIPT); the time zone is
UTC, the locale en-US and the page always focused; requests off the
build's server (https://, a third-party embed's iframe among them) get
stubs, a grey page for a document. The clock moves only while the
network (workers' requests included) has been quiet for NET_QUIET_S, the
document has nothing pending (a script or stylesheet it inserted with no
load or error event yet, a fetch unsettled, an XHR not ended: the page
can ask for more between the checker's look at the network and the
step, and a chunk's loader times out on the clock), and no worker is new
or has just exchanged a message, after the page has laid out what
changed: to the next timer until the root leaves
"loading", then on to `--settle` ms of clock time after it (and while
the root is empty, on in 100 ms steps up to `--timeout` s). A story frame
on the clock renders once i18next has its strings. Then finite
animations and transitions jump to their end, infinite ones (spinners,
animated emotes) and SVG animations hold at their start, videos pause,
images decode and fonts load, until nothing changes; each index's
hostMask selectors (the landing page's build hash, its text first set to
a fixed string so the box is the same in both builds) and the story's
`visual.mask` are painted over in both shots, and a story whose
`visual.skip` holds isn't shot (src/devtools/storybook/types.ts). The
root is shot until two shots two frames apart match (up to STABLE_SHOTS
more; the report notes one that never did).

The diff runs in Chromium (canvas pixels): a pixel changes when a channel
moves by more than `--tolerance` (default 2 of 255) or only one shot has
it (a size change). The report (`--report DIR`, default
target/storybook/compare/<time>-<head>-vs-<base>) is one directory:
index.html, report.json and PNGs of the stories not unchanged: changed
ones, the most changed first, with base, head and a diff (changed pixels
magenta over a greyed head), then failing (on either side), added,
removed and skipped ones; and the servers' logs, head-server.log (with
--port) and base-server.log, written there as they run. It prints a
table per mode, the changed stories, the two builds' commits and the
report's path. A story the checker itself fails on gets a fresh browser
and one more try, the failure said as it happens and, when the second
try passes, kept in the story's `retried`: a browser that stalls or
goes, or a capture still waiting at `--timeout` s on a request (no
answer, or a body unfinished) or on what its document has pending, which
names its URL. Exits 0, 1 with `--fail-on-diff` when a story changed,
failed, or is added or removed, and 2 on a checker error (a refused head
or base among them).
"""

import argparse
import base64
import concurrent.futures
import fcntl
import glob
import hashlib
import html
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
# Each device's viewport, as src/devtools/storybook/host/devices.ts sizes its
# frames (pc: about the storybook page's content area at 1600x1100).
VIEWPORTS = {"pc": (1372, 1017), "portrait": (390, 844), "landscape": (844, 390),
             "ipad_portrait": (820, 1180), "ipad_landscape": (1180, 820),
             "tablet_portrait": (800, 1280), "tablet_landscape": (1280, 800)}
TABLETS = ["ipad_portrait", "ipad_landscape", "tablet_portrait", "tablet_landscape"]
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
# Browsers at once by default: three in four of the machine's cores, at most
# 12. Each capture reloads its story's document, so the browsers are CPU-bound
# (16 cores are 80% busy at 12), and past 12 a loaded machine slows them more
# than they add.
JOBS = max(1, min(12, (os.cpu_count() or 4) * 3 // 4))
INDEX_TIMEOUT_S = 60
QUIET_S = 0.25
EXTRA_SETTLE_S = 3.0

# Compare mode's browsers, besides CHROMIUM_FLAGS: animations run on the
# main thread, where the capture clock holds them (one the compositor ran
# leaves its layer rastered at a scale of its own: an animated emote's
# sprite resampled), a tile changed in part is rastered whole (a partial
# raster's edges blend a few levels apart: a badge's corners over an
# image), and colours are sRGB's.
COMPARE_FLAGS = ["--disable-threaded-animation", "--disable-partial-raster", "--force-color-profile=srgb"]
# Compare mode. A mode is a theme and a device; --all-modes runs ALL_MODES,
# the Android tablet's two among them but not the iPad's, which take the same
# portable layout at about the same sizes.
ALL_MODES = {"dark": ("dark", "pc"), "light": ("light", "pc"),
             "portrait": ("dark", "portrait"), "landscape": ("dark", "landscape"),
             "tablet_portrait": ("dark", "tablet_portrait"), "tablet_landscape": ("dark", "tablet_landscape")}
MODES = {**ALL_MODES, **{d: ("dark", d) for d in TABLETS}}
# A mode's name in the printed tables.
MODE_WIDTH = max(map(len, MODES))
# The storybook page's own size, which a build from before the per-story
# frames (no src/devtools/storybook/host/protocol.ts) renders its stories in.
PAGE_VIEWPORT = (1600, 1100)
# The capture clock's Date.now() at 0: 2026-01-01T00:00:00Z.
FIXTURE_TIME_MS = 1767225600000
# The network's quiet, in real seconds, before the capture clock moves on.
NET_QUIET_S = 0.05
# The capture clock's step while an empty root waits for content (ms).
EMPTY_STEP_MS = 100
# Shots of a capture after its first, until two in a row match.
STABLE_SHOTS = 5
# A third-party document (an embed's iframe), answered by stub.
STUB_DOCUMENT = base64.b64encode(b'<!doctype html><html style="background:#808080"></html>').decode()
BASES = ROOT / "target" / "storybook" / "base"
REPORTS = ROOT / "target" / "storybook" / "compare"
# A tree's lockfile and the package manager it calls for (package-lock.json:
# a commit from before pnpm).
LOCKFILES = {"pnpm-lock.yaml": "pnpm", "package-lock.json": "npm"}
# What `pnpm run console:build` reads of the checkout (webpack.config.js's
# inputs): a build that began before a change to any of them doesn't hold it.
HEAD_INPUTS = ("src", "assets", "webpack", "vendor_modules", "package.json", "pnpm-lock.yaml",
               "webpack.config.js", "tsconfig.json")


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
        # The requests sent and not yet finished or failed: requestId ->
        # {url, at (monotonic), answered (its response's headers came)}.
        self.inflight = {}
        self.exceptions = []
        # When the network last moved (a request sent, finished or failed).
        self.net_at = time.monotonic()
        # Requests answered by stub (compare mode: off the build's server).
        self.stubbed = set()
        # Compare mode's attached targets (workers, out-of-process frames):
        # each one's requests, which end with it.
        self.sessions = {}

    def send(self, method, params=None, session=None):
        """A command whose result nobody waits for (to an attached target's
        `session`, else the page)."""
        self.next_id += 1
        self.ws.send(json.dumps({"id": self.next_id, "method": method, "params": params or {},
                                 **({"sessionId": session} if session else {})}))

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

    def drain(self):
        """Handles the events already sent, without waiting for more."""
        while (msg := self.ws.recv(0.001)) is not None:
            self.event(json.loads(msg))

    def event(self, m):
        method, p = m.get("method"), m.get("params", {})
        if method == "Network.requestWillBeSent":
            self.net_at = time.monotonic()
            url = p.get("request", {}).get("url", "")
            if not url.startswith(("data:", "blob:")) and p["requestId"] not in self.stubbed:
                self.inflight[p["requestId"]] = {"url": url, "at": self.net_at, "answered": False}
                if m.get("sessionId") in self.sessions:
                    self.sessions[m["sessionId"]].add(p["requestId"])
        elif method == "Network.responseReceived":
            if p.get("requestId") in self.inflight:
                self.inflight[p["requestId"]]["answered"] = True
        elif method in ("Network.loadingFinished", "Network.loadingFailed"):
            self.net_at = time.monotonic()
            self.inflight.pop(p.get("requestId"), None)
        elif method == "Fetch.requestPaused":
            self.stub(p)
        elif method == "Target.attachedToTarget":
            # A worker's requests (its script's end among them) come on its
            # own session; it waits for this before it runs.
            sid = p["sessionId"]
            self.sessions[sid] = set()
            self.send("Network.enable", session=sid)
            self.send("Runtime.runIfWaitingForDebugger", session=sid)
        elif method == "Target.detachedFromTarget":
            for rid in self.sessions.pop(p.get("sessionId"), ()):
                self.inflight.pop(rid, None)
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

    def stub(self, p):
        """Answers a request Fetch paused (one off the build's server): a
        document with a neutral page, anything else with a failure, so a
        third-party embed is a plain box of its frame's size."""
        self.net_at = time.monotonic()
        if p.get("networkId"):
            self.stubbed.add(p["networkId"])
            self.inflight.pop(p["networkId"], None)
        if p.get("resourceType") == "Document":
            self.send("Fetch.fulfillRequest", {
                "requestId": p["requestId"], "responseCode": 200, "body": STUB_DOCUMENT,
                "responseHeaders": [{"name": "Content-Type", "value": "text/html"}]})
        else:
            self.send("Fetch.failRequest", {"requestId": p["requestId"], "errorReason": "BlockedByClient"})

    def navigate(self, url):
        self.call("Page.navigate", {"url": "about:blank"})
        self.inflight.clear()
        self.stubbed.clear()
        self.exceptions = []
        self.call("Page.navigate", {"url": url})


def find_chromium(name):
    for c in [name] if name else CHROMIUMS:
        if c and (Path(c).is_file() or shutil.which(c)):
            return c if Path(c).is_file() else shutil.which(c)
    sys.exit("check: no Chromium found (--chromium PATH)")


class Browser:
    """A headless Chromium of its own, with one page `viewport` (w, h) in size."""

    def __init__(self, exe, profiles, i, viewport, flags=()):
        self.profile = Path(tempfile.mkdtemp(prefix=f"b{i}-", dir=profiles))
        self.log = open(self.profile.parent / f"{self.profile.name}.log", "wb")
        self.proc = subprocess.Popen(
            [exe, *CHROMIUM_FLAGS, *flags, f"--user-data-dir={self.profile}",
             f"--window-size={viewport[0]},{viewport[1]}", "about:blank"],
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
        self.viewport = None
        self.set_viewport(viewport)

    def set_viewport(self, viewport):
        if viewport == self.viewport:
            return
        # Not "mobile": the storybook page's iframes are desktop documents too;
        # the frame's own preload makes the realm a phone's.
        self.page.call("Emulation.setDeviceMetricsOverride", {
            "width": viewport[0], "height": viewport[1], "deviceScaleFactor": 1, "mobile": False})
        self.viewport = viewport

    def enable_capture(self):
        """Compare mode's page: every document starts on the capture clock
        (CAPTURE_SCRIPT), requests off the build's server get stubs, and
        the time zone, locale and focus are fixed."""
        self.page.call("Page.enable")  # else no script evaluates on new documents
        self.page.call("Page.addScriptToEvaluateOnNewDocument",
                       {"source": CAPTURE_SCRIPT.replace("__T0__", str(FIXTURE_TIME_MS))})
        self.page.call("Fetch.enable", {"patterns": [{"urlPattern": "https://*"}]})
        self.page.call("Target.setAutoAttach", {"autoAttach": True, "waitForDebuggerOnStart": True,
                                                "flatten": True})
        self.page.call("Emulation.setTimezoneOverride", {"timezoneId": "UTC"})
        self.page.call("Emulation.setLocaleOverride", {"locale": "en-US"})
        self.page.call("Emulation.setFocusEmulationEnabled", {"enabled": True})

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
  // The story's: what's in its root, and what it portals into the body.
  const portals = [...document.body.children].filter(
    (el) => !el.contains(root) && el.tagName !== "SCRIPT");
  const content = [root, ...portals].flatMap((el) => [...el.querySelectorAll("*")]).concat(portals);
  let visible = false;
  for (const el of content) {
    const r = el.getBoundingClientRect();
    if (r.width > 0 && r.height > 0) { visible = true; break; }
  }
  const r = root.getBoundingClientRect();
  return JSON.stringify({
    root: true,
    state: root.dataset.storyState,
    story: json ? JSON.parse(json) : null,
    visible,
    text: [root, ...portals].reduce((n, el) => n + el.innerText.trim().length, 0),
    rect: { x: r.x, y: r.y, width: r.width, height: r.height },
  });
})()"""


def first_line(text, width=110):
    line = (text or "").strip().splitlines()[0] if (text or "").strip() else ""
    return line if len(line) <= width else line[:width - 1] + "…"


def frame_url(base, route, args):
    """A story route's (/storybook/<module>/<name>) frame document."""
    query = urllib.parse.urlencode({k: v for k, v in (("theme", args.theme), ("device", args.device))
                                    if v and v not in ("dark", "pc")})
    return (f"{base}/devtools.html#/storybook-frame/{route.removeprefix('/storybook/')}"
            + (f"?{query}" if query else ""))


def load_index(browser, base, route="/storybook-index"):
    browser.page.navigate(f"{base}/devtools.html#{route}")
    deadline = time.monotonic() + INDEX_TIMEOUT_S
    while time.monotonic() < deadline:
        text = browser.page.try_eval('document.getElementById("storybook-index")?.textContent ?? null')
        if text:
            return json.loads(text)
        browser.page.pump(0.2)
    detail = "; ".join(browser.page.exceptions[:3])
    sys.exit(f"check: the index document gave no #storybook-index in {INDEX_TIMEOUT_S} s"
             + (f" ({detail})" if detail else ""))


def check_story(browser, base, story, args):
    route = story["route"]
    page = browser.page
    start = time.monotonic()
    result = {"route": route, "status": "ok", "error": "", "errors": [], "allowed": 0}
    page.navigate(frame_url(base, route, args))

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

    # A story whose content comes late (a mock's stream, a step of its own) can
    # still be empty once the network is quiet on a loaded machine: an empty
    # root fails only when it's still empty at --timeout, and late content
    # gets a moment to settle before it's read.
    probe = json.loads(page.eval(STORY_PROBE))
    waited = False
    while probe.get("state") != "error" and not probe.get("visible") and not probe.get("text"):
        if time.monotonic() - start > args.timeout:
            break
        waited = True
        page.pump(0.1)
        probe = json.loads(page.eval(STORY_PROBE))
    if waited:
        page.pump(QUIET_S * 2)
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


def node_tool(name):
    """`name` (pnpm or npm) and the environment to run it in: from PATH,
    else the newest nvm node's, that node's bin first on PATH."""
    found = shutil.which(name)
    if found:
        return found, os.environ.copy()
    found = sorted(glob.glob(os.path.expanduser(f"~/.nvm/versions/node/*/bin/{name}")),
                   key=lambda p: [int(x) for x in Path(p).parts[-3].lstrip("v").split(".") if x.isdigit()])
    if not found:
        sys.exit(f"check: no {name} on PATH or under ~/.nvm")
    env = os.environ.copy()
    env["PATH"] = str(Path(found[-1]).parent) + os.pathsep + env.get("PATH", "")
    return found[-1], env


def package_manager(root):
    """The package manager `root`'s lockfile calls for (LOCKFILES), and the lockfile."""
    for name, tool in LOCKFILES.items():
        if (root / name).is_file():
            return tool, root / name
    sys.exit(f"check: {root} has none of {', '.join(LOCKFILES)}")


def build():
    pnpm, env = node_tool("pnpm")
    start = time.monotonic()
    res = subprocess.run([pnpm, "run", "console:build"], cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if res.returncode:
        print(res.stdout[-6000:], file=sys.stderr)
        sys.exit("check: pnpm run console:build failed")
    print(f"check: built in {time.monotonic() - start:.1f} s", file=sys.stderr)


def start_server(port, log_dir, dist=None, log_name="server.log"):
    log = open(log_dir / log_name, "wb")
    proc = subprocess.Popen(
        [sys.executable, str(ROOT / "hack" / "console" / "server.py"), "--port", str(port),
         "--host", "127.0.0.1", "--no-index", *(["--dist", str(dist)] if dist else [])],
        cwd=ROOT, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
    base = f"http://127.0.0.1:{port}"
    deadline = time.monotonic() + 60
    while True:
        if proc.poll() is not None:
            log.close()
            sys.exit(f"check: the console server exited: {(log_dir / log_name).read_text()[-2000:]}")
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


# --- compare mode: each story's screenshot in two builds, diffed ---


# Evaluated in every document of a compare-mode page before its own
# scripts, so the story renders the same state however long the machine
# takes. Time is a clock only the checker moves (__storybookCapture.step):
# Date, performance.now and Intl's default date start at the fixture time,
# and setTimeout, setInterval, requestAnimationFrame and
# requestIdleCallback run on it, each callback in a task of its own, nested
# timeouts clamped as browsers clamp them; so do CSS animations and
# transitions (paused, and set to the clock's time since they began).
# Math.random and crypto's getRandomValues and randomUUID are a seeded
# PRNG. The document starts with empty local and session storage, no text
# caret, and no smooth scrolling (CSS's or a script's). DOM changes,
# scrolls, focus and animations' moves are counted, so a step lets the
# page lay out and its observers and events run (real frames) before the
# clock moves; and what the document has pending (scripts and stylesheets
# inserted and not yet loaded or failed, fetches and XHRs) holds it.
CAPTURE_SCRIPT = r"""(() => {
  if (window.__storybookCapture) return;
  const T0 = __T0__;
  let now = 0;
  const realFrame = window.requestAnimationFrame.bind(window);

  const RealDate = Date;
  function CaptureDate(...args) {
    if (!new.target) return new RealDate(T0 + now).toString();
    return args.length ? new RealDate(...args) : new RealDate(T0 + now);
  }
  CaptureDate.prototype = RealDate.prototype;
  CaptureDate.now = () => Math.floor(T0 + now);
  CaptureDate.parse = RealDate.parse;
  CaptureDate.UTC = RealDate.UTC;
  Object.defineProperty(CaptureDate, "length", { value: 7 });
  window.Date = CaptureDate;
  const realNow = performance.now.bind(performance);
  performance.now = () => now;
  const dtf = Intl.DateTimeFormat.prototype;
  const format = Object.getOwnPropertyDescriptor(dtf, "format");
  Object.defineProperty(dtf, "format", {
    ...format,
    get() {
      const f = format.get.call(this);
      return (d) => f(d === undefined ? T0 + now : d);
    },
  });
  const formatToParts = dtf.formatToParts;
  dtf.formatToParts = function (d) {
    return formatToParts.call(this, d === undefined ? T0 + now : d);
  };

  let seed = 0x5eed;
  const random = () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = seed;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  Math.random = random;
  if (window.crypto) {
    crypto.getRandomValues = (a) => {
      const bytes = new Uint8Array(a.buffer, a.byteOffset, a.byteLength);
      for (let i = 0; i < bytes.length; i++) bytes[i] = (random() * 256) | 0;
      return a;
    };
    crypto.randomUUID = () => {
      const b = crypto.getRandomValues(new Uint8Array(16));
      b[6] = (b[6] & 0x0f) | 0x40;
      b[8] = (b[8] & 0x3f) | 0x80;
      const h = [...b].map((x) => x.toString(16).padStart(2, "0")).join("");
      return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
    };
  }

  if (window.top === window) {
    try {
      localStorage.clear();
      sessionStorage.clear();
    } catch {}
  }
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(
    "*, *::before, *::after { caret-color: transparent !important; scroll-behavior: auto !important; }");
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
  // Changes a real frame must follow before the clock moves: the DOM's,
  // and scrolls and focus, whose events and observers come with a frame.
  let mutations = 0;
  // Scripts (a chunk's among them) and stylesheets the document has asked
  // for and not had, from their insertion to their load or error event: the
  // clock holds while one is pending, as a chunk's loader times out on it,
  // and its request can be unsent yet or done while its script hasn't run.
  const pending = new Map();
  const watched = new WeakSet();
  const SCRIPT = /^((text|application)\/(x-)?(java|ecma)script|text\/jscript|module)?$/;
  const watch = (el) => {
    if (watched.has(el)) return;
    let url;
    if (el instanceof HTMLScriptElement) {
      if (!el.src || el.noModule || !SCRIPT.test(el.type.trim().toLowerCase())) return;
      url = el.src;
    } else if (el instanceof HTMLLinkElement) {
      if (!el.href || el.disabled || !el.relList.contains("stylesheet") || el.relList.contains("alternate")) {
        return;
      }
      url = el.href;
    } else {
      return;
    }
    watched.add(el);
    pending.set(el, { url, at: realNow() });
    const end = () => {
      pending.delete(el);
      mutations++;
    };
    el.addEventListener("load", end);
    el.addEventListener("error", end);
  };
  // So are its fetches, until their promise settles, and its XHRs, until
  // their loadend: the checker's word that the network is quiet comes
  // before the step, and the page may ask for more in between.
  const ask = (url) => {
    const key = {};
    try {
      url = new URL(url, document.baseURI).href;
    } catch {}
    pending.set(key, { url: String(url), at: realNow() });
    return () => {
      pending.delete(key);
      mutations++;
    };
  };
  if (window.fetch) {
    const realFetch = window.fetch;
    window.fetch = function fetch(input, init) {
      const done = ask(input instanceof Request ? input.url : input);
      try {
        const p = realFetch.call(window, input, init);
        p.then(done, done);
        return p;
      } catch (e) {
        done();
        throw e;
      }
    };
  }
  const xhrOpen = XMLHttpRequest.prototype.open;
  const xhrSend = XMLHttpRequest.prototype.send;
  const xhrUrl = new WeakMap();
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    xhrUrl.set(this, url);
    return xhrOpen.call(this, method, url, ...rest);
  };
  XMLHttpRequest.prototype.send = function (...args) {
    const done = ask(xhrUrl.get(this));
    this.addEventListener("loadend", done, { once: true });
    try {
      return xhrSend.apply(this, args);
    } catch (e) {
      done();
      throw e;
    }
  };
  new MutationObserver((records) => {
    mutations++;
    let removed = false;
    for (const r of records) {
      removed ||= r.removedNodes.length > 0;
      for (const n of r.addedNodes) {
        if (n.nodeType !== Node.ELEMENT_NODE) continue;
        watch(n);
        if (n.firstElementChild) for (const e of n.querySelectorAll("script[src], link[href]")) watch(e);
      }
    }
    // One taken out of the document may never load (a stylesheet doesn't).
    if (removed) for (const el of pending.keys()) if (el instanceof Node && !el.isConnected) pending.delete(el);
  }).observe(document, {
    subtree: true, childList: true, attributes: true, characterData: true });
  const instant = (o) =>
    o && typeof o === "object" && o.behavior === "smooth" ? { ...o, behavior: "instant" } : o;
  for (const target of [Element.prototype, window]) {
    for (const name of ["scroll", "scrollTo", "scrollBy"]) {
      const f = target[name];
      target[name] = function (o, ...rest) {
        mutations++;
        return f.call(this, instant(o), ...rest);
      };
    }
  }
  const scrollIntoView = Element.prototype.scrollIntoView;
  Element.prototype.scrollIntoView = function (o) {
    mutations++;
    return scrollIntoView.call(this, instant(o));
  };
  for (const prop of ["scrollTop", "scrollLeft"]) {
    const d = Object.getOwnPropertyDescriptor(Element.prototype, prop);
    Object.defineProperty(Element.prototype, prop, {
      ...d,
      set(v) {
        mutations++;
        d.set.call(this, v);
      },
    });
  }
  const focus = HTMLElement.prototype.focus;
  HTMLElement.prototype.focus = function (...args) {
    mutations++;
    return focus.apply(this, args);
  };

  // Workers run on real time: the clock holds while one is new (up to 5 s
  // before it first answers) or has just exchanged a message.
  let workerAt = -Infinity;
  const unanswered = new Map();
  if (window.Worker) {
    const RealWorker = window.Worker;
    const heard = (w) => {
      workerAt = realNow();
      unanswered.delete(w);
    };
    window.Worker = class Worker extends RealWorker {
      constructor(...args) {
        super(...args);
        workerAt = realNow();
        unanswered.set(this, workerAt);
        for (const type of ["message", "messageerror", "error"]) {
          this.addEventListener(type, () => heard(this));
        }
      }
      postMessage(...args) {
        workerAt = realNow();
        return super.postMessage(...args);
      }
      terminate() {
        unanswered.delete(this);
        return super.terminate();
      }
    };
  }
  const workersBusy = (quietMs) => {
    const t = realNow();
    if (t - workerAt < quietMs) return true;
    for (const at of unanswered.values()) if (t - at < 5000) return true;
    return false;
  };

  const timers = new Map();
  let ids = 0;
  let nesting = 0;
  const add = (fn, ms, args, repeat) => {
    const id = ++ids;
    const level = nesting + 1;
    let delay = Math.max(0, Number(ms) || 0);
    if (level > 5 && delay < 4) delay = 4;
    if (repeat && delay < 1) delay = 1;
    timers.set(id, { id, fn, args, due: now + delay, every: repeat ? delay : 0, level });
    return id;
  };
  const clear = (id) => {
    timers.delete(id);
  };
  window.setTimeout = (fn, ms, ...args) => add(fn, ms, args, false);
  window.setInterval = (fn, ms, ...args) => add(fn, ms, args, true);
  window.clearTimeout = window.clearInterval = clear;
  const FRAME = 1000 / 60;
  let frames = new Map();
  let frameTimer = 0;
  window.requestAnimationFrame = (cb) => {
    const id = ++ids;
    frames.set(id, cb);
    if (!frameTimer) {
      frameTimer = add(() => {
        frameTimer = 0;
        const due = frames;
        frames = new Map();
        for (const f of due.values()) {
          try {
            f(now);
          } catch (e) {
            reportError(e);
          }
        }
      }, (Math.floor(now / FRAME) + 1) * FRAME - now, [], false);
    }
    return id;
  };
  window.cancelAnimationFrame = (id) => {
    frames.delete(id);
  };
  window.requestIdleCallback = (cb) =>
    add(() => cb({ didTimeout: false, timeRemaining: () => 50 }), 1, [], false);
  window.cancelIdleCallback = clear;

  // A real task's turn: what the page queued before it runs first.
  const channel = new MessageChannel();
  const turns = [];
  channel.port1.onmessage = () => turns.shift()?.();
  const turn = () => new Promise((r) => {
    turns.push(r);
    channel.port2.postMessage(0);
  });
  const frame = () => new Promise((r) => realFrame(() => r()));
  // CSS animations and transitions run on the clock too: each is held from
  // the clock's time when a step first sees it (the time it began, as the
  // clock only moves in steps), and set to the clock's time since; once
  // that passes its end, it finishes.
  const began = new WeakMap();
  let held = false;
  const animate = () => {
    if (held) return;
    for (const a of document.getAnimations()) {
      let t0 = began.get(a);
      if (t0 === undefined) {
        began.set(a, (t0 = now));
        try {
          a.pause();
        } catch {}
      }
      if (a.playState === "finished") continue;
      const end = a.effect?.getComputedTiming().endTime;
      if (Number.isFinite(end) && now - t0 >= end) {
        // Done, so it ends as a played one does (its events, its effect gone).
        try {
          a.finish();
        } catch {}
        mutations++;
      } else if (a.currentTime !== now - t0) {
        try {
          a.currentTime = now - t0;
        } catch {}
        mutations++;
      }
    }
  };
  const next = () => {
    let due = Infinity;
    for (const t of timers.values()) due = Math.min(due, t.due);
    return due;
  };

  // Lets the page settle (a task's turn, and real frames while the DOM
  // changes: its layout, observers and scroll events), then fires the
  // timers due first, each in a task of its own, if they're due now or,
  // with `advance` (the network quiet), nothing pending and no worker busy
  // for `quietMs`, by `limit` (null: whenever); else, so, moves the clock
  // to `limit`. `pending` lists what is still pending.
  let seen = 0;
  const step = async (limit, advance, quietMs) => {
    await turn();
    for (let i = 0; i < 10 && mutations !== seen; i++) {
      seen = mutations;
      await frame();
      await turn();
      await turn();
    }
    const waiting = advance && (pending.size > 0 || workersBusy(quietMs));
    advance = advance && !waiting;
    animate();
    const due = next();
    let fired = 0;
    if (due <= now || (advance && due <= (limit ?? Infinity))) {
      now = Math.max(now, due);
      animate();
      const batch = [...timers.values()].filter((t) => t.due <= now)
        .sort((a, b) => a.due - b.due || a.id - b.id);
      for (const t of batch) {
        if (!timers.has(t.id)) continue;
        if (t.every) t.due += t.every;
        else timers.delete(t.id);
        nesting = t.level;
        fired++;
        try {
          if (typeof t.fn === "function") t.fn.apply(window, t.args);
          else (0, eval)(String(t.fn));
        } catch (e) {
          reportError(e);
        } finally {
          nesting = 0;
        }
        await turn();
      }
    } else if (advance && limit !== null && limit > now) {
      now = limit;
      animate();
    }
    const root = document.querySelector("[data-story-root]");
    const after = next();
    const t = realNow();
    return { now, fired, waiting, next: Number.isFinite(after) ? after : null,
             state: root ? root.dataset.storyState ?? null : null,
             pending: [...pending.values()].map((p) => ({ url: p.url, ms: Math.round(t - p.at) })) };
  };
  // Stops the clock's hold on animations, for FINISH_SCRIPT's.
  const hold = () => {
    held = true;
  };
  window.__storybookCapture = { step, turn, frame, hold, now: () => now, mutations: () => mutations };
})()"""

# Holds what still moves before the screenshot: finite animations and
# transitions jump to their end, infinite ones (spinners, animated emotes'
# sprites) and SVG animations hold at their start, videos pause; then
# images decode, fonts load, two real frames pass and the timers due now
# fire, until a round changes nothing (the round, or -1 if none did).
FINISH_SCRIPT = r"""(async () => {
  const c = window.__storybookCapture;
  c.hold();
  // A composited layer keeps paint from its history (a move by part of a
  // pixel isn't painted again), so each transformed element (a message's
  // translateZ(0)) spends two frames untransformed, transitions off, and
  // its layer is made anew.
  const moved = [...document.querySelectorAll("*")].filter((e) => getComputedStyle(e).transform !== "none");
  if (moved.length) {
    const still = new CSSStyleSheet();
    still.replaceSync("*, *::before, *::after { transition: none !important; }");
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, still];
    const saved = moved.map((e) => [e, e.getAttribute("style")]);
    for (const [e] of saved) e.style.setProperty("transform", "none", "important");
    await c.frame();
    await c.frame();
    for (const [e, style] of saved) {
      if (style === null) e.removeAttribute("style");
      else e.setAttribute("style", style);
    }
    await c.frame();
    await c.frame();
    document.adoptedStyleSheets = document.adoptedStyleSheets.filter((s) => s !== still);
  }
  for (let round = 0; round < 10; round++) {
    const before = c.mutations();
    let changed = 0;
    for (const a of document.getAnimations()) {
      if (a.playState === "finished" || (a.playState === "paused" && a.currentTime === 0)) continue;
      changed++;
      try {
        if (Number.isFinite(a.effect?.getComputedTiming().endTime)) {
          a.finish();
        } else {
          a.pause();
          a.currentTime = 0;
        }
      } catch {}
    }
    for (const svg of document.querySelectorAll("svg")) {
      if (svg.animationsPaused && !svg.animationsPaused()) {
        svg.pauseAnimations();
        svg.setCurrentTime(0);
        changed++;
      }
    }
    for (const v of document.querySelectorAll("video")) {
      if (!v.paused) {
        v.pause();
        changed++;
      }
    }
    await Promise.all([...document.images].map((i) => Promise.race([
      i.decode().catch(() => {}), new Promise((r) => c.frame().then(c.frame).then(r))])));
    await document.fonts.ready;
    await c.frame();
    await c.frame();
    const s = await c.step(null, false);
    changed += s.fired + (c.mutations() !== before);
    if (!changed) return round;
  }
  return -1;
})()"""

# Paints over the masked selectors' boxes, as their own element at the end
# of the body: the number painted, or {error} for a selector that isn't one.
MASK_SCRIPT = r"""((masks) => {
  let n = 0;
  for (const [sel, text] of masks) {
    let found;
    try {
      found = document.querySelectorAll(sel);
    } catch (e) {
      return { error: `mask selector ${JSON.stringify(sel)}: ${e.message}` };
    }
    for (const el of found) {
      if (text != null) el.textContent = text;
      const r = el.getBoundingClientRect();
      if (!r.width || !r.height) continue;
      // Out to whole pixels, so no edge blends with what's under it.
      const left = Math.floor(r.left + scrollX), top = Math.floor(r.top + scrollY);
      const width = Math.ceil(r.right + scrollX) - left, height = Math.ceil(r.bottom + scrollY) - top;
      const box = document.createElement("div");
      box.style.cssText = `position: absolute; left: ${left}px; top: ${top}px; width: ${width}px;
        height: ${height}px; background: #ff00ff; z-index: 2147483647; pointer-events: none`;
      document.body.appendChild(box);
      n++;
    }
  }
  return { n };
})"""

# Diffs two PNGs (data URLs) pixel by pixel: changed where a channel moves
# by more than `tol`, or where only one image has the pixel. The diff image
# is the head dimmed to grey with changed pixels in magenta.
DIFF_SCRIPT = r"""(async (a, b, tol) => {
  const load = async (src) => {
    const img = new Image();
    img.src = src;
    await img.decode();
    const c = document.createElement("canvas");
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    const x = c.getContext("2d", { willReadFrequently: true });
    x.drawImage(img, 0, 0);
    return x.getImageData(0, 0, c.width, c.height);
  };
  const [A, B] = await Promise.all([load(a), load(b)]);
  const w = Math.max(A.width, B.width);
  const h = Math.max(A.height, B.height);
  const out = new ImageData(w, h);
  const o = out.data, pa = A.data, pb = B.data;
  let changed = 0, delta = 0;
  const box = [w, h, -1, -1];
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4;
      const inA = x < A.width && y < A.height;
      const inB = x < B.width && y < B.height;
      const ia = (y * A.width + x) * 4;
      const ib = (y * B.width + x) * 4;
      let diff = !(inA && inB);
      for (let k = 0; !diff && k < 4; k++) {
        const d = Math.abs(pa[ia + k] - pb[ib + k]);
        diff = d > tol;
        if (diff) delta = Math.max(delta, d);
      }
      if (diff) {
        changed++;
        box[0] = Math.min(box[0], x); box[1] = Math.min(box[1], y);
        box[2] = Math.max(box[2], x); box[3] = Math.max(box[3], y);
        o[i] = 255; o[i + 1] = 0; o[i + 2] = 170; o[i + 3] = 255;
      } else {
        const g = 0.299 * pb[ib] + 0.587 * pb[ib + 1] + 0.114 * pb[ib + 2];
        o[i] = o[i + 1] = o[i + 2] = 255 - (255 - g) * 0.3;
        o[i + 3] = 255;
      }
    }
  }
  let diff = null;
  if (changed) {
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    c.getContext("2d").putImageData(out, 0, 0);
    diff = c.toDataURL("image/png");
  }
  return { changed, width: w, height: h, base: [A.width, A.height], head: [B.width, B.height],
           box: changed ? [box[0], box[1], box[2] - box[0] + 1, box[3] - box[1] + 1] : null, delta, diff };
})"""


class Build:
    """A build of the storybook as compare mode serves it: `name` (head or
    base), its server's base URL, its commit, and how its stories load
    (`frames`: each in its frame document; `page`: in the storybook page,
    before the per-story frames)."""

    def __init__(self, name, url, commit, protocol):
        self.name, self.url, self.commit, self.protocol = name, url, commit, protocol
        self.index = None
        self.stories = {}

    def built(self):
        """When its build began, as its index says (None: it doesn't)."""
        b = (self.index or {}).get("build")
        return built_at(b) if b else None

    def viewport(self, device):
        return VIEWPORTS[device] if self.protocol == "frames" else PAGE_VIEWPORT

    def index_route(self):
        return "/storybook-index" if self.protocol == "frames" else "/storybook"

    def story_url(self, route, theme, device):
        query = urllib.parse.urlencode({k: v for k, v in (("theme", theme), ("device", device))
                                        if v not in ("dark", "pc")})
        path = (f"/storybook-frame/{route.removeprefix('/storybook/')}" if self.protocol == "frames"
                else route)
        return f"{self.url}/devtools.html#{path}" + (f"?{query}" if query else "")


def protocol_of(root):
    """`frames` when the source tree at `root` has the per-story frames'
    protocol module, else `page`."""
    return "frames" if (root / "src" / "devtools" / "storybook" / "host" / "protocol.ts").exists() else "page"


def git(*args, cwd=ROOT):
    res = subprocess.run(["git", *args], cwd=cwd, stdin=subprocess.DEVNULL, capture_output=True, text=True)
    if res.returncode:
        raise RuntimeError(f"git {' '.join(args)}: {res.stderr.strip()}")
    return res.stdout.strip()


def changed_since(ms, root=ROOT):
    """HEAD_INPUTS under `root` changed after `ms` (ms since the epoch):
    files written since, and directories an entry was added to, removed
    from or renamed in since."""
    limit = ms * 1_000_000
    out = []
    for name in HEAD_INPUTS:
        top = root / name
        paths = ((Path(d) / x for d, _, files in os.walk(top) for x in (".", *files)) if top.is_dir()
                 else [top])
        for p in paths:
            try:
                if p.lstat().st_mtime_ns > limit:
                    out.append(p)
            except OSError:
                pass
    return out


def built_at(build):
    """An index's build time (its `build`), as local time."""
    return time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(build["time"] / 1000))


def head_build(index):
    """(name, None) when the head build's index says it is the tree's,
    name being HEAD's commit (+dirty where HEAD_INPUTS have changes git
    sees, which the build holds); else (None, why not): it doesn't say, it
    is another commit's, or an input changed after it began."""
    head = git("rev-parse", "HEAD")
    b = index.get("build")
    if not b:
        return None, "the head build's index doesn't say what it was built from (a build from before it did)"
    if b["commit"] != head:
        ago = (time.time() * 1000 - b["time"]) / 60_000
        return None, (f"the head build is of {b['commit'][:12]} (built {built_at(b)}, {ago:.0f} min ago), "
                      f"but HEAD is {head[:12]}")
    newer = changed_since(b["time"])
    if newer:
        names = [str(p.relative_to(ROOT)) for p in newer[:3]]
        return None, (f"the head build (built {built_at(b)}) is older than changes to {len(newer)} of its inputs "
                      f"({', '.join(names)}{', ...' if len(newer) > 3 else ''})")
    dirty = bool(git("status", "--porcelain", "--", *HEAD_INPUTS))
    return head + ("+dirty" if dirty else ""), None


def drop_worktree(src):
    """Removes a base build's worktree, registered or not (`worktree add
    --force` takes a path still registered once its dir is gone)."""
    if src.exists():
        try:
            git("worktree", "remove", "--force", "--force", str(src))
        except RuntimeError:
            pass
        shutil.rmtree(src, ignore_errors=True)


def head_modules():
    """Why this checkout's node_modules isn't what its pnpm-lock.yaml
    installs, or None when it is: pnpm's own check (verify-deps-before-run,
    over the state its install left in node_modules), and the lockfile that
    install left (node_modules/.pnpm/lock.yaml) the same as pnpm-lock.yaml.
    pnpm's check reads the lockfile only while a package.json is newer than
    its last install or check, so it alone passes a lockfile that changed
    by itself."""
    lock = ROOT / "pnpm-lock.yaml"
    if not lock.is_file():
        return f"{ROOT} has no pnpm-lock.yaml"
    pnpm, env = node_tool("pnpm")
    res = subprocess.run([pnpm, "--config.verify-deps-before-run=error", "exec", "node", "-e", "0"], cwd=ROOT,
                         env=env, stdin=subprocess.DEVNULL, capture_output=True)
    try:
        installed = (ROOT / "node_modules" / ".pnpm" / "lock.yaml").read_bytes()
    except OSError:
        installed = None
    if res.returncode == 0 and installed == lock.read_bytes():
        return None
    return (f"{ROOT / 'node_modules'} isn't what {ROOT / 'pnpm-lock.yaml'} installs, so the head wouldn't "
            "build as the tree says: pnpm install --frozen-lockfile installs it")


def install(src, log):
    """Installs the base worktree `src`'s node_modules as its lockfile
    calls for (package_manager): (the tool's name, its executable and
    env, the lockfile's sha256)."""
    tool, lock = package_manager(src)
    exe, env = node_tool(tool)
    cmd = [exe, "install", "--frozen-lockfile"] if tool == "pnpm" else [exe, "ci", "--no-audit", "--no-fund"]
    start = time.monotonic()
    with open(log, "wb") as out:
        code = subprocess.run(cmd, cwd=src, env=env, stdin=subprocess.DEVNULL, stdout=out,
                              stderr=subprocess.STDOUT).returncode
    if code:
        sys.exit(f"check: the base's {tool} {cmd[1]} failed (its log: {log})")
    print(f"check: base: {tool} installed its {lock.name} in {time.monotonic() - start:.1f} s", file=sys.stderr)
    return tool, exe, env, hashlib.sha256(lock.read_bytes()).hexdigest()


def resolve(ref):
    """REF's commit, or the exit saying why there's none."""
    try:
        return git("rev-parse", "--verify", f"{ref}^{{commit}}")
    except RuntimeError as e:
        sys.exit(f"check: --compare {ref}: {e}")


def build_base(ref, rebuild):
    """REF's build: (sha, dist, protocol), built once per commit in a
    detached worktree under target/storybook/base/<sha>/src (its own
    node_modules, install's), its target/console/dist kept as
    target/storybook/base/<sha>/dist and the worktree removed. A cached
    build whose record has no lock_sha256 (from before the base's lockfile
    was recorded) is built again."""
    sha = resolve(ref)
    home = BASES / sha
    record = home / "build.json"
    home.mkdir(parents=True, exist_ok=True)
    with open(BASES / f"{sha}.lock", "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        rec = json.loads(record.read_text()) if record.is_file() else {}
        if rec.get("lock_sha256") and not rebuild:
            print(f"check: base {sha[:12]} ({ref}): cached build ({rec['protocol']} protocol)", file=sys.stderr)
            return sha, home / "dist", rec["protocol"]
        src, dist = home / "src", home / "dist"
        record.unlink(missing_ok=True)
        drop_worktree(src)
        shutil.rmtree(dist, ignore_errors=True)
        git("worktree", "add", "--force", "--detach", str(src), sha)
        try:
            start = time.monotonic()
            tool, exe, env, key = install(src, home / "install.log")
            protocol = protocol_of(src)
            print(f"check: building base {sha[:12]} ({ref}) in {src.relative_to(ROOT)}", file=sys.stderr)
            res = subprocess.run([exe, "run", "console:build"], cwd=src, env=env, stdin=subprocess.DEVNULL,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if res.returncode:
                (home / "build.log").write_text(res.stdout)
                print(res.stdout[-6000:], file=sys.stderr)
                sys.exit(f"check: the base's {tool} run console:build failed (its log: {home / 'build.log'})")
            shutil.move(src / "target" / "console" / "dist", dist)
            took = time.monotonic() - start
            record.write_text(json.dumps({"commit": sha, "ref": ref, "protocol": protocol, "build_s": round(took, 1),
                                          "lock_sha256": key, "installer": tool}) + "\n")
            print(f"check: built base {sha[:12]} in {took:.1f} s ({protocol} protocol)", file=sys.stderr)
        finally:
            drop_worktree(src)
    return sha, dist, protocol


def evaluate(page, expression, timeout=30):
    """Runtime.evaluate awaiting a promise: its value, or CDPError naming
    the exception."""
    r = page.call("Runtime.evaluate", {"expression": expression, "awaitPromise": True,
                                       "returnByValue": True}, timeout)
    if "exceptionDetails" in r:
        d = r["exceptionDetails"]
        raise CDPError(d.get("exception", {}).get("description") or d.get("text", "evaluation failed"))
    return r.get("result", {}).get("value")


def clock_step(page, limit, advance):
    """One step of the capture clock (CAPTURE_SCRIPT's step), or None while
    the page has no document with the clock in it yet."""
    try:
        return evaluate(page, f"window.__storybookCapture.step({json.dumps(limit)}, {json.dumps(advance)}, "
                              f"{NET_QUIET_S * 1000})")
    except CDPError:
        return None


class Stalled(TimeoutError):
    """A capture that still waited on a request (or on what its document
    has pending) at its --timeout: the checker's failure (its server's, or
    the browser's), not the story's."""


# The checker's own failures in a capture (the browser stalled or went, or
# a request was never answered): its story gets a fresh browser and one
# more try.
CHECKER_ERRORS = (CDPError, TimeoutError, ConnectionError, OSError, json.JSONDecodeError)


def waiting_on(page, pending):
    """What a capture waits on, the longest waited first: each request in
    flight, then each of the document's pending (`pending`, the step's:
    scripts, stylesheets, fetches, XHRs) with no request in flight, as text
    naming its URL."""
    t = time.monotonic()
    out, urls = [], set()
    for r in sorted(page.inflight.values(), key=lambda r: r["at"]):
        urls.add(r["url"])
        if r["answered"]:
            out.append(f"{r['url']}: its body unfinished after {t - r['at']:.0f} s")
        else:
            out.append(f"no answer for {r['url']} in {t - r['at']:.0f} s")
    for p in sorted(pending, key=lambda p: -p["ms"]):
        if p["url"] not in urls:
            out.append(f"{p['url']}: still pending in the page after {p['ms'] / 1000:.0f} s, no request in flight")
    return out


def capture(browser, build, route, theme, device, masks, args):
    """One story's screenshot in `build`, on the capture clock: the clock
    moves only while the network is quiet and the document has nothing
    pending, to the next timer until the story's root leaves "loading",
    then on to --settle ms after that, then (an empty root) by
    EMPTY_STEP_MS until it shows something or --timeout s of clock time
    pass. Then FINISH_SCRIPT holds what still moves, masks are painted, and
    the root is shot. {status, error, errors, png, time, rounds
    (FINISH_SCRIPT's), clock_ms, unstable (no two shots matched)}. Raises
    Stalled when --timeout s pass with a request in flight or anything
    pending."""
    page = browser.page
    start = time.monotonic()
    deadline = start + args.timeout
    out = {"status": "ok", "error": "", "errors": [], "png": None}

    def fail(error, errors=None):
        out.update(status="FAIL", error=first_line(error), errors=errors or [error],
                   time=time.monotonic() - start)
        return out

    browser.set_viewport(build.viewport(device))
    page.call("Storage.clearDataForOrigin", {"origin": build.url, "storageTypes": "all"})
    page.navigate(build.story_url(route, theme, device))
    phase, limit, mounted, now, pending = "load", None, None, 0, []
    while True:
        if time.monotonic() > deadline:
            waits = waiting_on(page, pending)
            if waits:
                more = f"; and {len(waits) - 1} more: " + "; ".join(waits[1:4]) if len(waits) > 1 else ""
                raise Stalled(f"{waits[0]} ({build.name} build, still waiting at --timeout {args.timeout:g} s; "
                              f"{phase}, clock at {now:.0f} ms){more}")
            return fail(f"timeout after {args.timeout} s ({phase}, clock at {now:.0f} ms)",
                        [f"timeout after {args.timeout} s ({phase})", *page.exceptions])
        page.drain()
        quiet = not page.inflight and time.monotonic() - page.net_at >= NET_QUIET_S
        r = clock_step(page, limit, quiet)
        if r is None:
            page.pump(0.05)
            continue
        pending = r["pending"]
        quiet = quiet and not r["waiting"]
        now = r["now"]
        if phase == "load" and r["state"] in ("ok", "error"):
            phase, mounted = "settle", now
            limit = mounted + args.settle
            continue
        idle = quiet and not r["fired"] and (r["next"] is None or r["next"] > (limit or 0))
        if phase != "load" and idle and now >= limit:
            probe = json.loads(page.eval(STORY_PROBE))
            if probe.get("state") == "error" or probe.get("visible") or probe.get("text"):
                break
            if now - mounted >= args.timeout * 1000:
                break
            phase, limit = "empty", limit + EMPTY_STEP_MS
            continue
        if not r["fired"] and (not quiet or (phase == "load" and r["next"] is None)):
            # Waiting on the network, or on the page's own work.
            page.pump(0.01)

    rounds = evaluate(page, FINISH_SCRIPT, timeout=max(5, deadline - time.monotonic()))
    probe = json.loads(page.eval(STORY_PROBE))
    errors = (probe.get("story") or {}).get("errors", [])
    failing = [f"[{e['kind']}] {e['text']}" for e in errors if not e.get("allowedBy")]
    if probe.get("state") == "error":
        fail(failing[0] if failing else "error state", failing)
    elif not probe.get("visible") and not probe.get("text"):
        fail("empty story root")
    out["rounds"] = rounds
    out["clock_ms"] = now
    if probe.get("root"):
        if masks:
            painted = page.eval(f"({MASK_SCRIPT})({json.dumps(masks)})")
            if painted.get("error"):
                return fail(painted["error"])
        r = probe["rect"]
        clip = {"x": max(0, r["x"]), "y": max(0, r["y"]), "width": max(1, r["width"]),
                "height": max(1, r["height"]), "scale": 1}
        vw, vh = browser.viewport
        beyond = clip["x"] + clip["width"] > vw or clip["y"] + clip["height"] > vh

        def shot():
            return base64.b64decode(page.call("Page.captureScreenshot", {
                "format": "png", "clip": clip, "captureBeyondViewport": beyond})["data"])

        # The raster can still change with nothing left to move (a late
        # decode): shots two frames apart until two match.
        out["png"] = shot()
        for _ in range(STABLE_SHOTS):
            evaluate(page, "window.__storybookCapture.frame().then(window.__storybookCapture.frame)")
            again = shot()
            if again == out["png"]:
                break
            out["png"] = again
        else:
            out["unstable"] = True
    out["time"] = time.monotonic() - start
    return out


def diff_images(browser, base_png, head_png, tolerance):
    """DIFF_SCRIPT over two PNGs, in `browser`'s page as the last capture
    left it: {changed, width, height, base, head (their sizes), diff (PNG
    bytes or None)}."""
    url = lambda png: "data:image/png;base64," + base64.b64encode(png).decode()
    r = evaluate(browser.page, f"({DIFF_SCRIPT})({json.dumps(url(base_png))}, {json.dumps(url(head_png))}, "
                               f"{tolerance})", timeout=120)
    if r["diff"]:
        r["diff"] = base64.b64decode(r["diff"].split(",", 1)[1])
    return r


def compare_story(browser, builds, mode, route, args):
    """A story in one mode: its capture in each build that has it, and their
    diff. {mode, route, kind (changed, unchanged, added, removed, failing,
    skipped), ...}."""
    theme, device = MODES[mode]
    entries = {b.name: b.stories.get(route) for b in builds}
    res = {"mode": mode, "route": route, "time": 0.0}
    visual = [e.get("visual") for e in entries.values() if e and e.get("visual")]
    skip = next((v for v in visual if v.get("skip")), None)
    if skip:
        res.update(kind="skipped", reason=skip.get("reason", ""))
        return res
    # [selector, text]: a host mask may name text to put in its element before
    # it's measured, so build-dependent text (the git hash) masks the same box
    # in both builds; an older base's entries lack it, so the head's wins.
    texts = {}
    for b in builds:
        for m in (b.index or {}).get("hostMask", []):
            if m.get("text") is not None or m["selector"] not in texts:
                texts[m["selector"]] = m.get("text")
    for v in visual:
        for s in v.get("mask", []):
            texts.setdefault(s, None)
    masks = sorted(texts.items())
    shots = {}
    for b in builds:
        if entries[b.name]:
            try:
                shots[b.name] = capture(browser, b, route, theme, device, masks, args)
            except CHECKER_ERRORS as e:
                e.side = b.name
                raise
            res["time"] += shots[b.name]["time"]
    res["shots"] = shots
    if "base" not in shots:
        res["kind"] = "added"
    elif "head" not in shots:
        res["kind"] = "removed"
    else:
        res["kind"] = "unchanged"
    if any(s["status"] != "ok" for s in shots.values()):
        res["kind"] = "failing"
    if "base" in shots and "head" in shots and shots["base"]["png"] and shots["head"]["png"]:
        a, b = shots["base"]["png"], shots["head"]["png"]
        if a == b:
            res["changed"], res["ratio"] = 0, 0.0
        else:
            d = diff_images(browser, a, b, args.tolerance)
            res.update(changed=d["changed"], ratio=d["changed"] / (d["width"] * d["height"]),
                       sizes=(d["base"], d["head"]), box=d["box"], delta=d["delta"], diff=d["diff"])
            if d["changed"] and res["kind"] == "unchanged":
                res["kind"] = "changed"
    return res


def slug(route):
    return route.removeprefix("/storybook/").replace("/", "__")


def write_report(path, results, builds, modes, args, elapsed):
    """index.html, report.json and the PNGs of every story that isn't
    unchanged, under `path`."""
    head, base = builds
    path.mkdir(parents=True, exist_ok=True)

    def save(r, side, png):
        if not png:
            return None
        name = f"{r['mode']}/{slug(r['route'])}.{side}.png"
        (path / r["mode"]).mkdir(exist_ok=True)
        (path / name).write_bytes(png)
        return urllib.parse.quote(name)

    esc = html.escape
    rows = []
    for r in results:
        if r["kind"] == "unchanged":
            continue
        shots = r.get("shots", {})
        imgs = {side: save(r, side, shots.get(side, {}).get("png")) for side in ("base", "head")}
        imgs["diff"] = save(r, "diff", r.get("diff"))
        rows.append((r, imgs))
    order = {"changed": 0, "failing": 1, "added": 2, "removed": 3, "skipped": 4}
    rows.sort(key=lambda x: (order[x[0]["kind"]], -x[0].get("ratio", 0), x[0]["mode"], x[0]["route"]))

    def counts(mode):
        c = {k: 0 for k in ("changed", "unchanged", "added", "removed", "failing", "skipped")}
        for r in results:
            if r["mode"] == mode:
                c[r["kind"]] += 1
        return c

    summary = "".join(
        f"<tr><td>{m}</td>" + "".join(f"<td class='n{' hot' if k != 'unchanged' and v else ''}'>{v}</td>"
                                      for k, v in counts(m).items()) + "</tr>"
        for m in modes)
    cards = []
    for r, imgs in rows:
        story = urllib.parse.unquote(r["route"].removeprefix("/storybook/"))
        if r["kind"] == "changed":
            (bw, bh), (hw, hh) = r["sizes"]
            size = "" if (bw, bh) == (hw, hh) else f" · size {bw}×{bh} → {hw}×{hh}"
            x, y, cw, ch = r["box"]
            what = (f"{100 * r['ratio']:.2f}% · {r['changed']:,} px in {cw}×{ch} at {x},{y}"
                    f" · channels up to {r['delta']}/255 apart{size}")
        elif r["kind"] == "skipped":
            what = f"not compared: {esc(r['reason'])}"
        else:
            what = r["kind"]
        notes = []
        for side, s in r.get("shots", {}).items():
            if s["status"] != "ok":
                notes.append(f"<pre><b>{side}:</b> {esc(chr(10).join(s['errors'][:3]))}</pre>")
            if s.get("rounds") == -1:
                notes.append(f"<p class='note'>{side}: still moving after 10 rounds</p>")
            if s.get("unstable"):
                notes.append(f"<p class='note'>{side}: no two shots in a row matched</p>")
        figs = "".join(
            f"<figure><figcaption>{k}</figcaption><a href='{imgs[k]}'><img loading='lazy' src='{imgs[k]}'></a>"
            f"</figure>" for k in ("base", "head", "diff") if imgs.get(k))
        cards.append(f"<section class='card {r['kind']}'><h3><span class='mode'>{r['mode']}</span> "
                     f"{esc(story)} <span class='what'>{what}</span></h3>"
                     f"{''.join(notes)}<div class='figs'>{figs}</div></section>")
    title = f"storybook: {head.commit[:12]} vs {base.commit[:12]}"
    built = lambda b: f", built {esc(b.built())}" if b.built() else ""  # noqa: E731
    logs = "".join(f" · <a href='{n}'>{n}</a>" for n in ("head-server.log", "base-server.log") if (path / n).exists())
    again = [r for r in results if r.get("retried")]
    retried = (f"<p class='note'>{len(again)} passed on a second try in a fresh browser, after the checker's own "
               "failure: " + "; ".join(f"{r['mode']} {esc(r['route'].removeprefix('/storybook/'))} "
                                      f"({esc(first_line(r['retried'][0], 200))})" for r in again) + "</p>"
               if again else "")
    doc = f"""<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{esc(title)}</title>
<style>
:root {{ --bg: #fff; --fg: #1b1f24; --muted: #5d6670; --line: #d8dde3; --card: #f6f8fa; --hot: #c0007a; }}
@media (prefers-color-scheme: dark) {{
  :root {{ --bg: #14171b; --fg: #e3e7eb; --muted: #98a2ad; --line: #2c3238; --card: #1b1f24; --hot: #ff5cbc; }}
}}
body {{ margin: 0; padding: 16px 24px; background: var(--bg); color: var(--fg);
  font: 14px/1.45 system-ui, sans-serif; }}
h1 {{ font-size: 20px; margin: 0 0 4px; }}
.meta {{ color: var(--muted); margin: 0 0 16px; }}
table {{ border-collapse: collapse; margin: 0 0 24px; }}
th, td {{ border-bottom: 1px solid var(--line); padding: 4px 12px; text-align: left; }}
td.n {{ text-align: right; font-variant-numeric: tabular-nums; }}
td.hot {{ color: var(--hot); font-weight: 600; }}
.card {{ background: var(--card); border: 1px solid var(--line); border-radius: 6px; padding: 8px 12px;
  margin: 0 0 16px; }}
.card h3 {{ font-size: 14px; margin: 0 0 8px; font-weight: 600; overflow-wrap: anywhere; }}
.mode {{ display: inline-block; border: 1px solid var(--line); border-radius: 4px; padding: 0 6px;
  font-weight: 400; color: var(--muted); }}
.what {{ color: var(--hot); font-weight: 400; margin-left: 8px; }}
.links {{ font-weight: 400; margin-left: 8px; }}
.links a {{ color: var(--muted); }}
.figs {{ display: flex; gap: 8px; align-items: flex-start; overflow-x: auto; }}
figure {{ margin: 0; flex: 1 1 0; min-width: 120px; max-width: 33%; }}
figcaption {{ color: var(--muted); font-size: 12px; }}
figure img {{ width: 100%; height: auto; border: 1px solid var(--line); display: block; }}
pre {{ white-space: pre-wrap; font-size: 12px; margin: 4px 0; }}
.note {{ color: var(--muted); margin: 4px 0; }}
</style></head><body>
<h1>{esc(title)}</h1>
<p class="meta">base {esc(base.commit)} ({esc(args.compare)}){built(base)} · head {esc(head.commit)}{built(head)}
 · tolerance {args.tolerance} per channel · settle {args.settle} ms · {elapsed:.0f} s{logs}</p>
{retried}
<table><tr><th>mode</th><th>changed</th><th>unchanged</th><th>added</th><th>removed</th><th>failing</th>
<th>skipped</th></tr>{summary}</table>
{''.join(cards) or '<p>No story changed.</p>'}
</body></html>
"""
    (path / "index.html").write_text(doc)
    (path / "report.json").write_text(json.dumps({
        "base": {"commit": base.commit, "ref": args.compare, "built": base.built()},
        "head": {"commit": head.commit, "built": head.built()},
        "tolerance": args.tolerance, "modes": modes, "elapsed_s": round(elapsed, 1),
        "stories": [{k: v for k, v in r.items() if k not in ("shots", "diff")}
                    | {"status": {s: v["status"] for s, v in r.get("shots", {}).items()},
                       "errors": {s: v["errors"][:3] for s, v in r.get("shots", {}).items() if v["errors"]},
                       "unstable": [s for s, v in r.get("shots", {}).items() if v.get("unstable")]}
                    for r in results]}, indent=1) + "\n")
    return path / "index.html"


def print_compare(results, builds, modes, report, elapsed, times, ref):
    kinds =("changed", "added", "removed", "failing", "skipped", "unchanged")
    print(f"{'mode':{MODE_WIDTH}}  {'stories':>7}  " + "  ".join(f"{k:>9}" for k in kinds) + "  done at")
    for m in modes:
        rs = [r for r in results if r["mode"] == m]
        print(f"{m:{MODE_WIDTH}}  {len(rs):7}  " + "  ".join(f"{sum(r['kind'] == k for r in rs):9}" for k in kinds)
              + f"  {times.get(m, 0):.0f} s")
    changed = sorted((r for r in results if r["kind"] == "changed"), key=lambda r: -r["ratio"])
    if changed:
        print("\nchanged, by the share of pixels:")
        width = max(len(r["route"]) for r in changed[:40]) - len("/storybook/")
        for r in changed[:40]:
            x, y, cw, ch = r["box"]
            print(f"  {r['mode']:{MODE_WIDTH}}  {r['route'].removeprefix('/storybook/').ljust(width)}  "
                  f"{100 * r['ratio']:6.2f}%  {r['changed']:>9,} px  in {cw}x{ch} at {x},{y}")
        if len(changed) > 40:
            print(f"  ... and {len(changed) - 40} more")
    for kind in ("added", "removed", "failing", "skipped"):
        rs = [r for r in results if r["kind"] == kind]
        if rs:
            print(f"\n{kind}:")
            for r in rs[:40]:
                detail = ""
                if kind == "failing":
                    detail = "; ".join(f"{s}: {v['error']}" for s, v in r["shots"].items() if v["status"] != "ok")
                elif kind == "skipped":
                    detail = r["reason"]
                print(f"  {r['mode']:{MODE_WIDTH}}  {r['route'].removeprefix('/storybook/')}  {detail}")
            if len(rs) > 40:
                print(f"  ... and {len(rs) - 40} more")
    again = [r for r in results if r.get("retried")]
    if again:
        print("\npassed on a second try, after the checker's own failure:")
        for r in again:
            print(f"  {r['mode']:{MODE_WIDTH}}  {r['route'].removeprefix('/storybook/')}  {first_line(r['retried'][0], 200)}")
    unstable = [(r, s) for r in results for s, v in r.get("shots", {}).items() if v.get("unstable")]
    if unstable:
        print("\nno two shots in a row matched:")
        for r, side in unstable:
            print(f"  {r['mode']:{MODE_WIDTH}}  {r['route'].removeprefix('/storybook/')}  ({side})")
    print(f"\n{len({r['route'] for r in results})} stories in {len(modes)} mode(s) compared in {elapsed:.0f} s")
    head, base = builds
    built = lambda b: f" (built {b.built()})" if b.built() else ""  # noqa: E731
    print(f"head {head.commit}{built(head)}\nbase {base.commit} ({ref}){built(base)}")
    print(f"report: {report}")


def compare_main(args, exe, work):
    """--compare: every story of either build in each mode, captured in both
    and diffed; the report; the exit code."""
    modes = list(ALL_MODES) if args.all_modes else [
        next(m for m, td in MODES.items() if td == (args.theme, args.device))]
    base_port = args.base_port or (args.port + 1 if args.port else None)
    if not base_port:
        sys.exit("check: --compare with --url needs --base-port")
    # The report's directory from the start: the servers log into it.
    report_dir = (Path(args.report) if args.report else REPORTS / (
        f"{time.strftime('%Y%m%dT%H%M%S')}-{git('rev-parse', 'HEAD')[:8]}-vs-{resolve(args.compare)[:8]}")).resolve()
    made = not report_dir.exists()
    report_dir.mkdir(parents=True, exist_ok=True)
    servers, browsers, ran = [], [], False
    try:
        if args.port:
            servers.append(start_server(args.port, report_dir, log_name="head-server.log"))
            head_url = servers[-1][2]
        else:
            head_url = args.url.rstrip("/").removesuffix("/devtools.html")
        new_browser = lambda i: Browser(exe, work, i, VIEWPORTS["pc"], COMPARE_FLAGS)
        browsers.append(new_browser(0))
        # The head first: one that isn't the tree's is refused before the
        # base builds.
        head = Build("head", head_url, None, protocol_of(ROOT))
        head.index = load_index(browsers[0], head.url, head.index_route())
        head.commit, why = head_build(head.index)
        if why:
            cure = "the tree changed while it built: run again" if args.build else "--build builds the tree as it is"
            if args.url:
                cure += f" (and does {args.url} serve {ROOT / 'target' / 'console' / 'dist'}?)"
            print(f"check: {why}: {cure}", file=sys.stderr)
            return 2
        sha, dist, base_protocol = build_base(args.compare, args.rebuild_base)
        servers.append(start_server(base_port, report_dir, dist=dist, log_name="base-server.log"))
        base = Build("base", servers[-1][2], sha, base_protocol)
        base.index = load_index(browsers[0], base.url, base.index_route())
        said = (base.index.get("build") or {}).get("commit")
        if said and said != sha:
            print(f"check: the base build in {dist} says it is of {said[:12]}, not {sha[:12]} ({args.compare}): "
                  "--rebuild-base builds it again", file=sys.stderr)
            return 2
        builds = (head, base)
        print(f"check: head {head.commit[:12]} (built {head.built()}) vs base {sha[:12]} ({args.compare}); "
              f"the report and the servers' logs in {report_dir}", file=sys.stderr)
        started, ran = time.monotonic(), True

        prefixes = [o if o.startswith("/") else f"/storybook/{o}" for o in args.only]
        for b in builds:
            b.stories = {s["route"]: s for s in b.index["stories"]
                         if not prefixes or any(s["route"].startswith(x) for x in prefixes)}
        routes = sorted(set(head.stories) | set(base.stories))
        if not routes:
            print(f"check: no story's route starts with {' or '.join(prefixes)}", file=sys.stderr)
            return 2
        tasks = [(m, r) for m in modes for r in routes]
        jobs = max(1, min(args.jobs, len(tasks)))
        with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
            browsers += list(pool.map(new_browser, range(1, jobs)))
        for b in browsers:
            b.enable_capture()
        todo = queue.Queue()
        for t in tasks:
            todo.put(t)
        results, lock = [], threading.Lock()
        mode_done = {}

        def worker(slot):
            while True:
                try:
                    mode, route = todo.get_nowait()
                except queue.Empty:
                    return
                # The checker's own failure (the browser stalled or went, a
                # request never answered) gets a fresh browser and one more
                # try; each is said as it happens, and a story that passes
                # on its second try keeps its first's in `retried`.
                retried = []
                for attempt in range(2):
                    try:
                        r = compare_story(browsers[slot], builds, mode, route, args)
                        if retried:
                            r["retried"] = retried
                        break
                    except CHECKER_ERRORS as e:
                        error = f"checker: {e}"
                        side = getattr(e, "side", "head")
                        r = {"mode": mode, "route": route, "kind": "failing", "time": 0.0, "checker": str(e),
                             "shots": {side: {"status": "FAIL", "error": error.splitlines()[0],
                                              "errors": [error], "png": None}}}
                        retried.append(f"{side}: {error}")
                        print(f"check: {mode} {route}: {side}: {error}"
                              + ("; trying again in a fresh browser" if not attempt else ""), file=sys.stderr)
                        browsers[slot].close()
                        try:
                            browsers[slot] = new_browser(slot)
                            browsers[slot].enable_capture()
                        except (RuntimeError, CDPError, TimeoutError, OSError) as e:
                            r["checker"] += f"; no fresh browser: {e}"
                            break
                with lock:
                    results.append(r)
                    mode_done[mode] = time.monotonic() - started
                    print(f"[{len(results)}/{len(tasks)}] {r['kind']:9} {r['time']:4.1f}s {mode:{MODE_WIDTH}} {route}",
                          file=sys.stderr)

        threads = [threading.Thread(target=worker, args=(i,), daemon=True) for i in range(jobs)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        elapsed = time.monotonic() - started
        results.sort(key=lambda r: (modes.index(r["mode"]), r["route"]))
        report = write_report(report_dir, results, builds, modes, args, elapsed)
        print_compare(results, builds, modes, report, elapsed, mode_done, args.compare)
        if any(r.get("checker") for r in results):
            return 2
        if args.fail_on_diff and any(r["kind"] not in ("unchanged", "skipped") for r in results):
            return 1
        return 0
    finally:
        for b in browsers:
            b.close()
        for proc, log, _ in servers:
            proc.terminate()
            try:
                proc.wait(15)
            except subprocess.TimeoutExpired:
                proc.kill()
            log.close()
        # A run refused before any story ran leaves no report.
        if made and not ran:
            shutil.rmtree(report_dir, ignore_errors=True)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    where = p.add_mutually_exclusive_group(required=True)
    where.add_argument("--port", type=int, help="start hack/console/server.py on this port")
    where.add_argument("--url", help="a running server's base URL")
    p.add_argument("--build", action="store_true", help="pnpm run console:build first")
    p.add_argument("-j", "--jobs", type=int, default=JOBS, help=f"browsers at once (default {JOBS})")
    p.add_argument("--only", action="append", default=[], metavar="PREFIX", help="stories whose route starts so")
    p.add_argument("--theme", choices=["dark", "light"], default="dark")
    p.add_argument("--device", choices=list(VIEWPORTS), default="pc")
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
    cmp = p.add_argument_group("compare mode")
    cmp.add_argument("--compare", metavar="REF", help="diff every story's screenshot against REF's build")
    cmp.add_argument("--base-port", type=int, metavar="N", help="the base's server (default --port + 1)")
    cmp.add_argument("--all-modes", action="store_true",
                     help="dark, light, portrait, landscape and the Android tablet's two in one run "
                          "(else --theme and --device's)")
    cmp.add_argument("--tolerance", type=int, default=2, metavar="N",
                     help="a channel's change that still counts as the same pixel (default 2 of 255)")
    cmp.add_argument("--report", metavar="DIR", help="the report's directory (default under target/storybook/compare)")
    cmp.add_argument("--fail-on-diff", action="store_true", help="exit 1 when any story differs")
    cmp.add_argument("--rebuild-base", action="store_true", help="build REF again even if cached")
    args = p.parse_args(argv)
    if args.compare and (args.list or args.screenshots or args.strict or args.files):
        p.error("--compare takes none of --list, --screenshots, --strict, --files")
    if args.compare and args.all_modes and (args.theme, args.device) != ("dark", "pc"):
        p.error("--all-modes takes no --theme or --device")
    if args.compare and (args.theme, args.device) not in MODES.values():
        p.error(f"--compare's modes are {', '.join(f'{t}/{d}' for t, d in MODES.values())}")

    exe = find_chromium(args.chromium)
    if args.compare and (why := head_modules()):
        print(f"check: {why}", file=sys.stderr)
        return 2
    if args.build:
        build()
    if args.screenshots:
        Path(args.screenshots).mkdir(parents=True, exist_ok=True)

    # The snap's Chromium sees only its own corner of the home directory.
    profiles_root = (Path.home() / "snap" / "chromium" / "common") if "/snap/" in exe else Path(tempfile.gettempdir())
    profiles_root.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="storybook-check-", dir=profiles_root))
    if args.compare:
        try:
            return compare_main(args, exe, work)
        finally:
            shutil.rmtree(work, ignore_errors=True)

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

        viewport = VIEWPORTS[args.device]
        browsers.append(Browser(exe, work, 0, viewport))
        index = load_index(browsers[0], base)
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
            browsers += list(pool.map(lambda i: Browser(exe, work, i, viewport), range(1, jobs)))
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
