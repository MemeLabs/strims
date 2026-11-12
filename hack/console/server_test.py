#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The console server's tests, over a small catalog of their own (a
DuckLake written as simlab.catalog writes one), simlab's schema, views
and farm, and stand-ins for index and fetch in their forkserver child,
the suite's matrices, farm.py and the Anthropic client (test_ask's):

  python3 hack/console/server_test.py
"""

import dataclasses
import http.client
import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock
from pathlib import Path

HERE = Path(__file__).resolve().parent
HACK = HERE.parent
sys.path.insert(0, str(HACK))
sys.path.insert(0, str(HERE))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import server  # noqa: E402
from simlab import ask, catalog, engine, farm, suite  # noqa: E402

sys.path.insert(0, str(HACK / "simlab" / "tests"))
import test_ask as fakes  # noqa: E402

TESTDATA = HACK / "simlab" / "tests" / "testdata"
# The API's golden query: a column of each DuckDB type, and the columns
# and rows it answers.
TYPES_SQL = (TESTDATA / "json_types.sql").read_text()
API = json.loads((TESTDATA / "json_types.json").read_text())
# The stand-ins in the child find the test's directory here.
DIR_ENV = "CONSOLE_TEST_DIR"


def make_lake(lake):
    """A catalog with the tables the server reads, and `nums` (100,000
    rows)."""
    (lake / "data").mkdir(parents=True)
    w = engine.writer(lake)
    try:
        w.execute("""
CREATE SCHEMA raw;
CREATE TABLE experiments AS SELECT 'e1' AS exp_id, 'farm' AS kind,
  TIMESTAMPTZ '2026-11-10 10:00:00+00' AS started_at, ['robust'] AS tiers, NULL::MAP(VARCHAR, VARCHAR) AS knob,
  NULL::VARCHAR[] AS sets, MAP {'base': 'aaaa', 'new': 'bbbb'} AS arms, ['base', 'new'] AS arm_labels,
  'base' AS base, 'does it?' AS question, 2::BIGINT AS cells, 8::BIGINT AS jobs;
CREATE TABLE experiment_cells AS SELECT * FROM (VALUES
  ('e1', 'calm', MAP {'SIM_VIEWERS': '8'}), ('e1', 'storm', MAP {'SIM_VIEWERS': '16'})) t(exp_id, cell, env);
CREATE TABLE raw.tier_cells AS SELECT * FROM (VALUES
  ('robust', 'calm', 'ppspp', MAP {'SIM_VIEWERS': '8'}), ('robust', 'storm', 'ppspp', MAP {'SIM_VIEWERS': '16'}))
  t(tier, cell, sim, env);
CREATE TABLE table_catalog AS SELECT 'nums' AS name, 'table' AS kind, 'one a number' AS grain,
  'i' AS keys, '' AS joins, 'numbers' AS description;
CREATE TABLE metric_catalog AS SELECT 'i' AS name, 'nums' AS tbl, 'BIGINT' AS type,
  NOW() AS first_seen, NOW() AS last_seen;
CREATE TABLE event_kinds AS SELECT 'join' AS kind, 'ppspp' AS sim, 'a viewer joins' AS description;
CREATE TABLE catalog_info AS SELECT 'catalog_version' AS key, '1' AS value;
CREATE TABLE nums AS SELECT range AS i FROM range(100000);""")
    finally:
        w.close()


# --- stand-ins: index and fetch, run in the forkserver's child ------------------------------------


def test_dir():
    return Path(os.environ[DIR_ENV])


def log_call(*call):
    with (test_dir() / "calls.log").open("a") as f:
        f.write(json.dumps([*call, os.getpid(), time.time()]) + "\n")


def mode(what):
    """What the next run of `what` does: index-mode or fetch-mode's text."""
    p = test_dir() / f"{what}-mode"
    return p.read_text().split() if p.exists() else []


def add_table(name):
    w = engine.writer(test_dir() / "lake")
    try:
        w.execute(f"CREATE OR REPLACE TABLE {name} AS SELECT 1 AS a")
    finally:
        w.close()


def wait_for(path, timeout=30):
    end = time.time() + timeout
    while not path.exists() and time.time() < end:
        time.sleep(0.02)


def act(what):
    """Does what `what`'s mode says: raise, exit with a message (say) or a
    code, kill, add a table, wait for a go file."""
    words = mode(what)
    if "raise" in words:
        raise RuntimeError("the cache is gone")
    if "say" in words:
        sys.exit("index: the cache is gone")
    if "exit" in words:
        os._exit(7)
    if "kill" in words:
        os.kill(os.getpid(), signal.SIGKILL)
    if "table" in words:
        add_table(words[words.index("table") + 1])
    if "slow" in words:
        wait_for(test_dir() / f"{what}-go")


def fake_index(progress=None, workers=None, wait=True):
    log_call("index", "begin")
    progress(catalog.Progress("scan", "scanning the cache", 0, 2))
    act("index")
    progress(catalog.Progress("write", "writing 1 table", 1, 2))
    log_call("index", "end")
    return catalog.Indexed(changed=True, sources=["a", "b"], failed={"b": "no runs"} if "fail-b" in mode("index")
                           else {}, elapsed_s=0.01)


def fake_fetch(exp, cells=None, arms=None, tables=(), workers=None, limit_mb=None, progress=None):
    log_call("fetch", exp, cells, arms, list(tables))
    progress(catalog.Progress("pull", "pulling 2 flushes", 0, 2))
    act("fetch")
    progress(catalog.Progress("pull", "pulled 2 flushes", 2, 2))
    log_call("fetched", exp)
    return catalog.Fetched(exp, s3_usd=0.0123, ledger_lines=2, pulled=["f1", "f2"])


def calls():
    p = test_dir() / "calls.log"
    return [json.loads(line) for line in p.read_text().splitlines()] if p.exists() else []


TASKS = {"index": "server_test:fake_index", "fetch": "server_test:fake_fetch"}


# --- stand-ins: the suite and farm.py ----------------------------------------------------------------


class FakeSuite:
    """suite.cells and suite.write_matrices, writing matrices as suite.py
    farm names them; each call kept."""
    TIERS = {"robust": [{"name": "calm", "sim": "ppspp", "env": {"SIM_VIEWERS": "8"}},
                        {"name": "storm", "sim": "ppspp", "env": {"SIM_VIEWERS": "32"}}]}

    def __init__(self, out):
        self.out = out
        self.calls = []

    def cells(self, tier, only=None, env=None):
        return [dict(c) for c in self.TIERS[tier] if only is None or c["name"] in only]

    def write_matrices(self, cells, seeds, **kw):
        self.calls.append(([c["name"] for c in cells], list(seeds), kw))
        self.out.mkdir(parents=True, exist_ok=True)
        ms = []
        for i, c in enumerate(cells):
            p = self.out / f"s0abcd-{i:02d}.toml"
            p.write_text(f"# sim-suite farm s0abcd: {c['name']}, why\n")
            ms.append(suite.Matrix(str(p), c["name"], c["env"]))
        return suite.Matrices("s0abcd", ms, {})


