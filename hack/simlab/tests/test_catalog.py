#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.catalog over golden_cli's fixture cache and farm state, and
simlab.verdict's derive pool:

  Index        an index's Progress and Indexed, and one with nothing to
               do; state and availability
  Child        an index in a forkserver's child, as the console runs one,
               its derives in a pool of the child's
  Derive       the forkserver pool's derived tables are those made in this
               process, with a DuckDB connection open here; a source that
               fails comes back failed, the others judged
  Schema       catalog.schema over an engine.Pool: the console's parts
  Fetch        the flushes a fetch pulls (store.pull_sources stood in) and
               what the S3 ledger's new lines cost; its refusals
  Tiers        an experiment's tiers from its cells' names, tests and env,
               its index recording none

  python3 hack/simlab/tests/test_catalog.py
"""

import concurrent.futures as cf
import dataclasses
import json
import multiprocessing
import os
import shutil
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
from simlab import catalog, engine, store, store_host, suite, verdict  # noqa: E402


class Fixture(unittest.TestCase):
    """golden_cli's fixture under a scratch root: the cache, the farm's and
    the suite's state; the main checkout is the root."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-catalog-")
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
                  mock.patch.object(simlab.simdb(), "main_checkout", lambda: self.root)):
            p.start()
            self.addCleanup(p.stop)

    def index(self, workers=2):
        events = []
        return catalog.index(workers, events.append), events


class Index(Fixture):
    def test_an_index_says_what_it_does_and_then_that_theres_nothing_to_do(self):
        got, events = self.index()
        self.assertTrue(got.changed)
        srcs = sorted(catalog.scan_cache())
        self.assertEqual((sorted(got.judged), got.failed), (srcs, {}))
        self.assertEqual(got.sources, srcs)
        self.assertGreater(got.new, 0)
        self.assertEqual((got.replaced, got.gone), (0, 0))
        kinds = [e.kind for e in events]
        self.assertEqual(kinds[0], "scan")
        self.assertEqual(kinds[-1], "done")
        self.assertIn(f"verdicts: {len(srcs)}/{len(srcs)} sources", [e.message for e in events])
        derive = [e for e in events if e.kind == "derive"][-1]
        self.assertEqual((derive.done, derive.total), (len(srcs), len(srcs)))
        self.assertRegex(events[0].message, r"^index: \d+ new files, 0 changed, 0 gone; ")
        again, events = self.index()
        self.assertFalse(again.changed)
        self.assertRegex(events[-1].message, r"^the catalog is up to date \(")
        info = {r["key"]: r["value"] for r in catalog.lake("SELECT * FROM catalog_info;")}
        self.assertEqual(info["verdict_version"], verdict.derived_version())
        self.assertEqual(catalog.state(), {"info": info | {"indexed_at": info["indexed_at"]}, "current": True})
        a = catalog.availability("sf0001")
        self.assertEqual(a.names[:3], ["k", "tbl", "state"])
        self.assertTrue(a.rows)
        self.assertEqual({r[0] for r in a.rows} - {r[0] for r in catalog.availability().rows}, set())

    def test_no_catalog_has_no_state(self):
        with mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.root / "nowhere")}):
            self.assertIsNone(catalog.state())


def child_index(root):
    """An index in a forkserver's child over the fixture at `root`: its
    Indexed, as a dict, and its Progress events' kinds."""
    os.environ["SIM_DB_ROOT"] = str(Path(root) / "target" / "sim-db")
    simlab.simdb().main_checkout = lambda: Path(root)
    events = []
    got = catalog.index(2, events.append)
    return dataclasses.asdict(got), [e.kind for e in events]


class Child(Fixture):
    def test_an_index_in_a_forkserver_child(self):
        held = engine.connect()
        self.addCleanup(held.close)
        with cf.ProcessPoolExecutor(1, mp_context=multiprocessing.get_context("forkserver")) as pool:
            got, kinds = pool.submit(child_index, str(self.root)).result(timeout=300)
        self.assertTrue(got["changed"])
        self.assertEqual((sorted(got["judged"]), got["failed"]), (sorted(catalog.scan_cache()), {}))
        self.assertIn("derive", kinds)
        self.assertFalse(catalog.index(0).changed)


