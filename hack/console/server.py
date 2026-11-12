#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sim-results console: an HTTP API over the sims' results catalog
(hack/simlab), its schema and saved views, the commands that bring data
into it, and the console's page.

  python3 hack/console/server.py [--port 8086] [--host ADDR] [--connections 2]
                                 [--memory 8GB] [--threads 8] [--no-index] [--build]
                                 [--dist DIR] [--allow-host NAME]... [--allow-origin ORIGIN]...

It listens on every interface, IPv6's and IPv4's where the machine has
both (`--host 127.0.0.1` for this machine alone), and says its loopback
URL and one for each of the machine's other IPv4 addresses. `/` redirects
to the console's page, the devtools app's Console
(src/devtools/pages/Console.tsx) as `npm run console:build` builds it into
target/console/dist: until there is a build the page answers 503,
`--build` builds it first, and one older than its sources gets a warning;
`--dist DIR` serves another build of the page in its place (another
commit's, as hack/storybook/check.py --compare builds it). A file's body
is the file as opened, at its size then (a rebuild meanwhile ends the
connection rather than the body overrunning or falling short of its
Content-Length); a client gone mid-file and a file slower than SLOW_S
are said in the log. The dev
server proxies /console to it too (webpack.config.js). The catalog is
simlab.lake_root() (simdb.py's cache root: `SIM_DB_ROOT`, else the main
checkout's target/sim-db), read through a pool of simlab.engine's guarded
readers (simlab.catalog.pool); an index runs every 2 minutes
(`--no-index` for none), and the queries after one that succeeded see
the new catalog. So do those after an index by another process (suite.py
run, simdb.py index, a CLI fetch): a query, the schema and GET
/console/index read the catalog's latest DuckLake snapshot first
(simlab.catalog.snapshot) and refresh the readers once it isn't the one
they were opened at.

  POST /console/sql          {sql, params?, max_rows?, timeout_s?}: one SELECT (or EXPLAIN of
                             one), run as given, `$name`s bound from params; returns {columns:
                             [{name, type}], rows, truncated, elapsed_ms, snapshot} (the id of
                             the catalog's snapshot the server had when it ran it, or null)
  GET  /console/schema       the catalog's tables and columns, table_catalog, metric_catalog,
                             event_kinds, experiments, catalog_info and the saved views
  GET  /console/views        the saved views
  PUT  /console/views/NAME   {title, description, params, sql, spec}: saves a view
  POST /console/fetch        {exp_id, cells?, arms?, tables?}: what S3 still holds of the
                             experiment, into the cache, indexed; streams NDJSON lines {line}
                             (a progress event's also {progress: {kind, done, total}}), then
                             {done, code, s3_usd, elapsed_ms}
  POST /console/regenerate   {exp_id, cells, arms, seeds}: writes the suite's farm matrices for
                             them and returns farm.py estimate's quote under an id; starts
                             nothing
  POST /console/regenerate/ID/approve
                             starts farm.py run on that quote's matrices: the only way the
                             console starts a farm run
  GET  /console/regenerate/ID
                             a regeneration's state and its run's log
  POST /console/index        starts an index (or joins the one running) and returns it when
                             it's done; with `Accept: application/x-ndjson`, streams its
                             progress as fetch does, then {done, ...the run}
  GET  /console/index        the index running and the last one, catalog_info, and the catalog's
                             latest snapshot, {id, at} (or null): a page whose rows' snapshot
                             is older has new results to load
  POST /console/ask          {question, conversation_id?, task_budget?}: Claude answers the question
                             by writing and running queries over the catalog (simlab.ask), in a
                             new conversation or as a follow-up in one; streams NDJSON events
                             (start, thinking, text, tool_start, tool, sql, result, chart,
                             progress, proposal, tool_result, fallback, usage, refusal, error,
                             ping every 15 s, done last). The client going cancels it.
  GET  /console/ask/ID       a conversation's questions and their events
  GET  /console/spend        what the questions cost, per day and per question (?day=YYYY-MM-DD)
  PUT  /console/tabs/ID      {url, focused, visible}: a console tab's address and state, which it
                             reports on each change and every 30 s; kept in memory, the last
                             report of each tab, a tab unseen for 2 minutes forgotten
  GET  /console/tabs         the tabs' reports, [{id, url, focused, visible, last_seen,
                             focused_ago}] (seconds since its last report, and since it last
                             said it was focused, or null), the latest focused first: what
                             hack/console/ui.py reads (simlab.views.tabs)

While it runs, target/console/servers/PORT.json in the main checkout says
where ({pid, port, urls}: its loopback URL, then those for the network),
for hack/console/ui.py (simlab.views.server_states); it goes at exit.
Each server has its own, so servers on other ports run side by side.

A saved view is hack/console/views/NAME.sql (one statement, with `$exp`,
`$cell`, `$arm`, `$seed`, `$delay` and the like for the UI's pickers),
NAME.vl.json (its Vega-Lite spec, whose data are the statement's rows)
and NAME.json ({title, description, params: [{name, kind, default,
label}]}): simlab.views, read from disk at each request.

Index and fetch run in a child of a forkserver (a fresh interpreter,
holding none of this one's DuckDB instances, threads or locks), which
sends their progress and result over a pipe: a crash or a memory blow-up
there fails that run, not the server.

Every request's Host (any port) names this machine: localhost or a
loopback address, the address the request connected to (a LAN IP, an IPv6
one in brackets), one of the machine's IPv4 addresses, its hostname, its
FQDN or <hostname>.local, or a name --allow-host gives. Its Origin, when
it has one, is http or https on one of those, or one --allow-origin gives.
Anything else is 403: another domain name, so a page whose name resolves
to this machine (DNS rebinding) can't call the API. Every POST and PUT
takes `Content-Type: application/json`, which a page of another origin
can't send without a preflight the console never answers.
"""

import argparse
import contextlib
import dataclasses
import importlib
import ipaddress
import json
import math
import mimetypes
import multiprocessing
import os
import queue
import re
import signal
import socket
import struct
import subprocess
import sys
import threading
import time
import traceback
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent
# The checkout this server is in.
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "hack"))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import ask, catalog, engine, farm, regenerate, suite, views  # noqa: E402
from simlab.regenerate import NAME, bad_name  # noqa: E402

INDEX_EVERY_S = 120
MAX_BODY = 4 << 20
MAX_ROWS = 200_000
MAX_TIMEOUT_S = 600
# The names a request's Host and Origin may give, with any port, besides
# the machine's own (machine_names) and its addresses.
LOCAL = ("localhost", "127.0.0.1", "[::1]")
# Linux's ioctl for an interface's IPv4 address.
SIOCGIFADDR = 0x8915
DIST = ROOT / "target" / "console" / "dist"
PAGE = "/devtools.html#/console"
BUILD = ["npm", "run", "console:build"]
# What the build is made from: a build older than any of them is stale.
SOURCES = [ROOT / "src" / "devtools", ROOT / "package-lock.json", ROOT / "webpack.config.js"]
TYPES = {".js": "text/javascript", ".map": "application/json", ".wasm": "application/wasm"}
NDJSON = "application/x-ndjson"
# A file served slower than this is said in the log.
SLOW_S = 1.0
# An ask stream's keepalive, and its question's longest text.
PING_S = 15
MAX_QUESTION = 20_000
# The functions index and fetch run in their child, as module:function.
TASKS = {"index": "simlab.catalog:index", "fetch": "simlab.catalog:fetch"}
# The tabs' reports: how long one unseen is kept, how many are, a tab's
# id and its address's longest text.
TAB_TTL_S = 120
MAX_TABS = 256
TAB_ID = re.compile(r"[A-Za-z0-9_-]{1,64}")
MAX_URL = 1 << 20
FORKSERVER = multiprocessing.get_context("forkserver")


class HttpError(Exception):
    def __init__(self, status, message):
        super().__init__(message)
        self.status = status


def dumps(body):
    return json.dumps(body, ensure_ascii=False, separators=(",", ":"), allow_nan=False, default=str)


def plain(x):
    """`x` (a dataclass, or what JSON holds) as JSON's values."""
    if dataclasses.is_dataclass(x) and not isinstance(x, type):
        x = dataclasses.asdict(x)
    return json.loads(json.dumps(x, default=str))


def js_number(v, default):
    """A body's number as JavaScript's Number(v ?? default) makes it: NaN
    for what it makes none of."""
    if v is None:
        v = default
    if isinstance(v, bool):
        return float(v)
    if isinstance(v, (int, float)):
        return float(v)
    if isinstance(v, str):
        s = v.strip()
        if not s:
            return 0.0
        try:
            return float(int(s, 0)) if re.fullmatch(r"0[xob][0-9a-f]+", s, re.I) else float(s)
        except ValueError:
            return math.nan
    if isinstance(v, list) and len(v) <= 1:
        return js_number(v[0], default) if v else 0.0
    return math.nan


def safe_int(v):
    """Whether `v` is a JSON number JavaScript calls a safe integer."""
    return (not isinstance(v, bool) and isinstance(v, (int, float)) and float(v).is_integer()
            and abs(v) <= engine.SAFE)


def is_param(v):
    """A value `$name` binds: null, a string, number or boolean, or a list
    of strings and numbers."""
    if v is None or isinstance(v, (str, bool, int, float)):
        return True
    return isinstance(v, list) and all(isinstance(x, (str, int, float)) and not isinstance(x, bool) for x in v)


def strings(v, what):
    if v is None:
        return []
    if not isinstance(v, list) or not all(isinstance(x, str) for x in v):
        raise HttpError(400, f"{what}: a list of strings")
    return v


# --- index and fetch, in a forkserver's child -----------------------------------------------------


def child(target, kwargs, out):
    """Runs `target` (module:function) with `kwargs` and a progress
    callback, in a forkserver's child, sending ("progress", event) for
    each event and then ("result", value) or ("error", {message, trace})
    over the pipe `out`."""
    try:
        mod, _, name = target.partition(":")
        fn = getattr(importlib.import_module(mod), name)
        result = fn(**kwargs, progress=lambda p: out.send(("progress", plain(p))))
        out.send(("result", plain(result)))
    except BaseException as e:
        # A SystemExit's is the CLI's message, saying why.
        message = str(e) if isinstance(e, SystemExit) else f"{type(e).__name__}: {e}"
        out.send(("error", {"message": message, "trace": traceback.format_exc()}))
    finally:
        out.close()


def progress_event(p):
    """A Progress (catalog's {kind, message, done, total}) as a stream's
    line."""
    p = p if isinstance(p, dict) else {"message": str(p)}
    return {"line": str(p.get("message", "")),
            "progress": {"kind": p.get("kind"), "done": p.get("done", 0), "total": p.get("total", 0)}}


class Task:
    """A run of `target` (module:function, called with `kwargs` and its
    progress callback) in a forkserver's child: its events as they come,
    and its end. `code` is 0 once it returns, 1 if it raised, and the
    child's exit code (or minus the signal that killed it) if it died
    without a word. `on_done(task)` runs once it has ended, before anyone
    waiting on it wakes."""

    TAIL = 40

    def __init__(self, what, target, kwargs, on_done=None):
        self.what = what
        self.started_at = farm.now()
        self.finished_at = None
        self.code = None
        self.elapsed_ms = None
        self.result = None
        self.error = None
        self.events = []
        self.done = False
        self._t0 = time.monotonic()
        self._cond = threading.Condition()
        self._on_done = on_done
        reader, writer = FORKSERVER.Pipe(duplex=False)
        self._proc = FORKSERVER.Process(target=child, args=(target, kwargs, writer), name=f"console {what}")
        try:
            self._proc.start()
        except BaseException:
            reader.close()
            raise
        finally:
            writer.close()
        threading.Thread(target=self._read, args=(reader,), name=f"console {what}", daemon=True).start()

    def add(self, event):
        with self._cond:
            self.events.append(event)
            self._cond.notify_all()

    def _read(self, reader):
        said = None
        try:
            while True:
                try:
                    kind, value = reader.recv()
                except (EOFError, OSError):
                    break
                if kind == "progress":
                    self.add(progress_event(value))
                elif kind == "result":
                    said, self.result = kind, value
                elif kind == "error":
                    said, self.error = kind, value["message"]
                    print(f"console: {self.what} failed: {value['trace']}", file=sys.stderr)
        finally:
            reader.close()
        self._proc.join()
        exit = self._proc.exitcode
        if said == "result":
            self.code = 0
        elif said == "error":
            self.code = 1
        else:
            self.code = exit or 1
            self.error = (f"its process was killed by signal {-exit}" if exit and exit < 0
                          else f"its process exited with code {exit} before it finished")
        if self.error:
            self.add({"line": f"{self.what} failed: {self.error}"})
        self.finished_at = farm.now()
        self.elapsed_ms = round((time.monotonic() - self._t0) * 1000)
        if self._on_done:
            try:
                self._on_done(self)
            except Exception:
                traceback.print_exc()
        with self._cond:
            self.done = True
            self._cond.notify_all()

    def follow(self):
        """Its events, from the first, as they come, until it ends."""
        i = 0
        while True:
            with self._cond:
                self._cond.wait_for(lambda: len(self.events) > i or self.done)
                batch, done = self.events[i:], self.done
            i += len(batch)
            yield from batch
            if done:
                return

    def wait(self):
        with self._cond:
            self._cond.wait_for(lambda: self.done)
        return self

    def state(self):
        """The run as GET /console/index shows it: {started_at,
        finished_at, code, elapsed_ms, tail} (its last lines), and its
        error."""
        with self._cond:
            tail = [e["line"] for e in self.events[-self.TAIL:]]
        out = {"started_at": self.started_at, "finished_at": self.finished_at, "code": self.code,
               "elapsed_ms": self.elapsed_ms, "tail": tail}
        if self.error:
            out["error"] = self.error
        return out


class Indexer:
    """The catalog's index, one at a time: a start while one runs joins
    it. `on_indexed()` runs after one that succeeded."""

    def __init__(self, target, on_indexed):
        self.target = target
        self.on_indexed = on_indexed
        self.current = None
        self.last = None
        self._lock = threading.Lock()

    def start(self):
        with self._lock:
            if self.current is None:
                self.current = Task("index", self.target, {}, on_done=self._done)
            return self.current

    def _done(self, task):
        if task.code == 0:
            failed = (task.result or {}).get("failed") or {}
            if failed:
                task.add({"line": f"the verdicts of {len(failed)} sources failed (they wait for the next index): "
                                  + ", ".join(sorted(failed))})
            self.on_indexed()
        with self._lock:
            self.last, self.current = task, None


def followed(task, last):
    """`task`'s events, then last(task)."""
    yield from task.follow()
    yield last(task)


def fetched_lines(r):
    r = r or {}
    out = []
    if r.get("pulled") or r.get("gone"):
        out.append(f"{len(r.get('pulled') or [])} flushes pulled"
                   + (f", {len(r['gone'])} gone from S3 (regenerate them)" if r.get("gone") else ""))
    out.append(f"S3 charged ${r.get('s3_usd') or 0:.4f} ({r.get('ledger_lines') or 0} ledger lines)")
    return out


# --- the tabs' reports ----------------------------------------------------------------------------


class Tabs:
    """Each console tab's last report (PUT /console/tabs/ID), in memory: a
    tab unseen for `ttl` seconds is forgotten, and past MAX_TABS the
    longest unseen go. `clock` is monotonic seconds (tests give their
    own)."""

    def __init__(self, clock=time.monotonic, ttl=TAB_TTL_S):
        self.clock = clock
        self.ttl = ttl
        self._tabs = {}
        self._lock = threading.Lock()

    def report(self, id, body):
        """Keeps tab `id`'s report {url, focused, visible}; returns it as
        listed."""
        if not TAB_ID.fullmatch(id):
            raise HttpError(400, f"a tab's id is ^{TAB_ID.pattern}$")
        if not isinstance(body, dict) or not isinstance(body.get("url"), str) or not all(
                isinstance(body.get(k), bool) for k in ("focused", "visible")):
            raise HttpError(400, "{url: string, focused: boolean, visible: boolean}")
        if len(body["url"]) > MAX_URL:
            raise HttpError(413, f"a tab's url is at most {MAX_URL:,} characters")
        now = self.clock()
        with self._lock:
            old = self._tabs.pop(id, None)
            self._tabs[id] = {"url": body["url"], "focused": body["focused"], "visible": body["visible"], "seen": now,
                              "focused_at": now if body["focused"] else old and old["focused_at"]}
            self._forget(now)
            return self._listed(id, self._tabs[id], now)

    def list(self):
        """The tabs' reports, the latest focused first (a tab never
        focused after them, the latest seen first): [{id, url, focused,
        visible, last_seen, focused_ago}], last_seen the seconds since its
        report, focused_ago since its last focused one (None: never)."""
        now = self.clock()
        with self._lock:
            self._forget(now)
            order = sorted(self._tabs.items(), key=lambda kv: (kv[1]["focused_at"] is not None,
                                                               kv[1]["focused_at"] or 0, kv[1]["seen"]), reverse=True)
            return [self._listed(id, t, now) for id, t in order]

    def _forget(self, now):
        for id in [id for id, t in self._tabs.items() if now - t["seen"] > self.ttl]:
            del self._tabs[id]
        while len(self._tabs) > MAX_TABS:
            del self._tabs[min(self._tabs, key=lambda id: self._tabs[id]["seen"])]

    @staticmethod
    def _listed(id, t, now):
        focused = t["focused_at"]
        return {"id": id, "url": t["url"], "focused": t["focused"], "visible": t["visible"],
                "last_seen": round(now - t["seen"], 1),
                "focused_ago": None if focused is None else round(now - focused, 1)}


def write_state(path, port, urls):
    """Writes the server's state file at `path` ({pid, port, urls})."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(f".{path.name}.{os.getpid()}")
    tmp.write_text(json.dumps({"pid": os.getpid(), "port": port, "urls": urls}, indent=1) + "\n")
    os.replace(tmp, path)


def remove_state(path):
    """Removes the state file at `path` if it is this process's."""
    try:
        if json.loads(path.read_text()).get("pid") == os.getpid():
            path.unlink()
    except (OSError, ValueError, AttributeError):
        pass


# --- the console ----------------------------------------------------------------------------------


class Console:
    """The console's HTTP API over `pool` (an engine.Pool of the catalog's
    readers; the module docs), and the build at `dist`.

    `views_root` is the saved views' directory, `regenerations` the
    farm's quotes and runs (farm.Regenerations), `tasks` the functions
    index and fetch run in their child, and `catalog` and `suite` the
    simlab modules it calls (tests give stand-ins). `asker` answers the
    questions (an ask.Asker over the same pool, schema, views, farm and
    fetch by default; `ask_client` and `ledger` its Anthropic client and
    spend ledger). `index_every` is the index's period in seconds, 0 for
    none; `hosts` and `origins` add to the names and origins requests may
    give. `tabs` holds the tabs' reports."""

    def __init__(self, pool, *, views_root=views.VIEWS, regenerations=None, dist=DIST, index_every=0, hosts=(),
                 origins=(), tasks=TASKS, catalog=catalog, suite=suite, asker=None, ask_client=None, ledger=None):
        self.pool = pool
        self.tabs = Tabs()
        self.views_root = Path(views_root)
        self.regenerations = regenerations or farm.Regenerations()
        self.dist = Path(dist)
        self.catalog, self.suite = catalog, suite
        self.tasks = dict(tasks)
        self.hosts = {*LOCAL, *machine_names(), *lan_addresses(), *(h.lower() for h in hosts)}
        self.origins = {o.lower().rstrip("/") for o in origins}
        self.indexer = Indexer(self.tasks["index"], self.refresh)
        self._fetching = threading.Lock()
        self._state = threading.Lock()
        self._assembling = threading.Lock()
        self._schema = None
        self._epoch = 0
        # The catalog's snapshot the readers were opened at (catalog.snapshot's), and the
        # lock that takes one refresh for each it moves to.
        self._snapshot = None
        self._freshening = threading.RLock()
        self._stop = threading.Event()
        self.asker = asker or ask.Asker(pool, self.schema, client=ask_client, ledger=ledger,
                                        views_root=lambda: self.views_root, regenerations=self.regenerations,
                                        suite=suite, fetch=self.ask_fetch)
        if index_every:
            threading.Thread(target=self._tick, args=(index_every,), name="console index", daemon=True).start()

    def _tick(self, every):
        while not self._stop.wait(every):
            try:
                self.indexer.start()
            except Exception:
                traceback.print_exc()

    def refresh(self, snapshot=None):
        """New readers and schema for the queries after this, at the
        catalog's latest snapshot (`snapshot`, else read now), which it
        records: read before the readers go, so that the new ones open at
        it or later. A snapshot that can't be read keeps the last one."""
        with self._freshening:
            snapshot = snapshot or self.catalog.snapshot(self.pool.lake)
            self.pool.refresh()
            with self._state:
                self._schema = None
                self._epoch += 1
                if snapshot is not None:
                    self._snapshot = snapshot

    def fresh(self):
        """The catalog's snapshot the queries after this see ({id, at}, or
        None before one could be read): when the latest isn't the one the
        readers were opened at, an index by another process moved it
        (suite.py run, simdb.py index, a CLI fetch), and the readers and
        schema are refreshed first, once. The server's own indexes and
        fetches refresh them as they end."""
        latest = self.catalog.snapshot(self.pool.lake)
        with self._freshening:
            if latest is not None and (self._snapshot is None or latest["id"] != self._snapshot["id"]):
                self.refresh(latest)
            with self._state:
                return self._snapshot

    def close(self):
        self._stop.set()
        self.pool.close()

    def listen(self, port, host=None):
        """A server on `port` (0 for any) at `host`, by default every
        interface's address (IPv6's and IPv4's where the machine has both);
        not yet serving."""
        if host is None:
            dual = socket.has_dualstack_ipv6()
            server = Server(("::" if dual else "0.0.0.0", port), Handler, family=socket.AF_INET6 if dual else
                            socket.AF_INET, dual=dual)
        else:
            family = socket.getaddrinfo(host, port, type=socket.SOCK_STREAM)[0][0]
            server = Server((host, port), Handler, family=family)
        server.console = self
        return server

    # --- requests ---

    def handle(self, h):
        url = urllib.parse.urlsplit(h.path)
        route = f"{h.command} {url.path.rstrip('/')}"
        try:
            self.check_origin(h)
            if url.path == "/console" or url.path.startswith("/console/"):
                self.api(h, route, url)
            else:
                self.static(h, route, url.path)
        except Exception as e:
            if isinstance(e, (HttpError, engine.QueryError, farm.QuoteError)):
                status = e.status
            else:
                status = 500
                traceback.print_exc()
            if h.headers_sent:
                h.close_connection = True
            else:
                if not h.body_read and (h.headers.get("Content-Length") or "0") != "0":
                    # Its body would be read as the next request.
                    h.close_connection = True
                try:
                    self.send(h, status, {"error": str(e)})
                except OSError:
                    h.close_connection = True

    def check_origin(self, h):
        try:
            here = address_name(h.connection.getsockname()[0])
        except (AttributeError, OSError, ValueError):
            here = None
        if not self.ours(host_name(h.headers.get("Host") or ""), here):
            raise HttpError(403, f"Host {h.headers.get('Host')!r} isn't this machine's name (--allow-host NAME "
                                 "admits another)")
        origin = h.headers.get("Origin")
        if origin is None:
            return
        o = origin.strip().lower().rstrip("/")
        if o in self.origins:
            return
        u = urllib.parse.urlsplit(o)
        if u.scheme in ("http", "https") and not u.path and self.ours(host_name(u.netloc), here):
            return
        raise HttpError(403, f"Origin {origin!r} isn't this machine's (--allow-origin ORIGIN admits another)")

    def ours(self, name, here):
        """Whether `name` (host_name's) names this machine: one of `hosts`,
        `here` (the address the request connected to) or a loopback
        address."""
        if name is None:
            return False
        if name in self.hosts or name == here:
            return True
        try:
            return ipaddress.ip_address(name.removeprefix("[").removesuffix("]")).is_loopback
        except ValueError:
            return False

    def api(self, h, route, url):
        if route == "POST /console/sql":
            return self.send(h, 200, self.sql(self.body(h)))
        if route == "GET /console/schema":
            # The views are read each time: they're files anyone may edit.
            return self.send(h, 200, {**self.schema(), "views": self.saved_views()})
        if route == "GET /console/views":
            return self.send(h, 200, self.saved_views())
        if m := re.fullmatch(r"PUT /console/views/([^/]+)", route):
            try:
                name = urllib.parse.unquote(m[1], errors="strict")
            except UnicodeDecodeError:
                raise HttpError(400, f"a view's name is ^{views.NAME.pattern}$") from None
            return self.send(h, 200, self.save_view(name, self.body(h)))
        if route == "POST /console/fetch":
            return self.fetch(h, self.body(h))
        if route == "POST /console/regenerate":
            return self.send(h, 200, self.regenerate(self.body(h)))
        if m := re.fullmatch(r"POST /console/regenerate/([\w-]+)/approve", route, re.A):
            self.body(h)
            return self.send(h, 200, self.approve(m[1]))
        if m := re.fullmatch(r"GET /console/regenerate/([\w-]+)", route, re.A):
            r = self.regenerations.get(m[1])
            if r is None:
                raise HttpError(404, "no such regeneration")
            return self.send(h, 200, r)
        if route == "POST /console/index":
            self.body(h)
            task = self.indexer.start()
            if NDJSON in (h.headers.get("Accept") or ""):
                return self.stream(h, followed(task, lambda t: {"done": True, **t.state()}))
            return self.send(h, 200, task.wait().state())
        if route == "GET /console/index":
            return self.send(h, 200, self.index_state())
        if route == "POST /console/ask":
            return self.ask(h, self.body(h))
        if m := re.fullmatch(r"GET /console/ask/([\w-]+)", route, re.A):
            c = self.asker.conversation(m[1])
            if c is None:
                raise HttpError(404, "no such conversation")
            return self.send(h, 200, c)
        if route == "GET /console/spend":
            day = urllib.parse.parse_qs(url.query).get("day", [None])[0]
            return self.send(h, 200, ask.spend(self.asker.ledger.path, day))
        if m := re.fullmatch(r"PUT /console/tabs/([^/]+)", route):
            return self.send(h, 200, self.tabs.report(urllib.parse.unquote(m[1]), self.body(h)))
        if route == "GET /console/tabs":
            return self.send(h, 200, self.tabs.list())
        raise HttpError(404, f"no {route}")

    def body(self, h):
        """The request's JSON body: 415 unless it says it is JSON, 413 past
        MAX_BODY."""
        if not re.match(r"application/json\b", h.headers.get("Content-Type") or ""):
            h.close_connection = True
            raise HttpError(415, "the body must be application/json")
        if h.headers.get("Transfer-Encoding"):
            h.close_connection = True
            raise HttpError(411, "a body needs its Content-Length")
        try:
            n = int(h.headers.get("Content-Length") or 0)
        except ValueError:
            h.close_connection = True
            raise HttpError(400, "Content-Length isn't a number") from None
        if n > MAX_BODY:
            h.close_connection = True
            raise HttpError(413, "body too large")
        data = h.rfile.read(n) if n > 0 else b""
        h.body_read = True
        try:
            return json.loads(data.decode("utf-8") or "{}", parse_constant=no_constant)
        except ValueError:
            raise HttpError(400, "the body isn't JSON") from None

    def send(self, h, status, body):
        data = dumps(body).encode("utf-8")
        h.send_response(status)
        h.send_header("Content-Type", "application/json; charset=utf-8")
        h.send_header("Cache-Control", "no-store")
        h.send_header("X-Content-Type-Options", "nosniff")
        h.send_header("Content-Length", str(len(data)))
        if h.close_connection:
            h.send_header("Connection", "close")
        h.end_headers()
        if h.command != "HEAD":
            h.wfile.write(data)

    def stream(self, h, events, on_gone=None):
        """`events` as NDJSON lines, until the client goes (a task they
        follow runs on unwatched; `on_gone()` runs then)."""
        h.send_response(200)
        h.send_header("Content-Type", f"{NDJSON}; charset=utf-8")
        h.send_header("Cache-Control", "no-store")
        h.send_header("X-Content-Type-Options", "nosniff")
        h.send_header("Connection", "close")
        h.end_headers()
        h.close_connection = True
        write = lambda event: h.wfile.write((dumps(event) + "\n").encode("utf-8"))  # noqa: E731
        try:
            for event in events:
                write(event)
        except OSError:
            if on_gone:
                on_gone()
            return
        except Exception as e:
            traceback.print_exc()
            with contextlib.suppress(OSError):
                write({"done": True, "code": None, "error": str(e)})

    # --- sql, schema and views ---

    def sql(self, body):
        if not isinstance(body, dict) or not isinstance(body.get("sql"), str):
            raise HttpError(400, "{sql: string}")
        params = body.get("params")
        params = {} if params is None else params
        if not isinstance(params, dict) or not all(is_param(v) for v in params.values()):
            raise HttpError(400, "params: an object of strings, numbers, booleans or lists")
        max_rows = js_number(body.get("max_rows"), 10_000)
        timeout = js_number(body.get("timeout_s"), 30)
        if not math.isfinite(max_rows) or not math.isfinite(timeout):
            raise HttpError(400, "max_rows and timeout_s: numbers")
        max_rows = min(MAX_ROWS, max(0, math.floor(max_rows)))
        timeout = min(MAX_TIMEOUT_S, max(0.1, timeout))
        snapshot = self.fresh()
        out = self.pool.query(body["sql"], params, max_rows=max_rows, timeout_s=timeout, temporal_text=True).json()
        return {**out, "snapshot": snapshot and snapshot["id"]}

    def rows(self, sql, params=None):
        """`sql`'s rows as dicts of the API's JSON values."""
        j = self.pool.query(sql, params, max_rows=MAX_ROWS, timeout_s=60).json()
        names = [c["name"] for c in j["columns"]]
        return [dict(zip(names, r)) for r in j["rows"]]

    def schema(self):
        """catalog.schema's, kept until a refresh (fresh's among them)."""
        self.fresh()
        with self._assembling:
            with self._state:
                schema, epoch = self._schema, self._epoch
            if schema is not None:
                return schema
            schema = self.catalog.schema(self.pool)
            with self._state:
                if self._epoch == epoch:
                    self._schema = schema
            return schema

    def saved_views(self):
        return [v.json() for v in views.saved(self.views_root)]

    def save_view(self, name, body):
        if not views.NAME.fullmatch(name):
            raise HttpError(400, f"a view's name is ^{views.NAME.pattern}$")
        if not isinstance(body, dict) or not isinstance(body.get("sql"), str) or not body["sql"].strip():
            raise HttpError(400, "{sql, spec?, title?, description?, params?}")
        text = lambda k, default: body[k] if isinstance(body.get(k), str) else default  # noqa: E731
        view = views.View(name=name, title=text("title", name), description=text("description", ""),
                          params=body["params"] if isinstance(body.get("params"), list) else [],
                          sql=body["sql"], spec=body.get("spec"))
        try:
            return views.save(name, view, self.views_root).json()
        except ValueError as e:
            raise HttpError(400, str(e)) from None

    # --- commands ---

    def fetch(self, h, body):
        if not isinstance(body, dict) or not isinstance(body.get("exp_id"), str):
            raise HttpError(400, "{exp_id}")
        exp = body["exp_id"]
        cells, arms, tables = (strings(body.get(k), k) for k in ("cells", "arms", "tables"))
        bad = bad_name([exp, *cells, *arms, *tables])
        if bad is not None:
            raise HttpError(400, f"not a name: {json.dumps(bad)}")
        command = ["$ simdb.py fetch", "--exp", exp]
        for flag, names in (("--cells", cells), ("--arms", arms), ("--tables", tables)):
            if names:
                command += [flag, ",".join(names)]
        kwargs = {"exp": exp, "cells": cells or None, "arms": arms or None, "tables": tables}
        out = {}

        def events():
            # One fetch at a time: each indexes what it fetched. The next
            # starts once this one's child ends, whether or not its
            # client stayed.
            self._fetching.acquire()
            try:
                task = Task("fetch", self.tasks["fetch"], kwargs, on_done=self._fetched)
            except BaseException:
                self._fetching.release()
                raise
            yield {"line": " ".join(command)}
            yield from task.follow()
            if task.code == 0:
                yield from ({"line": line} for line in fetched_lines(task.result))
            out.update(done=True, code=task.code, s3_usd=(task.result or {}).get("s3_usd"),
                       elapsed_ms=task.elapsed_ms, **({"error": task.error} if task.error else {}))
            yield out
        self.stream(h, events())

    def _fetched(self, task):
        try:
            if task.code == 0:
                self.refresh()
        finally:
            self._fetching.release()

    def regenerate(self, body):
        if not isinstance(body, dict) or not isinstance(body.get("exp_id"), str) or not NAME.fullmatch(body["exp_id"]):
            raise HttpError(400, "{exp_id, cells, arms, seeds}")
        seeds = body.get("seeds")
        if not isinstance(seeds, list) or not all(safe_int(s) for s in seeds):
            raise HttpError(400, "seeds: a list of integers")
        return regenerate.regenerate(self.pool, body["exp_id"], strings(body.get("cells"), "cells"),
                                     strings(body.get("arms"), "arms"), [int(s) for s in seeds], self.regenerations,
                                     suite=self.suite)

    def approve(self, id):
        farm.approve(id, self.regenerations)
        return self.regenerations.get(id)

    def ask(self, h, body):
        if not isinstance(body, dict) or not isinstance(body.get("question"), str) or not body["question"].strip():
            raise HttpError(400, "{question, conversation_id?, task_budget?}")
        question = body["question"].strip()
        if len(question) > MAX_QUESTION:
            raise HttpError(413, f"a question is at most {MAX_QUESTION:,} characters")
        cid = body.get("conversation_id")
        if cid is not None and not isinstance(cid, str):
            raise HttpError(400, "conversation_id: a string")
        if cid is not None and self.asker.conversation(cid) is None:
            raise HttpError(404, "no such conversation")
        if cid is not None and self.asker.busy(cid):
            raise HttpError(409, "the conversation is answering a question")
        budget = body.get("task_budget", ask.TASK_BUDGET)
        if not safe_int(budget) or not ask.MIN_TASK_BUDGET <= budget <= ask.MAX_TASK_BUDGET:
            raise HttpError(400, f"task_budget: an integer from {ask.MIN_TASK_BUDGET:,} to {ask.MAX_TASK_BUDGET:,}")
        events, cancel = queue.Queue(), threading.Event()

        def work():
            try:
                self.asker.ask(question, cid, int(budget), emit=events.put, cancel=cancel)
            except ask.Busy:
                events.put({"type": "error", "message": "the conversation is answering a question"})
                events.put({"type": "done", "stop_reason": "error"})
            except Exception as e:
                traceback.print_exc()
                events.put({"type": "error", "message": f"{type(e).__name__}: {e}"})
                events.put({"type": "done", "stop_reason": "error"})
            finally:
                events.put(None)

        def lines():
            while True:
                try:
                    event = events.get(timeout=PING_S)
                except queue.Empty:
                    yield {"type": "ping"}
                    continue
                if event is None:
                    return
                yield event

        threading.Thread(target=work, name="console ask", daemon=True).start()
        self.stream(h, lines(), on_gone=cancel.set)

    def ask_fetch(self, exp, cells=None, arms=None, tables=(), progress=None):
        """fetch_data's fetch: catalog.fetch in its child, as POST
        /console/fetch runs it (one at a time, the readers refreshed after
        one that succeeded), its lines passed to `progress`."""
        self._fetching.acquire()
        try:
            task = Task("fetch", self.tasks["fetch"], {"exp": exp, "cells": cells, "arms": arms,
                                                       "tables": list(tables)}, on_done=self._fetched)
        except BaseException:
            self._fetching.release()
            raise
        for event in task.follow():
            if progress and event.get("line"):
                progress(event["line"])
        if task.code != 0:
            raise RuntimeError(f"the fetch failed: {task.error}")
        return task.result

    def index_state(self):
        snapshot = self.fresh()
        try:
            info = self.rows("SELECT * FROM catalog_info")
        except engine.QueryError:
            info = []
        current, last = self.indexer.current, self.indexer.last
        return {"running": current and current.state(), "last": last and last.state(), "catalog_info": info,
                "snapshot": snapshot}

    # --- the build ---

    def static(self, h, route, path):
        if h.command not in ("GET", "HEAD"):
            raise HttpError(404, f"no {route}")
        if not (self.dist / "devtools.html").is_file():
            raise HttpError(503, f"no console build in {self.dist}: run {' '.join(BUILD)}")
        if path == "/":
            h.send_response(302)
            h.send_header("Location", PAGE)
            h.send_header("Content-Length", "0")
            h.end_headers()
            return
        dist = self.dist.resolve()
        try:
            p = (dist / urllib.parse.unquote(path, errors="strict").lstrip("/")).resolve()
        except (UnicodeDecodeError, ValueError, OSError):
            p = None
        if p is None or not p.is_relative_to(dist) or not p.is_file():
            raise HttpError(404, f"no {route}")
        try:
            f = p.open("rb")
        except OSError:
            raise HttpError(404, f"no {route}") from None
        with f:
            # The body is the file opened, its size then: a build that
            # replaces, grows or truncates it meanwhile can't make the body
            # longer than its Content-Length (the next response's start) or
            # leave it short with the connection open (the client waiting).
            size = os.fstat(f.fileno()).st_size
            t0 = time.monotonic()
            h.send_response(200)
            h.send_header("Content-Type", TYPES.get(p.suffix) or mimetypes.guess_type(p.name)[0]
                          or "application/octet-stream")
            h.send_header("Content-Length", str(size))
            h.send_header("Cache-Control", "no-cache")
            h.send_header("X-Content-Type-Options", "nosniff")
            h.end_headers()
            if h.command == "HEAD":
                return
            left = size
            try:
                while left:
                    chunk = f.read(min(left, 1 << 16))
                    if not chunk:
                        h.close_connection = True
                        log(f"{route}: the file was {size:,} bytes when opened and ended at {size - left:,}: "
                            "closed the connection")
                        return
                    h.wfile.write(chunk)
                    left -= len(chunk)
            except ConnectionError as e:
                # The client went (a page navigated away mid-response).
                h.close_connection = True
                log(f"{route}: the client went after {size - left:,} of {size:,} bytes ({type(e).__name__})")
                return
            took = time.monotonic() - t0
            if took > SLOW_S:
                log(f"{route}: {size:,} bytes took {took:.1f} s")


def no_constant(name):
    raise ValueError(f"{name} isn't JSON")


def log(line):
    sys.stderr.write(f"console: {line}\n")


def host_name(value):
    """A Host header's (or an origin's) name, without its port: lower case,
    an IPv6 address in its brackets; None for one that isn't."""
    v = value.strip().lower()
    if v.startswith("["):
        end = v.find("]")
        name, rest = v[:end + 1], v[end + 1:]
        if end < 0:
            return None
    else:
        name, _, port = v.partition(":")
        rest = ":" + port if _ else ""
    if rest and not re.fullmatch(r":[0-9]*", rest):
        return None
    return name or None


def address_name(addr):
    """An IP address as a Host header names it: an IPv4 one (an IPv4-mapped
    IPv6 one too) as itself, an IPv6 one in brackets, its zone left out."""
    ip = ipaddress.ip_address(addr.split("%", 1)[0])
    if ip.version == 6 and ip.ipv4_mapped:
        ip = ip.ipv4_mapped
    return str(ip) if ip.version == 4 else f"[{ip}]"


def machine_names():
    """This machine's names, lower case: its hostname, its FQDN and
    <hostname>.local (mDNS's)."""
    host = socket.gethostname().lower()
    return {host, socket.getfqdn().lower(), f"{host.split('.')[0]}.local"}


def lan_addresses():
    """This machine's IPv4 addresses but loopback's: the one its default
    route leaves from first, then each interface's (Linux's SIOCGIFADDR)."""
    out = []
    with contextlib.suppress(OSError), socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
        # Connecting a UDP socket sends nothing: it picks the route.
        s.connect(("192.0.2.1", 9))
        out.append(s.getsockname()[0])
    try:
        import fcntl
    except ImportError:
        fcntl = None
    if fcntl is not None:
        with contextlib.suppress(OSError):
            for _, name in socket.if_nameindex():
                with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
                    try:
                        got = fcntl.ioctl(s.fileno(), SIOCGIFADDR, struct.pack("256s", name.encode()[:15]))
                    except OSError:
                        continue
                out.append(socket.inet_ntoa(got[20:24]))
    return [a for a in dict.fromkeys(out) if not ipaddress.ip_address(a).is_loopback]


class Server(ThreadingHTTPServer):
    """The console's listener: `family`'s, and with `dual` (IPv6's) taking
    IPv4's connections too."""
    daemon_threads = True
    request_queue_size = 64

    def __init__(self, address, handler, family=socket.AF_INET, dual=False):
        self.address_family = family
        self.dual = dual
        super().__init__(address, handler)

    def server_bind(self):
        if self.dual:
            self.socket.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
        super().server_bind()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "sim-console"
    # A response's headers and body are two writes: with Nagle's algorithm
    # the body waits for the headers' ACK, which a client delays (40 ms on
    # Linux) on a kept-alive connection.
    disable_nagle_algorithm = True

    def __init__(self, *args, **kwargs):
        self.headers_sent = self.body_read = False
        super().__init__(*args, **kwargs)

    def end_headers(self):
        self.headers_sent = True
        super().end_headers()

    def do_GET(self):
        self.headers_sent = self.body_read = False
        self.server.console.handle(self)

    do_HEAD = do_POST = do_PUT = do_DELETE = do_PATCH = do_OPTIONS = do_GET

    def log_message(self, format, *args):
        pass

    def log_error(self, format, *args):
        log(format % args)


def stale(dist=DIST, sources=SOURCES):
    """The sources newer than the build at `dist` (none when it is
    missing)."""
    page = dist / "devtools.html"
    if not page.is_file():
        return []
    built = page.stat().st_mtime
    out = []
    for s in sources:
        files = (p for p in s.rglob("*") if p.is_file()) if s.is_dir() else [s] if s.exists() else []
        if any(p.stat().st_mtime > built for p in files):
            out.append(s)
    return out


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--port", type=int, default=8086)
    p.add_argument("--host", metavar="ADDR", help="the address to listen on (default: every interface's)")
    p.add_argument("--connections", type=int, default=2, help="queries that run at once")
    p.add_argument("--memory", default="8GB", help="the readers' memory limit")
    p.add_argument("--threads", type=int, default=8, help="the readers' threads")
    p.add_argument("--no-index", action="store_true", help="no index every 2 minutes")
    p.add_argument("--build", action="store_true", help=f"{' '.join(BUILD)} first")
    p.add_argument("--dist", type=Path, default=DIST, metavar="DIR",
                   help="the page's build to serve (default this checkout's target/console/dist)")
    p.add_argument("--allow-host", action="append", default=[], metavar="NAME",
                   help="a Host name requests may give besides this machine's")
    p.add_argument("--allow-origin", action="append", default=[], metavar="ORIGIN",
                   help="an Origin requests may give besides this machine's (scheme://name[:port])")
    a = p.parse_args(argv)
    if a.build and subprocess.run(BUILD, cwd=ROOT).returncode:
        sys.exit(f"console: {' '.join(BUILD)} failed")
    if not (a.dist / "devtools.html").is_file():
        print(f"console: no build in {a.dist}: run {' '.join(BUILD)} (or --build) for the page", file=sys.stderr)
    elif a.dist == DIST and (old := stale()):
        print(f"console: the build in {DIST} is older than {', '.join(str(s.relative_to(ROOT)) for s in old)}: "
              f"run {' '.join(BUILD)} (or --build)", file=sys.stderr)
    pool = catalog.pool("console", a.connections, engine.Guard(memory_limit=a.memory, threads=a.threads))
    lake = pool.lake
    console = Console(pool, dist=a.dist, index_every=0 if a.no_index else INDEX_EVERY_S, hosts=a.allow_host,
                      origins=a.allow_origin)
    server = console.listen(a.port, a.host)
    port = server.server_address[1]
    if a.host is None:
        urls = [f"http://127.0.0.1:{port}/"]
        print(f"console: {urls[0]} (catalog {lake})", file=sys.stderr)
        lan = [f"http://{addr}:{port}/" for addr in lan_addresses()]
        if lan:
            print(f"console: on the network: {', '.join(lan)}", file=sys.stderr)
        urls += lan
    else:
        urls = [f"http://{address_name(server.server_address[0])}:{port}/"]
        print(f"console: {urls[0]} (catalog {lake})", file=sys.stderr)
    state = views.state_path(port)
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    try:
        write_state(state, port, urls)
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        remove_state(state)
        server.server_close()
        console.close()
        if console.indexer.current or console._fetching.locked():
            print("console: waiting for the index or fetch running", file=sys.stderr)


if __name__ == "__main__":
    main()