# farm.py's stand-in: estimate --json's quote (its JSON on stdout, its text
# on stderr) and run's line, each call kept in farm-calls.log beside it.
FARM_PY = """import json, sys
from pathlib import Path
with (Path(__file__).parent / "farm-calls.log").open("a") as f:
    f.write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[1] == "estimate":
    print("16 jobs, ~$0.05", file=sys.stderr)
    print(json.dumps({"jobs": 16, "cost": 0.05}))
elif sys.argv[1] == "run":
    print("running")
"""


def farm_calls():
    p = DIR / "bin" / "farm-calls.log"
    return [json.loads(line) for line in p.read_text().splitlines()] if p.exists() else []


# --- the server under test ----------------------------------------------------------------------------


def setUpModule():
    global TMP, DIR, LAKE, POOL, CONSOLE, HTTPD, PORT, SUITE, REGS
    # json_types.json's TIMESTAMPTZ offsets are Los Angeles's.
    os.environ["TZ"] = "America/Los_Angeles"
    time.tzset()
    TMP = tempfile.TemporaryDirectory(prefix="console-test-")
    DIR = Path(TMP.name)
    os.environ[DIR_ENV] = str(DIR)
    LAKE = DIR / "lake"
    make_lake(LAKE)
    POOL = engine.Pool(LAKE, DIR / "spill", size=2, guard=engine.Guard("1GB", 2, "1GiB"))
    SUITE = FakeSuite(DIR / "matrices")
    (DIR / "bin").mkdir()
    (DIR / "bin" / "farm.py").write_text(FARM_PY)
    REGS = farm.Regenerations(DIR / "work" / "regenerate", farm_py=DIR / "bin" / "farm.py", python=sys.executable)
    CONSOLE = server.Console(POOL, views_root=DIR / "views", regenerations=REGS, dist=DIR / "dist", tasks=TASKS,
                             suite=SUITE)
    HTTPD = CONSOLE.listen(0)
    PORT = HTTPD.server_address[1]
    threading.Thread(target=HTTPD.serve_forever, daemon=True).start()


def tearDownModule():
    HTTPD.shutdown()
    HTTPD.server_close()
    CONSOLE.close()
    TMP.cleanup()


@dataclasses.dataclass
class Answer:
    status: int
    body: object
    headers: object
    text: bytes


def call(method, route, body=None, type="application/json", headers=None):
    c = http.client.HTTPConnection("127.0.0.1", PORT, timeout=60)
    try:
        h = dict(headers or {})
        data = None
        if body is not None:
            data = body if isinstance(body, bytes) else json.dumps(body).encode()
            h.setdefault("Content-Type", type)
        c.request(method, route, body=data, headers=h)
        r = c.getresponse()
        text = r.read()
        kind = r.headers.get("Content-Type") or ""
        parsed = json.loads(text) if kind.startswith("application/json") and text else None
        return Answer(r.status, parsed, r.headers, text)
    finally:
        c.close()


def post(route, body, type="application/json", headers=None):
    return call("POST", route, body, type, headers)


def sql(q, **more):
    return post("/console/sql", {"sql": q, **more})


def lines(text):
    return [json.loads(line) for line in text.decode().splitlines() if line]


def set_mode(what, text):
    p = DIR / f"{what}-mode"
    if text:
        p.write_text(text)
    else:
        p.unlink(missing_ok=True)
    (DIR / f"{what}-go").unlink(missing_ok=True)


def until(check, timeout=20):
    end = time.time() + timeout
    while time.time() < end:
        got = check()
        if got:
            return got
        time.sleep(0.05)
    return check()


def fresh_views():
    CONSOLE.views_root = Path(tempfile.mkdtemp(dir=DIR, prefix="views-"))
    return CONSOLE.views_root


