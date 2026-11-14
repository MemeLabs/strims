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
  Logs         a job of the HAVE removal's logs beside the fixture's: the
               forecast's predictions and HAVE samples apart, read by their
               columns, the decision logs (spec), their fields aligned on a
               sim's event about a node (--peer)
  Arrivals     arrivals is a sample table: a worker uploads it with the
               samples, a pull fetches it with them, and a pull's lines name
               the sample tables the run's flushes hold (simdb_test's
               SamplesPull's scratch S3, one flush with arrivals and no
               forecast)
  Asks         asks (the HAVE removal's decision log, every rival kept)
               is a sample table too: a pull fetches it with the reads,
               and the catalog says whose rows hold an outcome

  python3 hack/simlab/tests/test_query.py
"""

import argparse
import importlib.util
import json
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
from simlab import catalog, query, sketch, store, store_host  # noqa: E402

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


def quantiles(xs, ps):
    """sketch_quantiles' (simlab.sketch's)."""
    return [sketch.quantile(xs, p) for p in ps]


class Logs(Fixture):
    """A job whose sim wrote the HAVE removal's logs: the forecast's
    predictions (extrapolated, tail_ms), speculative requests (without the
    predicted p90 and p99) and parks, and scheduler events about other
    nodes; and the fixture's HAVE-sample forecast (suite/wt/inv1)."""

    SRC = "suite/wt/logs"
    BOTH = query.Selection(src=[SRC, "suite/wt/inv1"])
    LOGS = query.Selection(src=[SRC])
    # t_ms, node, viewer, peer, peer_honest, live, run, stream, seq, sample_ms, mean_ms, horizon_ms, tail_ms,
    # samples, fresh, extrapolated, seen_ms, held_ms, channel_open_ms
    PREDICTIONS = [(1000, 1, 0, 2, True, True, 0, 0, 5, 30, 50, 80, 150, 4, True, False, 1000, 1100, 0),
                   (1010, 1, 0, 2, True, True, 0, 0, 6, 30, 50, 80, 150, 4, True, False, 1000, 1060, 0),
                   (1020, 1, 0, 2, True, True, 0, 0, 7, 30, 50, 80, 150, 4, True, False, 1000, None, 0),
                   (1030, 1, 0, 3, False, True, 0, 0, 8, 30, 50, 80, 150, 4, True, False, 1000, 1000, 0)]

    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        with mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(cls.root / "target" / "sim-db")}):
            job = (cls.SRC, "k-logs", *golden_cli.JOBS[0][2:4], "logs-5", "ppspp-logs", *golden_cli.JOBS[0][6:])
            d = cls.root / "jobs-logs" / "0"
            meta = golden_cli.job_dir(d, job)
            b = "BIGINT"
            golden_cli.write_table(d, "forecast", [
                ("t_ms", b), ("node", b), ("viewer", b), ("peer", b), ("peer_honest", "BOOLEAN"), ("live", "BOOLEAN"),
                ("run", b), ("stream", b), ("seq", b), ("sample_ms", b), ("mean_ms", b), ("horizon_ms", b),
                ("tail_ms", b), ("samples", b), ("fresh", "BOOLEAN"), ("extrapolated", "BOOLEAN"), ("seen_ms", b),
                ("held_ms", b), ("channel_open_ms", b)], cls.PREDICTIONS)
            golden_cli.write_table(d, "spec_requests", [
                ("t_ms", b), ("node", b), ("viewer", b), ("honest", "BOOLEAN"), ("peer", b),
                ("peer_honest", "BOOLEAN"), ("stream", b), ("seq", b), ("grace_ms", b), ("rtt_ms", b),
                ("predictor", "VARCHAR"), ("predicted_ms", b), ("outcome", "VARCHAR"), ("reason", "VARCHAR"),
                ("t_outcome_ms", b), ("held_ms", b), ("live", "BOOLEAN"), ("err_ms", b)],
                [(1000, 1, 0, True, 2, True, 0, 5, 100, 20, "model", 1050, "answered", None, 1040, 1060, True, 20),
                 (2000, 1, 0, True, 2, True, 0, 6, 100, 20, "model", 2050, "not_yet", None, 2010, 2030, True, 5),
                 (2500, 4, 3, False, 2, True, 0, 7, 100, 20, "model", 2550, "expired", "grace", 2600, None, True,
                  None)])
            golden_cli.write_table(d, "parks", [
                ("t_ms", b), ("node", b), ("viewer", b), ("honest", "BOOLEAN"), ("peer", b),
                ("peer_honest", "BOOLEAN"), ("stream", b), ("seq", b), ("grace_ms", b), ("had", "BOOLEAN"),
                ("outcome", "VARCHAR"), ("reason", "VARCHAR"), ("t_outcome_ms", b), ("held_ms", b)],
                [(1000, 2, 1, True, 1, True, 0, 5, 100, False, "released", None, 1050, 1040),
                 (1500, 2, 1, True, 1, True, 0, 6, 100, True, "refused", "full", 1500, 1400),
                 (2000, 2, 1, True, 1, True, 0, 7, 100, False, "expired", "dataless", 2100, None)])
            golden_cli.write_table(d, "events", [
                ("t_us", b), ("kind", "VARCHAR"), ("node", b), ("peer", b), ("value", b), ("detail", "VARCHAR")],
                [(0, "join", 1, 0, 0, ""), (5_000_000, "prune", 2, 1, 0, ""), (6_000_000, "prune", 3, 4, 0, "")])
            db = cls.root / "jobs-logs" / "logs.duckdb"
            store_host.fold(db, [meta], cls.SRC)
            store_host.export(db, store_host.cache_root() / "cache" / cls.SRC)

    def test_the_forecasts_two_schemas_apart(self):
        got = query.forecast(self.LOGS)
        self.assertIsNone(got.have)
        (r,) = got.prediction.records()
        self.assertEqual((r["kind"], r["n"], r["held"]), ("prediction", 4, 2))
        ps = [10, 50, 90, 99]
        for k, want in (("truth_q", quantiles([50, 10], ps)), ("truth90_q", quantiles([20, -20], ps))):
            self.assertEqual(len(r[k]), 4)
            for a, b in zip(r[k], want):
                self.assertAlmostEqual(a, b, places=9)
        self.assertEqual((r["under_p50"], r["under_p90"], r["under_p99"]), (0.0, 0.5, 1.0))
        self.assertAlmostEqual(r["censored"], 1 / 3)
        self.assertEqual(r["backlog"], 0.0)
        # Beside the HAVE samples, theirs are as they are alone.
        both = query.forecast(self.BOTH)
        alone = query.forecast(query.Selection(src=["suite/wt/inv1"]))
        self.assertIsNone(alone.prediction)
        self.assertEqual(both.have.records(), alone.have.records())
        self.assertEqual(both.records(), [*alone.have.records(), r])
        text = query.forecast_text(both.records()).splitlines()
        self.assertTrue(text[0].startswith("predictions (arrival models)"))
        self.assertIn("HAVE samples (announcement-lag estimates)", text)
        self.assertEqual(text[text.index("HAVE samples (announcement-lag estimates)") + 1:],
                         query.forecast_text(alone.records()).splitlines())

    def test_the_decision_logs(self):
        got = query.spec(self.LOGS, "none")
        (r,) = got.requests.records()
        self.assertEqual((r["predictor"], r["phase"], r["n"], r["runs"], r["answered"], r["not_yet"], r["expired"]),
                         ("model", "all", 2, 1, 0.5, 0.5, 0.0))
        self.assertEqual((r["answer_q"], r["held"], r["early"], r["under_p50"], r["under_p90"], r["never_held"]),
                         ([40.0, 40.0, 40.0], 2, 1.0, 0.5, None, 0.0))
        self.assertEqual(r["pred_err_q"], quantiles([10, -20], [10, 50, 90, 99]))
        (p,) = got.parks.records()
        self.assertEqual((p["n"], p["parked"], p["refused_full"], p["refused_had"], p["parked_released"],
                          p["released_frac_q"], p["released_wait_q"]),
                         (3, 2, 1 / 3, 1 / 3, 0.5, [0.5, 0.5, 0.5], [50.0, 50.0, 50.0]))
        text = query.spec_text(got.records()["spec"], got.records()["parks"])
        self.assertIn("parked and refused requests (honest holders), shares %", text)
        phases = {r["phase"] for r in query.spec(self.LOGS).requests.records()}
        self.assertTrue(phases)
        self.assertNotIn("all", phases)
        with self.assertRaisesRegex(query.Refused, r"the selection has no spec_requests, parks .*SIM_SPEC=1"):
            query.spec(query.Selection(src=["suite/wt/inv1"]))

    def test_the_logs_fields_around_an_event_about_a_node(self):
        got = {r["rel_ms"]: r["p50"] for r in query.aligned(self.LOGS, "prune", "spec_n", peer=1).records()}
        self.assertEqual(got, {-4000: 1.0, -3000: 1.0})
        got = {r["rel_ms"]: r["p50"] for r in query.aligned(self.LOGS, "prune", "park_n", peer=4).records()}
        self.assertEqual(got, {-5000: 2.0, -4000: 1.0})
        n = {r["rel_ms"]: r["p50"] for r in query.aligned(self.LOGS, "prune", "forecast_n", peer=1).records()}
        self.assertEqual(n, {-4000: 2.0})
        with self.assertRaisesRegex(query.Refused, "the selection has no prune event about node 9; its kinds: "):
            query.aligned(self.LOGS, "prune", "spec_n", peer=9)