class Derive(Fixture):
    def tables(self, srcs):
        name = lambda s: s.replace("/", "~") + ".parquet"  # noqa: E731
        out = {}
        for t in [*verdict.DERIVED, "verdicts"]:
            files = [str(verdict.derived_dir(t) / name(s)) for s in srcs]
            out[t] = store.execute(f"SELECT * FROM read_parquet({verdict.plist(files)}, hive_partitioning=false) "
                                   "ORDER BY ALL;")
        return out

    def test_the_pools_tables_are_this_processs(self):
        srcs = sorted(catalog.scan_cache())
        derived = store_host.cache_root() / "derived"
        held = engine.connect()
        self.addCleanup(held.close)
        shutil.rmtree(derived, ignore_errors=True)
        with mock.patch.object(verdict, "RECYCLE", 2):
            got = verdict.derive(srcs, workers=2)
        self.assertEqual((sorted(got.judged), got.failed), (srcs, {}))
        pooled = self.tables(srcs)
        self.assertTrue(pooled["verdicts"])
        shutil.rmtree(derived)
        here = verdict.derive(srcs, workers=0)
        self.assertEqual(sorted(here.judged), srcs)
        self.assertEqual(self.tables(srcs), pooled)

    def test_a_source_that_fails_comes_back_failed(self):
        bad = store_host.cache_root() / "cache" / "suite" / "wt" / "broken"
        bad.mkdir(parents=True)
        self.addCleanup(shutil.rmtree, bad.parent)
        (bad / "runs.parquet").write_text("not parquet")
        srcs = sorted(catalog.scan_cache())
        said = []
        got = verdict.derive(srcs, workers=2, progress=lambda m, done, total: said.append(m))
        self.assertEqual(list(got.failed), ["suite/wt/broken"])
        self.assertEqual(sorted(got.judged), [s for s in srcs if s != "suite/wt/broken"])
        self.assertTrue(any(m.startswith("verdicts: suite/wt/broken: ") for m in said), said)


class Schema(Fixture):
    def test_the_consoles_parts(self):
        self.index()
        pool = engine.Pool(catalog.lake_root(), self.root / "pool-tmp", size=1)
        self.addCleanup(pool.close)
        got = catalog.schema(pool)
        self.assertEqual(list(got), ["tables", "table_catalog", "metric_catalog", "event_kinds", "experiments",
                                     "catalog_info"])
        names = {(t["schema"], t["name"]) for t in got["tables"]}
        self.assertTrue({("main", "experiments"), ("main", "jobs"), ("raw", "tier_cells")} <= names)
        jobs = next(t for t in got["tables"] if t["name"] == "jobs")
        self.assertEqual(jobs["columns"][0], {"name": "k", "type": "VARCHAR"})
        self.assertEqual([e["exp_id"] for e in got["experiments"]][:1], ["sf0001"])
        self.assertEqual(set(got["experiments"][0]), {"exp_id", "kind", "started_at", "tiers", "arm_labels", "base",
                                                      "question", "cells", "jobs"})
        self.assertNotIn("first_seen", got["metric_catalog"][0])
        self.assertIn("verdict_version", {r["key"] for r in got["catalog_info"]})


class Fetch(Fixture):
    def test_a_fetch_pulls_the_flushes_the_cache_lacks_and_prices_its_ledger_lines(self):
        self.index()
        lost = f"{golden_cli.FARM}/i-2/3"
        dest = store_host.cache_root() / "cache" / lost
        kept = self.root / "kept-flush"
        shutil.move(dest, kept)
        ledger = self.root / "target" / "sim-farm" / "s3.jsonl"
        calls = []

        def pull_sources(srcs):
            calls.append(sorted(srcs))
            shutil.move(kept, dest)
            use = {"region": "r", "tier1": {}, "tier2": {"pull_get": 1000}, "up": {}, "down": {"internet": 1 << 30},
                   "stored": 0}
            store.farm_ledger(golden_cli.FARM.split("/")[1], use)
            return [store.Pull(golden_cli.FARM.split("/")[1], dest.parents[1], [lost], use)], []

        events = []
        with mock.patch.object(store, "pull_sources", pull_sources):
            got = catalog.fetch("sf0001", progress=events.append)
        self.assertEqual(calls, [[lost]])
        self.assertEqual((got.exp, got.pulled, got.gone, got.ledger_lines), ("sf0001", [lost], [], 1))
        self.assertAlmostEqual(got.s3_usd, 1000 * 0.0044 / 10000 + 0.09, places=9)
        self.assertEqual(events[0], catalog.Progress(
            "pull", "sf0001: pulling the farm runs of 1 flushes the cache lacks", 0, 1))
        self.assertTrue(json.loads(ledger.read_text().splitlines()[-1])["tier2"])
        self.assertEqual(events[-1].kind, "done")

    def test_what_a_fetch_refuses(self):
        self.index()
        with self.assertRaisesRegex(SystemExit, "fetch takes the sample tables"):
            catalog.fetch("sf0001", tables=["bins"])
        with self.assertRaisesRegex(SystemExit, "experiment nope has no jobs"):
            catalog.fetch("nope")