class Sql(unittest.TestCase):
    def test_a_querys_columns_rows_and_types(self):
        r = sql("SELECT 2::BIGINT AS a, 9007199254740993::BIGINT AS b, 1.25::DECIMAL(10, 2) AS c, "
                "sum(i) AS d, MAP {'x': 1} AS e, [1, 2] AS f FROM nums")
        self.assertEqual(r.status, 200, r.body)
        self.assertEqual(r.body["columns"], [
            {"name": "a", "type": "BIGINT"},
            {"name": "b", "type": "BIGINT"},
            {"name": "c", "type": "DECIMAL(10,2)"},
            {"name": "d", "type": "HUGEINT"},
            {"name": "e", "type": "MAP(VARCHAR, INTEGER)"},
            {"name": "f", "type": "INTEGER[]"},
        ])
        self.assertEqual(r.body["rows"], [[2, "9007199254740993", 1.25, 4999950000, {"x": 1}, [1, 2]]])
        self.assertIs(r.body["truncated"], False)

    def test_parameters_bind_by_name_and_a_missing_one_fails(self):
        r = sql("SELECT $exp || '!' AS s, $seed + 1 AS n, $xs AS xs",
                params={"exp": "e1", "seed": 2001, "xs": ["a", "b"], "unused": 1})
        self.assertEqual(r.body["rows"], [["e1!", 2002, ["a", "b"]]])
        m = sql("SELECT $exp", params={})
        self.assertEqual(m.status, 400)
        self.assertRegex(m.body["error"], r"no value for \$exp")

    def test_explain_describe_and_the_like_run(self):
        for q in ["EXPLAIN SELECT count(*) FROM nums", "EXPLAIN ANALYZE SELECT count(*) FROM nums",
                  "-- a comment\nEXPLAIN (ANALYZE, FORMAT json) SELECT 1", "DESCRIBE nums", "SUMMARIZE nums",
                  "SHOW TABLES", "FROM nums LIMIT 1"]:
            r = sql(q)
            self.assertEqual(r.status, 200, f"{q}: {r.body}")
        self.assertEqual(sql("EXPLAIN SELECT 1; COPY nums TO '/tmp/x.csv'").status, 403)

    def test_rows_stop_at_max_rows(self):
        r = sql("SELECT i FROM nums ORDER BY i", max_rows=10)
        self.assertEqual(len(r.body["rows"]), 10)
        self.assertIs(r.body["truncated"], True)

    def test_a_long_query_is_interrupted_at_its_timeout_and_its_connection_works_after(self):
        t0 = time.time()
        r = sql("SELECT sum(hash(i)) FROM range(100000000000) t(i)", timeout_s=1)
        self.assertEqual(r.status, 408)
        self.assertRegex(r.body["error"], "interrupted at the timeout")
        self.assertLess(time.time() - t0, 5)
        for _ in range(3):
            self.assertEqual(sql("SELECT 1").status, 200)

    def test_writes_copy_attach_install_load_set_and_two_statements_are_refused(self):
        data = LAKE / "data"
        refused = [
            "INSERT INTO nums VALUES (1)",
            "UPDATE nums SET i = 0",
            "DELETE FROM nums",
            "CREATE TABLE t AS SELECT 1",
            "CREATE TEMP TABLE t AS SELECT 1",
            "DROP TABLE nums",
            f"COPY (SELECT 1) TO '{data}/x.csv'",
            f"COPY nums TO '{data}/y.parquet'",
            f"EXPORT DATABASE '{data}/z'",
            "ATTACH ':memory:' AS m",
            f"ATTACH '{data}/w.duckdb' AS m",
            "DETACH lake",
            "INSTALL httpfs",
            "LOAD httpfs",
            "SET threads = 64",
            "SET enable_external_access = true",
            "RESET lock_configuration",
            "USE memory",
            "PRAGMA enable_profiling",
            "CALL ducklake_snapshots('lake')",
            f"EXPLAIN ANALYZE COPY (SELECT 1) TO '{data}/v.csv'",
        ]
        self.assertEqual(len(refused), 21)
        for q in refused:
            r = sql(q)
            self.assertEqual(r.status, 403, f"{q}: {r.body}")
        two = sql("SELECT 1; SELECT 2")
        self.assertEqual(two.status, 400)
        self.assertRegex(two.body["error"], "one statement")
        inner = sql(f"SELECT * FROM query('COPY (SELECT 1) TO ''{data}/u.csv''')")
        self.assertEqual(inner.status, 400)
        for f in ["x.csv", "y.parquet", "z", "w.duckdb", "v.csv", "u.csv"]:
            self.assertFalse((data / f).exists(), f)
        self.assertEqual(sql("SELECT count(*) FROM nums").body["rows"], [[100000]])

    def test_the_connection_reads_only_the_lakes_files_its_settings_locked(self):
        for q in ["SELECT * FROM read_csv('/etc/passwd')", "SELECT * FROM read_text('/etc/hostname')",
                  "SELECT * FROM glob('/etc/*')", f"SELECT * FROM read_csv('{DIR / 'calls.log'}')"]:
            r = sql(q)
            self.assertEqual(r.status, 400, q)
            self.assertRegex(r.body["error"], "Permission Error|disabled", q)
        s = sql("SELECT current_setting('enable_external_access') AS ext, "
                "current_setting('lock_configuration') AS lock, "
                "current_setting('autoload_known_extensions') AS autoload, current_setting('threads') AS threads, "
                "current_setting('memory_limit') AS mem, "
                "(SELECT readonly FROM duckdb_databases() WHERE database_name = 'lake') AS ro")
        self.assertEqual(s.body["rows"], [[False, True, False, 2, "953.6 MiB", True]])
        files = sql(f"SELECT count(*) FROM read_parquet('{LAKE / 'data'}/**/*.parquet')")
        self.assertEqual(files.status, 200, files.body)

    def test_a_post_that_isnt_json_is_refused(self):
        self.assertEqual(post("/console/sql", {"sql": "SELECT 1"}, "text/plain").status, 415)

    def test_json_is_the_golden(self):
        r = sql(TYPES_SQL)
        self.assertEqual(r.status, 200, r.body)
        self.assertEqual([[c["name"], c["type"]] for c in r.body["columns"]], API["columns"])
        self.assertEqual(json.dumps(r.body["rows"], separators=(",", ":")), API["rows"])

    def test_bodies_and_numbers_it_refuses(self):
        for body, status in [(b"{", 400), (b"[1]", 400), (b'{"sql": 1}', 400), (b'{"sql": "SELECT 1", "x": NaN}', 400),
                             (b'{"sql": "SELECT 1", "params": [1]}', 400),
                             (b'{"sql": "SELECT 1", "params": {"a": [true]}}', 400),
                             (b'{"sql": "SELECT 1", "params": {"a": {"b": 1}}}', 400),
                             (b'{"sql": "SELECT 1", "max_rows": "many"}', 400),
                             (b'{"sql": "SELECT 1", "timeout_s": {}}', 400),
                             (b'{"sql": "SELECT 1", "max_rows": "5", "timeout_s": null}', 200),
                             (b'{"sql": "SELECT 1", "max_rows": [3]}', 200)]:
            self.assertEqual(post("/console/sql", body).status, status, body)
        s = socket.create_connection(("127.0.0.1", PORT))
        s.sendall(b"POST /console/sql HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n"
                  b"Content-Length: " + str(server.MAX_BODY + 1).encode() + b"\r\n\r\n{")
        self.assertRegex(s.makefile("rb").readline(), rb"^HTTP/1.1 413 ")
        s.close()
        self.assertEqual(call("GET", "/console/nothing").status, 404)
        self.assertEqual(call("GET", "/console/nothing").body, {"error": "no GET /console/nothing"})
        self.assertEqual(call("OPTIONS", "/console/sql").status, 404)


class Races(unittest.TestCase):
    def test_a_timeout_about_as_long_as_its_query_never_reaches_the_next_query(self):
        # The shortest timeout is 0.1 s: a query about that long, its
        # timeouts around its time, each followed by a query on the same
        # reader (the pool hands out the reader last released).
        q = "SELECT sum(hash(range)) AS s FROM range({})"
        n, took = 1_000_000, 0
        while took < 150 and n < 10 ** 10:
            n *= 2
            took = min(sql(q.format(n)).body["elapsed_ms"] for _ in range(3))
        timed_out = finished = 0
        for i in range(40):
            r = sql(q.format(n), timeout_s=took / 1000 * (0.6 + (i % 7) / 7))
            self.assertIn(r.status, (200, 408), r.body)
            timed_out += r.status == 408
            finished += r.status == 200
            self.assertEqual(sql("SELECT 7 AS x").body["rows"], [[7]])
        self.assertGreater(timed_out, 0)
        self.assertGreater(finished, 0)

    def test_the_timer_and_requests_start_one_index_at_a_time(self):
        set_mode("index", "")
        other = server.Console(POOL, views_root=DIR / "views", regenerations=REGS, dist=DIR / "dist", tasks=TASKS,
                               suite=SUITE, index_every=0.05)
        before = len(calls())
        try:
            ts = [threading.Thread(target=lambda: other.indexer.start().wait()) for _ in range(8)]
            for t in ts:
                t.start()
                time.sleep(0.03)
            for t in ts:
                t.join(30)
            until(lambda: len([c for c in calls()[before:] if c[:2] == ["index", "end"]]) >= 3)
        finally:
            other._stop.set()
            if other.indexer.current:
                other.indexer.current.wait()
        runs = [c for c in calls()[before:] if c[0] == "index"]
        open_ = 0
        for c in runs:
            open_ += 1 if c[1] == "begin" else -1
            self.assertIn(open_, (0, 1), runs)
        self.assertGreaterEqual(len(runs), 6)