PULL = golden_cli.simdb_test().SamplesPull


class Arrivals(unittest.TestCase):
    """SamplesPull's run (its scratch S3, the aws CLI stood in), with i-2's
    flush holding arrivals in its samples and no forecast."""

    setUp_pull, aws, gets, cli = PULL.setUp, PULL.aws, PULL.gets, PULL.cli
    RUN, C0 = golden_cli.simdb_test().RUN, golden_cli.simdb_test().C0

    def setUp(self):
        self.setUp_pull()
        st = golden_cli.simdb_test()
        data = self.s3 / "sim-data" / self.RUN / "i-2" / "0"
        samples = self.s3 / "sim-samples" / self.RUN / "i-2" / "0"
        (samples / "forecast.parquet").unlink()
        arrivals = samples / "arrivals.parquet"
        st.simdb.duck(f"COPY (SELECT 'k3' AS k, {st.simdb.q(st.src_of('i-2'))} AS src, 1 AS node, 5 AS seq) "
                      f"TO {st.simdb.q(arrivals)} (FORMAT parquet);", out=None)
        m = json.loads((data / "manifest.json").read_text())
        del m["tables"]["forecast"]
        m["tables"]["arrivals"] = {"rows": 1, "bytes": arrivals.stat().st_size, "sha256": st.simdb.sha256(arrivals)}
        (data / "manifest.json").write_text(json.dumps(m))

    def test_a_worker_uploads_it_with_the_samples(self):
        spec = importlib.util.spec_from_file_location("worker_for_arrivals", HACK / "sim-farm" / "worker.py")
        worker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(worker)
        self.assertIn("arrivals", worker.store().SAMPLE_TABLES)
        sink = worker.Sink({"data": {"prefix": "sim-data/r/i/"}, "samples": {"prefix": "sim-samples/r/i/"}}, "r", "i")
        posted = []
        sink.post = lambda seq, pol, key, path, abort_at: posted.append(key)
        names = ["runs.parquet", "manifest.json", "reads.parquet", "arrivals.parquet"]
        self.assertTrue(sink.put(0, [(n, Path(n)) for n in names], None))
        self.assertEqual(posted, ["sim-data/r/i/0/runs.parquet", "sim-data/r/i/0/manifest.json",
                                  "sim-samples/r/i/0/reads.parquet", "sim-samples/r/i/0/arrivals.parquet"])

    def test_a_pull_fetches_it_with_the_samples_and_names_what_the_flushes_hold(self):
        self.cli("pull", "--run", self.RUN)
        cache = store_host.cache_root() / "cache" / "farm" / self.RUN
        self.assertEqual(list(cache.glob("**/arrivals.parquet")), [])
        out, _ = self.cli("pull", "--run", self.RUN, "--reads", "--dry-run")
        self.assertIn("would GET 4 files", out)
        self.assertIn("jobs' reads and arrivals; S3 holds none for 1 jobs", out)
        self.assertIn(f"s3://bkt/sim-samples/{self.RUN}/i-2/0/arrivals.parquet", out)
        self.calls.clear()
        _, err = self.cli("pull", "--run", self.RUN, "--reads", "--cell", self.C0)
        self.assertIn("pulling the reads and arrivals of 4 jobs the selection lacks: 2 flushes", err)
        self.assertIn(f"sim-samples/{self.RUN}/i-2/0/arrivals.parquet", self.gets())
        self.assertEqual([p.relative_to(cache).as_posix() for p in cache.glob("**/arrivals.parquet")],
                         ["i-2/0/arrivals.parquet"])
        with query.views(query.Selection(src=[f"farm/{self.RUN}/*/*"], cell=[self.C0]), derived=False) as v:
            self.assertEqual(v.records("SELECT k, seq FROM arrivals;"), [{"k": "k3", "seq": 5}])


