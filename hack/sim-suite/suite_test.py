#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests of suite.py's scoring, the catalog's scoring macros
(hack/sim-db/scoring.sql), over DuckDB files made from fixtures:

  Golden      testdata/golden/ (golden.py fixtures): cells of the latest
              farm experiments and a made-up one, their runs' tables and
              what the Python scoring before the macros printed over them;
              suite.py prints the same, from its sides and from arm_side's.
  Arithmetic  fisher_p against the exact hypergeometric sum, t975_at's
              lookup, effect_mark.
  Dynamics    structural and dynamics over a made-up experiment.

  python3 hack/sim-suite/suite_test.py [-v]
"""

import gzip
import importlib.util
import json
import math
import re
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIXTURES = HERE / "testdata" / "golden"


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


suite = load(HERE / "suite.py", "suite")
golden = load(HERE / "golden.py", "golden")
golden._suite = suite


# The tables the scoring objects read, empty where a test's own SQL makes
# none.
TABLES = """
CREATE SCHEMA IF NOT EXISTS raw;
CREATE TABLE IF NOT EXISTS raw.runs (k VARCHAR, src VARCHAR, cell VARCHAR, arm VARCHAR, seed BIGINT, trace VARCHAR,
    frames BIGINT, bytes BIGINT, wall_s DOUBLE, rss_mb DOUBLE, "at" VARCHAR);
CREATE TABLE IF NOT EXISTS rows (k VARCHAR, src VARCHAR);
CREATE TABLE IF NOT EXISTS verdicts (k VARCHAR, src VARCHAR, collapsed BOOLEAN);
CREATE TABLE IF NOT EXISTS jobs (k VARCHAR, src VARCHAR, trace VARCHAR, reads_hash VARCHAR);
CREATE TABLE IF NOT EXISTS experiment_jobs (exp_id VARCHAR, cell VARCHAR, arm_label VARCHAR, seed BIGINT, k VARCHAR,
    src VARCHAR, flush VARCHAR, matrix VARCHAR);
CREATE TABLE IF NOT EXISTS experiment_cells (exp_id VARCHAR, cell VARCHAR, why VARCHAR, matrix VARCHAR);
CREATE TABLE IF NOT EXISTS viewers (k VARCHAR, src VARCHAR, viewer BIGINT, arrive_ms BIGINT, hostile BOOLEAN);
CREATE TABLE IF NOT EXISTS viewer_seconds (k VARCHAR, src VARCHAR, viewer BIGINT, sec BIGINT, lag_ms DOUBLE,
    frames BIGINT);
