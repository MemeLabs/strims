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
  Tables       a sim's tables the catalog has no code for (spec_requests,
               parks, arrivals), folded by the store, as they are; forecast
               in either schema (tail_ms and extrapolated, or first_sight
               and lazy), the other's columns NULL
  Commands     simdb.py's lake --param, view and describe over the
               catalog, as a caller runs them: params bound typed or as
               text, one missing named; the saved views listed, run with
               their defaults or params, an unknown one refused; describe's
               every topic and name, none naming the Ask panel's tools

  python3 hack/simlab/tests/test_catalog.py
"""

import concurrent.futures as cf
import dataclasses
import json
import multiprocessing
import os
import re
import shutil
import subprocess
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
from simlab import ask, catalog, describe, engine, store, store_host, suite, verdict, views  # noqa: E402


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
                  mock.patch.object(simlab.simdb(), "main_checkout", lambda: self.root),
                  mock.patch.object(store_host, "main_checkout", lambda: self.root)):
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
    simlab.simdb().main_checkout = store_host.main_checkout = lambda: Path(root)
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


class Tables(Fixture):
    # The two forecast schemas: the HAVE-era one (first_sight, lazy) and
    # its successor (tail_ms, extrapolated); held_ms empty when never held.
    COMMON = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"), ("peer", "BIGINT"), ("stream", "BIGINT"),
              ("seq", "BIGINT"), ("sample_ms", "BIGINT"), ("mean_ms", "BIGINT"), ("horizon_ms", "BIGINT")]
    FORECASTS = {
        "suite/wt/tail": (COMMON + [("tail_ms", "BIGINT"), ("extrapolated", "BOOLEAN"), ("held_ms", "BIGINT")],
                          [(1000, 1, 0, 2, 0, 5, 60, 50, 80, 120, True, None), (1100, 1, 0, 2, 0, 6, 40, 50, 80, 120,
                                                                              False, 1150)]),
        "suite/wt/lazy": (COMMON + [("first_sight", "BOOLEAN"), ("lazy", "BOOLEAN"), ("held_ms", "BIGINT")],
                          [(1000, 1, 0, 2, 0, 5, 60, 50, 80, False, True, -1)]),
    }

    def fold(self, src, forecast):
        job = (src, f"k-{src.rsplit('/', 1)[1]}", *golden_cli.JOBS[0][2:])
        d = self.root / "jobs-tables" / src.replace("/", "~")
        meta = golden_cli.job_dir(d, job)
        golden_cli.write_table(d, "forecast", *forecast)
        golden_cli.write_table(d, "spec_requests", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                    ("held_ms", "BIGINT"), ("held_by_p90", "BOOLEAN")],
                               [(900, 1, 5, None, None), (950, 1, 6, 1150, True)])
        golden_cli.write_table(d, "parks", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                            ("outcome", "VARCHAR")], [(800, 1, 5, "sent")])
        golden_cli.write_table(d, "arrivals", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                               ("how", "VARCHAR"), ("peer", "BIGINT")], [(1150, 1, 6, "answer", 2)])
        db = self.root / "jobs-tables" / f"{src.replace('/', '~')}.duckdb"
        store_host.fold(db, [meta], src)
        store_host.export(db, store_host.cache_root() / "cache" / src)
        return meta["k"]

    def test_tables_without_code_and_both_forecasts(self):
        keys = {src: self.fold(src, f) for src, f in self.FORECASTS.items()}
        got = self.index()[0]
        self.assertEqual(got.failed, {})
        for t in ("spec_requests", "parks", "arrivals"):
            rows = catalog.lake(f"SELECT k FROM {t} WHERE k IN ({', '.join(catalog.q(k) for k in keys.values())});")
            self.assertEqual(len(rows), len(keys) * (2 if t == "spec_requests" else 1), t)
        held = catalog.lake("SELECT seq, held_ms, held_by_p90 FROM spec_requests WHERE k = "
                            f"{catalog.q(keys['suite/wt/tail'])} ORDER BY seq;")
        self.assertEqual(held, [{"seq": 5, "held_ms": None, "held_by_p90": None},
                                {"seq": 6, "held_ms": 1150, "held_by_p90": True}])
        f = {(r["k"], r["seq"]): r for r in catalog.lake(
            "SELECT k, seq, tail_ms, extrapolated, first_sight, lazy, held_ms FROM forecast WHERE k IN "
            f"({', '.join(catalog.q(k) for k in keys.values())});")}
        tail, lazy = keys["suite/wt/tail"], keys["suite/wt/lazy"]
        self.assertEqual(f[(tail, 5)], {"k": tail, "seq": 5, "tail_ms": 120, "extrapolated": True,
                                        "first_sight": None, "lazy": None, "held_ms": None})
        self.assertEqual(f[(tail, 6)]["held_ms"], 1150)
        self.assertEqual(f[(lazy, 5)], {"k": lazy, "seq": 5, "tail_ms": None, "extrapolated": None,
                                        "first_sight": False, "lazy": True, "held_ms": None})
        names = {r["name"] for r in catalog.lake("SELECT name FROM table_catalog;")}
        self.assertTrue({"spec_requests", "parks", "arrivals"} <= names)



class Commands(Fixture):
    def cli(self, *args):
        """simdb.py ARGS as golden_cli runs it: (exit code, stdout, stderr)."""
        p = subprocess.run([sys.executable, str(golden_cli.SIMDB), *args], cwd=self.root,
                           env=golden_cli.environment(self.root), capture_output=True, text=True, timeout=300)
        return p.returncode, p.stdout, p.stderr

    def test_lake_binds_its_params(self):
        self.index()
        code, out, err = self.cli("lake", "SELECT $s AS s, $n AS n, $f AS f, $l AS l, $b AS b, $q AS q", "--param",
                                  "s=sf0001", "--param", "n=7", "--param", "f=2.5", "--param", "l=[1, 2]",
                                  "--param", "b=true", "--param", 'q="007"', "--json")
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(json.loads(out), [{"s": "sf0001", "n": 7, "f": 2.5, "l": [1, 2], "b": True, "q": "007"}])
        code, out, err = self.cli("lake", "SELECT exp_id, jobs FROM experiments WHERE exp_id = $exp", "--param",
                                  "exp=sf0001")
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(out.splitlines()[1:4], ["| exp_id | jobs |", "+--------+------+", "| sf0001 | 7    |"])
        # The same statement as a script, its literal inline, prints the same.
        self.assertEqual(self.cli("lake", "SELECT exp_id, jobs FROM experiments WHERE exp_id = 'sf0001'")[1], out)
        code, out, err = self.cli("lake", "SELECT count(*) AS n FROM structural($exp)", "--param", "exp=sf0001",
                                  "--json")
        self.assertEqual((code, err, len(json.loads(out))), (0, "", 1))

    def test_a_param_missing_is_named(self):
        self.index()
        code, out, err = self.cli("lake", "SELECT * FROM experiments WHERE exp_id = $exp", "--param", "other=1")
        self.assertEqual((code, out, err), (1, "", "lake: no value for $exp\n"))
        code, _, err = self.cli("lake", "SELECT 1", "--param", "nope")
        self.assertEqual(code, 2)
        self.assertIn("'nope' is no NAME=VALUE", err)

    def test_the_saved_views_listed_run_and_refused(self):
        self.index()
        saved = views.saved()
        code, out, err = self.cli("view")
        self.assertEqual((code, err), (0, ""))
        for v in saved:
            self.assertRegex(out, rf"(?m)^\| {re.escape(v.name)} +\| {re.escape(v.title)} +\| ")
        code, out, _ = self.cli("view", "--json")
        self.assertEqual([v["name"] for v in json.loads(out)], [v.name for v in saved])
        # experiments' $search takes its default, '': every experiment.
        code, out, err = self.cli("view", "experiments", "--json")
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(sorted(r["exp_id"] for r in json.loads(out)), ["sf0001", "suite/wt/inv1", "suite/wt/inv2"])
        code, out, err = self.cli("view", "experiments", "--param", "search=inv2")
        self.assertEqual((code, err), (0, ""))
        self.assertIn("suite/wt/inv2", out)
        self.assertNotIn("sf0001", out)
        code, out, err = self.cli("view", "scorecard", "--param", "exp=sf0001", "--param", "arm=a", "--param",
                                  "arm2=off", "--json")
        self.assertEqual((code, err), (0, ""))
        cells = {r["cell"] for r in catalog.lake("SELECT cell FROM experiment_cells WHERE exp_id = 'sf0001';")}
        self.assertEqual({r["cell"] for r in json.loads(out)}, cells)
        code, out, err = self.cli("view", "experiments", "--max-rows", "1")
        self.assertEqual((code, err), (0, "view experiments: its first 1 rows; --max-rows for more\n"))
        self.assertEqual(len([line for line in out.splitlines() if line.startswith("| ")]), 2)
        code, out, err = self.cli("view", "scorecard", "--param", "exp=sf0001")
        self.assertEqual((code, out), (1, ""))
        self.assertEqual(err, "view scorecard: no value for $arm (its params: exp (exp), arm (arm), arm2 (arm2))\n")
        code, out, err = self.cli("view", "nope")
        self.assertEqual((code, out), (1, ""))
        self.assertTrue(err.startswith("view: no view 'nope': collapse_diff, "), err)

    def test_describe_every_topic_and_name(self):
        self.index()
        code, out, err = self.cli("describe")
        self.assertEqual((code, err), (0, ""))
        pool = catalog.pool("test", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        self.addCleanup(pool.close)
        self.assertEqual(out, describe.describe(None, pool))
        schema = catalog.schema(pool)
        names = sorted({t["name"] for t in schema["tables"] if t["schema"] == "main"}
                       | {m["name"] for m in describe.macros(schema)} | {v.name for v in views.saved()})
        self.assertTrue({"experiments", "verdicts", "structural", "effects", "scorecard"} <= set(names))
        for topic in [*describe.TOPICS, *names]:
            got = describe.describe(topic, pool)
            self.assertTrue(got.strip(), topic)
            for tool in (t["name"] for t in ask.TOOLS if "_" in t["name"]):
                self.assertNotRegex(got, rf"\b{tool}\b", topic)
        code, out, err = self.cli("describe", "verdict")
        self.assertEqual((code, out), (1, ""))
        self.assertTrue(err.startswith("describe: no topic 'verdict' (did you mean verdicts"), err)


if __name__ == "__main__":
    unittest.main()