class Asks(unittest.TestCase):
    """SamplesPull's run, with i-2's flush holding asks and arrivals in its
    samples: node 1's two requests of chunk 6, the first weighing peers 2
    (in its second incarnation, whose first hold of the chunk came at 150
    after its first's at 50), 3 (that never held it) and 4 (that held it
    before the channel opened), the second asking 2 again in the ms its
    hold, the requester's incarnation and the channel all began."""

    setUp_pull, aws, gets, cli = PULL.setUp, PULL.aws, PULL.gets, PULL.cli
    RUN, C0 = golden_cli.simdb_test().RUN, golden_cli.simdb_test().C0
    ASKS = ("t_ms, node, ask, round, picked, peer, seq, rtt_ms, predicted_ms, predicted_p90_ms, predicted_p99_ms, "
            "channel_open_ms, since_ms, peer_incarnation",
            [(100, 1, 0, "'spec'", "true", 2, 6, 20, 120, 160, 200, 10, 0, 1),
             (100, 1, 0, "'spec'", "false", 3, 6, 30, 90, 90, 90, 10, 0, 0),
             (100, 1, 0, "'spec'", "false", 4, 6, 40, "NULL", "NULL", "NULL", 95, 0, 0),
             (200, 1, 1, "'flight'", "true", 2, 6, 20, 150, 150, 150, 150, 150, 1)])
    ARRIVALS = ("t_ms, node, seq, incarnation", [(50, 2, 6, 0), (150, 2, 6, 1), (170, 2, 6, 1), (90, 4, 6, 0)])

    def setUp(self):
        self.setUp_pull()
        st = golden_cli.simdb_test()
        data = self.s3 / "sim-data" / self.RUN / "i-2" / "0"
        samples = self.s3 / "sim-samples" / self.RUN / "i-2" / "0"
        m = json.loads((data / "manifest.json").read_text())
        for t, (cols, rows) in {"asks": self.ASKS, "arrivals": self.ARRIVALS}.items():
            f = samples / f"{t}.parquet"
            values = ", ".join("(" + ", ".join(map(str, r)) + ")" for r in rows)
            st.simdb.duck(f"COPY (SELECT 'k3' AS k, {st.simdb.q(st.src_of('i-2'))} AS src, * FROM (VALUES {values}) "
                          f"v({cols})) TO {st.simdb.q(f)} (FORMAT parquet);", out=None)
            m["tables"][t] = {"rows": len(rows), "bytes": f.stat().st_size, "sha256": st.simdb.sha256(f)}
        (data / "manifest.json").write_text(json.dumps(m))

    def test_a_pull_fetches_it_with_the_samples(self):
        self.assertIn("asks", store_host.SAMPLE_TABLES)
        self.assertIn("Every rival kept", catalog.NORMALIZED["asks"])
        self.cli("pull", "--run", self.RUN)
        cache = store_host.cache_root() / "cache" / "farm" / self.RUN
        self.assertEqual(list(cache.glob("**/asks.parquet")), [])
        _, err = self.cli("pull", "--run", self.RUN, "--reads", "--cell", self.C0)
        self.assertIn("asks", err)
        self.assertIn(f"sim-samples/{self.RUN}/i-2/0/asks.parquet", self.gets())
        with query.views(query.Selection(src=[f"farm/{self.RUN}/*/*"], cell=[self.C0]), derived=False) as v:
            self.assertEqual(v.records("SELECT k, ask, round, picked, peer, seq FROM asks ORDER BY ask, peer;")[0],
                             {"k": "k3", "ask": 0, "round": "spec", "picked": True, "peer": 2, "seq": 6})

    def test_its_view_has_the_truth_of_the_peers_holds_in_their_incarnations(self):
        self.cli("pull", "--run", self.RUN, "--reads", "--cell", self.C0)
        truth = list(query.ASK_TRUTH)
        with query.views(query.Selection(src=[f"farm/{self.RUN}/*/*"], cell=[self.C0]), derived=False) as v:
            got = v.records(f"SELECT ask, peer, {', '.join(truth)} FROM asks ORDER BY ask, peer;")
            log = {r["column_name"] for r in v.records("DESCRIBE asks_log;")}
            cols = [r["column_name"] for r in v.records("DESCRIBE asks;")]
        self.assertFalse(log & set(truth))
        self.assertEqual(cols, [c for c in cols if c in log] + truth)
        # The line's asks predict quantiles, no horizon: held_by_horizon
        # NULL.
        want = [(0, 2, 150, True, 40, 30, False, True, True, None),
                (0, 3, None, False, None, None, None, None, None, None),
                (0, 4, 90, False, -30, None, None, None, None, None),
                (1, 2, 150, True, -60, 0, True, True, True, None)]
        self.assertEqual(got, [dict(zip(["ask", "peer", *truth], w)) for w in want])

    def test_a_pull_of_the_asks_brings_their_truth(self):
        self.cli("pull", "--run", self.RUN)
        self.gets()
        self.assertEqual(store.with_truth(("asks", "reads")), ("asks", "reads", "arrivals"))
        sel = query.Selection(src=[f"farm/{self.RUN}/*/*"], cell=[self.C0])
        with query.views(sel, derived=False) as v:
            self.assertTrue(store.pull_samples(v.sql, ("asks",), say=lambda m: None))
        self.assertEqual(self.gets(), {f"sim-samples/{self.RUN}/i-2/0/asks.parquet",
                                       f"sim-samples/{self.RUN}/i-2/0/arrivals.parquet"})
        with query.views(sel, derived=False) as v:
            self.assertEqual(v.records("SELECT peer, held_ms, live FROM asks WHERE ask = 0 ORDER BY peer;"),
                             [{"peer": 2, "held_ms": 150, "live": True}, {"peer": 3, "held_ms": None, "live": False},
                              {"peer": 4, "held_ms": 90, "live": False}])
            query.need(v.have, "asks")

    def test_a_log_that_carries_its_truth_keeps_it(self):
        new = "(SELECT 'k' AS k, 's' AS src, 2 AS peer, 6 AS seq, 1 AS peer_incarnation, 100 AS t_ms, 20 AS rtt_ms, " \
              "120 AS predicted_ms, 160 AS predicted_p90_ms, 200 AS predicted_p99_ms, 0 AS since_ms, " \
              "10 AS channel_open_ms, NULL::BIGINT AS held_ms, NULL::BOOLEAN AS live)"
        old = "(SELECT 'k' AS k, 's' AS src, 3 AS peer, 6 AS seq, NULL::BIGINT AS peer_incarnation, 100 AS t_ms, " \
              "20 AS rtt_ms, 120 AS predicted_ms, 160 AS predicted_p90_ms, 200 AS predicted_p99_ms, " \
              "NULL::BIGINT AS since_ms, 10 AS channel_open_ms, 90 AS held_ms, true AS live)"
        arrivals = "(SELECT 'k' AS k, 's' AS src, 150 AS t_ms, 2 AS node, 6 AS seq, 1 AS incarnation)"
        both = {"k", "src", "peer", "seq", "peer_incarnation", "t_ms", "rtt_ms", "predicted_ms", "predicted_p90_ms",
                "predicted_p99_ms", "since_ms", "channel_open_ms", "held_ms", "live"}
        view = query.asks_view("v", f"({new} UNION ALL {old})", arrivals, both, {"incarnation"})
        got = query.records(f"{view} SELECT peer, held_ms, live, err_ms FROM v ORDER BY peer;")
        self.assertEqual(got, [{"peer": 2, "held_ms": 150, "live": True, "err_ms": 40},
                               {"peer": 3, "held_ms": 90, "live": True, "err_ms": None}])
        self.assertEqual(query.asks_view("v", "x", "y", {"held_ms"}, set()), "CREATE OR REPLACE VIEW v AS SELECT * FROM x;")



