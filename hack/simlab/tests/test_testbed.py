#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.testbed: testbed runs as their twins' cells, and the tier.

  Cells    a run's knobs name its tier cell's testbed variant
           (`<cell>@<tree>`) but for what a testbed run always sets
           (TESTBED_FIXED, tbf links); one with another TESTBED_ setting or
           of no cell names its knobs; each tier cell's run.sh settings,
           through knobs_of, name that cell
  Joins    an arrivals profile's joins hold its bounds and the twin's
           arrival order (soonest first)
  Tier     the tier's cells are hostable, honest, of the robust and
           browsers tiers; hack/sim-farm/matrices/testbed-tier is what
           tier_matrices writes

  python3 hack/simlab/tests/test_testbed.py
"""

import json
import os
import random
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import store_host, suite, testbed  # noqa: E402


class Cells(unittest.TestCase):
    def test_a_runs_knobs_name_its_tier_cells_testbed_variant(self):
        c = next(c for c in suite.TIERS["robust"] if c["name"] == "star-15-10")
        env = {**c["env"], "STRIMS_SIM_HOSTILE": "off", "STRIMS_SIM_CLOCK_SKEW": "off",
               "STRIMS_SIM_CAL_SEED_BURST": "3028", "STRIMS_SIM_CAL_VIEWER_BURST": "3028",
               "TESTBED_QDISC": "clsact>ifb-netem>root-class", "STRIMS_SIM_CAL_SEED_LINK": "tbf",
               "STRIMS_SIM_CAL_VIEWER_LINK": "tbf"}
        for k in testbed.JOB_KNOBS:
            env.pop(k, None)
        got = testbed.cells_of([
            env, {**env, "STRIMS_SIM_CAL_LINKS": "1:tbf 2:tbf"},
            # The tree that shaped it is in the name: two trees never pair.
            {**env, "TESTBED_QDISC": "netem>tbf"},
            # Another link class, a setting no twin knob takes, another cell.
            {**env, "STRIMS_SIM_CAL_VIEWER_LINK": "cable,slot=2-4,aqm=fifo"},
            {**env, "TESTBED_SEED_LOSS_PCT": "1"}, {**env, "STRIMS_SIM_CAL_SECS": "61"},
            {**env, "STRIMS_SIM_SEED": "3"}])
        self.assertEqual(got[:3], ["robust:star-15-10@root-class", "robust:star-15-10@root-class",
                                   "robust:star-15-10@tbf"])
        self.assertTrue(all(x.startswith("tb-") for x in got[3:6]), got)
        self.assertEqual(len(set(got[3:6])), 3)
        # A job key is no knob of its cell's.
        self.assertEqual(got[6], got[0])

    def test_a_sources_cell_of_two_knobs_has_no_twin(self):
        c = next(c for c in suite.TIERS["robust"] if c["name"] == "star-15-10")
        env = {**c["env"], "STRIMS_SIM_HOSTILE": "off", "TESTBED_QDISC": "clsact>ifb-netem>root-class"}
        with tempfile.TemporaryDirectory() as d, mock.patch.dict(os.environ, {"SIM_DB_ROOT": d}):
            runs = Path(d) / "cache" / "testbed" / "two"
            runs.mkdir(parents=True)
            rows = [{"sim": "testbed", "env": json.dumps(e)} for e in
                    (env, {**env, "STRIMS_SIM_CAL_SEED_BURST": "1514"})]
            (runs / "runs.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
            store_host.duck(f"COPY (SELECT * FROM read_json('{runs / 'runs.jsonl'}')) TO '{runs / 'runs.parquet'}';",
                            out=None)
            with self.assertRaisesRegex(ValueError, "two knobs \\(STRIMS_SIM_CAL_SEED_BURST\\)"):
                testbed.cells_in("testbed/two")

    def test_the_tiers_runs_name_their_cells_through_knobs_of(self):
        # run.sh's settings for each tier cell (run_env), read by
        # testbed_tables --knobs (strims-sim's knobs_of) as run.sh's out
        # dir holds them, name that cell's testbed variant.
        if not testbed.BINARY.exists():
            self.skipTest(f"no {testbed.BINARY}: cargo build --release -p strims-sim --example testbed_tables")
        matrices = HACK / "sim-farm" / "matrices"
        envs, want = [], []
        with tempfile.TemporaryDirectory() as d:
            for kind, cell in testbed.tier_cells():
                settings, knobs, topology = testbed.run_env(kind, cell)
                out = Path(d) / f"{kind}-{cell['name']}"
                out.mkdir()
                viewers = int(settings["VIEWERS"])
                (out / "settings").write_text("".join(f"{k}={v}\n" for k, v in settings.items())
                                              + "QDISC=clsact>ifb-netem>root-class\n")
                links = ((matrices / "topologies" / topology).read_text() if topology else
                         "".join(f"link 0 {v}\n" for v in range(1, viewers + 1)))
                (out / "topology").write_text(links)
                up = lambda i: settings["SEED_UP_MBIT"] if i == 0 else settings.get("VIEWER_UP_MBIT", "20")
                (out / "links").write_text("".join(f"node {i} tbf {up(i)} 10 100\n" for i in range(viewers + 1)))
                p = subprocess.run([str(testbed.BINARY), "--knobs", str(out)], capture_output=True, text=True)
                self.assertEqual(p.returncode, 0, f"{kind} {cell['name']}: {p.stderr}")
                envs.append({**json.loads(p.stdout), **knobs})
                want.append(f"{'robust' if kind == 'native' else 'browsers'}:{cell['name']}@root-class")
        self.assertEqual(testbed.cells_of(envs), want)


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