CREATE TABLE IF NOT EXISTS verdict_seconds (k VARCHAR, src VARCHAR, sec BIGINT, phase VARCHAR);
CREATE TABLE IF NOT EXISTS episodes (k VARCHAR, src VARCHAR, kind VARCHAR, len BIGINT);
"""


def database(path, sql):
    """A DuckDB file at `path` made by `sql` (and TABLES); suite.py scores
    over it."""
    suite.simdb().duck(sql + TABLES, db=path, out=None)
    suite.SCORE_DB = path


def scored(sql):
    """The rows of `sql` over the scoring objects."""
    return suite.score_query(sql)


class Golden(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.tmp.name) / "golden.duckdb"
        q = lambda t: suite.q(str(FIXTURES / f"{t}.parquet"))
        cls.sql = (f"CREATE SCHEMA raw; CREATE TABLE raw.runs AS FROM read_parquet({q('runs')});\n"
                   + "".join(f"CREATE TABLE {t} AS FROM read_parquet({q(t)});\n"
                             for t in ("rows", "verdicts", "jobs", "experiment_jobs", "experiment_cells")))
        cls.cases = json.loads(gzip.decompress((FIXTURES / "golden.json.gz").read_bytes()))["cases"]

    @classmethod
    def tearDownClass(cls):
        suite.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        if not self.db.exists():
            database(self.db, self.sql)
        suite.SCORE_DB = self.db

    def check(self, case, sides):
        got = golden.texts(suite, case["cells"], sides[case["base"]], sides[case["arm"]], case["exp"], case["base"],
                           case["arm"])
        for name, want in case["want"].items():
            with self.subTest(case=case["exp"], text=name):
                self.assertEqual(got[name].splitlines(), want.splitlines())

    def test_sides(self):
        """suite.py's sides, as it reads them from the farm's indexes."""
        for case in self.cases:
            sides = {arm: {(c, seed): (k, src) for c, seed, k, src in side} for arm, side in case["sides"].items()}
            self.check(case, sides)

    def test_arm_side(self):
        """The experiments' arms as the catalog's experiment_jobs has them."""
        for case in self.cases:
            names = {c["name"] for c in case["cells"]}
            sides = {}
            for arm in (case["base"], case["arm"]):
                rows = scored(f"SELECT unnest(arm_side({suite.q(case['exp'])}, {suite.q(arm)}), recursive := true);")
                sides[arm] = {(r["cell"], r["seed"]): (r["k"], r["src"]) for r in rows if r["cell"] in names}
            self.check(case, sides)

    def test_cases(self):
        """The fixtures hold farm runs of both kinds, a broken guardrail
        and collapses."""
        text = "".join(t for c in self.cases for t in c["want"].values())
        self.assertGreaterEqual(len(self.cases), 6)
        self.assertIn("collapsed", text)
        self.assertRegex(text, r"guardrails (?!ok)")
        self.assertIn(" [0.00,inf]", text)
        self.assertIn("[-inf,+inf]", text)


class Arithmetic(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.tmp.name) / "empty.duckdb"
        database(cls.db, "CREATE SCHEMA raw; CREATE TABLE raw.runs (k VARCHAR, src VARCHAR, cell VARCHAR, arm VARCHAR, "
                         "seed BIGINT, trace VARCHAR, frames BIGINT, bytes BIGINT, wall_s DOUBLE, rss_mb DOUBLE, "
                         "\"at\" VARCHAR); CREATE TABLE rows (k VARCHAR, src VARCHAR); "
                         "CREATE TABLE verdicts (k VARCHAR, src VARCHAR, collapsed BOOLEAN); "
                         "CREATE TABLE jobs (k VARCHAR, src VARCHAR, trace VARCHAR, reads_hash VARCHAR); "
                         "CREATE TABLE experiment_jobs (exp_id VARCHAR, cell VARCHAR, arm_label VARCHAR, seed BIGINT, "
                         "k VARCHAR, src VARCHAR, flush VARCHAR, matrix VARCHAR); "
                         "CREATE TABLE experiment_cells (exp_id VARCHAR, cell VARCHAR, why VARCHAR, matrix VARCHAR);")

    @classmethod
    def tearDownClass(cls):
        suite.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        suite.SCORE_DB = self.db

    def test_fisher(self):
        """fisher_p is the exact two-sided sum, near enough that no p
        crosses 0.05 or prints otherwise."""
        def exact(k, n, m, n2):
            total, hits = n + n2, k + m
            prob = lambda x: math.comb(n, x) * math.comb(n2, hits - x) / math.comb(total, hits)
            seen = prob(k)
            return min(1.0, sum(prob(x) for x in range(max(0, hits - n2), min(n, hits) + 1)
                                if prob(x) <= seen * (1 + 1e-9)))
        cases = [(k, n, m, n) for n in (4, 8, 32) for k in range(n + 1) for m in range(n + 1)] + [(3, 8, 9, 18)]
        values = ", ".join(f"({k}, {n}, {m}, {n2})" for k, n, m, n2 in cases)
        got = scored(f"SELECT k, n, m, n2, fisher_p(k, n, m, n2) AS p FROM (VALUES {values}) v(k, n, m, n2);")
        self.assertEqual(len(got), len(cases))
        for r in got:
            want = exact(r["k"], r["n"], r["m"], r["n2"])
            self.assertAlmostEqual(r["p"], want, delta=1e-12 * max(1.0, want))
            self.assertEqual(r["p"] < 0.05, want < 0.05)
            self.assertEqual(f"{r['p']:.2f}", f"{want:.2f}")

    def test_t975(self):
        got = {r["df"]: r["t"] for r in scored(
            "SELECT df, t975_at(df) AS t FROM range(1, 40) r(df);")}
        self.assertEqual(got[1], 12.71)
        self.assertEqual(got[16], 2.12)
        self.assertEqual(got[17], 2.04)
        self.assertEqual(got[20], 2.09)
        self.assertEqual(got[29], 2.04)
        self.assertEqual(got[30], 2.04)
        self.assertEqual(got[31], 2.0)

    def test_mark(self):
        cases = [("lower", "ratio", 0.05, 0.9, 0.85, 0.95, "+"), ("lower", "ratio", 0.05, 0.97, 0.95, 0.99, " "),
                 ("higher", "ratio", 0.05, 0.9, 0.85, 0.95, "-"), (None, "ratio", 0.05, 1.2, 1.1, 1.3, "~"),
                 ("lower", "ratio", 0.05, 1.2, 0.9, 1.5, " "), ("lower", "diff", 0.5, 1.0, 0.2, 1.8, "-"),
                 ("higher", "diff", 0.5, 1.0, 0.2, 1.8, "+"), ("lower", "diff", 0.5, 0.4, 0.2, 0.6, " "),
                 ("lower", "diff", 0.5, -1.0, float("-inf"), float("inf"), " ")]
        lit = lambda x: "NULL" if x is None else suite.q(x) if isinstance(x, str) else f"'{x}'::DOUBLE"
        values = ", ".join(f"({i}, {', '.join(lit(x) for x in c[:6])})" for i, c in enumerate(cases))
        got = scored(f"SELECT i, effect_mark(b, kind, tie, x, lo, hi) AS m FROM (VALUES {values}) "
                     f"v(i, b, kind, tie, x, lo, hi) ORDER BY i;")
        self.assertEqual([r["m"] for r in got], [c[6] for c in cases])


class Lake(unittest.TestCase):
    def test_persisted(self):
        """The scoring objects as the catalog's index keeps them: a DuckLake
        that holds them reads back, read-only, each of them working."""
        with tempfile.TemporaryDirectory() as tmp:
            path = suite.q(f"ducklake:sqlite:{tmp}/catalog.sqlite")
            cat = suite.catalog()
            try:
                suite.simdb().duck("INSTALL ducklake; INSTALL sqlite; LOAD ducklake; LOAD sqlite; "
                                   f"ATTACH {path} AS lake (DATA_PATH {suite.q(tmp + '/data/')}); USE lake;\n"
                                   + TABLES + cat.scoring_sql(cat.scoring_cols({}), suite=suite.scoring()), out=None)
            except suite.simdb().DuckError as e:
                if "ducklake" in str(e).lower() and "extension" in str(e).lower():
                    self.skipTest(f"no ducklake extension: {e}")
                raise
            read = f"LOAD ducklake; LOAD sqlite; ATTACH {path} AS lake (READ_ONLY); USE lake;\n"
            names = {r["n"] for r in suite.simdb().duck(
                read + "SELECT function_name AS n FROM duckdb_functions() WHERE database_name = 'lake';")}
            want = set(re.findall(r"^CREATE OR REPLACE MACRO ([a-z_0-9]+)", cat.SCORING.read_text(), re.M))
            self.assertEqual(want - names, set())
            self.assertIn("job_metrics_of", names)
            got = suite.simdb().duck(read + """
