#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.store's executor against the DuckDB CLI's: scripts' rows, text
and failures as simdb.duck gives them; simdb.py's fold and export of both
sims' jobs (golden_cli's fixture jobs, in batches that add columns and
fold a job again) run both ways into Parquet of the same rows and types
(the same bytes: one DuckDB release writes them);
a fold that fails rolling back in both; `install`; farm.py's constants
read from its source:

  python3 hack/simlab/tests/test_store.py
"""

import contextlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from compression import zstd
from pathlib import Path
from unittest import mock

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import duckdb  # noqa: E402

import golden_cli  # noqa: E402
import simlab  # noqa: E402
from simlab import store, store_host  # noqa: E402

SRC = "suite/wt/parity"


def simdb():
    """simdb_test's simdb, which simlab.simdb() gives too."""
    sm = golden_cli.simdb_test().simdb
    assert simlab.simdb() is sm
    return sm


@contextlib.contextmanager
def executor(fn):
    """simdb.duck running its scripts in `fn` (None: the CLI); yields how
    many scripts `fn` ran."""
    calls = []

    def counted(*args, **kw):
        calls.append(args)
        return fn(*args, **kw)

    with mock.patch.object(store_host, "EXECUTOR", counted if fn else None):
        yield calls


def parquet(path):
    """A Parquet file's (name, type) columns and its rows, in its order."""
    c = duckdb.connect()
    try:
        lit = "'" + str(path).replace("'", "''") + "'"
        cols = [(r[0], r[1]) for r in c.execute(f"DESCRIBE SELECT * FROM read_parquet({lit})").fetchall()]
        return cols, c.execute(f"SELECT * FROM read_parquet({lit})").fetchall()
    finally:
        c.close()


