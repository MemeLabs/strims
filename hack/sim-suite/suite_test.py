#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests of suite.py's scoring, simlab.scoring and the catalog's
scoring macros (hack/simlab/sql/scoring.sql), over DuckDB files made from
fixtures:

  Golden      testdata/golden/ (golden.py fixtures): cells of the latest
              farm experiments and a made-up one, their runs' tables and
              what golden.py's reference suite.py printed over them;
              simlab.scoring prints the same, from suite.py's sides and
              from arm_side's, and scoring an experiment's arms by name
              (score, arm_scores), whose JSON holds what the text shows.
              Zeros compare without their sign, as in golden.compare.
  Arithmetic  fisher_p against the exact hypergeometric sum, t975_at
              against Student's t, effect_mark.
  Dynamics    structural, dynamics and dynamics_phases over a made-up
              experiment.
  LagSince    lag_since over a made-up run's phase windows.

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
# simlab.scoring, whose scorecards and arm scores suite.py prints.
scoring = suite.scoring
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
CREATE TABLE IF NOT EXISTS jobs (k VARCHAR, src VARCHAR, sim VARCHAR, trace VARCHAR, reads_hash VARCHAR);
CREATE TABLE IF NOT EXISTS experiment_jobs (exp_id VARCHAR, cell VARCHAR, arm_label VARCHAR, seed BIGINT, k VARCHAR,
    src VARCHAR, flush VARCHAR, matrix VARCHAR);
CREATE TABLE IF NOT EXISTS experiment_cells (exp_id VARCHAR, cell VARCHAR, why VARCHAR, matrix VARCHAR);
CREATE TABLE IF NOT EXISTS viewers (k VARCHAR, src VARCHAR, viewer BIGINT, node BIGINT, arrive_ms BIGINT,
    hostile BOOLEAN);
CREATE TABLE IF NOT EXISTS viewer_seconds (k VARCHAR, src VARCHAR, viewer BIGINT, sec BIGINT, lag_ms DOUBLE,
    frames BIGINT);
CREATE TABLE IF NOT EXISTS viewer_phases (k VARCHAR, src VARCHAR, viewer BIGINT, phase VARCHAR, start BIGINT,
    stop BIGINT);
CREATE TABLE IF NOT EXISTS verdict_seconds (k VARCHAR, src VARCHAR, sec BIGINT, phase VARCHAR);
CREATE TABLE IF NOT EXISTS phase_windows (k VARCHAR, src VARCHAR, viewer BIGINT, kind VARCHAR, node BIGINT,
    phase VARCHAR, anchor_s DOUBLE, end_s DOUBLE, stop_s DOUBLE, horizon_s DOUBLE);
