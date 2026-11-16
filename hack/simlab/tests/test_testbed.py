#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.testbed: testbed runs as their twins' cells, and the tier.

  Cells    a run's knobs name its tier cell but for what a testbed run
           always sets (TESTBED_FIXED, TESTBED_*); a run of no cell names
           its knobs'
  Joins    an arrivals profile's joins hold its bounds and the twin's
           arrival order (soonest first)
  Tier     the tier's cells are hostable, honest, of the robust and
           browsers tiers; hack/sim-farm/matrices/testbed-tier is what
           tier_matrices writes

  python3 hack/simlab/tests/test_testbed.py
"""

import random
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import suite, testbed  # noqa: E402


class Cells(unittest.TestCase):
    def test_a_runs_knobs_name_its_tier_cell(self):
        c = next(c for c in suite.TIERS["robust"] if c["name"] == "star-15-10")
        env = {**c["env"], "STRIMS_SIM_HOSTILE": "off", "STRIMS_SIM_CLOCK_SKEW": "off",
               "STRIMS_SIM_CAL_SEED_BURST": "3028", "STRIMS_SIM_CAL_VIEWER_BURST": "3028",
               "TESTBED_QDISC": "clsact>ifb-netem>root-class", "STRIMS_SIM_CAL_SEED_LINK": "tbf",
               "STRIMS_SIM_CAL_VIEWER_LINK": "tbf"}
        for k in testbed.JOB_KNOBS:
            env.pop(k, None)
        self.assertEqual(testbed.cell_of(env), "star-15-10")
        self.assertEqual(testbed.cell_of({**env, "STRIMS_SIM_CAL_LINKS": "1:tbf 2:tbf"}), "star-15-10")
        # Another link class is another cell's (none of the tier's).
        self.assertIsNone(testbed.cell_of({**env, "STRIMS_SIM_CAL_VIEWER_LINK": "cable,slot=2-4,aqm=fifo"}))
        # A setting no twin knob takes makes it no cell's.
        self.assertIsNone(testbed.cell_of({**env, "TESTBED_SEED_LOSS_PCT": "1"}))
        self.assertIsNone(testbed.cell_of({**env, "STRIMS_SIM_CAL_SECS": "61"}))

    def test_a_run_of_no_cell_names_its_knobs(self):
        a = testbed.knob_cell({"STRIMS_SIM_CAL_VIEWERS": "3"})
        self.assertTrue(a.startswith("tb-"))
        self.assertEqual(a, testbed.knob_cell({"STRIMS_SIM_CAL_VIEWERS": "3"}))
        self.assertNotEqual(a, testbed.knob_cell({"STRIMS_SIM_CAL_VIEWERS": "7"}))


class Joins(unittest.TestCase):
    def parse(self, joins):
        # A node's --join-at takes whole seconds.
        return [(int(i), int(t)) for i, t in (p.split(":") for p in joins.split())]

    def test_a_trickle_joins_within_the_streams_first_third_soonest_first(self):
        js = self.parse(testbed.joins_of("trickle", 15, 60, random.Random(1)))
        self.assertEqual([i for i, _ in js], list(range(1, 16)))
        ts = [t for _, t in js]
        self.assertEqual(ts, sorted(ts))
        self.assertTrue(all(0 <= t <= 20 for t in ts))

    def test_an_events_warm_share_joins_before_its_burst(self):
        js = self.parse(testbed.joins_of("event:20:20:10", 15, 60, random.Random(2)))
        ts = [t for _, t in js]
        self.assertEqual(sum(t < 20 for t in ts), 3)
        self.assertTrue(all(t <= 30 for t in ts))

    def test_another_profile_is_refused(self):
        with self.assertRaises(ValueError):
            testbed.joins_of("trickle:2", 3, 60, random.Random(1))


class Tier(unittest.TestCase):
    def test_the_tiers_cells_are_hostable_and_honest(self):
        cells = testbed.tier_cells()
        self.assertTrue(cells)
        for kind, c in cells:
            with self.subTest(cell=c["name"], kind=kind):
                env = c["env"]
                self.assertEqual(c["test"], "calibration_star")
                self.assertEqual(env["STRIMS_SIM_HOSTILE"], "off")
                self.assertFalse(any(k in env for k in testbed.UNHOSTED))
                self.assertLessEqual(int(env["STRIMS_SIM_CAL_VIEWERS"]), testbed.MAX_VIEWERS[kind])
                self.assertEqual("STRIMS_SIM_RTC" in env, kind == "browsers")

    def test_the_committed_tier_is_tier_matrices(self):
        committed = HACK / "sim-farm" / "matrices" / "testbed-tier"
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / "testbed-tier"
            paths = testbed.tier_matrices(out, repeats=2, ref="main")
            self.assertEqual(sorted(p.name for p in paths), sorted(p.name for p in committed.glob("*.toml")))
            # A topology's path is relative to its matrix's dir: compare
            # its file.
            def plain(text):
                return [Path(line.split('"')[1]).name if line.startswith("TOPOLOGY = ") else line
                        for line in text.splitlines()]

            for p in paths:
                with self.subTest(matrix=p.name):
                    self.assertEqual(plain((committed / p.name).read_text()), plain(p.read_text()))


if __name__ == "__main__":
    unittest.main()
