#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.verdict: hack/capture/analyze.py's verdicts, which analyze.py
re-exports as they are, under the version the CLI golden's derived tables
were made under, judged by a Python without the duckdb module; and the
phase rule (Phases) over made-up series:

  python3 hack/simlab/tests/test_verdict.py
"""

import importlib.util
import json
import math
import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
ANALYZE = HACK / "capture" / "analyze.py"
sys.path.insert(0, str(HACK))
sys.path.insert(0, str(HERE))

import goldens  # noqa: E402
from simlab import verdict, vocab  # noqa: E402

# The names analyze.py's importers read (simdb.py, catalog.py, suite.py,
# sim-suite's golden.py, the testbed's twin.py, scripts): its verdicts.
NAMES = (
    "E2E_TARGET_MS", "E2E_MAX_MS", "PIPELINE_MS", "DELIVERY_TARGET_MS", "DELIVERY_MAX_MS", "TREES_SHARE",
    "TREES_SMOOTH", "TREES_SECS", "DELIVERY_SHARE", "DELIVERY_SECS", "LAG_QUANTILE", "LAG_HIGH_MS", "LAG_HIGH_SECS",
    "LAG_SLOPE", "LAG_SLOPE_SECS", "LAG_SLOPE_FLOOR_MS", "RECOVER_SECS", "ABSORB_SECS", "STARTUP_SECS",
    "COLLAPSED_SECS", "STEP_SECS", "OSC_ACF", "OSC_MIN_PERIOD", "OVERHEAD", "med", "quantile", "lags", "presence",
    "startup", "measures", "slope", "runs_of", "verdict", "run_startup", "collapsed", "steps", "timeline", "phases",
    "events", "episodes", "acf", "run_metrics", "verdict_version", "feasible", "VERDICT_VERSION", "phase_lags",
)

# A run of two viewers there from the start whose seed's uplink steps
# down at 20 s and back up at 36 s, no pushes coming and its lag climbing
# meanwhile: it collapses at 20 s and recovers.
SECS = 90
DOWN = range(20, 36)
SERIES = {
    "secs": SECS, "rate": 256 * 1024, "chunk": 1024, "frame": 1024, "viewer_up": 2 * 20 * 125000,
    "seed_up": [[0, 10 * 125000], [20, 2 * 125000], [36, 10 * 125000]],
    "present": [2] * SECS,
    "hits": [0 if k in DOWN else 2 * 256 for k in range(SECS)],
    "lag": [[(400 + 300 * (k - 19) if k in DOWN else 400 + 50 * (k % 4)) for k in range(SECS)] for _ in range(2)],
    "frames": [[(64 if k in DOWN else 256) for k in range(SECS)] for _ in range(2)],
}

# The plain python3's judgement, analyze.py loaded by path (as simdb.py
# loads it) with every import of duckdb failing.
JUDGE = f"""\
import importlib.util, json, sys
sys.modules["duckdb"] = None
spec = importlib.util.spec_from_file_location("analyze", {str(ANALYZE)!r})
a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a)
s = json.loads(sys.stdin.read())
v = a.verdict(s)
print(json.dumps({{"version": a.VERDICT_VERSION, "verdict": v, "collapsed": a.collapsed(v),
                  "phases": a.phases(s, v), "events": a.events(s, v), "episodes": a.episodes(v),
                  "metrics": a.run_metrics(s, v), "engine": "simlab.engine" in sys.modules}}))