CREATE TABLE IF NOT EXISTS episodes (k VARCHAR, src VARCHAR, kind VARCHAR, len BIGINT);
CREATE TABLE IF NOT EXISTS events (k VARCHAR, src VARCHAR, t_ms DOUBLE, kind VARCHAR, node BIGINT);
CREATE TABLE IF NOT EXISTS event_kinds (kind VARCHAR, sim VARCHAR, class VARCHAR, scope VARCHAR, replaced_by VARCHAR);
"""


def database(path, sql):
    """A DuckDB file at `path` made by `sql` (and TABLES); simlab.scoring
    scores over it."""
    suite.simdb().duck(sql + TABLES, db=path, out=None)
    scoring.SCORE_DB = path


def scored(sql):
    """The rows of `sql` over the scoring objects."""
    return scoring.query(sql)


def lines(text):
    """`text`'s lines, each printed -0.000 as +0.000 (golden.unsigned_zeros),
    as golden.compare reads both sides: the fixtures are the reference's
    text, which prints the sign of an effect its Python sums land a last bit
    below zero, and simlab.scoring.signed prints every zero +0.000."""
    return golden.unsigned_zeros(text).splitlines()


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
        scoring.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        if not self.db.exists():
            database(self.db, self.sql)
        scoring.SCORE_DB = self.db

    def check(self, case, sides):
        got = golden.texts(case["cells"], sides[case["base"]], sides[case["arm"]], case["exp"], case["base"],
                           case["arm"])
        for name, want in case["want"].items():
            with self.subTest(case=case["exp"], text=name):
                self.assertEqual(lines(got[name]), lines(want))

    def test_sides(self):
        """suite.py's sides, as it reads them from the farm's indexes."""
        for case in self.cases:
            sides = {arm: {(c, seed): (k, src) for c, seed, k, src in side} for arm, side in case["sides"].items()}
            self.check(case, sides)

    def test_arm_side(self):
        """The experiments' arms as the catalog's experiment_jobs has them."""
        for case in self.cases:
            names = {c["name"] for c in case["cells"]}
            sides = {arm: {at: v for at, v in side.items() if at[0] in names}
                     for arm, side in scoring.sides(case["exp"], [case["base"], case["arm"]]).items()}
            self.check(case, sides)

    def test_experiments(self):
        """An experiment's arms scored by name (score, arm_scores), its
        cells experiment_cells', print the same; their JSON holds the
        numbers the text shows."""
        for case in self.cases:
            names = [c["name"] for c in case["cells"]]
            exp, base, arm = case["exp"], case["base"], case["arm"]
            self.assertEqual([{"name": c["name"], "why": c["why"]} for c in case["cells"]],
                             scoring.cells_of(exp, only=names))
            for show_all in (False, True):
                card = scoring.score(exp, base, exp, arm, cells=names, show_all=show_all)
                with self.subTest(case=exp, text=f"scorecard all={show_all}"):
                    self.assertEqual(lines(card.text()), lines(case["want"][f"scorecard all={show_all}"]))
            for kind, budgets in golden.RANKS.items():
                for budget in budgets:
                    [sc] = scoring.arm_scores(exp, base, [arm], cells=names, rank=kind, lag_budget=budget)
                    with self.subTest(case=exp, text=f"arm {kind} {budget}"):
                        self.assertEqual(lines(sc.text()), lines(case["want"][f"arm {kind} {budget}"]))
            j = json.loads(json.dumps(card.json()))
            moved = re.search(r"moved: (\d+) better, (\d+) worse, (\d+) changed; (\d+) ties", card.text()).groups()
            self.assertEqual([j["moved"][k] for k in ("better", "worse", "changed", "ties")], [int(x) for x in moved])
            self.assertEqual([c["name"] for c in j["cells"]], names)
            a = json.loads(json.dumps(sc.json()))
            self.assertEqual((a["collapsed"], a["base_collapsed"], a["broken"], len(a["cells"])),
                             (sc.collapsed, sc.base_collapsed, sc.broken, len(sc.cells)))

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
        scoring.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        scoring.SCORE_DB = self.db

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
        """Student's t at 97.5 %: the closed forms at 1 and 2 degrees of
        freedom, the tables' to six places, and the expansion past the
        table meeting it."""
        got = {r["df"]: r["t"] for r in scored(
            "SELECT df, t975_at(df) AS t FROM range(1, 2001) r(df);")}
        self.assertAlmostEqual(got[1], math.tan(math.pi * 0.475), places=11)
        self.assertAlmostEqual(got[2], 0.95 / math.sqrt(2 * 0.975 * 0.025), places=11)
        for df, t in {3: 3.182446, 7: 2.364624, 16: 2.119905, 17: 2.109816, 20: 2.085963, 29: 2.045230,
                      30: 2.042272, 31: 2.039513, 60: 2.000298, 120: 1.979930, 1000: 1.962339}.items():
            self.assertAlmostEqual(got[df], t, places=6, msg=df)
        self.assertAlmostEqual(got[101], 1.9837310030, places=9)
        self.assertLess(abs(got[100] - 2 * got[101] + got[102]), 1e-5)
        self.assertTrue(all(got[df] > got[df + 1] > 1.959963984 for df in range(1, 2000)))

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

    def test_a_zero_prints_without_a_sign(self):
        # A difference within a last bit of zero prints +0.000 whichever side
        # its sum fell, and the golden compares a reference's -0.000 as one.
        e = {"kind": "diff", "effect": -1e-17, "lo": -0.0004, "hi": -0.0006}
        self.assertEqual(scoring.effect_text(e), "+0.000 [+0.000,-0.001]")
        self.assertEqual(golden.unsigned_zeros("   ~ flaps_per_min -0.000 [-0.005,+0.004] 10-0.000"),
                         "   ~ flaps_per_min +0.000 [-0.005,+0.004] 10-0.000")


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
                                   + TABLES + cat.scoring_sql(cat.scoring_cols({})), out=None)
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
    """A made-up experiment `dyn`, arm x, of 50 s runs: cell c's seed 1
    (k1) and seed 2 (k2, collapsed; its seed's uplink steps down at 35 s
    and a viewer departs at 45 s), cell d's seeds both collapsed. The
    viewers' phases are the verdict's rule's (viewer_phases): the run's
    startup STARTUP_SECS, a join's join's horizon."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.tmp.name) / "dyn.duckdb"
        from simlab import verdict

        start, own = verdict.STARTUP_SECS, verdict.ANCHORS["join"]["horizon_s"]
        vs = []
        for sec in range(50):
            # k1, viewer 0 (there from the start): a startup spike, then
            # 2000, nothing read (3000 carried) and 5000 in steady seconds.
            lag0 = {3: 3000, 30: 2000, 31: None, 32: 5000}.get(sec, 100)
            vs.append(("k1", 0, sec, lag0, 1))
            # k1, viewer 1 (arriving at 15 s, reading from 17 s: its own
            # startup from its join's second, 15 s, for join's horizon), a
            # spike of 6000 within it.
            if sec >= 15:
                vs.append(("k1", 1, sec, None if sec < 17 else 6000 if sec == 18 else 200, 0 if sec < 17 else 1))
            vs.append(("k1", 2, sec, 9000, 1))
            # k2, viewer 0: 4500 after the step down, 3500 after the
            # departure.
            vs.append(("k2", 0, sec, {40: 4500, 47: 3500}.get(sec, 100), 1))
            for k in ("k3", "k4"):
                vs.append((k, 0, sec, 100, 1))
        cls.start, cls.own = start, own
        phase = lambda k, sec: ("after_departure" if k == "k2" and sec >= 45 else "after_down" if k == "k2"
                                and sec >= 35 else "startup" if sec < start else "steady")
        spans = []
        for k, v, join in [("k1", 0, 0), ("k1", 1, 15), ("k1", 2, 0), ("k2", 0, 0), ("k3", 0, 0), ("k4", 0, 0)]:
            for sec in range(join, 50):
                p = "startup" if join and sec < join + own else phase(k, sec)
                if spans and spans[-1][:3] == [k, v, p] and spans[-1][4] == sec:
                    spans[-1][4] = sec + 1
                else:
                    spans.append([k, v, p, sec, sec + 1])
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
            "CREATE TABLE viewers AS SELECT * FROM (VALUES ('k1', 's', 0, NULL, false), ('k1', 's', 1, 15000, false), "
            "('k1', 's', 2, NULL, true), ('k2', 's', 0, NULL, false), ('k3', 's', 0, NULL, false), "
            "('k4', 's', 0, NULL, false)) v(k, src, viewer, arrive_ms, hostile);",
            "CREATE TABLE viewer_seconds AS SELECT * FROM (VALUES "
            + ", ".join(f"('{k}', 's', {v}, {sec}, {lit(lag)}, {f})" for k, v, sec, lag, f in vs)
            + ") v(k, src, viewer, sec, lag_ms, frames);",
            "CREATE TABLE viewer_phases AS SELECT * FROM (VALUES "
            + ", ".join(f"('{k}', 's', {v}, '{p}', {a}, {b})" for k, v, p, a, b in spans)
            + ") v(k, src, viewer, phase, start, stop);",
            "CREATE TABLE episodes AS SELECT * FROM (VALUES ('k1', 's', 'unhealthy', 3), ('k1', 's', 'unhealthy', 5), "
            "('k1', 's', 'healthy', 22), ('k2', 's', 'unhealthy', 12), ('k3', 's', 'unhealthy', 20), "
            "('k4', 's', 'unhealthy', 25)) v(k, src, kind, len);",
        ]
        database(cls.db, "\n".join(sql))

    @classmethod
    def tearDownClass(cls):
        scoring.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        scoring.SCORE_DB = self.db

    def test_the_rule_the_fixture_takes(self):
        # Its numbers below hold for these: a run's startup of 25 s
        # outlasting viewer 1's own of 20 s from 15 s by 10 s.
        self.assertEqual((self.start, self.own), (25, 20))

    def test_structural(self):
        self.assertEqual(scoring.structural("dyn"), [{"cell": "d", "arms": 1, "runs": 2}])

    def test_dynamics(self):
        got = {r["cell"]: r for r in scoring.dynamics("dyn")}
        c = got["c"]
        self.assertEqual((c["arm_label"], c["runs"], c["collapsed"]), ("x", 2, 1))
        self.assertEqual([x for x in c if x.startswith("lag_p99_")],
                         ["lag_p99_startup_ms", "lag_p99_steady_ms", "lag_p99_after_ms"])
        # Startup: the runs' first 25 s and viewer 1's own 15-34 s (its
        # 15-16 read nothing yet): 100 x49, 200 x17, 3000, 6000.
        self.assertAlmostEqual(c["lag_p99_startup_ms"], 3000 + 0.33 * 3000, places=6)
        # Steady: 100 x32, 200 x15, 2000, 3000 (carried), 5000.
        self.assertAlmostEqual(c["lag_p99_steady_ms"], 3000 + 0.51 * 2000, places=6)
        # After: k2's after_down (35-44 s) and after_departure (45-49 s)
        # pooled: 100 x13, 3500, 4500.
        self.assertAlmostEqual(c["lag_p99_after_ms"], 3500 + 0.86 * 1000, places=6)
        # k1 spikes past 1250 at 30-32 and past 4250 at 32, k2 at 40 and
        # past 1250 at 47.
        self.assertEqual((c["spike_s_1250"], c["spike_s_4250"]), (2.5, 1.0))
        self.assertEqual((c["stalls_per_min_1250"], c["stalls_per_min_4250"]), (3.0, 1.0))
        self.assertAlmostEqual(c["stalled_share_1250"], 0.02)
        self.assertEqual((c["startup_ms_1250"], c["startup_ms_4250"]), (1000.0, 4100.0))
        self.assertEqual((c["unhealthy_max_s"], c["unhealthy_p50_s"]), (12, 8.5))
        d = got["d"]
        self.assertEqual((d["runs"], d["collapsed"], d["spike_s_1250"], d["unhealthy_max_s"]), (2, 2, 0.0, 25))

    def test_dynamics_phases(self):
        got = scored("SELECT * FROM dynamics_phases('dyn');")
        self.assertEqual(list(got[0]), ["cell", "arm_label", "phase", "viewer_s", "share", "lag_p50_ms",
                                        "lag_p90_ms", "lag_p99_ms", "lag_max_ms"])
        # By cell, then in the vocabulary's order of the phases.
        self.assertEqual([(r["cell"], r["phase"]) for r in got], [
            ("c", "startup"), ("c", "after_down"), ("c", "steady"), ("c", "after_departure"),
            ("d", "startup"), ("d", "steady")])
        c = {r["phase"]: r for r in got if r["cell"] == "c"}
        self.assertEqual({p: r["viewer_s"] for p, r in c.items()},
                         {"startup": 68, "steady": 50, "after_down": 10, "after_departure": 5})
        self.assertAlmostEqual(sum(r["share"] for r in c.values()), 1.0)
        self.assertAlmostEqual(c["after_down"]["share"], 10 / 133)
        # dynamics' p99s are its own.
        dyn = {r["cell"]: r for r in scoring.dynamics("dyn")}["c"]
        self.assertAlmostEqual(c["startup"]["lag_p99_ms"], dyn["lag_p99_startup_ms"])
        self.assertAlmostEqual(c["steady"]["lag_p99_ms"], dyn["lag_p99_steady_ms"])
        self.assertEqual((c["after_down"]["lag_p50_ms"], c["after_down"]["lag_max_ms"]), (100, 4500))
        self.assertAlmostEqual(c["after_departure"]["lag_p90_ms"], 100 + 0.6 * 3400)
        self.assertEqual(c["startup"]["lag_max_ms"], 6000)