class FarmConstants(unittest.TestCase):
    """store.farm_constant over farm.py sources of every shape a constant
    takes (the reviewer's probe's among them)."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.path = Path(tmp.name) / "farm.py"

    def read(self, text, name="S3_KEPT_DAYS"):
        self.path.write_text(text)
        return store.farm_constant(name, self.path)

    def refused(self, text, name="S3_KEPT_DAYS"):
        self.path.write_text(text)
        with self.assertRaises(ValueError) as cm:
            store.farm_constant(name, self.path)
        return str(cm.exception)

    def test_a_constant_of_any_shape_reads_as_its_number(self):
        for text, want in (("S3_KEPT_DAYS = 7\n", 7),
                           ("S3_KEPT_DAYS = 7  # S3's lifecycle rule\n", 7),
                           ("S3_KEPT_DAYS: int = 7\n", 7),
                           ("S3_KEPT_DAYS = 1_0\n", 10),
                           ("DAY_S = 86400\nRETAIN_S = 6 * DAY_S\nS3_KEPT_DAYS = RETAIN_S // DAY_S + 1\n", 7),
                           ("S3_KEPT_DAYS = (\n    7\n)\n", 7),
                           ("S3_KEPT_DAYS = 3\nS3_KEPT_DAYS = -(-7)\n", 7)):
            with self.subTest(text=text):
                got = self.read(text)
                self.assertEqual((got, type(got)), (want, int))
        self.assertEqual(self.read("S3_TIER1 = 0.0055 / 1000\n", "S3_TIER1"), 0.0055 / 1000)
        self.assertEqual(self.read("GB = 1 << 30\n", "GB"), 1 << 30)

    def test_a_constant_it_cant_read_fails_naming_the_line(self):
        path = self.path
        self.assertEqual(self.refused("DAY_S = 86400\nS3_KEPT_DAYS = RETAIN_S // DAY_S + 1\n"),
                         f"{path}:2: S3_KEPT_DAYS = RETAIN_S // DAY_S + 1: S3_KEPT_DAYS isn't a number the lab can "
                         "read (RETAIN_S isn't a number farm.py defines before it)")
        self.assertEqual(self.refused("import os\nS3_KEPT_DAYS = int(os.environ['DAYS'])\n"),
                         f"{path}:2: S3_KEPT_DAYS = int(os.environ['DAYS']): S3_KEPT_DAYS isn't a number the lab can "
                         "read (int(os.environ['DAYS']) isn't arithmetic of numbers)")
        self.assertEqual(self.refused("S3_KEPT_DAYS = 7 // 0\n"),
                         f"{path}:1: S3_KEPT_DAYS = 7 // 0: S3_KEPT_DAYS isn't a number the lab can read "
                         "(division by zero)")
        self.assertEqual(self.refused("def f():\n    S3_KEPT_DAYS = 7\n"),
                         f"{path}: farm.py assigns no S3_KEPT_DAYS at its top level")
        self.assertTrue(self.refused("S3_KEPT_DAYS = = 7\n").startswith(
            f"{path}: farm.py's S3_KEPT_DAYS can't be read: "))
        self.path.unlink()
        with self.assertRaisesRegex(ValueError, "farm.py's S3_KEPT_DAYS can't be read"):
            store.farm_constant("S3_KEPT_DAYS", self.path)

    def test_the_lab_reads_this_checkouts_farm_py_when_it_needs_a_constant(self):
        self.assertEqual(store.FARM_PY, Path(store.__file__).resolve().parents[1] / "sim-farm" / "farm.py")
        self.assertEqual(store.s3_kept_days(), 7)
        # Importing the lab's modules reads no farm.py.
        p = subprocess.run([sys.executable, "-c", "from simlab import catalog, query, scoring, store, suite, verdict; "
                                                  "print(store._numbers.cache_info().currsize)"],
                           cwd=HACK, capture_output=True, text=True, timeout=120)
        self.assertEqual((p.returncode, p.stdout), (0, "0\n"), p.stderr)


class Scratch(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-store-")
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        # The cache root both executors spill to (its tmp/).
        p = mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.dir / "sim-db")})
        p.start()
        self.addCleanup(p.stop)
        (self.dir / "sim-db").mkdir()


class Execute(Scratch):
    SCRIPTS = [
        "SELECT 1 AS a;",
        "CREATE TABLE t AS SELECT range AS i FROM range(5); INSERT INTO t VALUES (9); "
        "SELECT i, i * 2 AS j FROM t ORDER BY i; SELECT sum(i) AS s, avg(i) AS m, 1.50::DECIMAL(4,2) AS d FROM t;",
        "SELECT 'x' AS s; CREATE TABLE u AS SELECT 2 AS b; INSERT INTO u VALUES (3);",
        "SELECT 'a' AS x, 'bb' AS x, NULL AS y, [1, 2] AS l, {'k': 'v'} AS st, DATE '2026-11-10' AS d;",
        "CREATE TABLE e (i BIGINT); SELECT * FROM e;",
        "SET threads = 1; PRAGMA version;",
        "BEGIN; CREATE TABLE w AS SELECT 1 AS a; COMMIT; SELECT count(*) AS n FROM w;",
    ]

    def test_scripts_give_what_the_cli_gives(self):
        sm = simdb()
        for n, sql in enumerate(self.SCRIPTS):
            for out in ("json", "text", None):
                db = self.dir / f"s{n}-{out}.duckdb"
                with executor(None):
                    want = sm.duck(sql, db=db, out=out, threads=2, memory="1GB")
                with executor(store.execute) as ran:
                    got = sm.duck(sql, db=db.with_suffix(".mod.duckdb"), out=out, threads=2, memory="1GB")
                self.assertEqual(len(ran), 1)
                if out == "text":
                    # PRAGMA version's source id
                    want, got = (x.replace(duckdb.__version__, "V") for x in (want, got))
                self.assertEqual(json.dumps(got), json.dumps(want), (sql, out))
        with executor(None):
            want = sm.duck("SELECT sum(range) AS s FROM range(10);")
        self.assertEqual(want, [{"s": 45}])
        self.assertEqual(store.execute("SELECT sum(range) AS s FROM range(10);"), want)

    def test_a_failure_is_a_duck_error_and_rolls_back(self):
        sm = simdb()
        for name, fn in (("cli", None), ("module", store.execute)):
            db = self.dir / f"{name}.duckdb"
            with executor(fn):
                sm.duck("CREATE TABLE t (i BIGINT); INSERT INTO t VALUES (1);", db=db, out=None)
                with self.assertRaises(sm.DuckError) as cm:
                    sm.duck("BEGIN; INSERT INTO t VALUES (2); INSERT INTO t VALUES ('x'); COMMIT;", db=db, out=None)
                self.assertIn("Conversion Error", str(cm.exception), name)
                self.assertEqual(sm.duck("SELECT i FROM t;", db=db), [{"i": 1}], name)
                with self.assertRaises(sm.DuckError):
                    sm.duck("SELECT nope FROM t;", db=db)

    def test_install_runs_duck_in_the_module(self):
        sm = simdb()
        with mock.patch.object(store_host, "EXECUTOR", None):
            store.install()
            self.assertIs(store_host.EXECUTOR, store.execute)
            with mock.patch.object(store_host, "duckdb_bin", lambda fetch=True: self.fail("ran the CLI")):
                self.assertEqual(sm.duck("SELECT 2 AS b;"), [{"b": 2}])


class FoldExport(Scratch):
    def jobs(self):
        """golden_cli's jobs, written as the sim writes them, with the
        meter samples', speculative requests', parks', arrivals', asks',
        haves', the seed log's, the peer samples', a testbed run's sockets',
        the packets', the hosts', the parent checks', the timers' and the
        link log's tables golden_cli's don't write: the suite's, then the
        farm's (whose reads, asks and haves are zstd-compressed,
        `<table>.csv.zst`, as the sim writes its large tables, and the seed
        log's, the peer samples', the sockets', the packets', the parent
        checks', the timers' and the link log's too, so the fold takes the
        tables registered last in either form), then a job folded again."""
        jobs = [golden_cli.job_dir(self.dir / "jobs" / f"{i:02d}", j) for i, j in enumerate(golden_cli.JOBS)]
        for n, j in enumerate(jobs):
            d = Path(j["dir"])
            # The seed's meter reading and a viewer's not read yet.
            golden_cli.write_table(d, "meter_samples", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("role", "VARCHAR"),
                                                        ("capacity_bps", "BIGINT"), ("avail_bps", "BIGINT")],
                                   [(1000 + n, 0, "seed", 1_200_000, 1_250_000), (1000 + n, 1, "honest", None, 375_000)])
            golden_cli.write_table(d, "spec_requests", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                        ("held_ms", "BIGINT"), ("held_by_p90", "BOOLEAN")],
                                   [(900 + n, 1, 5, None, None), (950, 2, 6, 1150 + n, True)])
            golden_cli.write_table(d, "parks", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                ("outcome", "VARCHAR")], [(800 + n, 1, 5, "sent")])
            golden_cli.write_table(d, "arrivals", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                   ("how", "VARCHAR"), ("peer", "BIGINT")],
                                   [(1150 + n, 1, 6, "answer", 2)])
            golden_cli.write_table(d, "asks", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("ask", "BIGINT"),
                                               ("round", "VARCHAR"), ("picked", "BOOLEAN"), ("peer", "BIGINT"),
                                               ("seq", "BIGINT")],
                                   [(900 + n, 1, 0, "flight", True, 2, 6)])
            golden_cli.write_table(d, "haves", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                ("seq", "BIGINT"), ("peer_incarnation", "BIGINT")],
                                   [(890 + n, 1, 2, 6, 0)])
            golden_cli.write_table(d, "seed_decisions", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                         ("kind", "VARCHAR"), ("stream", "BIGINT"),
                                                         ("granted", "BOOLEAN")],
                                   [(880 + n, 0, 1, "request", 2, False), (881 + n, 0, -1, "window", None, None)])
            golden_cli.write_table(d, "seed_sends", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                     ("how", "VARCHAR"), ("seq", "BIGINT"), ("fate", "VARCHAR")],
                                   [(885 + n, 0, 1, "push", 6, "written")])
            # The seed's channel to a viewer, and one to a peer it can't name.
            golden_cli.write_table(d, "peer_samples", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                       ("sent_bytes", "BIGINT"), ("rtt_ms", "BIGINT")],
                                   [(1000 + n, 0, 1, 40_960, 31), (1000 + n, 0, None, 0, 0)])
            golden_cli.write_table(d, "sockets", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                  ("rtt_us", "BIGINT"), ("notsent_bytes", "BIGINT")],
                                   [(1000 + n, 0, 1, 31_250, None)])
            # A packet the seed's link carried and one its queue dropped.
            golden_cli.write_table(d, "packets", [("node", "BIGINT"), ("n", "BIGINT"), ("t_us", "BIGINT"),
                                                  ("sent_us", "BIGINT"), ("lost", "VARCHAR"), ("kind", "VARCHAR")],
                                   [(0, 0, 1000 + n, 1200 + n, None, "data"), (0, 1, 1001 + n, None, "queue", "ack")])
            golden_cli.write_table(d, "hosts", [("node", "BIGINT"), ("name", "VARCHAR"), ("role", "VARCHAR"),
                                                ("link_spec", "VARCHAR")],
                                   # As the sim quotes a field with a comma.
                                   [(0, "seed", "seed", '"cable,aqm=fifo"'), (1, "v0", "viewer", None)])
            # A viewer's graft onto the seed, and a check with no candidate.
            golden_cli.write_table(d, "parent_checks", [("t_us", "BIGINT"), ("node", "BIGINT"), ("stream", "BIGINT"),
                                                        ("action", "VARCHAR"), ("chosen", "BIGINT"),
                                                        ("best_lag_us", "BIGINT")],
                                   [(343_000 + n, 1, 0, "graft", 0, 12_000), (343_000 + n, 1, 1, "none", None, None)])
            # The seed's node task's timer and its writer's, a millisecond late.
            golden_cli.write_table(d, "timers", [("t_us", "BIGINT"), ("node", "BIGINT"), ("n", "BIGINT"),
                                                 ("source", "VARCHAR"), ("armed_us", "BIGINT"),
                                                 ("deadline_us", "BIGINT"), ("late_us", "BIGINT"),
                                                 ("requested_us", "BIGINT"), ("context", "VARCHAR")],
                                   [(1_001_000 + n, 0, 0, "node_task", 990_000, 1_000_000 + n, 1000 - n, 10_000,
                                     "task"),
                                    (1_251_000, 0, 1, "writer", 1_000_000, 1_250_000, 1000, 250_000, "task")])
            # The seed's write to node 1 on their first link, its viewer's read
            # of it, and the seed's reading of the link's queues.
            golden_cli.write_table(d, "link_writes", [("t_us", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                      ("link", "BIGINT"), ("kind", "VARCHAR"), ("off", "BIGINT"),
                                                      ("n", "BIGINT"), ("bytes", "BIGINT"), ("queued_us", "BIGINT"),
                                                      ("done_us", "BIGINT"), ("unsent_before", "BIGINT"),
                                                      ("wait", "VARCHAR"), ("rec_kind", "VARCHAR"),
                                                      ("seq", "BIGINT")],
                                   [(1_000_200 + n, 0, 1, 0, "webrtc", 0, 0, 1200, 1_000_000, 1_000_300, 900,
                                     "channel", None, None)])
            golden_cli.write_table(d, "link_reads", [("t_us", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                     ("link", "BIGINT"), ("kind", "VARCHAR"), ("off", "BIGINT"),
                                                     ("n", "BIGINT"), ("bytes", "BIGINT"), ("taken_us", "BIGINT"),
                                                     ("rec_kind", "VARCHAR"), ("seq", "BIGINT")],
                                   [(1_020_000 + n, 1, 0, 0, "webrtc", 0, 0, 1200, 1_020_400, None, None)])
            golden_cli.write_table(d, "link_samples", [("t_us", "BIGINT"), ("node", "BIGINT"), ("peer", "BIGINT"),
                                                       ("link", "BIGINT"), ("source", "VARCHAR"),
                                                       ("cause", "VARCHAR"), ("link_pending_bytes", "BIGINT"),
                                                       ("write_held", "BIGINT"), ("qos_backlog_bytes", "BIGINT")],
                                   [(1_000_000 + n, 0, 1, 0, "queues", "sample", 4096, 2, None)])
            if ":" in j["sim"]:
                for t in ("reads", "asks", "haves", "seed_decisions", "seed_sends", "peer_samples", "sockets",
                          "packets", "parent_checks", "timers", "link_writes",
                          "link_reads", "link_samples"):
                    csv = d / f"{t}.csv"
                    (d / f"{t}.csv.zst").write_bytes(zstd.compress(csv.read_bytes()))
                    csv.unlink()
        pp = [j for j in jobs if ":" not in j["sim"]]
        ss = [j for j in jobs if ":" in j["sim"]]
        self.assertTrue(pp and ss)
        return [pp, ss, [dict(pp[0], wall_s=9.5)]]

    def fold_export(self, name, fn, batches):
        sm = simdb()
        db = self.dir / f"{name}.duckdb"
        with executor(fn) as ran:
            wrote = [sm.fold(db, batch, SRC) for batch in batches]
            manifest = sm.export(db, self.dir / name)
        if fn:
            self.assertGreater(len(ran), len(batches))
        return wrote, manifest

    def test_fold_and_export_are_the_clis(self):
        batches = self.jobs()
        want = self.fold_export("cli", None, batches)
        got = self.fold_export("module", store.execute, batches)
        self.assertEqual(got[0], want[0])
        (_, wm), (_, gm) = want, got
        # One DuckDB release writes the same files, byte for byte.
        self.assertEqual(dict(gm, at=None), dict(wm, at=None))
        self.assertEqual(set(wm["tables"]), {"runs", "scalars", *simdb().TABLES})
        for t in wm["tables"]:
            cols, rows = parquet(self.dir / "cli" / f"{t}.parquet")
            self.assertEqual(parquet(self.dir / "module" / f"{t}.parquet"), (cols, rows), t)
            self.assertTrue(rows, t)
        cols, rows = parquet(self.dir / "module" / "viewers.parquet")
        self.assertIn(("host", "BIGINT"), cols)
        runs = dict(zip([c for c, _ in parquet(self.dir / "module" / "runs.parquet")[0]],
                        zip(*parquet(self.dir / "module" / "runs.parquet")[1])))
        self.assertEqual(sorted(runs["k"]), sorted({j["k"] for b in batches for j in b}))
        self.assertIn(9.5, runs["wall_s"])
        # Each job's asks row, the compressed ones' too.
        cols, rows = parquet(self.dir / "module" / "asks.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(runs["k"]))
        cols, rows = parquet(self.dir / "module" / "haves.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(runs["k"]))
        # The seed log's: two decisions and a send a job.
        cols, rows = parquet(self.dir / "module" / "seed_decisions.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(2 * list(runs["k"])))
        cols, rows = parquet(self.dir / "module" / "seed_sends.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(runs["k"]))
        # Two peer samples and a socket a job, a peer left out NULL.
        cols, rows = parquet(self.dir / "module" / "peer_samples.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(2 * list(runs["k"])))
        self.assertIn(None, [r[[c for c, _ in cols].index("peer")] for r in rows])
        cols, rows = parquet(self.dir / "module" / "sockets.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(runs["k"]))
        # Two packets and two hosts a job, a link spec's commas kept.
        cols, rows = parquet(self.dir / "module" / "packets.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(2 * list(runs["k"])))
        cols, rows = parquet(self.dir / "module" / "hosts.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(2 * list(runs["k"])))
        self.assertIn("cable,aqm=fifo", [r[[c for c, _ in cols].index("link_spec")] for r in rows])
        # Two parent checks a job, a check that chose no one NULL.
        cols, rows = parquet(self.dir / "module" / "parent_checks.parquet")
        self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(2 * list(runs["k"])))
        self.assertIn(None, [r[[c for c, _ in cols].index("chosen")] for r in rows])
        # Two timers a job, each its own (k, node, n).
        cols, rows = parquet(self.dir / "module" / "timers.parquet")
        names = [c for c, _ in cols]
        self.assertEqual(sorted(r[names.index("k")] for r in rows), sorted(2 * list(runs["k"])))
        self.assertEqual(len({(r[names.index("k")], r[names.index("node")], r[names.index("n")]) for r in rows}),
                         len(rows))
        # A row of each link table a job.
        for t in ("link_writes", "link_reads", "link_samples"):
            cols, rows = parquet(self.dir / "module" / f"{t}.parquet")
            self.assertEqual(sorted(r[[c for c, _ in cols].index("k")] for r in rows), sorted(runs["k"]), t)

    def test_an_empty_store_exports_nothing(self):
        sm = simdb()
        for name, fn in (("cli", None), ("module", store.execute)):
            db = self.dir / f"{name}.duckdb"
            with executor(fn):
                self.assertEqual(sm.fold(db, [], SRC), {})
                sm.duck("CREATE TABLE runs (k VARCHAR);", db=db, out=None)
                self.assertIsNone(sm.export(db, self.dir / name))
                self.assertEqual(sm.tables_in(db), ["runs"])


if __name__ == "__main__":
    unittest.main()
