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
  Reference   golden.py's reference reads a farm run's host numbers as
              the macros do: its copy's in its flush.
  Arithmetic  compensated_sum against Python's sum(), a mean difference
              on a printed tie against the reference's, fisher_p against
              the exact hypergeometric sum, t975_at against Student's t,
              effect_mark.
  Dynamics    structural, dynamics and dynamics_phases over a made-up
              experiment.
  LagSince    lag_since over a made-up run's phase windows.

  python3 hack/sim-suite/suite_test.py [-v]
"""

import gzip
import importlib.util
import json
import math
import random
import re
import tempfile
import types
import unittest
from pathlib import Path
from unittest import mock

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
    wall_s DOUBLE, rss_mb DOUBLE, "at" VARCHAR);
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
CREATE TABLE IF NOT EXISTS reads (k VARCHAR, src VARCHAR, viewer BIGINT, t_ms BIGINT, seq BIGINT, lag_ms BIGINT);
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


class Reference(unittest.TestCase):
    def test_a_farm_runs_host_numbers_are_its_flush_copys(self):
        """The reference (golden.reference) reads a farm run's wall_s and
        rss_mb as METRICS does (job_runs'): its copy's in its flush, which
        simdb.rows gives, not its index line's. A key that ran twice into
        one flush, a job and its verify copy on one host, has a line of
        each in its matrix's index and one copy in the flush
        (store_host.fold keeps its batch's last), whose numbers needn't be
        the line's that load_rows keeps (an id's last): s2e215's
        halve-15-10 off at seed 2005, 113.0 MB in its flush and 113.5 on
        its kept line."""
        with tempfile.TemporaryDirectory() as tmp:
            database(Path(tmp) / "empty.duckdb", "")
            try:
                ref = golden.reference(golden.REFERENCE)
            finally:
                scoring.SCORE_DB = None
            src = "farm/r/i-0/0"
            index = Path(tmp) / "target" / "sim-farm" / "sx-00" / "index.jsonl"
            index.parent.mkdir(parents=True)
            index.write_text("".join(json.dumps(
                {"id": "j", "key": "k", "arm": "x", "test": "t", "env": {"STRIMS_SIM_SEED": "2005"}, "copy": copy,
                 "src": src, "max_rss_mb": rss, "wall_s": wall, "row": {}}) + "\n"
                for copy, rss, wall in (("v", 113.0, 8.996), ("", 113.5, 9.193))))
            flush = {(src, "k"): {"trace": "t1", "_rss_mb": 113.0, "_wall_s": 8.996}}
            simdb = types.SimpleNamespace(pull_sources=lambda srcs: [],
                                          rows=lambda pairs: {p: dict(flush[p]) for p in pairs if p in flush},
                                          arm_label=lambda knob, sim, arm, env: arm)
            with mock.patch.object(ref, "farm_root", lambda: Path(tmp)), mock.patch.object(ref, "_simdb", simdb):
                got = ref.farm_rows("sx", [{"name": "halve-15-10", "sim": "strims-sim", "test": "t"}], None)
        self.assertEqual({at: (r["_rss_mb"], r["_wall_s"]) for at, r in got["x"].items()},
                         {("halve-15-10", 2005): (113.0, 8.996)})


class Arithmetic(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.tmp.name) / "empty.duckdb"
        database(cls.db, "CREATE SCHEMA raw; CREATE TABLE raw.runs (k VARCHAR, src VARCHAR, cell VARCHAR, arm VARCHAR, "
                         "seed BIGINT, trace VARCHAR, wall_s DOUBLE, rss_mb DOUBLE, "
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

    @staticmethod
    def twin_lake(path):
        """A lake of four runs of one cell (SECS 40, recording 10 s past
        it, as a twin does): two testbed runs, `tb1` whose seed's meter
        samples record its channel bytes and `tb0` whose don't, and their
        twins `tw`, which records them, and `tw0`, which doesn't (a twin
        takes S_c as it measures it, whatever it records). Each seed's
        meter reads its truth at 15, 25 and 35 s, and 0.003 of it at 45 s,
        past the stream; each viewer lags 100 ms each second to 40 s, its
        lag carried after."""
        runs = [("tb1", "testbed/x", "testbed", True), ("tb0", "testbed/y", "testbed", False),
                ("tw", "testbed/x-twin", "strims-sim", True), ("tw0", "testbed/y-twin", "strims-sim", False)]
        jobs = ", ".join(f"('{k}', '{src}', '{sim}', 100, MAP {{'STRIMS_SIM_CAL_SECS': '40'}})"
                         for k, src, sim, _ in runs)
        meter = ", ".join(f"('{k}', '{src}', 'seed', {t}, {3 if t > 40000 else 1000}, 1000, "
                          f"{t // 10 if counted else 'NULL'})"
                          for k, src, _, counted in runs for t in (15000, 25000, 35000, 45000))
        lags = ", ".join(f"('{k}', '{src}', 0, {sec}, {100 if sec < 40 else 'NULL'}, {1 if sec < 40 else 0})"
                         for k, src, _, _ in runs for sec in range(50))
        viewers = ", ".join(f"('{k}', '{src}', 0, 1, NULL, false)" for k, src, _, _ in runs)
        database(path, f"""
            CREATE TABLE jobs (k VARCHAR, src VARCHAR, sim VARCHAR, bin_ms BIGINT, env MAP(VARCHAR, VARCHAR));
            INSERT INTO jobs VALUES {jobs};
            CREATE TABLE meter_samples (k VARCHAR, src VARCHAR, role VARCHAR, t_ms BIGINT, capacity_bps BIGINT,
                avail_bps BIGINT, channel_bytes BIGINT, rate_bps BIGINT, children BIGINT, budget BIGINT,
                copies BIGINT, queue_delay_ms BIGINT, busy_permille BIGINT, latest_floor_ms BIGINT,
                path_latest_ms BIGINT, requests_in BIGINT);
            INSERT INTO meter_samples (k, src, role, t_ms, capacity_bps, avail_bps, channel_bytes) VALUES {meter};
            CREATE TABLE viewers (k VARCHAR, src VARCHAR, viewer BIGINT, node BIGINT, arrive_ms BIGINT,
                hostile BOOLEAN);
            INSERT INTO viewers VALUES {viewers};
            CREATE TABLE viewer_seconds (k VARCHAR, src VARCHAR, viewer BIGINT, sec BIGINT, lag_ms DOUBLE,
                frames BIGINT);
            INSERT INTO viewer_seconds VALUES {lags};
            CREATE TABLE events (k VARCHAR, src VARCHAR, t_us BIGINT, kind VARCHAR, node BIGINT, value DOUBLE);
            CREATE TABLE seconds (k VARCHAR, src VARCHAR, sec BIGINT, busy DOUBLE, copies DOUBLE, pushed DOUBLE,
                sent DOUBLE, hits DOUBLE, misses DOUBLE, pruned DOUBLE, avail DOUBLE);
            CREATE TABLE bins (k VARCHAR, src VARCHAR, t_ms BIGINT, seed_up_bytes BIGINT, seed_up_sack_bytes BIGINT,
                seed_up_packets BIGINT, seed_up_sack_packets BIGINT, seed_small_packets BIGINT,
                seed_up_ack_packets BIGINT, seed_up_ack_bytes BIGINT, seed_drops BIGINT, viewer_up_bytes BIGINT,
                viewer_up_sack_bytes BIGINT, viewer_up_packets BIGINT, viewer_up_sack_packets BIGINT,
                viewer_switched BIGINT, viewer_shed BIGINT, seed_up_queue_ms BIGINT);
            CREATE TABLE node_bins (k VARCHAR, src VARCHAR, node BIGINT, t_ms BIGINT, up_bytes BIGINT,
                up_sack_bytes BIGINT, up_ack_bytes BIGINT, up_packets BIGINT, up_sack_packets BIGINT,
                up_ack_packets BIGINT, data_bytes BIGINT, ovh BIGINT);
            CREATE TABLE peer_samples (k VARCHAR, src VARCHAR, node BIGINT, peer BIGINT, t_ms BIGINT,
                rtt_ms BIGINT, cto_ms BIGINT, requests_out BIGINT, requests_in BIGINT);
            """)
        return "[" + ", ".join(f"{{'cell': 'c', 'seed': 1, 'k': '{k}'}}" for k, _, _, _ in runs) + "]"

    def test_a_testbed_runs_meter_readings_say_s_c_1_only_without_its_channel_bytes(self):
        """twin_runs_of labels a testbed run's meter_p* rows `basis` only
        where its seed's samples lack channel_bytes (channel_counted_of):
        not a run that records them, nor a twin, one without them too."""
        with tempfile.TemporaryDirectory() as tmp:
            side = self.twin_lake(Path(tmp) / "twin.duckdb")
            got = {(r["k"], r["measure"]): r["basis"]
                   for r in scored(f"FROM twin_runs_of({side}) WHERE measure LIKE 'meter_p%';")}
        scoring.SCORE_DB = self.db
        self.assertEqual(sorted({k for k, _ in got}), ["tb0", "tb1", "tw", "tw0"])
        self.assertEqual({k: b for (k, _), b in got.items()},
                         {"tb0": "truth with S_c = 1 (no channel_bytes)", "tb1": None, "tw": None, "tw0": None})

    def test_twin_fit_rows(self):
        """twin_fit_rows: a side's median is the mean of its middle two at an
        even count; a testbed run stands at its mid-rank quantile among the
        twin's seeds; Spearman's rank_corr gives ties their mean rank; a
        fit whose testbed runs took something as given (their basis) says
        how many did, in its cell and across them."""
        def rows(side, given=()):
            basis = lambda c, i: "'S_c = 1'" if (c, i) in given else "NULL"
            return "[" + ", ".join(f"{{'cell': '{c}', 'measure': 'm', 'k': '{c}{i}', 'value': {v}, "
                                   f"'basis': {basis(c, i)}}}"
                                   for c, vs in side.items() for i, v in enumerate(vs)) + "]"
        twin = {"x": [1.0, 2.0, 3.0, 4.0], "y": [10.0, 20.0, 30.0, 40.0], "z": [10.0, 20.0, 30.0, 40.0]}
        bed = {"x": [2.0, 5.0], "y": [25.0, 25.0], "z": [0.0, 40.0]}
        got = {r["cell"]: r for r in scored(f"FROM twin_fit_rows({rows(twin)}, {rows(bed, given={('x', 1)})});")}
        self.assertEqual([got[c]["caveat"] for c in ("x", "y", None)],
                         ["the testbed's S_c = 1 in 1 of its 2 runs", None, "the testbed's S_c = 1 in 1 of its 6 runs"])
        self.assertEqual((got["x"]["a_median"], got["x"]["b_median"]), (2.5, 3.5))
        # x: 2 ties one of four and beats one (0.375), 5 beats all (1).
        self.assertAlmostEqual(got["x"]["b_quantile"], (0.375 + 1.0) / 2)
        self.assertAlmostEqual(got["y"]["b_quantile"], 0.5)
        # z: 0 below all, 40 ties the top (0.875).
        self.assertAlmostEqual(got["z"]["b_quantile"], (0.0 + 0.875) / 2)
        across = got[None]
        self.assertEqual((across["cells"], across["a_runs"], across["b_runs"]), (3, 12, 6))
        # The twin's medians tie at y and z (25, 25): their ranks are 2.5
        # each, the testbed's (25, 20) 3 and 2; x is 1 on both.
        ra, rb = [1, 2.5, 2.5], [1, 3, 2]
        mean = lambda v: sum(v) / len(v)
        cov = sum((p - mean(ra)) * (q - mean(rb)) for p, q in zip(ra, rb))
        sd = lambda v: sum((p - mean(v)) ** 2 for p in v) ** 0.5
        self.assertAlmostEqual(across["rank_corr"], cov / (sd(ra) * sd(rb)))

    def test_a_twins_window_is_its_runs_own_stream(self):
        """twin_window_of: by default each run's window is its own stream,
        [10 s, its STRIMS_SIM_CAL_SECS), calibration_star's 180 s where its
        knobs lack it, so the seconds a run records past its stream stay
        out; a window given is every run's."""
        with tempfile.TemporaryDirectory() as tmp:
            database(Path(tmp) / "jobs.duckdb",
                     "CREATE TABLE jobs (k VARCHAR, src VARCHAR, env MAP(VARCHAR, VARCHAR)); INSERT INTO jobs VALUES "
                     "('tb', 'testbed/x', MAP {'STRIMS_SIM_CAL_SECS': '40'}), "
                     "('tw', 'testbed/x-twin', MAP {'STRIMS_SIM_CAL_SECS': '120'}), "
                     "('old', 'testbed/y', MAP {'STRIMS_SIM_CAL_VIEWERS': '3'});")
            side = "[{'cell': 'c', 'seed': 1, 'k': 'tb'}, {'cell': 'c', 'seed': 1, 'k': 'tw'}, " \
                   "{'cell': 'c', 'seed': 2, 'k': 'old'}, {'cell': 'd', 'seed': 1, 'k': 'tb'}]"
            got = lambda args="": {r["k"]: (r["lo"], r["hi"]) for r in scored(f"FROM twin_window_of({side}{args});")}
            self.assertEqual(got(), {"tb": (10.0, 40.0), "tw": (10.0, 120.0), "old": (10.0, 180.0)})
            # One window a run, though the side lists it under two cells.
            self.assertEqual(len(scored(f"FROM twin_window_of({side});")), 3)
            self.assertEqual(got(", lo_s := 20, hi_s := 1e9"),
                             {"tb": (20.0, 1e9), "tw": (20.0, 1e9), "old": (20.0, 1e9)})
            self.assertEqual(got(", hi_s := 30")["tw"], (10.0, 30.0))
        scoring.SCORE_DB = self.db

    def test_a_twins_measures_by_default_are_its_runs_streams(self):
        """twin_runs_of's default window, each run's own stream: the meter's
        sample past the stream (0.003 of its truth) and the viewer's carried
        lag after it stay out, where a window to the run's end takes them
        in."""
        with tempfile.TemporaryDirectory() as tmp:
            side = self.twin_lake(Path(tmp) / "twin.duckdb")
            measures = lambda args="": {(r["k"], r["measure"]): r["value"]
                                        for r in scored(f"FROM twin_runs_of({side}{args});")}
            stream, to_end = measures(), measures(", lo_s := 10, hi_s := 1e9")
            same = measures(", lo_s := 10, hi_s := 40")
        scoring.SCORE_DB = self.db
        self.assertEqual(stream, same)
        for k in ("tb1", "tb0", "tw"):
            self.assertAlmostEqual(stream[(k, "meter_p10")], 1.0, delta=0.02)
            self.assertAlmostEqual(to_end[(k, "meter_p10")], 0.003, delta=0.001)
            # Carried 1000 ms a second from 40 s: past the target from 41 s.
            self.assertEqual((stream[(k, "over_target_s")], to_end[(k, "over_target_s")]), (0.0, 9.0))

    def test_a_runs_truth_is_the_mean_of_its_seeds_truths_over_its_stream(self):
        """twin_runs_of's truth_bps: the mean of the seed's meter samples'
        truth (avail_bps) over the run's window, an outage's 0 among them,
        a sample without a truth left out, and the sample past the stream
        out; a testbed run whose seed's samples lack channel_bytes says S_c
        = 1 in its basis, as its meter readings do. twin_fit_of scores it
        testbed over twin, with that run's caveat."""
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "twin.duckdb"
            side = self.twin_lake(path)
            database(path, "UPDATE meter_samples SET avail_bps = CASE t_ms WHEN 15000 THEN 900 WHEN 25000 THEN 0 "
                           "WHEN 35000 THEN NULL ELSE 5000 END WHERE k = 'tw';")
            got = {r["k"]: (r["value"], r["basis"])
                   for r in scored(f"FROM twin_runs_of({side}) WHERE measure = 'truth_bps';")}
            twin = "[{'cell': 'c', 'seed': 1, 'k': 'tw'}]"
            bed = "[{'cell': 'c', 'seed': 1, 'k': 'tb1'}, {'cell': 'c', 'seed': 2, 'k': 'tb0'}]"
            fit = {r["cell"]: r for r in scored(f"FROM twin_fit_of({twin}, {bed}) WHERE measure = 'truth_bps';")}
        scoring.SCORE_DB = self.db
        self.assertEqual(got, {"tb1": (1000.0, None), "tb0": (1000.0, "truth with S_c = 1 (no channel_bytes)"),
                               "tw": (450.0, None), "tw0": (1000.0, None)})
        self.assertEqual((fit["c"]["a_runs"], fit["c"]["b_runs"], fit["c"]["a_median"], fit["c"]["b_median"]),
                         (1, 2, 450.0, 1000.0))
        self.assertAlmostEqual(fit["c"]["ratio"], 1000.0 / 450.0)
        self.assertEqual(fit["c"]["caveat"], "the testbed's truth with S_c = 1 (no channel_bytes) in 1 of its 2 runs")

    def test_a_seeds_budget_median_leaves_its_unmeasured_samples_out(self):
        """twin_runs_of's seed_budget is the median of the seed's measured
        budgets: an unmeasured one, empty or 0 (no measured budget is 0),
        is left out, where taking it in would set a median of two
        unmeasured samples and one of 30 to 0."""
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "twin.duckdb"
            side = self.twin_lake(path)
            database(path, "UPDATE meter_samples SET budget = CASE WHEN t_ms = 35000 THEN 30 "
                           "WHEN k = 'tw' THEN 0 END;")
            got = {r["k"]: r["value"]
                   for r in scored(f"FROM twin_runs_of({side}) WHERE measure = 'seed_budget';")}
        scoring.SCORE_DB = self.db
        self.assertEqual(sorted(got), ["tb0", "tb1", "tw", "tw0"])
        for k, v in got.items():
            self.assertAlmostEqual(v, 30, delta=0.5, msg=k)

    def test_a_capture_that_ends_early_scores_the_bins_it_measured(self):
        """twin_runs_of's seed_upload and seed_charged take their bytes over
        the uplink's rate times the bins that measure them, and the seed's
        packets, drops and viewers' bytes a second, and the seed's wire
        bytes over what its driver wrote, over the bins that measure them:
        a testbed run whose capture and tc sampler stopped at 25 s (bins'
        and node bins' wire columns NULL from then, the driver's counters
        measured to the end) reads as the run whose capture ran to the end,
        where its whole window would read it at half."""
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "twin.duckdb"
            side = self.twin_lake(path)
            ended = lambda t, v: f"{v} " if t < 25000 else "NULL "
            bins = ", ".join(
                f"('{k}', '{src}', {t}, "
                + ", ".join(ended(t, v) if k == "tb0" else f"{v} " for v in (50, 0, 1, 0, 0, 1, 10, 2, 30, 0, 1, 0))
                + ", 0, 0, 0)"
                for k, src in (("tb1", "testbed/x"), ("tb0", "testbed/y")) for t in range(0, 50000, 100))
            # The seed's node bins: 60 wire bytes of 50 written and 10 of
            # overhead a bin, its wire columns ending with tb0's capture.
            nodes = ", ".join(
                f"('{k}', '{src}', 0, {t}, "
                + ", ".join(ended(t, v) if k == "tb0" else f"{v} " for v in (60, 0, 0, 1, 0, 0))
                + ", 50, 10)"
                for k, src in (("tb1", "testbed/x"), ("tb0", "testbed/y")) for t in range(0, 50000, 100))
            database(path, "INSERT INTO events VALUES ('tb1', 'testbed/x', 0, 'uplink_rate', 0, 1000), "
                           "('tb0', 'testbed/y', 0, 'uplink_rate', 0, 1000); "
                           f"INSERT INTO bins VALUES {bins}; INSERT INTO node_bins VALUES {nodes};")
            got = {(r["k"], r["measure"]): r["value"]
                   for r in scored(f"FROM twin_runs_of({side}) WHERE measure IN "
                                   "('seed_upload', 'seed_charged', 'seed_pps', 'seed_drops_s', 'viewer_up_bps', "
                                   "'seed_wire_per_written');")}
        scoring.SCORE_DB = self.db
        for k in ("tb1", "tb0"):
            # 50 IP bytes a 100 ms bin against 100 of the rate; a data
            # packet and a pure ACK of 10 bytes, 14 each of tbf's framing.
            self.assertAlmostEqual(got[(k, "seed_upload")], 0.5)
            self.assertAlmostEqual(got[(k, "seed_charged")], (50 + 10 + 2 * 14) / 100)
            self.assertAlmostEqual(got[(k, "seed_pps")], 20.0)
            self.assertAlmostEqual(got[(k, "seed_drops_s")], 20.0)
            self.assertAlmostEqual(got[(k, "viewer_up_bps")], 300.0)
            self.assertAlmostEqual(got[(k, "seed_wire_per_written")], 1.0)

    def test_sums(self):
        """compensated_sum adds a list in its order as Python's sum() adds
        floats, bit for bit (signed zeros, the infinities and NaN too), where
        a left fold strays by last bits: over tenths, logs, values far apart
        in magnitude and edge values."""
        rng = random.Random(2935)
        edges = [0.0, -0.0, 1e-300, -1e-300, 5e-324, 1.0, -1.0, 2.0 ** 53, -(2.0 ** 53), 0.1, 1e308, -1e308]
        lists = [[0.0], [-0.0], [-0.0, -0.0], [math.inf], [-math.inf, 1.0], [math.inf, -math.inf], [math.nan, 1.0],
                 [1.0, math.nan], [1e308, 1e308, -1e308], [1e16, 1.0, -1e16], [0.1] * 10]
        for i in range(2000):
            n = rng.randint(1, 20)
            lists.append([[round(rng.uniform(-2000, 2000), 1) for _ in range(n)],
                          [math.log(rng.uniform(0.01, 100)) for _ in range(n)],
                          [rng.uniform(-1, 1) * 10 ** rng.randint(-20, 20) for _ in range(n)],
                          [rng.choice(edges) for _ in range(n)]][i % 4])
        lit = lambda xs: "[" + ", ".join(f"'{x!r}'::DOUBLE" for x in xs) + "]"
        values = ", ".join(f"({i}, {lit(xs)})" for i, xs in enumerate(lists))
        got = scored(f"SELECT i, compensated_sum(xs) AS s, list_reduce(xs, lambda a, y: a + y) AS fold "
                     f"FROM (VALUES {values}) v(i, xs) ORDER BY i;")
        same = lambda a, b: math.isnan(a) and math.isnan(b) or a == b and math.copysign(1, a) == math.copysign(1, b)
        self.assertEqual([i for i, r in enumerate(got) if not same(r["s"], float(sum(lists[i])))], [])
        self.assertGreater(sum(1 for i, r in enumerate(got) if not same(r["fold"], float(sum(lists[i])))), 100)
        self.assertEqual(scored("SELECT compensated_sum([]::DOUBLE[]) AS e, compensated_sum(NULL::DOUBLE[]) AS n, "
                                "compensated_sum([1.0, NULL]) AS h;"), [{"e": 0.0, "n": None, "h": None}])

    def test_a_mean_on_a_printed_tie(self):
        """A difference's effect and interval are the reference's (golden.py:
        suite.py at REFERENCE, its t_interval, sum() over the pairs'
        differences in seed order), bit for bit. A farm run's rss_mb in
        tenths over 8 seeds (sb2935's halve-15-10, decl against main) differ
        by -371.8875 on the mean, a tie at the three places a scorecard
        prints: sum() lands under it, -371.887, and a left fold a last bit
        past it, -371.888."""
        a = [543.8, 500.1, 528.5, 518.2, 531.9, 551.1, 546.2, 547.9]
        b = [176.5, 154.7, 155.2, 157.9, 179.3, 157.4, 157.3, 154.3]
        runs = [(f"{arm}{seed}", arm, seed, v) for arm, vs in (("a", a), ("b", b)) for seed, v in enumerate(vs, 1)]
        db = Path(self.tmp.name) / "tie.duckdb"
        database(db, "CREATE SCHEMA raw; CREATE TABLE raw.runs (k VARCHAR, src VARCHAR, cell VARCHAR, arm VARCHAR, "
                     "seed BIGINT, trace VARCHAR, wall_s DOUBLE, rss_mb DOUBLE, \"at\" VARCHAR); "
                     "INSERT INTO raw.runs VALUES "
                     + ", ".join(f"('{k}', 's', 'c', '{arm}', {seed}, NULL, NULL, '{v!r}'::DOUBLE, NULL)"
                                 for k, arm, seed, v in runs)
                     + "; CREATE TABLE rows AS SELECT k, src FROM raw.runs; "
                     "CREATE TABLE jobs AS SELECT k, src, trace, NULL::VARCHAR AS reads_hash FROM raw.runs;")
        side = lambda arm: {("c", seed): (k, "s") for k, arm_, seed, _ in runs if arm_ == arm}
        try:
            card = scoring.scorecard([{"name": "c", "why": "a tie"}], side("a"), side("b"), "a", "b")
            t = scored("SELECT t975_at(7) AS t;")[0]["t"]
        finally:
            scoring.SCORE_DB = self.db
        [e] = [e for e in card.cells[0]["effects"] if e["metric"] == "rss_mb"]
        v = [y - x for x, y in zip(a, b)]
        mean = sum(v) / len(v)
        half = t * math.sqrt(sum((x - mean) ** 2 for x in v) / (len(v) - 1)) / math.sqrt(len(v))
        self.assertEqual((e["n"], e["effect"], e["lo"], e["hi"]), (8, mean, mean - half, mean + half))
        self.assertEqual(scoring.effect_text(e), f"{mean:+.3f} [{mean - half:+.3f},{mean + half:+.3f}]")
        self.assertEqual(f"{mean:+.3f}", "-371.887")
        fold = 0.0
        for x in v:
            fold += x
        self.assertEqual(f"{fold / len(v):+.3f}", "-371.888")

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
            # The tables TABLES makes, as the index tells scoring_sql the
            # catalog's: a macro needing another is made one that fails
            # saying so (DuckDB binds a table macro as it makes it).
            have = {t: set() for t in re.findall(r"CREATE TABLE IF NOT EXISTS (?:raw\.)?([a-z_]+)", TABLES)}
            try:
                suite.simdb().duck("INSTALL ducklake; INSTALL sqlite; LOAD ducklake; LOAD sqlite; "
                                   f"ATTACH {path} AS lake (DATA_PATH {suite.q(tmp + '/data/')}); USE lake;\n"
                                   + TABLES + cat.scoring_sql(cat.scoring_cols({}, have=have)), out=None)
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
    startup STARTUP_SECS, a join's join's horizon. k1's viewer 2 is
    hostile, its row's playback every viewer's and, apart, the honest
    viewers' (`_honest`); k2's is a row from before the sims split them
    (none of those), whose honest-only run makes them the same."""

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
            "wall_s DOUBLE, rss_mb DOUBLE, \"at\" VARCHAR);",
            "CREATE TABLE jobs AS SELECT 'k' || i AS k, 's' AS src, NULL::VARCHAR AS trace, "
            "NULL::VARCHAR AS reads_hash FROM range(1, 5) r(i);",
            "CREATE TABLE experiment_jobs AS SELECT * FROM (VALUES ('dyn', 'c', 'x', 1, 'k1', 's', 's', 'dyn-00'), "
            "('dyn', 'c', 'x', 2, 'k2', 's', 's', 'dyn-00'), ('dyn', 'd', 'x', 1, 'k3', 's', 's', 'dyn-01'), "
            "('dyn', 'd', 'x', 2, 'k4', 's', 's', 'dyn-01')) v(exp_id, cell, arm_label, seed, k, src, flush, matrix);",
            "CREATE TABLE experiment_cells AS SELECT * FROM (VALUES ('dyn', 'c', 'c', 'dyn-00'), ('dyn', 'd', 'd', 'dyn-01')) "
            "v(exp_id, cell, why, matrix);",
            "CREATE TABLE verdicts AS SELECT * FROM (VALUES ('k1', 's', false), ('k2', 's', true), ('k3', 's', true), "
            "('k4', 's', true)) v(k, src, collapsed);",
            "CREATE TABLE rows AS SELECT * FROM (VALUES "
            "('k1', 's', 2.0, 0.5, 0.01, 0.0, 900.0, 4000.0, 1.0, 0.25, 0.005, 0.0, 800.0, 3900.0), "
            "('k2', 's', 4.0, 1.5, 0.03, 0.02, 1100.0, 4200.0, NULL, NULL, NULL, NULL, NULL, NULL), "
            "('k3', 's', 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0), "
            "('k4', 's', 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)) v(k, src, "
            "stalls_per_min_1250, stalls_per_min_4250, stalled_share_1250, stalled_share_4250, startup_ms_1250, "
            "startup_ms_4250, stalls_per_min_1250_honest, stalls_per_min_4250_honest, stalled_share_1250_honest, "
            "stalled_share_4250_honest, startup_ms_1250_honest, startup_ms_4250_honest);",
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
        # 15-16 read nothing yet): 100 x49, 200 x17, 3000, 6000; the p99 is
        # the 68th (ceil(0.99 x 68)), 6000, read through the sketch (32 ms
        # buckets from 4096 ms) as 5984.
        self.assertEqual(c["lag_p99_startup_ms"], 5984)
        # Steady: 100 x32, 200 x15, 2000, 3000 (carried), 5000: the 50th,
        # read as 4992.
        self.assertEqual(c["lag_p99_steady_ms"], 4992)
        # After: k2's after_down (35-44 s) and after_departure (45-49 s)
        # pooled: 100 x13, 3500, 4500: the 15th, read as 4480.
        self.assertEqual(c["lag_p99_after_ms"], 4480)
        # k1 spikes past 1250 at 30-32 and past 4250 at 32, k2 at 40 and
        # past 1250 at 47.
        self.assertEqual((c["spike_s_1250"], c["spike_s_4250"]), (2.5, 1.0))
        # The playback is the honest viewers': k1's _honest fields (its
        # hostile viewer's stalls and startup left out), k2's own (a row
        # from before the split): 1.0 and 4.0 stalls a minute at 1250 ms,
        # where every viewer's would read 3.0.
        self.assertEqual((c["stalls_per_min_1250"], c["stalls_per_min_4250"]), (2.5, 0.875))
        self.assertAlmostEqual(c["stalled_share_1250"], 0.0175)
        self.assertAlmostEqual(c["stalled_share_4250"], 0.01)
        self.assertEqual((c["startup_ms_1250"], c["startup_ms_4250"]), (950.0, 4050.0))
        # The runs' longest unhealthy spells, 5 and 12 s: the median is the
        # first (rank ceil(0.5 x 2)).
        self.assertEqual((c["unhealthy_max_s"], c["unhealthy_p50_s"]), (12, 5))
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
        # after_departure: 100 x4, 3500: the p90 is the 5th, 3500, read
        # through the sketch (16 ms buckets from 2048 ms) as 3488.
        self.assertEqual(c["after_departure"]["lag_p90_ms"], 3488)
        self.assertEqual(c["startup"]["lag_max_ms"], 6000)


class PhaseLags(unittest.TestCase):
    """dynamics_phases and dynamics against simlab.verdict over one
    made-up run: a series of four viewers (the fourth hostile, the third
    joining at 30.3 s, a latency step at 50 s), its lags spread over
    0-6000 ms with seconds of no read; viewer_seconds, viewers and
    viewer_phases (the verdict's own viewer_phases) made of it. Each
    phase's lag quantiles in SQL are verdict.phase_lags' through
    verdict.quantile, and dynamics' p99s run_metrics'."""

    @classmethod
    def setUpClass(cls):
        from simlab import verdict

        cls.verdict = verdict
        secs, n = 60, 4
        rnd, x = [], 7
        for _ in range(secs * n):
            x = (x * 1103515245 + 12345) % 2 ** 31
            rnd.append(x)
        lag = [[-1 if rnd[i * secs + k] % 9 == 0 else rnd[i * secs + k] % 6000 for k in range(secs)]
               for i in range(n)]
        lag[2][:31] = [-1] * 31
        frames = [[0 if lag[i][k] < 0 else 1 + rnd[i * secs + k] % 10 for k in range(secs)] for i in range(n)]
        s = {"secs": secs, "rate": 10 * 4096, "chunk": 4096, "frame": 4096, "viewer_up": 10 ** 9,
             "seed_up": [[0, 10 ** 9]], "present": [n] * secs, "hits": [10 * n] * secs, "lag": lag, "frames": frames,
             "node": list(range(1, n + 1)), "viewer": list(range(n)), "hostile": [3],
             "events": [[30.3, "join", 3, None, None, None], [50, "latency", 0, None, 150, None]]}
        cls.s, v = s, verdict.verdict(s)
        cls.metrics = verdict.run_metrics(s, v)
        lit = lambda x: "NULL" if x is None else repr(x)
        vs = ", ".join(f"('k1', 's', {i}, {k}, {lit(lag[i][k] if frames[i][k] and lag[i][k] >= 0 else None)}, "
                       f"{frames[i][k]})" for i in range(n) for k in range(secs))
        spans = ", ".join(f"('k1', 's', {i}, '{p}', {a}, {b})" for i, p, a, b in verdict.viewer_phases(s, v))
        sql = [
            "CREATE TABLE jobs AS SELECT 'k1' AS k, 's' AS src, 'strims-sim' AS sim, 't' AS trace, '' AS reads_hash;",
            "CREATE TABLE experiment_jobs AS SELECT 'pl' AS exp_id, 'c' AS cell, 'x' AS arm_label, 1 AS seed, "
            "'k1' AS k, 's' AS src, 's' AS flush, 'pl-00' AS matrix;",
            "CREATE TABLE verdicts AS SELECT 'k1' AS k, 's' AS src, false AS collapsed;",
            "CREATE TABLE rows AS SELECT 'k1' AS k, 's' AS src;",
            "CREATE TABLE viewers AS SELECT * FROM (VALUES "
            + ", ".join(f"('k1', 's', {i}, {i + 1}, {30300 if i == 2 else 'NULL'}, {str(i == 3).lower()})"
                        for i in range(n)) + ") v(k, src, viewer, node, arrive_ms, hostile);",
            f"CREATE TABLE viewer_seconds AS SELECT * FROM (VALUES {vs}) v(k, src, viewer, sec, lag_ms, frames);",
            f"CREATE TABLE viewer_phases AS SELECT * FROM (VALUES {spans}) v(k, src, viewer, phase, start, stop);",
            "CREATE TABLE verdict_seconds AS SELECT 'k1' AS k, 's' AS src, range AS sec, NULL::VARCHAR AS phase "
            f"FROM range({verdict.seconds(s)});",
        ]
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.tmp.name) / "phases.duckdb"
        database(cls.db, "\n".join(sql))

    @classmethod
    def tearDownClass(cls):
        scoring.SCORE_DB = None
        cls.tmp.cleanup()

    def setUp(self):
        scoring.SCORE_DB = self.db

    def test_each_phases_lag_is_phase_lags(self):
        by = self.verdict.phase_lags(self.s)
        self.assertGreaterEqual(set(by), {"startup", "steady", "after_disruption"})
        got = {r["phase"]: r for r in scored("SELECT * FROM dynamics_phases('pl');")}
        self.assertEqual(set(got), set(by))
        for phase, lags in by.items():
            r = got[phase]
            self.assertEqual(r["viewer_s"], len(lags), phase)
            for p in (50, 90, 99):
                self.assertEqual(r[f"lag_p{p}_ms"], self.verdict.quantile(lags, p), (phase, p))
            self.assertEqual(r["lag_max_ms"], max(lags), phase)
        self.assertGreaterEqual(len(by["steady"]), 40)

    def test_dynamics_p99s_are_run_metrics(self):
        (d,) = scoring.dynamics("pl")
        self.assertEqual((d["lag_p99_startup_ms"], d["lag_p99_steady_ms"]),
                         (self.metrics["lag_p99_startup_ms"], self.metrics["lag_p99_steady_ms"]))
        self.assertEqual(d["lag_p99_after_ms"],
                         self.verdict.quantile(self.verdict.phase_lags(self.s)["after_disruption"], 99))


class TestbedSide(unittest.TestCase):
    """testbed_side: one kind and one arm by default, a named kind or arm
    checked, no two runs at one cell, arm and seed, and a twin's source
    refused."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()

    @classmethod
    def tearDownClass(cls):
        scoring.SCORE_DB = None
        cls.tmp.cleanup()

    def side(self, rows, call):
        db = Path(self.tmp.name) / f"side-{abs(hash((tuple(rows), call)))}.duckdb"
        values = ", ".join(f"('{k}', '{src}', '{sim}', MAP {{{env}}}::MAP(VARCHAR, VARCHAR), '{cell}', '{arm}', {seed})"
                           for k, env, cell, arm, seed, src, sim in (r + ("testbed/x", "testbed")[len(r) - 5:]
                                                                     for r in rows))
        database(db, "CREATE TABLE jobs (k VARCHAR, src VARCHAR, sim VARCHAR, env MAP(VARCHAR, VARCHAR), "
                     f"stored_cell VARCHAR, stored_arm VARCHAR, seed BIGINT); INSERT INTO jobs VALUES {values};")
        return scored(f"SELECT {call} AS side;")[0]["side"]

    def test_a_side_is_one_kind_one_arm_and_one_run_a_key(self):
        rtc = "'STRIMS_SIM_RTC': 'chromium'"
        runs = [("a", "", "c", "main", 1), ("b", "", "c", "main", 2), ("c", rtc, "c", "main", 1)]
        with self.assertRaisesRegex(suite.simdb().DuckError, "native and browser runs"):
            self.side(runs, "testbed_side('testbed/%')")
        self.assertEqual([r["k"] for r in self.side(runs, "testbed_side('testbed/%', kind := 'native')")], ["a", "b"])
        with self.assertRaisesRegex(suite.simdb().DuckError, "kind browser is none of"):
            self.side(runs, "testbed_side('testbed/%', kind := 'browser')")
        with self.assertRaisesRegex(suite.simdb().DuckError, "no testbed run of arm late"):
            self.side(runs, "testbed_side('testbed/%', kind := 'any', arm := 'late')")
        two = [("a", "", "c", "main", 1), ("b", "", "c", "main", 1)]
        with self.assertRaisesRegex(suite.simdb().DuckError, "1 runs share a cell, arm and seed"):
            self.side(two, "testbed_side('testbed/%')")

    def test_a_twins_source_is_refused_and_a_source_of_none_is_no_runs(self):
        # A twin's runs are a sim's: its side is its jobs, which
        # testbed_side would leave out.
        runs = [("a", "", "c", "main", 1), ("t1", "", "c", "twin", 1, "testbed/x-twin", "strims-sim"),
                ("t2", "", "c", "twin", 2, "testbed/x-twin", "strims-sim")]
        with self.assertRaisesRegex(suite.simdb().DuckError,
                                    "testbed/x-twin names 2 sim runs and no testbed run; a twin's side is its jobs"):
            self.side(runs, "testbed_side('testbed/x-twin', kind := 'native')")
        self.assertEqual([r["k"] for r in self.side(runs, "testbed_side('testbed/x%')")], ["a"])
        self.assertEqual(self.side(runs, "testbed_side('testbed/none')"), [])


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
        self.assertEqual(one["lag_p99_ms"], 1120)
        # Of two, the median is the first (rank ceil(0.5 x 2)).
        self.assertEqual(by[("uplink_rate", "anchor", -2)]["lag_p50_ms"], 120)
        # The start's: viewer 0's lag from 0 s (viewer 1 not there yet).
        self.assertEqual([by[("start", "anchor", r)]["lag_p50_ms"] for r in range(3)], [100, 101, 102])
        # The crash from its anchor (28-32 s) and from its end (31-35 s).
        self.assertEqual(by[("crash", "anchor", -2)]["lag_p50_ms"], 128)
        self.assertEqual(by[("crash", "end", 0)]["lag_p50_ms"], 133)
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
        # Without arrivals the coverage macros say so; the asks fallback,
        # whose comment gives its columns (`-- else none:`), has no rows.
        with self.assertRaisesRegex(suite.simdb().DuckError, "coverage_runs_of reads arrivals, parents"):
            scored("SELECT * FROM coverage_runs_of([{'k': 'k1'}]);")
        self.assertEqual(scored("SELECT * FROM coverage_ask_streams_of([{'k': 'k1'}]);"), [])


if __name__ == "__main__":
    unittest.main()