class LagSince(unittest.TestCase):
    """A made-up experiment `since`, cell c, arm x, one strims-sim run k1:
    viewers 0 (node 1, lag 100 + its second, nothing read at 21 s), 1
    (node 2, arriving at 5 s, reading 200 from 7 s) and 2 (hostile,
    9000); its phase_windows the stream's start's, two uplink steps down
    at 20 s (of two nodes) and one at 22 s, viewer 1's join at 5.3 s and a
    crash at 30 s that ends at 33 s."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.tmp.name) / "since.duckdb"
        vs = []
        for sec in range(40):
            vs.append((0, sec, None if sec == 21 else 100 + sec))
            if sec >= 5:
                vs.append((1, sec, None if sec < 7 else 200))
            vs.append((2, sec, 9000))
        lit = lambda x: "NULL" if x is None else repr(x)
        # (viewer, kind, node, phase, anchor_s, end_s, stop_s, horizon_s)
        windows = [(None, "start", None, "startup", 0.0, None, 25.0, 25.0),
                   (None, "uplink_rate", 0, "after_down", 20.0, None, 80.0, 60.0),
                   (None, "uplink_rate", 7, "after_down", 20.0, None, 80.0, 60.0),
                   (None, "uplink_rate", 0, "after_down", 22.0, None, 82.0, 60.0),
                   (1, "join", 2, "startup", 5.3, None, 25.3, 20.0),
                   (None, "crash", 3, "after_departure", 30.0, 33.0, 53.0, 20.0)]
        sql = [
            "CREATE TABLE jobs AS SELECT 'k1' AS k, 's' AS src, 'strims-sim' AS sim, NULL::VARCHAR AS trace, "
            "NULL::VARCHAR AS reads_hash;",
            "CREATE TABLE experiment_jobs AS SELECT 'since' AS exp_id, 'c' AS cell, 'x' AS arm_label, 1 AS seed, "
            "'k1' AS k, 's' AS src, 's' AS flush, 'since-00' AS matrix;",
            "CREATE TABLE viewers AS SELECT * FROM (VALUES ('k1', 's', 0, 1, NULL, false), ('k1', 's', 1, 2, 5000, false), "
            "('k1', 's', 2, 3, NULL, true)) v(k, src, viewer, node, arrive_ms, hostile);",
            "CREATE TABLE viewer_seconds AS SELECT * FROM (VALUES "
            + ", ".join(f"('k1', 's', {v}, {sec}, {lit(lag)}, 1)" for v, sec, lag in vs)
            + ") v(k, src, viewer, sec, lag_ms, frames);",
            "CREATE TABLE phase_windows AS SELECT * FROM (VALUES "
            + ", ".join(f"('k1', 's', {lit(v)}::BIGINT, '{kind}', {lit(node)}::BIGINT, '{p}', {a}, {lit(e)}::DOUBLE, "
                        f"{b}, {h})" for v, kind, node, p, a, e, b, h in windows)
            + ") v(k, src, viewer, kind, node, phase, anchor_s, end_s, stop_s, horizon_s);",
        ]
        database(cls.db, "\n".join(sql))

    @classmethod
    def tearDownClass(cls):
        scoring.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        scoring.SCORE_DB = self.db

    def test_lag_since(self):
        got = scored("SELECT * FROM lag_since('since', before := 2, after := 3);")
        self.assertEqual(list(got[0]), ["cell", "arm_label", "kind", "phase", "since", "rel_s", "windows",
                                        "viewer_s", "lag_p50_ms", "lag_p90_ms", "lag_p99_ms"])
        by = {(r["kind"], r["since"], r["rel_s"]): r for r in got}
        # The windows' anchors and the crash's end; the two steps at 20 s
        # are one window; the start's seconds before it aren't there.
        self.assertEqual(sorted(by), [("crash", "anchor", r) for r in range(-2, 3)]
                         + [("crash", "end", r) for r in range(-2, 3)]
                         + [("join", "anchor", r) for r in range(3)] + [("start", "anchor", r) for r in range(3)]
                         + [("uplink_rate", "anchor", r) for r in range(-2, 3)])
        self.assertEqual({r["phase"] for r in got if r["kind"] == "uplink_rate"}, {"after_down"})
        # Viewer 1's join takes its own seconds alone: 5 and 6 s hold no
        # lag yet, 7 s its first.
        self.assertEqual([(by[("join", "anchor", r)]["windows"], by[("join", "anchor", r)]["viewer_s"])
                          for r in range(3)], [(1, 0), (1, 0), (1, 1)])
        self.assertIsNone(by[("join", "anchor", 0)]["lag_p50_ms"])
        self.assertEqual(by[("join", "anchor", 2)]["lag_p99_ms"], 200)
        # The steps at 20 s and 22 s: each of their seconds 20-22 counts in
        # both windows, so 20 viewer-seconds where the honest viewers have
        # 14 from 18 s to 24 s. At rel_s 1: 21 s (viewer 0's carried 1120
        # and 200) and 23 s (123 and 200).
        self.assertEqual([by[("uplink_rate", "anchor", r)]["windows"] for r in range(-2, 3)], [2] * 5)
        self.assertEqual(sum(by[("uplink_rate", "anchor", r)]["viewer_s"] for r in range(-2, 3)), 20)
        one = by[("uplink_rate", "anchor", 1)]
        self.assertEqual(one["lag_p50_ms"], 200)
        self.assertAlmostEqual(one["lag_p99_ms"], 200 + 0.97 * 920, places=6)
        self.assertEqual(by[("uplink_rate", "anchor", -2)]["lag_p50_ms"], (120 + 200) / 2)
        # The start's: viewer 0's lag from 0 s (viewer 1 not there yet).
        self.assertEqual([by[("start", "anchor", r)]["lag_p50_ms"] for r in range(3)], [100, 101, 102])
        # The crash from its anchor (28-32 s) and from its end (31-35 s).
        self.assertEqual(by[("crash", "anchor", -2)]["lag_p50_ms"], (128 + 200) / 2)
        self.assertEqual(by[("crash", "end", 0)]["lag_p50_ms"], (133 + 200) / 2)
        # By default 10 s before to 120 s after, the run's 40 s bounding it.
        got = scored("SELECT kind, since, min(rel_s) AS lo, max(rel_s) AS hi FROM lag_since('since') "
                     "GROUP BY ALL ORDER BY ALL;")
        self.assertEqual([(r["kind"], r["since"], r["lo"], r["hi"]) for r in got],
                         [("crash", "anchor", -10, 9), ("crash", "end", -10, 6), ("join", "anchor", 0, 34),
                          ("start", "anchor", 0, 39), ("uplink_rate", "anchor", -10, 19)])

    def test_a_catalog_without_what_it_needs(self):
        # A catalog indexed before the verdicts had viewer_phases and
        # phase_windows: lag_since, dynamics and dynamics_phases say what
        # they lack, the other macros still score.
        db = Path(self.tmp.name) / "old.duckdb"
        database(db, "")
        suite.simdb().duck("DROP TABLE viewer_phases; DROP TABLE phase_windows;", db=db, out=None)
        with self.assertRaisesRegex(suite.simdb().DuckError, "lag_since reads phase_windows, which this catalog "
                                                             "lacks"):
            scored("SELECT * FROM lag_since('since');")
        with self.assertRaisesRegex(suite.simdb().DuckError, "dynamics reads viewer_phases, which this catalog"):
            scored("SELECT * FROM dynamics('since');")
        with self.assertRaisesRegex(suite.simdb().DuckError, "dynamics_phases reads viewer_phases"):
            scored("SELECT * FROM dynamics_phases('since');")
        self.assertEqual(scored("SELECT * FROM structural('since');"), [])


if __name__ == "__main__":
    unittest.main()
