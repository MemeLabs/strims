#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.store's executor against the DuckDB CLI's: scripts' rows, text
and failures as simdb.duck gives them; simdb.py's fold and export of both
sims' jobs (golden_cli's fixture jobs, in batches that add columns and
fold a job again) run both ways into Parquet of the same rows and types
(the same bytes: one DuckDB release writes them);
a fold that fails rolling back in both; `install`:

  python3 hack/simlab/tests/test_store.py
"""

import contextlib
import json
import os
import sys
import tempfile
import unittest
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
        """golden_cli's jobs, written as the sims write them, with the
        speculative requests', parks' and arrivals' tables golden_cli's
        don't write: the ppspp sim's, then strims-sim's (whose viewers add
        host and name), then a job folded again."""
        jobs = [golden_cli.job_dir(self.dir / "jobs" / f"{i:02d}", j) for i, j in enumerate(golden_cli.JOBS)]
        for n, j in enumerate(jobs):
            d = Path(j["dir"])
            golden_cli.write_table(d, "spec_requests", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                        ("held_ms", "BIGINT"), ("held_by_p90", "BOOLEAN")],
                                   [(900 + n, 1, 5, None, None), (950, 2, 6, 1150 + n, True)])
            golden_cli.write_table(d, "parks", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                ("outcome", "VARCHAR")], [(800 + n, 1, 5, "sent")])
            golden_cli.write_table(d, "arrivals", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                   ("how", "VARCHAR"), ("peer", "BIGINT")],
                                   [(1150 + n, 1, 6, "answer", 2)])
        pp = [j for j in jobs if "ppspp" in j["sim"]]
        ss = [j for j in jobs if j["sim"].startswith("strims-sim")]
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