"""


def analyze():
    spec = importlib.util.spec_from_file_location("analyze", ANALYZE)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


class Verdict(unittest.TestCase):
    def test_analyze_holds_simlabs_verdicts(self):
        a = analyze()
        for name in NAMES:
            self.assertIs(getattr(a, name), getattr(verdict, name), name)

    def test_the_version_is_the_one_the_golden_derived_under(self):
        catalog = goldens.read(goldens.TESTDATA / "cli.txt.gz")["catalog"]
        info = json.loads(catalog.split("=== catalog_info\n", 1)[1].splitlines()[0])
        want = {r["key"]: r["value"] for r in info}["analyze_verdict_version"]
        self.assertEqual(verdict.VERDICT_VERSION, want)
        self.assertEqual(verdict.verdict_version(), want)

    def test_a_python_without_duckdb_judges_a_run(self):
        p = subprocess.run([sys.executable, "-I", "-c", JUDGE], input=json.dumps(SERIES), capture_output=True,
                           text=True, cwd="/", timeout=60)
        self.assertEqual(p.returncode, 0, p.stderr)
        got = json.loads(p.stdout)
        v = verdict.verdict(SERIES)
        want = {"version": verdict.VERDICT_VERSION, "verdict": v, "collapsed": verdict.collapsed(v),
                "phases": verdict.phases(SERIES, v), "events": verdict.events(SERIES, v),
                "episodes": verdict.episodes(v), "metrics": verdict.run_metrics(SERIES, v), "engine": False}
        self.assertEqual(got, json.loads(json.dumps(want)))
        self.assertTrue(got["collapsed"])
        self.assertEqual(got["metrics"]["collapses"], 1)
        self.assertEqual(got["verdict"]["onset"], 20)
        self.assertIsNotNone(got["verdict"]["recovered"])

    def test_the_versions_hash_the_code_that_loaded(self):
        # A process that outlives an edit to its source files (the console
        # server's forkserver children) hashes what it runs: the versions
        # don't read the files again, so an edited file on disk moves
        # neither.
        from unittest import mock

        from simlab import catalog

        derived, scoring = verdict.derived_version(), catalog.scoring_version()
        self.addCleanup(verdict.derived_version.cache_clear)
        verdict.derived_version.cache_clear()
        with mock.patch("inspect.getsource", lambda f: "an edited file's text"):
            self.assertEqual(verdict.derived_version(), derived)
            self.assertEqual(catalog.scoring_version(), scoring)


def run(secs=70, viewers=2, events=None, **fields):
    """A series of `secs` seconds whose `viewers` (nodes 1 up) read their
    10 chunks a second at 200 ms throughout, the seed node 0 at 1 Gbit/s;
    with `events`, the series' (rule_events'), and `fields` over it."""
    s = {"secs": secs, "rate": 10 * 4096, "chunk": 4096, "frame": 4096, "viewer_up": 10 ** 9,
         "seed_up": [[0, 10 ** 9]], "present": [viewers] * secs, "hits": [10 * viewers] * secs,
         "lag": [[200] * secs for _ in range(viewers)], "frames": [[10] * secs for _ in range(viewers)],
         "node": list(range(1, viewers + 1)), "viewer": list(range(viewers))}
    if events is not None:
        s["events"] = [list(e) for e in events]
    return {**s, **fields}


def ev(t, kind, node, value=None, peer=None, detail=None):
    return [t, kind, node, peer, value, detail]


class Phases(unittest.TestCase):
    """The phase rule (verdict.timeline) over made-up series: which events
    anchor, a level's direction, a span's end, which window holds a
    second, each viewer's incarnations, and the windows a series without
    events implies."""

    START = verdict.STARTUP_SECS
    JOIN = verdict.ANCHORS["join"]["horizon_s"]

    def phases(self, s):
        return verdict.phases(s, verdict.verdict(s))

    def spans(self, s):
        return verdict.viewer_phases(s, verdict.verdict(s))

    def test_the_horizons_are_the_documents(self):
        self.assertEqual(self.START, vocab.kind("start")["horizon_s"])
        self.assertEqual(verdict.STEP_SECS, vocab.kind("uplink_rate")["horizon_s"])
        self.assertFalse(hasattr(verdict, "JOIN_GRACE_SECS"))

    def test_setup_events_anchor_nothing(self):
        # Joins, a latency, a starting rate and a starting cap, all at t <=
        # 0: the run's startup, then steady; the start's window alone.
        s = run(events=[ev(-5, "join", 1), ev(-5, "join", 2), ev(-5, "latency", 0, 150),
                        ev(-5, "uplink_rate", 0, 10 ** 9), ev(0, "rate_limit", 1, 10 ** 6)])
        self.assertEqual(self.phases(s), ["startup"] * self.START + ["steady"] * (70 - self.START))
        self.assertEqual(verdict.phase_windows(s), [(None, "start", None, "startup", 0, None, self.START,
                                                     self.START)])
        self.assertTrue(all(x == "counts" for st in verdict.presence(s) for x in st))

    def test_a_slow_joiner_counts_from_its_join_plus_the_horizon(self):
        # Viewer 1 joins at 10.3 s and reads at once, or only from 22 s:
        # either way its own startup holds its seconds from its join's
        # (10 s, which it joined in) to its join plus join's horizon.
        for first in (10, 22):
            s = run(events=[ev(10.3, "join", 2)])
            s["frames"][1] = [0] * first + [10] * (70 - first)
            s["lag"][1] = [-1] * first + [200] * (70 - first)
            st = verdict.presence(s)[1]
            self.assertEqual(st[:10], [None] * 10, first)
            self.assertEqual(st[10:10 + self.JOIN], ["own"] * self.JOIN, first)
            self.assertEqual(set(st[10 + self.JOIN:]), {"counts"}, first)
            self.assertEqual(self.spans(s)[-2:], [(1, "startup", 10, 10 + self.JOIN),
                                                  (1, "steady", 10 + self.JOIN, 70)], first)

    def test_a_levels_direction_is_against_the_nodes_previous_level(self):
        # The seed's rate: 100 from the start, 50 at 30 s (down), 70 at 40
        # s (up, though under the start's), 70 again at 45 s (no change);
        # a viewer's cap at 50 s without a cap before (down, from none).
        s = run(events=[ev(-1, "uplink_rate", 0, 100), ev(30, "uplink_rate", 0, 50), ev(40, "uplink_rate", 0, 70),
                        ev(45, "uplink_rate", 0, 70), ev(50, "rate_limit", 1, 30)])
        got = [(kind, p, a) for _, kind, _, p, a, *_ in verdict.phase_windows(s)]
        self.assertEqual(got, [("start", "startup", 0), ("uplink_rate", "after_down", 30),
                               ("uplink_rate", "after_up", 40), ("rate_limit", "after_down", 50)])
        ph = self.phases(s)
        self.assertEqual((ph[29], ph[30], ph[39], ph[40], ph[49], ph[50]),
                         ("steady", "after_down", "after_down", "after_up", "after_up", "after_down"))
        # The seed's steps, its derived events' (the new over the old).
        self.assertEqual(verdict.steps(s), [(30, "seed_down", 0.5), (40, "seed_up", 1.4)])

    def test_a_span_runs_to_its_end_plus_its_horizon(self):
        h = verdict.ANCHORS["crash"]["horizon_s"]
        s = run(events=[ev(30, "crash", 5), ev(34, "restart", 5)])
        ph = self.phases(s)
        self.assertEqual(ph[29:31], ["steady", "after_departure"])
        self.assertEqual(ph[34 + h - 1:34 + h + 1], ["after_departure", "steady"])
        self.assertEqual(verdict.phase_windows(s)[1], (None, "crash", 5, "after_departure", 30, 34, 34 + h, h))
        # Without its end, to the run's end (and its horizon past it).
        s = run(events=[ev(40, "competing_start", 0)])
        self.assertEqual(self.phases(s)[40:], ["after_down"] * 30)
        self.assertEqual(verdict.phase_windows(s)[1][5:7], (None, 70 + verdict.ANCHORS["competing_start"]["horizon_s"]))
        # A partition ends at the repair of its node and peer, not
        # another's.
        s = run(events=[ev(30, "partition", 1, peer=2), ev(31, "repair", 1, peer=3), ev(33, "repair", 1, peer=2)])
        self.assertEqual(verdict.phase_windows(s)[1][5], 33)

    def test_the_latest_window_wins(self):
        # A fault during the run's startup takes the seconds from it on,
        # and ends the startup a collapse must last past.
        s = run(events=[ev(5, "latency", 0, 150)])
        self.assertEqual(self.phases(s)[:6], ["startup"] * 5 + ["after_disruption"])
        self.assertEqual(verdict.startup(s), 5)
        # A join during after_down: the joiner's own startup, then the
        # step's window again while it lasts; the run's after_down
        # throughout.
        s = run(events=[ev(-1, "uplink_rate", 0, 100), ev(30, "uplink_rate", 0, 50), ev(40, "join", 2)])
        self.assertEqual(self.phases(s)[30:], ["after_down"] * 40)
        self.assertEqual([x for x in self.spans(s) if x[0] == 1],
                         [(1, "startup", 40, 40 + self.JOIN), (1, "after_down", 40 + self.JOIN, 70)])
        # A step during a joiner's own startup takes its seconds' phase,
        # but the joiner counts in the verdict only from its join plus the
        # horizon still.
        s = run(events=[ev(-1, "uplink_rate", 0, 100), ev(30, "join", 2), ev(35, "uplink_rate", 0, 50)])
        self.assertEqual([x for x in self.spans(s) if x[0] == 1], [(1, "startup", 30, 35), (1, "after_down", 35, 70)])
        self.assertEqual(verdict.presence(s)[1][30:], ["own"] * self.JOIN + ["counts"] * (40 - self.JOIN))
        # A tie goes to the kind the document lists first: a viewer's join
        # at its crash's restart is its own startup's, the run's the
        # crash's.
        s = run(events=[ev(20, "crash", 2), ev(25, "restart", 2), ev(25, "join", 2)])
        self.assertEqual([x for x in self.spans(s) if x[0] == 1][-2], (1, "startup", 25, 25 + self.JOIN))
        self.assertEqual(self.phases(s)[25], "after_departure")

    def test_each_churn_incarnation_gets_its_own_startup(self):
        # Viewer 1 is there from the start, leaves at 20.6 s, takes its
        # place again at 25.6 s, leaves at 50.2 s and is back at 55.2 s.
        s = run(80, events=[ev(-1, "join", 1), ev(-1, "join", 2), ev(20.6, "leave", 2), ev(25.6, "join", 2),
                            ev(50.2, "leave", 2), ev(55.2, "join", 2)])
        self.assertEqual(verdict.incarnations(s, verdict.rule_events(s))[1],
                         [(None, 20.6), (25.6, 50.2), (55.2, None)])
        st = verdict.presence(s)[1]
        own = lambda join: math.floor(join + self.JOIN - 0.5) + 1
        self.assertEqual(st[19:22], ["counts", "leaving", None])
        self.assertEqual(st[25:own(25.6) + 1], ["own"] * (own(25.6) - 25) + ["counts"])
        self.assertEqual(st[49:56], ["counts", "leaving", None, None, None, None, "own"])
        self.assertEqual(st[own(55.2) - 1:own(55.2) + 1], ["own", "counts"])
        spans = [x for x in self.spans(s) if x[0] == 1]
        # Its leaving second 20 is the run's (the leave's window from 20.6
        # s holds 21 on).
        self.assertEqual([x for x in spans if x[1] == "startup"],
                         [(1, "startup", 0, 21), (1, "startup", 25, own(25.6)), (1, "startup", 55, own(55.2))])
        # Each leave's window runs to its successor's join, then the
        # horizon.
        stop = lambda join: math.ceil(join + verdict.ANCHORS["leave"]["horizon_s"] - 0.5)
        self.assertEqual(self.phases(s), ["startup"] * 21 + ["after_departure"] * (stop(25.6) - 21)
                         + ["steady"] * (50 - stop(25.6)) + ["after_departure"] * (stop(55.2) - 50)
                         + ["steady"] * (80 - stop(55.2)))

    def test_implied_anchors_and_real_events_give_the_same_phases(self):
        # The seed's uplink from seed_up and the viewers' stays, against
        # the same as events: every table of the rule's the same.
        up = [[0, 10 ** 9], [30, 5 * 10 ** 8], [50, 10 ** 9]]
        implied = run(viewers=3, seed_up=up, arrive_ms=[-1000, 12500, -1000], depart_ms=[None, None, 40500])
        events = [ev(-1, "join", 1), ev(-1, "join", 3), ev(12.5, "join", 2), ev(40.5, "depart", 3),
                  ev(-1, "uplink_rate", 0, 10 ** 9), ev(30, "uplink_rate", 0, 5 * 10 ** 8),
                  ev(50, "uplink_rate", 0, 10 ** 9)]
        real = {**implied, "events": events}
        # And events that lack the seed's starting level (strims-sim's
        # older up_step), or any of its levels (the ppspp sim's older).
        older = {**implied, "events": [e for e in events if not (e[1] == "uplink_rate" and e[0] <= 0)]}
        oldest = {**implied, "events": [e for e in events if e[1] != "uplink_rate"]}
        want = self.tables(real)
        for s in (implied, older, oldest):
            self.assertEqual(self.tables(s), want)
        self.assertEqual({p for p in want["phases"]}, {"startup", "steady", "after_down", "after_up",
                                                       "after_departure"})

    def tables(self, s):
        v = verdict.verdict(s)
        return {"verdict": v, "phases": verdict.phases(s, v), "viewer_phases": verdict.viewer_phases(s, v),
                "presence": verdict.presence(s), "windows": verdict.phase_windows(s), "steps": verdict.steps(s),
                "events": verdict.events(s, v), "metrics": verdict.run_metrics(s, v)}

    def test_the_events_time_where_seed_up_reads_a_seconds_end(self):
        # The ppspp sim's seed_up reads the uplink at each second's end: a
        # step at 9.0 s shows at second 8. The events' time wins: the
        # step's window from second 9, seed_up's alone from 8.
        up = [[0, 10 ** 9], [8, 5 * 10 ** 8]]
        events = [ev(-1, "uplink_rate", 0, 10 ** 9), ev(9.0, "uplink_rate", 0, 5 * 10 ** 8)]
        self.assertEqual(self.phases(run(seed_up=up, events=events))[7:10], ["startup", "startup", "after_down"])
        self.assertEqual(verdict.steps(run(seed_up=up, events=events)), [(9.0, "seed_down", 0.5)])
        self.assertEqual(self.phases(run(seed_up=up))[7:10], ["startup", "after_down", "after_down"])

    def test_the_lag_p99s_are_the_honest_viewers_over_their_own_phases(self):
        # Viewers 0 and 1 are there from the start and lag 10 k and 10 k + 5
        # ms in second k; viewer 2 joins at 30.3 s, reads first at 31 s
        # (3000 ms), nothing in 32 and 33 (carried: 4000 and 5000) and at
        # 300 ms from 34 s; viewer 3, hostile, lags 9000 ms throughout; a
        # latency step at 55 s opens after_disruption.
        s = run(60, viewers=4, events=[ev(30.3, "join", 3), ev(55, "latency", 0, 150)], hostile=[3])
        s["lag"] = [[10 * k for k in range(60)], [10 * k + 5 for k in range(60)],
                    [-1] * 31 + [3000, -1, -1] + [300] * 26, [9000] * 60]
        s["frames"][2] = [-1] * 30 + [0, 10, 0, 0] + [10] * 26
        by = verdict.phase_lags(s)
        # Startup: viewers 0 and 1's seconds 0-24 (0-240 and 5-245) and
        # viewer 2's own 30-49 but 30, without a lag yet (3000, 4000, 5000
        # and 300 x 16): 69 lags, whose p99 (index round(68 x 0.99) = 67)
        # is the second largest, 4000.
        self.assertEqual(sorted(by["startup"]), sorted([10 * k for k in range(25)] + [10 * k + 5 for k in range(25)]
                                                       + [3000, 4000, 5000] + [300] * 16))
        # Steady: 0 and 1's 25-54 (250-540, 255-545) and 2's 50-54 (300 x
        # 5): 65 lags, p99 index 63, 540. The latency's seconds are
        # after_disruption's.
        self.assertEqual(sorted(by["steady"]), sorted([10 * k for k in range(25, 55)]
                                                      + [10 * k + 5 for k in range(25, 55)] + [300] * 5))
        self.assertEqual(sorted(by["after_disruption"]), sorted([10 * k for k in range(55, 60)]
                                                                + [10 * k + 5 for k in range(55, 60)] + [300] * 5))
        m = verdict.run_metrics(s, verdict.verdict(s))
        self.assertEqual((m["lag_p99_startup_ms"], m["lag_p99_steady_ms"]), (4000, 540))
        # A run of its startup alone has no steady lag.
        m = verdict.run_metrics(run(20), verdict.verdict(run(20)))
        self.assertEqual((m["lag_p99_startup_ms"], m["lag_p99_steady_ms"]), (200, None))

    def test_the_vocabulary_is_the_one_the_module_loaded(self):
        # The rule and VERDICT_VERSION read the snapshot taken at import,
        # whatever the document on disk says later.
        from unittest import mock

        doc = vocab.parse(vocab.DOCUMENT.read_text().replace("horizon_s = 25", "horizon_s = 99"))
        self.assertNotEqual(vocab.anchor_fields(doc)["start"], verdict.ANCHORS["start"])
        with mock.patch.object(vocab, "load", lambda path=None: doc):
            self.assertEqual(verdict.verdict_version(), verdict.VERDICT_VERSION)
            self.assertEqual(verdict.startup(run()), self.START)
        # And it hashes them: another horizon is another version.
        with mock.patch.dict(verdict.ANCHORS, {"start": {**verdict.ANCHORS["start"], "horizon_s": 99}}):
            self.assertNotEqual(verdict.verdict_version(), verdict.VERDICT_VERSION)


if __name__ == "__main__":
    unittest.main()