class SchemaAndViews(unittest.TestCase):
    def test_schema_has_the_catalogs_tables_catalogs_and_experiments(self):
        fresh_views()
        r = call("GET", "/console/schema").body
        names = [t["name"] for t in r["tables"]]
        self.assertTrue({"experiments", "nums", "tier_cells"} <= set(names), names)
        self.assertEqual(len(r["experiments"]), 1)
        self.assertEqual(len(r["table_catalog"]), 1)
        self.assertEqual(len(r["event_kinds"]), 1)
        self.assertEqual(r["views"], [])

    def test_a_saved_view_comes_back(self):
        fresh_views()
        res = call("PUT", "/console/views/my-view", {"title": "Mine", "sql": "SELECT $exp AS e",
                                                     "spec": {"mark": "bar"},
                                                     "params": [{"name": "exp", "kind": "exp"}]})
        self.assertEqual(res.status, 200, res.body)
        self.assertEqual(res.body["sql"], "SELECT $exp AS e\n")
        got = call("GET", "/console/views").body
        self.assertEqual([[v["name"], v["spec"]] for v in got], [["my-view", {"mark": "bar"}]])
        self.assertEqual(call("PUT", "/console/views/..%2Fescape", {"sql": "SELECT 1"}).status, 400)
        self.assertEqual(call("PUT", "/console/views/x", {"sql": " "}).status, 400)
        bad = call("PUT", "/console/views/x", {"sql": "SELECT 1", "params": [{"name": "a", "kind": "colour"}]})
        self.assertEqual(bad.status, 400)
        self.assertFalse((CONSOLE.views_root / "../escape.sql").exists())

    def test_views_are_read_from_disk_at_each_request(self):
        root = fresh_views()
        self.assertEqual(call("GET", "/console/schema").body["views"], [])
        (root / "hand.sql").write_text("SELECT 1 AS one\n")
        (root / "hand.json").write_text(json.dumps({"title": "By hand", "params": []}))
        views = call("GET", "/console/schema").body["views"]
        self.assertEqual([(v["name"], v["title"], v["spec"]) for v in views], [("hand", "By hand", None)])
        (root / "hand.json").write_text(json.dumps({"title": "Edited", "todo": "check it"}))
        (root / "hand.vl.json").write_text(json.dumps({"mark": "point"}))
        views = call("GET", "/console/schema").body["views"]
        self.assertEqual([(v["title"], v["spec"], v["todo"]) for v in views],
                         [("Edited", {"mark": "point"}, "check it")])
        (root / "hand.sql").unlink()
        self.assertEqual(call("GET", "/console/views").body, [])


class Commands(unittest.TestCase):
    def setUp(self):
        set_mode("index", "")
        set_mode("fetch", "")

    def test_fetch_streams_its_progress_and_its_s3_cost(self):
        res = post("/console/fetch", {"exp_id": "e1", "cells": ["calm"], "tables": ["reads"]})
        self.assertEqual(res.status, 200)
        self.assertTrue(res.headers["Content-Type"].startswith("application/x-ndjson"))
        got = lines(res.text)
        self.assertEqual(got[-1], {"done": True, "code": 0, "s3_usd": 0.0123, "elapsed_ms": got[-1]["elapsed_ms"]})
        self.assertEqual(got[0], {"line": "$ simdb.py fetch --exp e1 --cells calm --tables reads"})
        self.assertIn({"line": "pulling 2 flushes", "progress": {"kind": "pull", "done": 0, "total": 2}}, got)
        self.assertIn({"line": "S3 charged $0.0123 (2 ledger lines)"}, got)
        self.assertEqual([c[:5] for c in calls() if c[0] == "fetch"][-1], ["fetch", "e1", ["calm"], None, ["reads"]])

    def test_fetch_refuses_a_name_that_isnt_one(self):
        self.assertEqual(post("/console/fetch", {"exp_id": "e1", "cells": ["--all"]}).status, 400)
        self.assertEqual(post("/console/fetch", {"exp_id": "e1", "cells": "calm"}).status, 400)
        self.assertEqual(post("/console/fetch", {"cells": ["calm"]}).status, 400)

    def test_index_runs_and_reports_it(self):
        r = post("/console/index", {})
        self.assertEqual(r.status, 200)
        self.assertEqual(r.body["code"], 0)
        self.assertEqual(r.body["tail"], ["scanning the cache", "writing 1 table"])
        set_mode("index", "fail-b")
        self.assertEqual(post("/console/index", {}).body["tail"][-1],
                         "the verdicts of 1 sources failed (they wait for the next index): b")
        self.assertEqual(calls()[-1][:2], ["index", "end"])
        s = call("GET", "/console/index").body
        self.assertEqual(s["last"]["code"], 0)
        self.assertIsNone(s["running"])
        self.assertEqual(s["catalog_info"], [{"key": "catalog_version", "value": "1"}])

    def test_regenerate_quotes_and_only_approve_starts_a_farm_run(self):
        SUITE.calls.clear()
        before = len(farm_calls())
        r = post("/console/regenerate", {"exp_id": "e1", "cells": ["calm", "storm"], "arms": ["new", "base"],
                                         "seeds": [2001, 2002, 2003]})
        self.assertEqual(r.status, 200, r.body)
        self.assertEqual(r.body["state"], "quoted")
        self.assertEqual(r.body["quote"], "16 jobs, ~$0.05")
        self.assertEqual(r.body["estimate"], {"jobs": 16, "cost": 0.05})
        self.assertEqual(r.body["warnings"], ["storm: env differs from e1's at SIM_VIEWERS (16 -> 32)"])
        self.assertEqual(SUITE.calls,
                         [(["calm", "storm"], [2001, 2002, 2003], {"arms": {"base": "aaaa", "new": "bbbb"}})])
        self.assertEqual(r.body["commands"], [["suite.py", "farm", "--tier", "robust", "--only", "calm,storm",
                                               "--seeds", "3", "--first-seed", "2001", "--refs", "base=aaaa,new=bbbb",
                                               "--no-run"]])
        id = r.body["id"]
        made = lambda: farm_calls()[before:]  # noqa: E731
        self.assertEqual([c[:2] for c in made()], [["estimate", "--json"]])
        # Nothing runs without the approve call: not a read of it, nor another quote.
        self.assertEqual(call("GET", f"/console/regenerate/{id}").body, r.body)
        post("/console/regenerate", {"exp_id": "e1", "cells": ["calm"], "arms": ["new"], "seeds": [2001]})
        time.sleep(0.3)
        self.assertFalse(any(c[0] == "run" for c in made()))
        self.assertEqual(post("/console/regenerate/nope/approve", {}).status, 404)
        self.assertEqual(call("GET", "/console/regenerate/nope").status, 404)
        a = post(f"/console/regenerate/{id}/approve", {})
        self.assertEqual(a.status, 200, a.body)
        self.assertEqual(a.body["state"], "running")
        until(lambda: any(c[0] == "run" for c in made()))
        runs = [c for c in made() if c[0] == "run"]
        self.assertEqual([[Path(p).name for p in c[1:]] for c in runs], [["s0abcd-00.toml", "s0abcd-01.toml"]])
        self.assertEqual(post(f"/console/regenerate/{id}/approve", {}).status, 409)
        done = until(lambda: (g := call("GET", f"/console/regenerate/{id}").body)["state"] == "finished" and g)
        self.assertEqual((done["code"], done["log_tail"][0]), (0, "running"))

    def test_regenerate_refuses_what_the_suite_cant_run(self):
        for body, status in [({"exp_id": "e9", "cells": ["calm"], "arms": ["new"], "seeds": [1]}, 404),
                             ({"exp_id": "e1", "cells": ["calm"], "arms": ["new"], "seeds": [1.5]}, 400),
                             ({"exp_id": "e1", "cells": ["calm"], "arms": ["new"], "seeds": "1"}, 400),
                             ({"exp_id": "--x", "cells": ["calm"], "arms": ["new"], "seeds": [1]}, 400),
                             ({"exp_id": "e1", "cells": "calm", "arms": ["new"], "seeds": [1]}, 400)]:
            self.assertEqual(post("/console/regenerate", body).status, status, body)
        for body, error in [({"cells": ["calm"], "arms": ["zz"], "seeds": [1]}, "not an arm of e1"),
                            ({"cells": ["calm"], "arms": ["new"], "seeds": [1, 3]}, "consecutive"),
                            ({"cells": ["gone"], "arms": ["new"], "seeds": [1]}, "no tier of e1 has gone")]:
            failed = post("/console/regenerate", {"exp_id": "e1", **body})
            self.assertEqual((failed.status, failed.body["state"]), (200, "failed"))
            self.assertRegex(failed.body["error"], error)
            # Refused before any matrix was written: not kept.
            self.assertEqual(call("GET", f"/console/regenerate/{failed.body['id']}").status, 404)