class Haves(unittest.TestCase):
    """Main's HAVE log and request log: node 1 hears of chunk 6 from peer 2
    (its second incarnation held it at 150, its first at 50), from peer 3,
    which never held it, and from peer 4, which held it before the channel
    opened; it asks 2 for it, weighing 3 by its route."""

    ARRIVALS = ("(SELECT * FROM (VALUES ('k', 's', 50, 2, 6, 0), ('k', 's', 150, 2, 6, 1), "
                "('k', 's', 170, 2, 6, 1), ('k', 's', 90, 4, 6, 0)) v(k, src, t_ms, node, seq, incarnation))")
    HAVES = ("(SELECT * FROM (VALUES ('k', 's', 160, 1, 2, 6, 10, 1), ('k', 's', 180, 1, 3, 6, 10, 0), "
             "('k', 's', 100, 1, 4, 6, 95, 0)) v(k, src, t_ms, node, peer, seq, channel_open_ms, peer_incarnation))")

    def test_the_tables_are_kept_with_the_samples(self):
        self.assertIn("haves", store_host.TABLES)
        self.assertIn("haves", store_host.SAMPLE_TABLES)
        self.assertIn("knowledge delay", catalog.NORMALIZED["haves"])
        self.assertIn("'flight'", catalog.NORMALIZED["asks"])
        self.assertEqual(query.LOGGED["haves"], "haves_log")

    def test_its_view_has_the_announcing_peers_hold_and_the_knowledge_delay(self):
        cols = {"k", "src", "t_ms", "node", "peer", "seq", "channel_open_ms", "peer_incarnation"}
        view = query.haves_view("v", self.HAVES, self.ARRIVALS, cols, {"incarnation"})
        got = query.records(f"{view} SELECT peer, held_ms, delay_ms, live FROM v ORDER BY peer;")
        self.assertEqual(got, [{"peer": 2, "held_ms": 150, "delay_ms": 10, "live": True},
                               {"peer": 3, "held_ms": None, "delay_ms": None, "live": False},
                               {"peer": 4, "held_ms": 90, "delay_ms": 10, "live": False}])
        none = query.haves_view("v", self.HAVES, self.ARRIVALS, cols, set())
        self.assertEqual(query.records(f"{none} SELECT count(held_ms) AS n, count(live) AS live FROM v;"),
                         [{"n": 0, "live": 0}])

    def test_a_job_whose_holds_the_views_lack_has_no_truth(self):
        # Job k2's HAVE and ask, of a source the arrivals relation holds
        # nothing of: their truth unknown, not never held.
        haves = self.HAVES.replace("('k', 's', 100, 1, 4, 6, 95, 0)",
                                   "('k', 's', 100, 1, 4, 6, 95, 0), ('k2', 's2', 100, 1, 2, 6, 10, 0)")
        cols = {"k", "src", "t_ms", "node", "peer", "seq", "channel_open_ms", "peer_incarnation"}
        view = query.haves_view("v", haves, self.ARRIVALS, cols, {"incarnation"})
        got = query.records(f"{view} SELECT k, peer, held_ms, delay_ms, live FROM v ORDER BY k, peer;")
        self.assertEqual(got[-1], {"k": "k2", "peer": 2, "held_ms": None, "delay_ms": None, "live": None})
        self.assertEqual(got[1], {"k": "k", "peer": 3, "held_ms": None, "delay_ms": None, "live": False})
        asks = ("(SELECT * FROM (VALUES ('k', 's', 100, 1, 2, 6, 20, 120, 160, 200, 10, 0, 1), "
                "('k2', 's2', 100, 1, 2, 6, 20, 120, 160, 200, 10, 0, 1)) v(k, src, t_ms, node, peer, seq, rtt_ms, "
                "predicted_ms, predicted_p90_ms, predicted_p99_ms, channel_open_ms, since_ms, peer_incarnation))")
        cols = {"k", "src", "t_ms", "node", "peer", "seq", "rtt_ms", "predicted_ms", "predicted_p90_ms",
                "predicted_p99_ms", "channel_open_ms", "since_ms", "peer_incarnation"}
        view = query.asks_view("v", asks, self.ARRIVALS, cols, {"incarnation"})
        got = query.records(f"{view} SELECT k, held_ms, live, err_ms FROM v ORDER BY k;")
        self.assertEqual(got, [{"k": "k", "held_ms": 150, "live": True, "err_ms": 40},
                               {"k": "k2", "held_ms": None, "live": None, "err_ms": None}])

    def test_need_refuses_the_logs_without_their_truth(self):
        query.need({"asks", "haves", "arrivals"}, "asks", "haves")
        with self.assertRaisesRegex(store.Refused, r"^the selection has no arrivals \(nor does S3: run with "
                                                   r"SIM_FORECAST=1 / STRIMS_SIM_FORECAST=1 or SIM_SPEC=1"):
            query.need({"asks", "reads"}, "asks", "reads")

    def test_mains_asks_take_the_view_of_the_lines(self):
        # A rival's route expectation is a mean and a horizon, no
        # quantiles: held_by_horizon is its truth, held_by_p90 NULL.
        asks = ("(SELECT * FROM (VALUES ('k', 's', 100, 1, 0, 'flight', true, 2, 6, 20, 500, NULL, NULL, NULL, NULL, "
                "95, 10, 0, 1), ('k', 's', 100, 1, 0, 'flight', false, 3, 6, 30, NULL, 130, NULL, NULL, 170, NULL, "
                "10, 0, 0), ('k', 's', 100, 1, 0, 'flight', false, 4, 6, 40, NULL, 80, NULL, NULL, 100, NULL, 10, 0, "
                "0)) v(k, src, t_ms, node, ask, round, picked, peer, seq, rtt_ms, timeout_ms, predicted_ms, "
                "predicted_p90_ms, predicted_p99_ms, predicted_horizon_ms, have_ms, channel_open_ms, since_ms, "
                "peer_incarnation))")
        cols = {"k", "src", "t_ms", "node", "ask", "round", "picked", "peer", "seq", "rtt_ms", "timeout_ms",
                "predicted_ms", "predicted_p90_ms", "predicted_p99_ms", "predicted_horizon_ms", "have_ms",
                "channel_open_ms", "since_ms", "peer_incarnation"}
        view = query.asks_view("v", asks, self.ARRIVALS, cols, {"incarnation"})
        got = query.records(f"{view} SELECT peer, held_ms, live, err_ms, pred_err_ms, held_by_p50, held_by_p90, "
                            "held_by_horizon FROM v ORDER BY peer;")
        none = {"pred_err_ms": None, "held_by_p50": None, "held_by_p90": None, "held_by_horizon": None}
        self.assertEqual(got, [{"peer": 2, "held_ms": 150, "live": True, "err_ms": 40, **none},
                               {"peer": 3, "held_ms": None, "live": False, "err_ms": None, **none},
                               {"peer": 4, "held_ms": 90, "live": True, "err_ms": -30, "pred_err_ms": 10,
                                "held_by_p50": False, "held_by_p90": None, "held_by_horizon": True}])

    def test_the_catalogs_view_is_the_same(self):
        sql = catalog.view_sql("haves", {"t_ms": "BIGINT"}, "haves", {}, {"incarnation": "BIGINT"})
        self.assertIn("delay_ms", sql)
        self.assertIn("h.incarnation = a.peer_incarnation", sql)


