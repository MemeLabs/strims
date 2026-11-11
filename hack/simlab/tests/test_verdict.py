#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.verdict: hack/capture/analyze.py's verdicts, which analyze.py
re-exports as they are, under the version the CLI golden's derived tables
were made under, judged by a Python without the duckdb module:

  python3 hack/simlab/tests/test_verdict.py
"""

import importlib.util
import json
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
from simlab import verdict  # noqa: E402

# The names analyze.py's importers read (simdb.py, catalog.py, suite.py,
# sim-suite's golden.py, the testbed's twin.py, scripts): its verdicts.
NAMES = (
    "E2E_TARGET_MS", "E2E_MAX_MS", "PIPELINE_MS", "DELIVERY_TARGET_MS", "DELIVERY_MAX_MS", "TREES_SHARE",
    "TREES_SMOOTH", "TREES_SECS", "DELIVERY_SHARE", "DELIVERY_SECS", "LAG_QUANTILE", "LAG_HIGH_MS", "LAG_HIGH_SECS",
    "LAG_SLOPE", "LAG_SLOPE_SECS", "LAG_SLOPE_FLOOR_MS", "RECOVER_SECS", "ABSORB_SECS", "STARTUP_SECS",
    "COLLAPSED_SECS", "JOIN_GRACE_SECS", "STEP_SECS", "OSC_ACF", "OSC_MIN_PERIOD", "OVERHEAD", "med", "quantile",
    "lags", "presence", "startup", "measures", "slope", "runs_of", "verdict", "run_startup", "collapsed", "steps",
    "phases", "events", "episodes", "acf", "run_metrics", "verdict_version", "feasible", "VERDICT_VERSION",
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


if __name__ == "__main__":
    unittest.main()