class SlowStream(fakes.FakeStream):
    """A turn whose text comes a word every `delay` s, for a client to
    leave mid-turn."""
    delay = 0.05

    def __iter__(self):
        for ev in super().__iter__():
            time.sleep(self.delay)
            yield ev


class Ask(unittest.TestCase):
    def setUp(self):
        self.ledger = Path(tempfile.mkdtemp(dir=DIR, prefix="spend-")) / "spend.jsonl"

    def asker(self, script):
        self.client = fakes.FakeClient(script)
        CONSOLE.asker = ask.Asker(POOL, CONSOLE.schema, client=self.client, ledger=self.ledger,
                                  views_root=lambda: CONSOLE.views_root, regenerations=REGS, suite=SUITE,
                                  fetch=CONSOLE.ask_fetch)
        return CONSOLE.asker

    def test_a_question_streams_ndjson_and_a_follow_up_continues_it(self):
        count = fakes.call("t1", "run_sql", sql="SELECT count(*) AS n FROM nums", purpose="count")
        self.asker([fakes.msg([fakes.thinking("count"), count], "tool_use"),
                    fakes.msg([fakes.text("100,000 numbers.")]), fakes.msg([fakes.text("Still 100,000.")])])
        res = post("/console/ask", {"question": "how many numbers?", "task_budget": 30_000})
        self.assertEqual(res.status, 200)
        self.assertTrue(res.headers["Content-Type"].startswith("application/x-ndjson"))
        got = lines(res.text)
        types = [e["type"] for e in got]
        self.assertEqual((types[0], types[-1]), ("start", "done"))
        for t in ("thinking", "tool_start", "tool", "sql", "result", "tool_result", "usage", "text"):
            self.assertIn(t, types)
        self.assertEqual(got[0]["task_budget"], 30_000)
        self.assertEqual([e["result"]["rows"] for e in got if e["type"] == "result"], [[[100_000]]])
        self.assertEqual("".join(e["text"] for e in got if e["type"] == "text"), "100,000 numbers.")
        self.assertEqual((got[-1]["stop_reason"], got[-1]["turns"]), ("end_turn", 2))
        cid = got[0]["conversation_id"]
        self.assertEqual([r["output_config"]["task_budget"]["total"] for r in self.client.requests], [30_000, 30_000])
        follow = lines(post("/console/ask", {"question": "and now?", "conversation_id": cid}).text)
        self.assertEqual((follow[0]["conversation_id"], follow[0]["question"]), (cid, 2))
        self.assertEqual(self.client.requests[2]["messages"][:4], self.client.requests[1]["messages"]
                         + [self.client.requests[2]["messages"][3]])
        c = call("GET", f"/console/ask/{cid}").body
        self.assertEqual([q["question"] for q in c["questions"]], ["how many numbers?", "and now?"])
        self.assertEqual(c["questions"][0]["events"][-1]["type"], "done")
        self.assertNotIn("text", {e["type"] for e in c["questions"][0]["events"]})
        self.assertEqual(call("GET", "/console/ask/nope").status, 404)
        s = call("GET", "/console/spend").body
        self.assertEqual(s["days"][0]["questions"], 2)
        self.assertEqual(s["days"][0]["requests"], 3)
        self.assertAlmostEqual(s["total_usd"], 3 * (100 * 4 + 50 * 20) / 1e6)
        self.assertEqual(call("GET", f"/console/spend?day={s['days'][0]['day']}").body["questions"][0]["text"],
                         "and now?")

    def test_bad_questions_are_refused_before_any_request(self):
        self.asker([])
        for body, status in [({}, 400), ({"question": "  "}, 400), ({"question": 3}, 400),
                             ({"question": "x", "task_budget": 19_999}, 400),
                             ({"question": "x", "task_budget": 25_000.5}, 400),
                             ({"question": "x", "task_budget": "30000"}, 400),
                             ({"question": "x", "conversation_id": "nope"}, 404),
                             ({"question": "x", "conversation_id": 7}, 400),
                             ({"question": "x" * (server.MAX_QUESTION + 1)}, 413)]:
            self.assertEqual(post("/console/ask", body).status, status, body)
        self.assertEqual(post("/console/ask", b'{"question": "x"}', type="text/plain").status, 415)
        self.assertEqual(self.client.requests, [])

    def test_a_slow_turn_pings_and_a_client_that_goes_cancels_it(self):
        a = self.asker([])
        slow = fakes.msg([fakes.text("word " * 400)])
        self.client.beta.messages.stream = lambda **kw: SlowStream(self.client, slow)
        saved = server.PING_S
        server.PING_S = 0.01
        try:
            c = http.client.HTTPConnection("127.0.0.1", PORT, timeout=30)
            c.request("POST", "/console/ask", body=json.dumps({"question": "slow"}),
                      headers={"Content-Type": "application/json"})
            r = c.getresponse()
            first = json.loads(r.fp.readline())
            seen = [json.loads(r.fp.readline())["type"] for _ in range(3)]
            r.close()
            c.close()
        finally:
            server.PING_S = saved
        self.assertEqual(first["type"], "start")
        self.assertIn("ping", seen)
        q = until(lambda: (s := a.conversation(first["conversation_id"])["questions"][0])["stop_reason"] and s)
        self.assertEqual(q["stop_reason"], "cancelled")
        self.assertEqual(q["events"][-1]["type"], "done")

    def test_a_proposal_quotes_and_only_the_approve_route_runs_it(self):
        SUITE.calls.clear()
        before = len(farm_calls())
        self.asker([fakes.msg([fakes.call("p1", "propose_farm_run", exp_id="e1", cells=["calm", "storm"],
                                          arms=["new", "base"], seeds=[2001, 2002, 2003], why="more seeds")],
                              "tool_use"),
                    fakes.msg([fakes.text("Quoted: approve it in the console.")])])
        got = lines(post("/console/ask", {"question": "rerun e1"}).text)
        proposal = [e for e in got if e["type"] == "proposal"][0]
        id = proposal["regeneration"]["id"]
        self.assertEqual((proposal["regeneration"]["state"], proposal["why"]), ("quoted", "more seeds"))
        result = json.loads([e for e in got if e["type"] == "tool_result"][0]["preview"])
        self.assertEqual((result["quote_id"], result["cost_usd"], result["jobs"]), (id, 0.05, 16))
        time.sleep(0.3)
        self.assertEqual([c[:2] for c in farm_calls()[before:]], [["estimate", "--json"]])
        self.assertEqual(call("GET", f"/console/regenerate/{id}").body["state"], "quoted")