class SeedLog(unittest.TestCase):
    """The seed log's tables: kept as tables, not samples (a few thousand
    rows a run), with their notes, keys, joins and knob; both views read an
    older run's peer -1 (none) as NULL, as the sims now write it; the sends'
    view takes each chunk's hold by the peer from arrivals, its truth, which
    a job whose holds the views lack doesn't have, and which need and a
    pull bring along."""

    def test_the_tables_are_kept_with_their_notes(self):
        for t in ("seed_decisions", "seed_sends"):
            self.assertIn(t, store_host.TABLES)
            self.assertNotIn(t, store_host.SAMPLE_TABLES)
            self.assertIn(t, catalog.NORMALIZED)
            self.assertIn(t, catalog.STANDARD_KEYS)
            self.assertIn(t, catalog.STANDARD_JOINS)
            self.assertEqual(query.TABLE_KNOBS[t], ("SPEC",))
        self.assertIn("arrivals.incarnation = seed_sends.peer_incarnation", catalog.STANDARD_JOINS["seed_sends"])
        self.assertEqual({t: catalog.STANDARD_KEYS[t] for t in ("seed_decisions", "seed_sends")},
                         {"seed_decisions": "k, node, n", "seed_sends": "k, node, n"})
        with self.assertRaisesRegex(query.Refused, r"seed_sends \(run with SIM_SPEC=1"):
            query.need({"arrivals"}, "seed_sends")

    def test_the_views_read_no_peer_as_null(self):
        cols = {"k": "VARCHAR", "src": "VARCHAR", "t_ms": "BIGINT", "peer": "BIGINT", "kind": "VARCHAR"}
        sql = catalog.view_sql("seed_decisions", cols, "seed_decisions", {})
        self.assertIn('nullif(x."peer", -1) AS "peer"', sql)
        self.assertIn('x."kind"', sql)
        sends = catalog.view_sql("seed_sends", {**cols, "fate": "VARCHAR"}, "seed_sends", {})
        self.assertIn('nullif(x."peer", -1) AS "peer"', sends)
        self.assertIn("AS held_ms", sends)

    def test_the_sends_view_takes_the_peers_hold(self):
        # Pushes of chunk 6 to peer 2 (its second incarnation held it at
        # 150) and to peer 3 (never held it), an answer to peer 4 (held it
        # at 90), and job k2's push, of a source the arrivals relation
        # holds nothing of: unknown, not never held.
        sends = ("(SELECT * FROM (VALUES ('k', 's', 160, 0, 0, 2, 1, 'push', 6, 'written', 120), "
                 "('k', 's', 180, 0, 1, 3, 0, 'push', 6, 'written', 170), "
                 "('k', 's', 100, 0, 2, 4, 0, 'answer', 6, 'written', 80), "
                 "('k2', 's2', 100, 0, 0, 2, 1, 'push', 6, 'written', 90)) "
                 "v(k, src, t_ms, node, n, peer, peer_incarnation, how, seq, fate, queued_ms))")
        cols = {"k", "src", "t_ms", "node", "n", "peer", "peer_incarnation", "how", "seq", "fate", "queued_ms"}
        view = query.seed_sends_view("v", sends, Haves.ARRIVALS, cols, {"incarnation"})
        got = query.records(f"{view} SELECT k, n, held_ms FROM v ORDER BY k, n;")
        self.assertEqual(got, [{"k": "k", "n": 0, "held_ms": 150}, {"k": "k", "n": 1, "held_ms": None},
                               {"k": "k", "n": 2, "held_ms": 90}, {"k": "k2", "n": 0, "held_ms": None}])
        none = query.seed_sends_view("v", sends, Haves.ARRIVALS, cols, set())
        self.assertEqual(query.records(f"{none} SELECT count(held_ms) AS n FROM v;"), [{"n": 0}])
        self.assertEqual(query.LOGGED["seed_sends"], "seed_sends_log")

    def test_the_sends_bring_their_truth(self):
        self.assertEqual(store.with_truth(("seed_sends",)), ("seed_sends", "arrivals"))
        query.need({"seed_sends", "arrivals"}, "seed_sends")
        with self.assertRaisesRegex(store.Refused, r"^the selection has no arrivals \(nor does S3: run with "
                                                   r"SIM_FORECAST=1 / STRIMS_SIM_FORECAST=1 or SIM_SPEC=1"):
            query.need({"seed_sends", "reads"}, "seed_sends", "reads")


if __name__ == "__main__":
    unittest.main()
