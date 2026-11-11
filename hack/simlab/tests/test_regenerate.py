#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.regenerate: an experiment's cells, arms and seeds as the suite's
write_matrices calls (--refs and --sets runs, and what can't run), the
matrices' env against the experiment's, and regenerate over a catalog of
its own with a stand-in suite and farm.py:

  python3 hack/simlab/tests/test_regenerate.py
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import engine, farm, regenerate, suite  # noqa: E402

SETS = {"exp_id": "e1", "kind": "farm", "knob": {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"},
        "sets": ["off", "a", "b"], "arms": {"x": "cccc"}, "base": "off", "tiers": {"calm": "robust"}}
REFS = {**SETS, "knob": None, "sets": None, "arms": {"main": "aaaa", "fix": "bbbb"}, "base": "main",
        "tiers": {"calm": "robust", "storm": "robust", "peer": "peers"}}


def plan(exp=SETS, **r):
    return regenerate.plan(exp, {"cells": ["calm"], "arms": ["a"], "seeds": [1], **r})


class Plan(unittest.TestCase):
    def test_a_sets_run_takes_the_sets_chosen_and_its_commit(self):
        planned = plan(arms=["b", "a"])
        self.assertEqual(planned, [("robust", ["calm"], [1], {
            "sets": ["a", "b"], "ref": "cccc", "knob": {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"},
            "against": "a", "no_off": True})])
        self.assertEqual(regenerate.suite_command(*planned[0]), [
            "suite.py", "farm", "--tier", "robust", "--only", "calm", "--seeds", "1", "--first-seed", "1",
            "--knob", "SIM_EXP,STRIMS_SIM_EXP", "--ref", "cccc", "--sets", "a;b", "--no-off", "--against", "a",
            "--no-run"])
        off = plan(arms=["off", "b"])[0][3]
        self.assertEqual((off["sets"], off["no_off"], off["against"]), (["b"], False, "off"))

    def test_a_refs_run_takes_its_arms_base_first_a_call_a_tier(self):
        planned = plan(REFS, cells=["peer", "calm", "storm"], arms=["fix", "main"], seeds=[3, 2, 2, 4])
        arms = {"arms": {"main": "aaaa", "fix": "bbbb"}}
        self.assertEqual(planned, [("peers", ["peer"], [2, 3, 4], arms),
                                   ("robust", ["calm", "storm"], [2, 3, 4], arms)])
        self.assertEqual(regenerate.suite_command(*planned[1]), [
            "suite.py", "farm", "--tier", "robust", "--only", "calm,storm", "--seeds", "3", "--first-seed", "2",
            "--refs", "main=aaaa,fix=bbbb", "--no-run"])

    def test_what_cant_run_says_why(self):
        self.assertRegex(plan(seeds=[1, 3]), "consecutive")
        self.assertRegex(plan(cells=["gone"]), "no tier of e1 has gone")
        self.assertRegex(plan(arms=["zz"]), r"zz: not a set of e1 \(off, a, b\)")
        self.assertRegex(plan(REFS, arms=["zz"]), "zz: not an arm of e1")
        self.assertRegex(plan(cells=["--all"]), 'not a name: "--all"')
        self.assertRegex(plan(arms=[]), "one each at least")
        self.assertRegex(plan({**SETS, "kind": "local"}), "local invocation")

    def test_a_matrixs_env_against_the_experiments(self):
        ms = [suite.Matrix("a.toml", "calm", {"SIM_A": "1", "SIM_B": 'x "y"'}), suite.Matrix("b.toml", "storm", {})]
        self.assertEqual(regenerate.env_warnings("e1", ms, {"calm": {"SIM_A": "1", "SIM_C": "2"}, "storm": {}}),
                         ["calm: env differs from e1's at SIM_B (unset -> x \"y\"), SIM_C (2 -> unset)"])


class Suite:
    """suite.cells and write_matrices: each call kept, `fail` raised."""

    def __init__(self, out):
        self.out = out
        self.calls = []
        self.fail = None

    def cells(self, tier, only=None, env=()):
        return [{"name": c, "env": {"SIM_VIEWERS": "32" if c == "storm" else "8"}} for c in only]

    def write_matrices(self, cells, seeds, **kw):
        self.calls.append(([c["name"] for c in cells], list(seeds), kw))
        if self.fail:
            raise self.fail
        ms = []
        for i, c in enumerate(cells):
            p = self.out / f"s0abcd-{i:02d}.toml"
            p.write_text(f"# {c['name']}\n")
            ms.append(suite.Matrix(str(p), c["name"], c["env"]))
        return suite.Matrices("s0abcd", ms, {})


class Regenerate(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-regenerate-")
        d = cls.dir = Path(cls.tmp.name)
        (d / "lake" / "data").mkdir(parents=True)
        w = engine.writer(d / "lake")
        try:
            w.execute("""
CREATE SCHEMA raw;
CREATE TABLE experiments AS SELECT * FROM (VALUES
  ('e1', 'farm', ['browsers', 'robust'], NULL::MAP(VARCHAR, VARCHAR), NULL::VARCHAR[],
   MAP {'base': 'aaaa', 'new': 'bbbb'}, 'base'),
  ('l1', 'local', ['robust'], NULL, NULL, MAP {'base': 'aaaa'}, 'base')) t(exp_id, kind, tiers, knob, sets, arms, base);
CREATE TABLE experiment_cells AS SELECT * FROM (VALUES
  ('e1', 'calm', MAP {'SIM_VIEWERS': '8'}), ('e1', 'storm', MAP {'SIM_VIEWERS': '16'})) t(exp_id, cell, env);
CREATE TABLE raw.tier_cells AS SELECT * FROM (VALUES
  ('robust', 'calm', MAP {'SIM_VIEWERS': '8'}), ('browsers', 'storm', MAP {'SIM_VIEWERS': '32'}),
  ('robust', 'storm', MAP {'SIM_VIEWERS': '16'})) t(tier, cell, env);""")
        finally:
            w.close()
        cls.pool = engine.Pool(d / "lake", d / "spill", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        (d / "farm.py").write_text(
            "import json, sys\n"
            "if sys.argv[1:3] == ['estimate', '--json']:\n"
            "    print('2 jobs, ~$0.01', file=sys.stderr)\n"
            "    sys.exit(3) if 'refuse' in sys.argv[3] else print(json.dumps({'jobs': 2}))\n")
        cls.regs = farm.Regenerations(d / "regenerate", farm_py=d / "farm.py", python=sys.executable)

    @classmethod
    def tearDownClass(cls):
        cls.pool.close()
        cls.tmp.cleanup()

    def setUp(self):
        self.out = Path(tempfile.mkdtemp(dir=self.dir))
        self.suite = Suite(self.out)

    def run_(self, exp="e1", cells=("calm", "storm"), arms=("new", "base"), seeds=(2001, 2002)):
        return regenerate.regenerate(self.pool, exp, cells, arms, seeds, self.regs, suite=self.suite)

    def test_a_quote_is_saved_with_its_request_and_warnings(self):
        r = self.run_()
        self.assertEqual(self.suite.calls,
                         [(["calm", "storm"], [2001, 2002], {"arms": {"base": "aaaa", "new": "bbbb"}})])
        self.assertEqual({k: r[k] for k in ("exp_id", "cells", "arms", "seeds", "state", "quote", "estimate")},
                         {"exp_id": "e1", "cells": ["calm", "storm"], "arms": ["new", "base"], "seeds": [2001, 2002],
                          "state": "quoted", "quote": "2 jobs, ~$0.01", "estimate": {"jobs": 2}})
        self.assertEqual(r["warnings"], ["storm: env differs from e1's at SIM_VIEWERS (16 -> 32)"])
        self.assertEqual([Path(m).name for m in r["matrices"]], ["s0abcd-00.toml", "s0abcd-01.toml"])
        self.assertEqual(self.regs.get(r["id"]), r)
        self.assertEqual(json.loads((self.dir / "regenerate" / f"{r['id']}.json").read_text()), r)

    def test_the_suite_or_the_estimate_failing_fails_it_kept(self):
        self.suite.fail = SystemExit("--against 'zz' is none of the sets")
        r = self.run_()
        self.assertEqual((r["state"], r["error"]), ("failed", "the suite's matrices for robust: --against 'zz' is "
                                                              "none of the sets"))
        self.assertEqual(self.regs.get(r["id"])["state"], "failed")
        self.suite.fail = subprocess.CalledProcessError(128, ["git", "rev-parse", "nope"])
        self.assertRegex(self.run_()["error"], "returned non-zero exit status 128")
        self.suite.fail = None
        self.out = self.out.rename(self.out.with_name("refuse"))
        self.suite.out = self.out
        r = self.run_()
        self.assertEqual((r["state"], r["error"]), ("failed", "farm.py estimate: exit 3: 2 jobs, ~$0.01"))
        self.assertEqual(self.regs.get(r["id"])["warnings"], r["warnings"])

    def test_one_that_cant_run_is_refused_unkept_and_an_unknown_experiment_is_404(self):
        r = self.run_(exp="l1", cells=["calm"], arms=["base"])
        self.assertEqual((r["state"], r["error"]), ("failed", "l1 is a local invocation; only farm runs regenerate"))
        self.assertIsNone(self.regs.get(r["id"]))
        self.assertEqual(self.suite.calls, [])
        with self.assertRaises(farm.QuoteError) as cm:
            self.run_(exp="nope")
        self.assertEqual(cm.exception.status, 404)

    def test_a_cell_runs_in_the_tier_whose_cell_has_its_env(self):
        exp = regenerate.experiment(self.pool, "e1")
        self.assertEqual(exp["tiers"], {"calm": "robust", "storm": "robust"})
        self.assertEqual(exp["env"], {"calm": {"SIM_VIEWERS": "8"}, "storm": {"SIM_VIEWERS": "16"}})


if __name__ == "__main__":
    unittest.main()
