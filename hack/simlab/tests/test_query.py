#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.query over golden_cli's fixture cache and farm state (the text
simdb.py prints of it is golden_cli's):

  Views        a selection's views and their connection; a selection of
               nothing refused
  Sql          a script's Results, their text the CLI's, an error's LINE
               the CLI's
  Commands     the commands' Results, their columns, a refusal, the rows
               by key and the selection's rows, a pull's dry run

  python3 hack/simlab/tests/test_query.py
"""

import argparse
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))
sys.path.insert(0, str(HERE))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import golden_cli  # noqa: E402
import simlab  # noqa: E402
from simlab import query, store_host  # noqa: E402

SUITE = query.Selection(src=["suite/**"])


class Fixture(unittest.TestCase):
    """golden_cli's fixture under a scratch root: the cache, the farm's and
    the suite's state; the main checkout is the root."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-query-")
        cls.root = Path(cls.tmp.name)
        saved = os.environ.get("SIM_DB_ROOT")
        try:
            golden_cli.build(cls.root)
        finally:
            os.environ.pop("SIM_DB_ROOT", None)
            if saved is not None:
                os.environ["SIM_DB_ROOT"] = saved

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def setUp(self):
        for p in (mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.root / "target" / "sim-db"), "TZ": "UTC"}),
                  mock.patch.object(store_host, "main_checkout", lambda: self.root),
                  mock.patch.object(simlab.simdb(), "main_checkout", lambda: self.root),
                  mock.patch.object(query, "stderr", lambda m: None)):
            p.start()
            self.addCleanup(p.stop)

    def cli(self, sql, out):
        """`sql` over SUITE's views in the DuckDB CLI."""
        v = query.views(SUITE)
        with mock.patch.object(store_host, "EXECUTOR", None):
            return store_host.duck(v.sql + "\n" + sql, out=out)


class Views(Fixture):
    def test_a_selections_views_and_their_connection(self):
        v = query.views(SUITE)
        self.assertTrue({"runs", "reads", "viewers", "events", "verdicts", "derived_events"} <= v.have)
        self.assertEqual(v.records("SELECT count(DISTINCT k) AS n FROM runs;"), [{"n": 6}])
        self.assertIsNotNone(v.conn)
        v.close()
        self.assertIsNone(v.conn)
        with query.views(SUITE, derived=False) as v:
            self.assertNotIn("verdicts", v.have)
            self.assertEqual(v.run("SELECT 1 AS one;").names, ["one"])
        self.assertIsNone(v.conn)

    def test_a_selection_of_nothing_is_refused(self):
        with self.assertRaises(query.Refused) as cm:
            query.views(query.Selection(src=["nowhere/*"]))
        self.assertIsInstance(cm.exception, SystemExit)
        self.assertEqual(str(cm.exception), "no runs in the cache match --src")

    def test_a_selection_of_simdbs_options(self):
        a = argparse.Namespace(src=["x"], cell=None, arm=["a*"], seed=[1], bin=500, since=2.0, until=None,
                               json=True)
        self.assertEqual(query.Selection.of(a), query.Selection(src=["x"], arm=["a*"], seed=[1], bin=500, since=2.0))
        self.assertEqual(query.Selection.of(argparse.Namespace(src=None, cell=None, arm=None, seed=None)),
                         query.Selection())


class Sql(Fixture):
    QUERY = "SELECT k, seed FROM runs ORDER BY k; CREATE TEMP TABLE t AS SELECT 1 AS x; SELECT 1.5::DECIMAL(4,2) AS d"

    def test_a_scripts_results_are_the_clis(self):
        got = query.sql(SUITE, self.QUERY)
        self.assertEqual([r.names for r in got], [["k", "seed"], ["d"]])
        self.assertEqual(got[-1].records(), self.cli(self.QUERY + ";", "json"))
        self.assertEqual(query.sql_text(query.sql(SUITE, self.QUERY, text=True)), self.cli(self.QUERY + ";", "text"))

    def test_an_errors_line_is_the_clis(self):
        bad = "\n  -- a comment\n  SELECT nope FROM runs"
        with self.assertRaises(store_host.DuckError) as cm:
            query.sql(SUITE, bad)
        self.assertIn("LINE 1: SELECT nope FROM runs;", str(cm.exception))
        with self.assertRaises(store_host.DuckError) as cli:
            self.cli(bad + ";", "json")
        self.assertEqual(str(cm.exception), str(cli.exception))


class Commands(Fixture):
    def test_the_commands_results(self):
        got = query.pooled(SUITE, "phase")
        self.assertEqual(got.names, ["cell", "arm", "g", "runs", "reads", "lag_p50", "lag_p90", "lag_p99", "lag_max",
                                     "mean_run_p99"])
        self.assertTrue({"after_down", "after_up"} <= {r["g"] for r in got.records()})
        band = query.aligned(SUITE, "onset", "lag_q", after=20)
        self.assertEqual(band.names, ["cell", "arm", "rel_ms", "n", "p10", "p50", "p90"])
        self.assertTrue(band.rows)
        pooled = query.aligned(SUITE, "first_read", "lag_ms", per="node", nth=0, pool=True, after=5)
        self.assertTrue(all(r["p10"] == r["p50"] == r["p90"] for r in pooled.records()))
        self.assertIn("runs", query.events(SUITE).names)
        self.assertIn("unhealthy_spells", query.osc(SUITE).names)
        self.assertEqual({r["pace"] for r in query.forecast(query.Selection()).records()}, {"lazy", "prompt"})

    def test_a_refusal_says_why(self):
        with self.assertRaises(query.Refused) as cm:
            query.aligned(SUITE, "onset", per="node")
        self.assertIn("onset is run-wide", str(cm.exception))
        with self.assertRaises(query.Refused) as cm:
            query.aligned(SUITE, "nothing")
        self.assertIn("the selection has no nothing event; its kinds: ", str(cm.exception))

    def test_the_rows_by_key_and_the_selections(self):
        lines = query.selected_rows(SUITE, ["trace"])
        self.assertEqual([list(r) for r in lines[:1]],
                         [["cell", "arm", "seed", "k", "src", "trace", "_verdict", "_collapsed"]])
        self.assertEqual(len(lines), 6)
        got = query.rows([(r["src"], r["k"]) for r in lines], series=True)
        self.assertEqual({(r["src"], r["k"]) for r in lines}, set(got))
        for line in lines:
            row = got[(line["src"], line["k"])]
            self.assertEqual(row["trace"], line["trace"])
            self.assertIn("lag", row["_series"])

    def test_a_pulls_dry_run(self):
        run = golden_cli.FARM.split("/")[1]
        p = query.pull_run(run, reads=True, dry_run=True)
        self.assertEqual((p.jobs, p.lost, [n for *_, n in p.files]), (2, 0, [2]))
        self.assertTrue(p.files[0][0].endswith("/i-2/3/reads.parquet"))
        self.assertTrue(query.pull_text(p).startswith(f"{run}: would GET 1 files, 0.0 MB, for 2 jobs' "))
        with self.assertRaises(query.Refused):
            query.pull_run(run, dry_run=True)


if __name__ == "__main__":
    unittest.main()