class Children(unittest.TestCase):
    def setUp(self):
        set_mode("index", "")
        set_mode("fetch", "")

    def test_the_index_child_streams_its_progress(self):
        res = post("/console/index", {}, headers={"Accept": "application/x-ndjson"})
        got = lines(res.text)
        self.assertEqual([g.get("progress", {}).get("kind") for g in got[:2]], ["scan", "write"])
        self.assertEqual(got[-1]["done"], True)
        self.assertEqual(got[-1]["code"], 0)
        self.assertEqual(len(got), 3)

    def test_a_crash_in_the_index_child_fails_that_index_not_the_server(self):
        for how, code, error in [("exit", 7, "its process exited with code 7 before it finished"),
                                 ("kill", -9, "its process was killed by signal 9"),
                                 ("raise", 1, "RuntimeError: the cache is gone"),
                                 ("say", 1, "index: the cache is gone")]:
            set_mode("index", how)
            r = post("/console/index", {})
            self.assertEqual((r.status, r.body["code"], r.body["error"]), (200, code, error), how)
            self.assertEqual(r.body["tail"][-1], f"index failed: {error}")
            self.assertEqual(call("GET", "/console/index").body["last"]["code"], code)
            self.assertEqual(sql("SELECT 1 AS x").body["rows"], [[1]])
        set_mode("index", "")
        self.assertEqual(post("/console/index", {}).body["code"], 0)

    def test_a_successful_index_refreshes_the_readers_and_schema_and_a_failed_one_doesnt(self):
        self.assertEqual(sql("SELECT a FROM fresh_1").status, 400)
        self.assertNotIn("fresh_1", [t["name"] for t in call("GET", "/console/schema").body["tables"]])
        set_mode("index", "table fresh_1")
        self.assertEqual(post("/console/index", {}).body["code"], 0)
        self.assertEqual(sql("SELECT a FROM fresh_1").body["rows"], [[1]])
        self.assertIn("fresh_1", [t["name"] for t in call("GET", "/console/schema").body["tables"]])
        before = POOL.generation
        set_mode("index", "raise")
        post("/console/index", {})
        sql("SELECT 1")
        self.assertEqual(POOL.generation, before)

    def test_a_fetch_whose_client_goes_still_finishes_and_the_next_one_runs(self):
        set_mode("fetch", "slow table fetched_1")
        s = socket.create_connection(("127.0.0.1", PORT))
        body = json.dumps({"exp_id": "e1"}).encode()
        s.sendall(b"POST /console/fetch HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n"
                  b"Content-Length: " + str(len(body)).encode() + b"\r\n\r\n" + body)
        got = b""
        while b"pulling 2 flushes" not in got:
            chunk = s.recv(4096)
            self.assertTrue(chunk, got)
            got += chunk
        s.close()
        (DIR / "fetch-go").touch()
        self.assertTrue(until(lambda: sql("SELECT a FROM fetched_1").status == 200))
        self.assertIn(["fetched", "e1"], [c[:2] for c in calls()])
        set_mode("fetch", "")
        got = lines(post("/console/fetch", {"exp_id": "e1"}).text)
        self.assertEqual(got[-1]["code"], 0)
        self.assertEqual(sql("SELECT 1").status, 200)

    def test_a_crash_in_the_fetch_child_ends_its_stream(self):
        set_mode("fetch", "kill")
        got = lines(post("/console/fetch", {"exp_id": "e1"}).text)
        self.assertEqual(got[-1], {"done": True, "code": -9, "s3_usd": None, "elapsed_ms": got[-1]["elapsed_ms"],
                                   "error": "its process was killed by signal 9"})
        set_mode("fetch", "")
        self.assertEqual(lines(post("/console/fetch", {"exp_id": "e1"}).text)[-1]["code"], 0)


def index_elsewhere(table):
    """An index by another process (as suite.py run's or simdb.py index's
    is): a child of its own adds `table` to the test's catalog."""
    subprocess.run([sys.executable, "-c", f"import server_test; server_test.add_table({table!r})"], cwd=HERE,
                   check=True, timeout=120)