SELECT (SELECT count(*) FROM metric_defs) AS defs, (SELECT count(*) FROM job_metrics) AS values,
       (SELECT count(*) FROM effects('e', 'a', 'e', 'b')) AS effects,
       (SELECT collapsed FROM arm_score('e', 'a', 'e', 'b')) AS collapsed,
       (SELECT count(*) FROM arm_score('e', 'a', 'e', 'b', overhead := true, lag_budget := 1.05)) AS overhead,
       (SELECT count(*) FROM dynamics('e')) AS dynamics, (SELECT count(*) FROM structural('e')) AS structural,
       fisher_p(1, 8, 6, 8) AS p;""")
            self.assertEqual(got, [{"defs": len(suite.METRICS), "values": 0, "effects": 0, "collapsed": 0,
                                    "overhead": 1, "dynamics": 0, "structural": 0, "p": got[0]["p"]}])
            self.assertAlmostEqual(got[0]["p"], 0.040559, places=6)


class Dynamics(unittest.TestCase):
    """A made-up experiment `dyn`, arm x: cell c's seed 1 (k1) and seed 2
    (k2, collapsed), cell d's seeds both collapsed."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.tmp.name) / "dyn.duckdb"
        vs = []
        for sec in range(30):
            # k1, viewer 0 (there from the start): a startup spike, then
            # 2000, nothing read (3000 carried) and 5000 in steady seconds.
            lag0 = {3: 3000, 20: 2000, 21: None, 22: 5000}.get(sec, 100)
            vs.append(("k1", 0, sec, lag0, 1))
            # k1, viewer 1 (arriving at 5 s, reading from 7 s: its startup
            # until 17 s), a spike of 6000 within it.
            if sec >= 5:
                vs.append(("k1", 1, sec, None if sec < 7 else 6000 if sec == 8 else 200, 0 if sec < 7 else 1))
            vs.append(("k1", 2, sec, 9000, 1))
            vs.append(("k2", 0, sec, 4500 if sec == 25 else 100, 1))
            for k in ("k3", "k4"):
                vs.append((k, 0, sec, 100, 1))
        phase = lambda k, sec: ("startup" if sec < 15 else "after_down" if k == "k2" and sec >= 20 else "steady")
        lit = lambda x: "NULL" if x is None else repr(x)
        sql = [
            "CREATE SCHEMA raw;",
            "CREATE TABLE raw.runs (k VARCHAR, src VARCHAR, cell VARCHAR, arm VARCHAR, seed BIGINT, trace VARCHAR, "
            "frames BIGINT, bytes BIGINT, wall_s DOUBLE, rss_mb DOUBLE, \"at\" VARCHAR);",
            "CREATE TABLE jobs AS SELECT 'k' || i AS k, 's' AS src, NULL::VARCHAR AS trace, "
            "NULL::VARCHAR AS reads_hash FROM range(1, 5) r(i);",
            "CREATE TABLE experiment_jobs AS SELECT * FROM (VALUES ('dyn', 'c', 'x', 1, 'k1', 's', 's', 'dyn-00'), "
            "('dyn', 'c', 'x', 2, 'k2', 's', 's', 'dyn-00'), ('dyn', 'd', 'x', 1, 'k3', 's', 's', 'dyn-01'), "
            "('dyn', 'd', 'x', 2, 'k4', 's', 's', 'dyn-01')) v(exp_id, cell, arm_label, seed, k, src, flush, matrix);",
            "CREATE TABLE experiment_cells AS SELECT * FROM (VALUES ('dyn', 'c', 'c', 'dyn-00'), ('dyn', 'd', 'd', 'dyn-01')) "
            "v(exp_id, cell, why, matrix);",
            "CREATE TABLE verdicts AS SELECT * FROM (VALUES ('k1', 's', false), ('k2', 's', true), ('k3', 's', true), "
            "('k4', 's', true)) v(k, src, collapsed);",
            "CREATE TABLE rows AS SELECT * FROM (VALUES ('k1', 's', 2.0, 0.5, 0.01, 0.0, 900.0, 4000.0), "
            "('k2', 's', 4.0, 1.5, 0.03, 0.02, 1100.0, 4200.0), ('k3', 's', 0.0, 0.0, 0.0, 0.0, 0.0, 0.0), "
            "('k4', 's', 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)) v(k, src, stalls_per_min_1250, stalls_per_min_4250, "
            "stalled_share_1250, stalled_share_4250, startup_ms_1250, startup_ms_4250);",
            "CREATE TABLE viewers AS SELECT * FROM (VALUES ('k1', 's', 0, NULL, false), ('k1', 's', 1, 5000, false), "
            "('k1', 's', 2, NULL, true), ('k2', 's', 0, NULL, false), ('k3', 's', 0, NULL, false), "
            "('k4', 's', 0, NULL, false)) v(k, src, viewer, arrive_ms, hostile);",
            "CREATE TABLE viewer_seconds AS SELECT * FROM (VALUES "
            + ", ".join(f"('{k}', 's', {v}, {sec}, {lit(lag)}, {f})" for k, v, sec, lag, f in vs)
            + ") v(k, src, viewer, sec, lag_ms, frames);",
            "CREATE TABLE verdict_seconds AS SELECT * FROM (VALUES "
            + ", ".join(f"('{k}', 's', {sec}, '{phase(k, sec)}')" for k in ("k1", "k2", "k3", "k4") for sec in range(30))
            + ") v(k, src, sec, phase);",
            "CREATE TABLE episodes AS SELECT * FROM (VALUES ('k1', 's', 'unhealthy', 3), ('k1', 's', 'unhealthy', 5), "
            "('k1', 's', 'healthy', 22), ('k2', 's', 'unhealthy', 12), ('k3', 's', 'unhealthy', 20), "
            "('k4', 's', 'unhealthy', 25)) v(k, src, kind, len);",
        ]
        database(cls.db, "\n".join(sql))

    @classmethod
    def tearDownClass(cls):
        suite.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        suite.SCORE_DB = self.db

    def test_structural(self):
        self.assertEqual(scored("SELECT * FROM structural('dyn');"), [{"cell": "d", "arms": 1, "runs": 2}])

    def test_dynamics(self):
        got = {r["cell"]: r for r in scored("SELECT * FROM dynamics('dyn');")}
        c = got["c"]
        self.assertEqual((c["arm_label"], c["runs"], c["collapsed"]), ("x", 2, 1))
        # Startup: 40 seconds, the runs' first 15 and viewer 1's own 7-16
        # (its 5-6 read nothing yet): 100 x29, 200 x9, 3000, 6000.
        self.assertAlmostEqual(c["lag_p99_startup_ms"], 3000 + 0.61 * 3000, places=6)
        # Steady: 100 x17, 200 x13, 2000, 3000 (carried), 5000.
        self.assertAlmostEqual(c["lag_p99_steady_ms"], 3000 + 0.68 * 2000, places=6)
        self.assertAlmostEqual(c["lag_p99_after_down_ms"], 100 + 0.91 * 4400, places=6)
        self.assertIsNone(c["lag_p99_after_up_ms"])
        # k1 spikes past 1250 at 20-22 and past 4250 at 22, k2 at 25.
        self.assertEqual((c["spike_s_1250"], c["spike_s_4250"]), (2.0, 1.0))
        self.assertEqual((c["stalls_per_min_1250"], c["stalls_per_min_4250"]), (3.0, 1.0))
        self.assertAlmostEqual(c["stalled_share_1250"], 0.02)
        self.assertEqual((c["startup_ms_1250"], c["startup_ms_4250"]), (1000.0, 4100.0))
        self.assertEqual((c["unhealthy_max_s"], c["unhealthy_p50_s"]), (12, 8.5))
        d = got["d"]
        self.assertEqual((d["runs"], d["collapsed"], d["spike_s_1250"], d["unhealthy_max_s"]), (2, 2, 0.0, 25))


if __name__ == "__main__":
    unittest.main()