class Tiers(Fixture):
    """Farm-runs indexes of cells without env, their matrices' jobs giving
    it: each cell's tier is the one holding the most of the experiment's
    cells among those with a cell of its name, test and env."""

    def setUp(self):
        super().setUp()
        self.runs = self.root / "target" / "sim-suite" / "farm-runs"
        self.farm = self.root / "target" / "sim-farm"

    def experiment(self, name, cells):
        """A farm run's index naming `cells` (tier cells, each with its
        env or another) by name, sim and test alone, and a job of each."""
        (self.runs / f"{name}.json").write_text(json.dumps({"cells": [
            {"name": c["name"], "sim": c["sim"], "test": c["test"], "why": "w"} for c in cells]}))
        for i, c in enumerate(cells):
            seed = "SIM_SEED" if c["sim"] == suite.PPSPP else "STRIMS_SIM_SEED"
            d = self.farm / f"{name}-{i:02d}"
            d.mkdir()
            (d / "index.jsonl").write_text(json.dumps({
                "id": f"{name}{i}", "key": f"k-{name}-{i}", "arm": "main", "commit": "1" * 40, "test": c["test"],
                "env": {**c["env"], seed: "7"}, "src": f"farm/{name}-00+0more-aaaa/i-1/0"}) + "\n")

    def tiers(self):
        catalog.index(workers=0)
        return {r["exp_id"]: r["tiers"] for r in catalog.lake("SELECT exp_id, tiers FROM experiments;")}

    def test_a_cells_env_decides_its_tier(self):
        by = {}
        for t, cs in suite.TIERS.items():
            for c in cs:
                by.setdefault(c["name"], {})[t] = c
        # A name two tiers give different env, and two cells only one of
        # them holds.
        name, both = next((n, ts) for n, ts in sorted(by.items())
                          if len(ts) >= 2 and len({json.dumps(c["env"], sort_keys=True) for c in ts.values()}) == len(ts))
        a, b = sorted(both)[:2]
        only_b = [c for c in suite.TIERS[b] if c["name"] not in by or set(by[c["name"]]) == {b}][:2]
        self.experiment("sx0001", [both[b], *only_b])
        self.experiment("sx0002", [both[a]])
        self.experiment("sx0003", [{**both[a], "env": {**both[a]["env"], "SIM_SUITE_UNKNOWN": "1"}}])
        got = self.tiers()
        self.assertEqual((got["sx0001"], got["sx0002"], got["sx0003"]), ([b], [a], []))

    def test_the_tier_holding_the_most_of_its_cells(self):
        # Cells of one tier, one of which another tier holds as well (its
        # name and env alike): all of them that tier's.
        shared = None
        for t, cs in sorted(suite.TIERS.items()):
            for c in cs:
                key = (c["name"], json.dumps(c["env"], sort_keys=True))
                others = [u for u, us in suite.TIERS.items() if u != t and any(
                    (x["name"], json.dumps(x["env"], sort_keys=True)) == key for x in us)]
                if others and len(cs) >= 3:
                    shared = (t, c)
                    break
            if shared:
                break
        t, c = shared
        rest = [x for x in suite.TIERS[t] if x["name"] != c["name"]][:2]
        self.experiment("sx0004", [c, *rest])
        self.assertEqual(self.tiers()["sx0004"], [t])


if __name__ == "__main__":
    unittest.main()