class Snapshots(unittest.TestCase):
    def setUp(self):
        set_mode("index", "")

    def test_the_index_state_and_a_querys_result_carry_the_catalogs_snapshot(self):
        latest = catalog.snapshot(LAKE)
        self.assertIsInstance(latest["id"], int)
        self.assertRegex(latest["at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\+00:00$")
        self.assertEqual(sql("SELECT 1 AS x").body["snapshot"], latest["id"])
        self.assertEqual(call("GET", "/console/index").body["snapshot"], latest)

    def test_an_index_by_another_process_reaches_the_next_query_with_one_refresh(self):
        seen = sql("SELECT 1").body["snapshot"]
        before = POOL.generation
        self.assertEqual(sql("SELECT a FROM elsewhere_1").status, 400)
        index_elsewhere("elsewhere_1")
        latest = catalog.snapshot(LAKE)
        self.assertNotEqual(latest["id"], seen)
        r = sql("SELECT a FROM elsewhere_1")
        self.assertEqual((r.status, r.body["rows"], r.body["snapshot"]), (200, [[1]], latest["id"]))
        self.assertIn("elsewhere_1", [t["name"] for t in call("GET", "/console/schema").body["tables"]])
        self.assertEqual(call("GET", "/console/index").body["snapshot"], latest)
        sql("SELECT 1")
        self.assertEqual(POOL.generation, before + 1)

    def test_with_no_change_nothing_refreshes(self):
        seen = sql("SELECT 1").body["snapshot"]
        before = POOL.generation
        for _ in range(3):
            self.assertEqual(sql("SELECT 1").body["snapshot"], seen)
            self.assertEqual(call("GET", "/console/index").body["snapshot"]["id"], seen)
            self.assertEqual(call("GET", "/console/schema").status, 200)
        self.assertEqual(POOL.generation, before)

    def test_a_snapshot_that_cant_be_read_keeps_the_last(self):
        seen = sql("SELECT 1").body["snapshot"]
        before = POOL.generation
        with mock.patch.object(catalog, "snapshot", lambda lake=None: None):
            self.assertEqual(sql("SELECT 1").body["snapshot"], seen)
            self.assertEqual(call("GET", "/console/index").body["snapshot"]["id"], seen)
        self.assertEqual(POOL.generation, before)


class Static(unittest.TestCase):
    def test_the_build_is_served_beside_the_api(self):
        dist = DIR / "dist"
        self.assertEqual(call("GET", "/").status, 503)
        self.assertRegex(call("GET", "/").body["error"], "npm run console:build")
        self.assertEqual(call("GET", "/devtools.html").status, 503)
        (dist / "console").mkdir(parents=True)
        (dist / "devtools.html").write_text("<!doctype html><title>devtools</title>")
        (dist / "devtools.abc.js").write_text("console.log(1)")
        (dist / "console" / "schema").write_text("not the API")
        (DIR / "secret.txt").write_text("outside")
        r = call("GET", "/")
        self.assertEqual((r.status, r.headers["Location"]), (302, "/devtools.html#/console"))
        r = call("GET", "/devtools.html")
        self.assertEqual((r.status, r.headers["Content-Type"], r.text),
                         (200, "text/html", b"<!doctype html><title>devtools</title>"))
        self.assertEqual(call("GET", "/devtools.abc.js").headers["Content-Type"], "text/javascript")
        h = call("HEAD", "/devtools.abc.js")
        self.assertEqual((h.status, h.headers["Content-Length"], h.text), (200, "14", b""))
        for path in ["/../secret.txt", "/%2e%2e/secret.txt", "/nope.js", "/console/../../secret.txt", "/%ff"]:
            self.assertEqual(call("GET", path).status, 404, path)
        self.assertEqual(post("/", {}).status, 404)
        self.assertIn("tables", call("GET", "/console/schema").body)


class Files(unittest.TestCase):
    """The build's files over kept-alive connections, as a browser loads
    a page's chunks, from a server of their own."""

    @classmethod
    def setUpClass(cls):
        cls.dist = Path(tempfile.mkdtemp(dir=DIR, prefix="files-"))
        (cls.dist / "devtools.html").write_text("<!doctype html>")
        (cls.dist / "small.js").write_text("console.log(1)")
        cls.console = server.Console(POOL, regenerations=REGS, dist=cls.dist, suite=SUITE)
        cls.httpd = cls.console.listen(0, "127.0.0.1")
        threading.Thread(target=cls.httpd.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.httpd.shutdown()
        cls.httpd.server_close()

    def connect(self):
        c = http.client.HTTPConnection("127.0.0.1", self.httpd.server_address[1], timeout=10)
        self.addCleanup(c.close)
        return c

    def test_small_files_one_after_another_wait_for_no_ack(self):
        c = self.connect()
        took = []
        for _ in range(30):
            t = time.monotonic()
            c.request("GET", "/small.js")
            self.assertEqual(c.getresponse().read(), b"console.log(1)")
            took.append(time.monotonic() - t)
        # With Nagle's algorithm each body waits for its headers' delayed ACK (40 ms).
        self.assertLess(sorted(took)[len(took) // 2], 0.02)

    def test_a_file_truncated_while_it_is_sent_ends_its_connection(self):
        big = self.dist / "truncated.js"
        big.write_bytes(b"a" * (64 << 20))
        c = self.connect()
        c.request("GET", "/truncated.js")
        r = c.getresponse()
        self.assertEqual(r.headers["Content-Length"], str(64 << 20))
        r.read(1 << 20)
        with big.open("r+b") as f:
            f.truncate(2 << 20)
        # The rest stops short and the connection closes: the client isn't left waiting.
        with self.assertRaises(http.client.IncompleteRead):
            r.read()

    def test_a_file_grown_while_it_is_sent_keeps_to_its_length(self):
        big = self.dist / "grown.js"
        big.write_bytes(b"a" * (16 << 20))
        c = self.connect()
        c.request("GET", "/grown.js")
        r = c.getresponse()
        first = r.read(1 << 20)
        with big.open("ab") as f:
            f.write(b"b" * (1 << 20))
        self.assertEqual(first + r.read(), b"a" * (16 << 20))
        # The next response on the connection is whole.
        c.request("GET", "/small.js")
        self.assertEqual(c.getresponse().read(), b"console.log(1)")


class HostAndOrigin(unittest.TestCase):
    def test_only_this_machines_names_and_origins_are_answered(self):
        for host, status in [(f"127.0.0.1:{PORT}", 200), (f"localhost:{PORT}", 200), ("LOCALHOST", 200),
                             (f"[::1]:{PORT}", 200), ("evil.example", 403), (f"evil.example:{PORT}", 403),
                             ("localhost.evil.example", 403), ("127.0.0.1.nip.io", 403), ("[::1]x", 403), ("", 403)]:
            self.assertEqual(call("GET", "/console/views", headers={"Host": host}).status, status, host)
        for origin, status in [("http://localhost:8080", 200), ("https://localhost:8080", 200),
                               ("https://127.0.0.1", 200), ("http://[::1]:3000", 200), ("http://evil.example", 403),
                               ("http://localhost.evil.example", 403), ("null", 403), ("file://", 403),
                               ("ftp://localhost", 403), ("http://localhost/x", 403)]:
            self.assertEqual(call("GET", "/console/views", headers={"Origin": origin}).status, status, origin)
        r = post("/console/sql", {"sql": "SELECT 1"}, headers={"Origin": "http://evil.example"})
        self.assertEqual(r.status, 403)
        self.assertRegex(r.body["error"], "--allow-origin")

    def test_the_flags_admit_others(self):
        other = server.Console(POOL, regenerations=REGS, dist=DIR / "dist", suite=SUITE,
                               hosts=["Box.lan"], origins=["https://192.168.1.5:8080/"])

        class H:
            def __init__(self, headers):
                self.headers = headers
        for headers, ok in [({"Host": "box.lan:8086"}, True), ({"Host": "other.lan"}, False),
                            ({"Host": "localhost", "Origin": "https://192.168.1.5:8080"}, True),
                            ({"Host": "localhost", "Origin": "https://192.168.1.5:8081"}, False),
                            ({"Host": "box.lan", "Origin": "http://box.lan:8080"}, True)]:
            try:
                other.check_origin(H(headers))
                got = True
            except server.HttpError as e:
                self.assertEqual(e.status, 403)
                got = False
            self.assertEqual(got, ok, headers)

    def test_a_lan_address_and_the_machines_names_pass_other_names_dont(self):
        lan = server.lan_addresses()
        if not lan:
            self.skipTest("no IPv4 address but loopback's")
        ip, name = lan[0], socket.gethostname()
        for addr, headers, status in [(ip, {}, 200), (ip, {"Host": "some.other.name"}, 403),
                                      (ip, {"Origin": f"http://{ip}:8086"}, 200),
                                      ("127.0.0.1", {"Origin": f"http://{ip}:8086"}, 200),
                                      ("127.0.0.1", {"Host": f"{name}.local:{PORT}"}, 200),
                                      ("127.0.0.1", {"Host": name.upper()}, 200),
                                      ("127.0.0.1", {"Host": f"{name}.evil.example"}, 403),
                                      ("127.0.0.1", {"Origin": "http://some.other.name:8086"}, 403)]:
            self.assertEqual(status_at(addr, PORT, headers), status, (addr, headers))

    def test_the_ipv6_address_connected_to_passes_in_brackets(self):
        if not socket.has_dualstack_ipv6():
            self.skipTest("no dual-stack IPv6")
        try:
            with socket.socket(socket.AF_INET6, socket.SOCK_DGRAM) as s:
                s.connect(("2001:db8::1", 9))
                v6 = s.getsockname()[0]
        except OSError:
            self.skipTest("no IPv6 route")
        # Not a name the console knew: the request's own address admits it.
        self.assertNotIn(f"[{v6}]", CONSOLE.hosts)
        self.assertEqual(status_at(v6, PORT), 200)
        self.assertEqual(status_at(v6, PORT, {"Origin": f"http://[{v6}]:3000"}), 200)
        self.assertEqual(status_at(v6, PORT, {"Host": f"[2001:db8::2]:{PORT}"}), 403)
        self.assertEqual(status_at("::1", PORT), 200)

    def test_a_server_on_loopback_refuses_the_network(self):
        lan = server.lan_addresses()
        if not lan:
            self.skipTest("no IPv4 address but loopback's")
        other = server.Console(POOL, regenerations=REGS, dist=DIR / "dist", suite=SUITE)
        httpd = other.listen(0, "127.0.0.1")
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        try:
            port = httpd.server_address[1]
            with self.assertRaises(ConnectionRefusedError):
                socket.create_connection((lan[0], port), timeout=10).close()
            self.assertEqual(status_at("127.0.0.1", port), 200)
        finally:
            httpd.shutdown()
            httpd.server_close()


class TabReports(unittest.TestCase):
    def setUp(self):
        self.now = [1000.0]
        saved = CONSOLE.tabs
        CONSOLE.tabs = server.Tabs(clock=lambda: self.now[0])
        self.addCleanup(setattr, CONSOLE, "tabs", saved)

    def put(self, id, url, focused=False, visible=True, at=None, **kw):
        if at is not None:
            self.now[0] = at
        return call("PUT", f"/console/tabs/{id}", {"url": url, "focused": focused, "visible": visible}, **kw)

    def listed(self, at=None):
        if at is not None:
            self.now[0] = at
        r = call("GET", "/console/tabs")
        self.assertEqual(r.status, 200, r.body)
        return r.body

    def test_reports_are_kept_and_the_latest_focused_first(self):
        page = "http://127.0.0.1:8086/devtools.html#/console/"
        r = self.put("a", page + "tree", focused=True, at=1000)
        self.assertEqual((r.status, r.body), (200, {"id": "a", "url": page + "tree", "focused": True, "visible": True,
                                                    "last_seen": 0, "focused_ago": 0}))
        self.put("b", page + "scorecard", at=1010)
        self.put("c", page + "run_timeline", focused=True, at=1020)
        self.assertEqual([t["id"] for t in self.listed()], ["c", "a", "b"])
        # A tab focused again goes first; once it loses focus it stays the last focused.
        self.put("a", page + "tree?exp=e1", focused=True, at=1030)
        self.put("a", page + "tree?exp=e1&t=45", focused=False, visible=False, at=1040)
        got = self.listed(at=1045)
        self.assertEqual(got, [
            {"id": "a", "url": page + "tree?exp=e1&t=45", "focused": False, "visible": False, "last_seen": 5,
             "focused_ago": 15},
            {"id": "c", "url": page + "run_timeline", "focused": True, "visible": True, "last_seen": 25,
             "focused_ago": 25},
            {"id": "b", "url": page + "scorecard", "focused": False, "visible": True, "last_seen": 35,
             "focused_ago": None}])
        # Tabs never focused follow, the latest seen first.
        self.put("d", page + "playback", at=1050)
        self.assertEqual([t["id"] for t in self.listed()], ["a", "c", "d", "b"])

    def test_a_tab_unseen_for_two_minutes_is_forgotten(self):
        self.put("a", "http://h/devtools.html#/console/tree", focused=True, at=1000)
        self.put("b", "http://h/devtools.html#/console/tree", at=1100)
        self.assertEqual([t["id"] for t in self.listed(at=1120)], ["a", "b"])
        self.assertEqual([t["id"] for t in self.listed(at=1121)], ["b"])
        self.assertEqual(self.listed(at=1221), [])

    def test_reports_take_the_apis_checks(self):
        good = {"url": "http://h/devtools.html#/console/tree", "focused": True, "visible": True}
        for headers, status in [({"Host": "evil.example"}, 403), ({"Origin": "http://evil.example"}, 403),
                                ({"Origin": f"http://localhost:{PORT}"}, 200)]:
            self.assertEqual(call("PUT", "/console/tabs/x", good, headers=headers).status, status, headers)
            self.assertEqual(call("GET", "/console/tabs", headers=headers).status, status, headers)
        self.assertEqual(call("PUT", "/console/tabs/x", json.dumps(good).encode(), type="text/plain").status, 415)
        for id, body in [("a.b", good), ("a" * 65, good), ("x", {**good, "url": None}), ("x", {**good, "focused": 1}),
                         ("x", {"url": "u", "focused": True}), ("x", [good])]:
            with self.subTest(id=id, body=body):
                self.assertEqual(call("PUT", f"/console/tabs/{id}", body).status, 400)
        self.assertEqual([t["url"] for t in self.listed()], [good["url"]])
        self.assertEqual(call("POST", "/console/tabs", good).status, 404)


class State(unittest.TestCase):
    def test_each_server_writes_its_own_state_file_and_removes_it_at_exit(self):
        repo = DIR / "checkout"
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        states = repo / "target" / "console" / "servers"
        env = {**os.environ, "SIM_DB_ROOT": str(DIR), "GIT_DIR": str(repo / ".git")}

        def start():
            before = set(states.glob("*.json"))
            p = subprocess.Popen([sys.executable, str(HERE / "server.py"), "--port", "0", "--host", "127.0.0.1",
                                  "--no-index"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
            found = until(lambda: [f for f in states.glob("*.json") if f not in before], timeout=60)
            self.assertTrue(found, p.stderr.read() if p.poll() is not None else "")
            return p, found[0]

        def stop(p):
            p.send_signal(signal.SIGTERM)
            p.wait(timeout=60)
            p.stderr.close()
            self.assertEqual(p.returncode, 0)

        first, first_state = start()
        try:
            got = json.loads(first_state.read_text())
            port = got["port"]
            self.assertEqual(first_state.name, f"{port}.json")
            self.assertEqual(got, {"pid": first.pid, "port": port, "urls": [f"http://127.0.0.1:{port}/"]})
            self.assertEqual(status_at("127.0.0.1", port), 200)
            # A second server, on another port, leaves the first's file, and removes its own alone.
            second, second_state = start()
            self.assertNotEqual(second_state, first_state)
            self.assertEqual(json.loads(second_state.read_text())["pid"], second.pid)
            stop(second)
            self.assertFalse(second_state.exists())
            self.assertEqual(json.loads(first_state.read_text())["pid"], first.pid)
        finally:
            if first.poll() is None:
                stop(first)
        self.assertFalse(first_state.exists())
        # Another process's file stays.
        first_state.write_text(json.dumps({"pid": 1, "port": 1, "urls": ["http://127.0.0.1:1/"]}))
        server.remove_state(first_state)
        self.assertTrue(first_state.exists())


def status_at(addr, port, headers=None):
    """GET /console/views's status at `addr`, with `headers`."""
    c = http.client.HTTPConnection(addr, port, timeout=30)
    try:
        c.request("GET", "/console/views", headers=headers or {})
        r = c.getresponse()
        r.read()
        return r.status
    finally:
        c.close()


if __name__ == "__main__":
    unittest.main()
