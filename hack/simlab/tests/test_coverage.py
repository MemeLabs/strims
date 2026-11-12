#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.coverage over a fixture catalog of its own: four strims-sim jobs
of golden_cli's job dirs (arm decl at seeds 1 and 2, its viewers there
from the start; arm main at seed 1 with the seed's uplink stepped down at
20 s and up at 32 s, and at seed 2 a collapse, its series_meta recording
its seed and its stream count), arm bare's without push parents, arm
noarr's without arrivals, and a ppspp sim job (arm pp: decl's seed-1
holds, but ending a second after its last publication, as the ppspp
sim's runs do, with empty parents, as it writes them without hybrid push,
and its row recording its stream count), each with the holds and push
parents the measure reads, folded
and exported by simdb.py into a local suite invocation's source, and a
second invocation's source that labels two of them again (decl's seed 1
in a cell of its own, star-5b; its seed 2 in star-5, as the first has
it), indexed by the catalog. Every run publishes the same chunks: two
streams, a chunk of each every 250 ms from 20 s to 39.75 s of a 40 s run,
so the last 4 s are censored.

  Prepare      the measure's per-run facts: the seed, the streams, the
               interval, the run's end, the phases, the censored chunks,
               the parents rows' order at one ms, the subscriptions
  Sections     each section's numbers on the fixture, worked by hand:
               latency's counts, shares, quantiles, anomalies and the
               runs' spread, `rest` only where two phases pool, collapsed
               runs apart; routes'; I1's uncovered time, seed children
               and subscription ends; late's classes
  Grain        a job counts in each cell and arm the selection labels it
               with, once in each; the ppspp sim's job: decl's seed 1's
               latency, its censored chunks, no I1 or classes without
               parents rows; the stream count read where a sim recorded
               it, a warning for every run whose count is inferred, and
               none failing where the catalog has no asks; a run without
               arrivals left out with a warning, and a selection of none
               with them refused
  Cli          simdb.py coverage-report's tables and JSON, as a caller runs
               it, and a section the catalog can't answer yet

  python3 hack/simlab/tests/test_coverage.py
"""

import json
import os
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
from simlab import catalog, coverage, forecast, query, store_host, verdict  # noqa: E402

SRC = "suite/wt/cv"
# (arm, seed): the run's shape (golden_cli's series).
RUNS = {("decl", 1): None, ("decl", 2): None, ("main", 1): "step", ("main", 2): "collapse"}
# The second invocation, whose jobs are SRC's labelled again.
RELABEL = "suite/wt/cv-relabel"
# The ppspp sim's job's arm label.
PP = "ppspp-decl"
NAMES = {f"strims-sim-{arm}": arm for arm in ("decl", "main", "bare", "noarr")} | {PP: "pp"}
SEL = forecast.Selection(exp=[SRC], names=NAMES, phases="all")
SEED = 0
END_MS = golden_cli.SECS * 1000
CHUNKS = 160
# The ppspp job's: its last published at 39 s, a second before its end.
PP_CHUNKS = 154


def pub(q):
    """Chunk q's publication: two streams (q mod 2), a chunk of each every
    250 ms from 20 s."""
    return 20000 + 250 * (q // 2)


ARRIVAL_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"), ("seq", "BIGINT"), ("how", "VARCHAR"),
                ("peer", "BIGINT"), ("parent", "BIGINT"), ("released_ms", "BIGINT"), ("incarnation", "BIGINT")]


def arrivals(seed):
    """The holds: the seed (0) publishes chunks 0-159; host 1, the seed's
    child, holds each 40 ms after by its push, and host 2, host 1's
    child, 80 ms after. But:

      2    host 1 holds it 400 ms after (startup: pushed late)
      1    at seed 2 host 1 holds it 1000 ms after, by an answer (startup:
           stream 1 has no seed child until 21 s)
      100  no host holds it at seed 1 (never)
      101  only host 2, 300 ms after, by the seed's answer (stream 1's
           seed child pruned it from 32 s to 33 s)
      102  host 1 600 ms after (pushed late)
      103  only host 3, 100 ms after, from no source it names (an
           anomaly, on time)
      104  host 1 500 ms after by the seed's answer, its push released at
           250 ms (released)
      106  only host 2, 700 ms after, by the seed's answer (its child
           never held it)
      108  host 1 260 ms after by the seed's answer, no release (other)
      114  host 1 300 ms after (pushed late with two seed children: host
           3 is one, from 34 s to 35 s, and never holds it)
      157  host 1 400 ms after (censored)
      158  no host holds it (censored)
    """
    rows = [(pub(q), SEED, -1, q, "own", -1, -1, None, 0) for q in range(CHUNKS)]
    special = {2: [(1, 400, "push", 0, None), (2, 440, "push", 1, None)],
               100: [] if seed == 1 else None,
               101: [(2, 300, "answer", 0, None)],
               102: [(1, 600, "push", 0, None), (2, 640, "push", 1, None)],
               103: [(3, 100, "other", -1, None)],
               104: [(1, 500, "answer", 0, 250), (2, 540, "push", 1, None)],
               106: [(2, 700, "answer", 0, None)],
               108: [(1, 260, "answer", 0, None), (2, 300, "push", 1, None)],
               114: [(1, 300, "push", 0, None), (2, 340, "push", 1, None)],
               157: [(1, 400, "push", 0, None)],
               158: []}
    if seed == 2:
        special[1] = [(1, 1000, "answer", 0, None), (2, 1040, "push", 1, None)]
    for q in range(CHUNKS):
        holds = special.get(q)
        if holds is None:
            holds = [(1, 40, "push", 0, None), (2, 80, "push", 1, None)]
        for node, after, how, peer, released in holds:
            rows.append((pub(q) + after, node, node - 1, q, how, peer, peer, None if released is None
                         else pub(q) + released, 0))
    return rows


PARENT_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("stream", "BIGINT"), ("parent", "BIGINT"),
               ("reason", "VARCHAR")]


def parents(seed):
    """The push trees: host 1 the seed's child of both streams from the
    start (stream 1 from 21 s at seed 2), pruning stream 1 at 32 s and
    back at 33 s; host 2 host 1's child; host 3 the seed's child of
    stream 0 from 34 s, lost at 35 s; host 4 opening to the seed and shed
    at one ms (36 s), which ends no subscription; host 5 host 1's child of
    stream 1, shed by it and opening to the seed at one ms (37 s): the
    seed's child to the end."""
    rows = [(0 if seed == 1 else 21000, 1, 1, 0, "open"), (0, 1, 0, 0, "open"),
            (32000, 1, 1, -1, "prune"), (33000, 1, 1, 0, "open"),
            (0, 2, 0, 1, "open"), (0, 2, 1, 1, "open"),
            (34000, 3, 0, 0, "open"), (35000, 3, 0, -1, "lost"),
            (36000, 4, 0, -1, "shed"), (36000, 4, 0, 0, "open"),
            (0, 5, 1, 1, "open"), (37000, 5, 1, 0, "open"), (37000, 5, 1, -1, "shed")]
    return rows


ASK_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("honest", "BOOLEAN"), ("ask", "BIGINT"), ("round", "VARCHAR"),
            ("peer", "BIGINT"), ("peer_honest", "BOOLEAN"), ("picked", "BOOLEAN"), ("stream", "BIGINT"),
            ("seq", "BIGINT")]


def build(root):
    """The fixture's cache under `root` (target/sim-db), indexed, and an
    empty git repository (its main checkout). Arm bare's run has no push
    parents (its streams are its asks'), arm noarr's no arrivals."""
    sm = golden_cli.simdb_test().simdb
    os.environ["SIM_DB_ROOT"] = str(root / "target" / "sim-db")
    jobs = []
    for (arm, seed), shape in [*RUNS.items(), (("bare", 1), None), (("noarr", 1), None)]:
        label = f"strims-sim-{arm}"
        d = root / "jobs" / f"{arm}-{seed}"
        job = (SRC, f"{label}:star-5:{seed}", "strims-sim", "calibration_star", "star-5", label, seed,
               golden_cli.SS_ENV, golden_cli.T0, shape)
        rec = golden_cli.job_dir(d, job)
        if arm != "noarr":
            golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, arrivals(seed))
        if arm == "bare":
            for f in ("parents.csv", "parents.schema.json"):
                (d / f).unlink()
            golden_cli.write_table(d, "asks", ASK_COLS, [(26000, 1, True, 0, "flight", 0, True, True, 1, 81)])
        else:
            golden_cli.write_table(d, "parents", PARENT_COLS, parents(seed))
        if (arm, seed) == ("main", 2):
            meta = json.loads((d / "series_meta.json").read_text())
            (d / "series_meta.json").write_text(json.dumps(meta | {"seed": SEED, "streams": 2}))
        jobs.append(rec)
    d = root / "jobs" / "pp-1"
    rec = golden_cli.job_dir(d, (SRC, f"{PP}:star-5:1", "ppspp", "sim_job", "star-5", PP, 1, golden_cli.PP_ENV,
                                 golden_cli.T0, None))
    rec["row"]["streams"] = 2
    golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, [r for r in arrivals(1) if r[3] < PP_CHUNKS])
    golden_cli.write_table(d, "parents", PARENT_COLS, [])
    jobs.append(rec)
    db = root / "store.duckdb"
    sm.fold(db, jobs, SRC)
    sm.export(db, root / "target" / "sim-db" / "cache" / SRC)
    db.unlink()
    label = "strims-sim-decl"
    golden_cli.relabel(root, RELABEL, [
        (f"{label}:star-5:{seed}", "strims-sim", "calibration_star", cell, label, seed, golden_cli.SS_ENV)
        for seed, cell in ((1, "star-5b"), (2, "star-5"))])
    subprocess.run(["git", "init", "-q", str(root)], check=True)


class Fixture(unittest.TestCase):
    """The fixture under a scratch root, indexed once; the main checkout
    is the root."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-coverage-")
        cls.root = Path(cls.tmp.name)
        saved = os.environ.get("SIM_DB_ROOT")
        try:
            build(cls.root)
            with cls.patched(cls):
                catalog.index(2, lambda e: None)
                cls.warned = []
                cls.results = {s: r.records() for s, r in coverage.report(SEL, warn=cls.warned.append).items()}
        finally:
            os.environ.pop("SIM_DB_ROOT", None)
            if saved is not None:
                os.environ["SIM_DB_ROOT"] = saved

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def patched(self):
        stack = mock.patch.multiple(store_host, main_checkout=lambda: self.root)
        env = mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.root / "target" / "sim-db"), "TZ": "UTC"})
        sm = mock.patch.object(simlab.simdb(), "main_checkout", lambda: self.root)

        class Both:
            def __enter__(s):
                for p in (stack, env, sm):
                    p.start()

            def __exit__(s, *exc):
                for p in (sm, env, stack):
                    p.stop()

        return Both()

    def setUp(self):
        p = self.patched()
        p.__enter__()
        self.addCleanup(p.__exit__, None, None, None)

    def rows(self, section, **want):
        return [r for r in self.results[section] if all(r.get(k) == v for k, v in want.items())]

    def one(self, section, **want):
        got = self.rows(section, **want)
        self.assertEqual(len(got), 1, f"{section} {want}: {got}")
        return got[0]

    def close(self, got, want):
        """`got`, a list of numbers, is `want` to rounding."""
        self.assertEqual(len(got), len(want), got)
        for g, w in zip(got, want):
            self.assertAlmostEqual(g, w)


def p99(xs):
    """quantile_cont's 0.99 of `xs`."""
    xs = sorted(xs)
    at = 0.99 * (len(xs) - 1)
    i = int(at)
    return xs[i] + (at - i) * (xs[min(i + 1, len(xs) - 1)] - xs[i])


# Every run's steady late chunks but 100 (seed 1's never), with their
# latencies, and the steady chunks held 40 ms after.
STEADY_LATE = {101: 300, 102: 600, 104: 500, 106: 700, 108: 260, 114: 300}
JUDGED_STEADY = 88


class Prepare(Fixture):
    def test_the_runs_facts(self):
        with forecast.connected() as c:
            coverage.prepare(c, SEL)
            jobs = c.execute("SELECT arm, seed, collapsed, seed_node, seed_from, streams, streams_from, end_ms, "
                             "interval_ms, parents_rows FROM cv_jobs ORDER BY ALL").fetchall()
            phases = c.execute("SELECT arm, seed, phase, list(sec ORDER BY sec) FROM cv_phase "
                               "JOIN cv_jobs USING (k, src) WHERE arm IN ('decl', 'main') GROUP BY ALL "
                               "ORDER BY ALL").fetchall()
            chunks = c.execute("SELECT judged, count(*), min(pub_ms), max(pub_ms) FROM cv_chunks "
                               "WHERE arm = 'decl' AND seed = 1 GROUP BY ALL ORDER BY ALL").fetchall()
        # The seed (main's seed-2 series_meta records it, the others'
        # arrivals give it) and the run's end (its last second, 39, ends at
        # 40 s); two streams, read from main's seed-2 series_meta and the
        # ppspp job's row, else from the parents, or from bare's asks; 250
        # ms within a stream; 13 parents rows, none for bare and the ppspp
        # job. noarr's run, without arrivals, is left out.
        rows = len(parents(1))
        self.assertEqual(jobs, [("bare", 1, False, SEED, "arrivals", 2, "asks", END_MS, 250.0, 0),
                                ("decl", 1, False, SEED, "arrivals", 2, "parents", END_MS, 250.0, rows),
                                ("decl", 2, False, SEED, "arrivals", 2, "parents", END_MS, 250.0, rows),
                                ("main", 1, False, SEED, "arrivals", 2, "parents", END_MS, 250.0, rows),
                                ("main", 2, True, SEED, "series_meta", 2, "series_meta", END_MS, 250.0, rows),
                                ("pp", 1, False, SEED, "arrivals", 2, "row", END_MS, 250.0, 0)])
        # golden_cli's latency event at 5 s holds 5-19 s; main's seed 1
        # steps down at 20 s and up at 32 s.
        span = lambda a, b: list(range(a, b))  # noqa: E731
        self.assertEqual(phases[:3], [("decl", 1, "after_disruption", span(5, 20)),
                                      ("decl", 1, "startup", span(0, 5) + span(20, 25)),
                                      ("decl", 1, "steady", span(25, 40))])
        self.assertEqual([p[2:] for p in phases if p[:2] == ("main", 1)],
                         [("after_disruption", span(5, 20)), ("after_down", span(20, 32)),
                          ("after_up", span(32, 40)), ("startup", span(0, 5))])
        # A chunk published past 35.75 s (40 s less the latency budget's
        # most) is censored.
        self.assertEqual(verdict.DELIVERY_MAX_MS, 4250)
        self.assertEqual(chunks, [(False, 32, 36000, 39750), (True, 128, 20000, 35750)])

    def test_parents_at_one_ms_and_the_subscriptions(self):
        with forecast.connected() as c:
            coverage.prepare(c, SEL)
            subs = c.execute("SELECT node, stream, open_ms, close_ms, end_reason FROM cv_subs "
                             "JOIN cv_jobs USING (k, src) WHERE arm = 'decl' AND seed = 1 ORDER BY ALL").fetchall()
        # Host 4 had no parent at 36 s: its open came first, then the shed,
        # a subscription of no length. Host 5 had host 1: shed, then the
        # open, the seed's child to the end (end_reason NULL).
        self.assertEqual(subs, [(1, 0, 0, END_MS, None), (1, 1, 0, 32000, "prune"), (1, 1, 33000, END_MS, None),
                                (3, 0, 34000, 35000, "lost"), (5, 1, 37000, END_MS, None)])

    def test_the_definitions_are_the_catalogs_macros(self):
        # Each with its comment and parameters, as describe gives them.
        got = {r["name"]: r for r in catalog.scoring_docs()}
        want = {"coverage_runs_of": "(jobs)", "coverage_ask_streams_of": "(jobs)"}
        for name in ("seed_children", "coverage_chunks", "seed_cover"):
            want |= {f"{name}_in": "(runs)", f"{name}_of": "(jobs)"}
        for name, params in want.items():
            self.assertEqual((got[name]["kind"], got[name]["keys"]), ("table macro", params), name)
            self.assertTrue(got[name]["description"], name)
        self.assertIn(f"published at least {verdict.DELIVERY_MAX_MS} ms",
                      got["coverage_chunks_in"]["description"])
        # The vocabulary's names, filled in; the fallback's columns line
        # isn't its doc.
        self.assertIn("a 'switch' first, an 'open' after", got["seed_children_in"]["description"])
        self.assertNotIn("else none", got["coverage_ask_streams_of"]["description"])
        self.assertIn("I1, the invariant that every stream a viewer watches", got["seed_cover_in"]["description"])


class Sections(Fixture):
    def test_latency(self):
        r = self.one("latency", arm="decl", phase="steady")
        # Two runs of 88 judged chunks (32 censored each); seed 1's chunk
        # 100 never held; the late chunks; 103 from no named source.
        self.assertEqual((r["collapsed"], r["runs"], r["seeds"], r["interval_runs"], r["chunks"], r["censored"]),
                         (False, 2, [1, 2], [250.0] * 3, 2 * JUDGED_STEADY, 64))
        self.assertEqual((r["held"], r["never"], r["late"], r["anomalies"]), (175, 1, 2 * len(STEADY_LATE) + 1, 2))
        self.assertAlmostEqual(r["never_share"], 1 / 176)
        self.assertAlmostEqual(r["late_share"], 13 / 176)
        held = [100] * 2 + [ms for ms in STEADY_LATE.values()] * 2
        held += [40] * (175 - len(held))
        self.assertEqual((r["p50_ms"], r["p90_ms"], r["max_ms"]), (40.0, 40.0, 700))
        self.assertAlmostEqual(r["p99_ms"], p99(held))
        self.assertAlmostEqual(r["p99_ms"], 626)
        # Each run's: seed 1 7 late of 88 and a never, seed 2 6 late; seed
        # 1 holds 87 (80 at 40 ms), seed 2 88.
        self.close(r["late_runs"], [6 / 88, 6.5 / 88, 7 / 88])
        self.close(r["never_runs"], [0.0, 0.5 / 88, 1 / 88])
        one = [100] + list(STEADY_LATE.values())
        lo, hi = p99(one + [40] * 81), p99(one + [40] * 80)
        self.close(r["p99_runs"], [lo, (lo + hi) / 2, hi])
        self.close(r["p99_runs"], [613, 613.5, 614])
        # The startup's: chunk 2 at 400 ms in both runs, chunk 1 at 1000 ms
        # at seed 2.
        s = self.one("latency", arm="decl", phase="startup")
        self.assertEqual((s["chunks"], s["censored"], s["late"], s["max_ms"]), (80, 0, 3, 1000))
        self.assertAlmostEqual(s["p99_ms"], p99([40] * 77 + [400, 400, 1000]))
        self.close(s["late_runs"], [0.025, 0.0375, 0.05])

    def test_a_pooled_row_of_two_phases_and_collapsed_runs_apart(self):
        # decl's chunks are startup and steady alone: no pooled row. main's
        # seed 1 has two phases past startup (down at 20 s, up at 32 s),
        # pooled in a row of phase NULL, its collapsed seed 2 one.
        self.assertEqual([r["phase"] for r in self.rows("latency", arm="decl")], ["startup", "steady"])
        main = [(r["collapsed"], r["phase"], r["chunks"], r["censored"], r["late"], r["never"])
                for r in self.rows("latency", arm="main")]
        self.assertEqual(main, [(False, "after_down", 96, 0, 1, 0), (False, "after_up", 32, 32, 7, 1),
                                (False, None, 128, 32, 8, 1),
                                (True, "startup", 40, 0, 2, 0), (True, "steady", 88, 32, 6, 0)])

    def test_routes(self):
        got = {(r["route"], r["source"]): r for r in self.rows("routes", arm="decl", phase="steady")}
        self.assertEqual(sorted(got, key=str),
                         [("answer", "seed"), ("never", None), ("other", "unnamed"), ("push", "seed")])
        push, answer = got[("push", "seed")], got[("answer", "seed")]
        # Pushes: 82 and 83 of 88, 102 and 114 late; answers: 101, 104,
        # 106 and 108 in both runs.
        self.assertEqual((push["chunks"], push["late"], push["max_ms"]), (165, 4, 600))
        self.assertAlmostEqual(push["share"], 165 / 176)
        self.assertAlmostEqual(push["p99_ms"], p99([40] * 161 + [300, 300, 600, 600]))
        self.close(push["share_runs"], [82 / 88, 82.5 / 88, 83 / 88])
        self.assertEqual((answer["chunks"], answer["late"], answer["p50_ms"], answer["p99_ms"]), (8, 8, 400.0, 700.0))
        other = got[("other", "unnamed")]
        self.assertEqual((other["chunks"], other["late"], other["p50_ms"]), (2, 0, 100.0))
        # The never-held chunk is seed 1's alone: seed 2's share is 0.
        never = got[("never", None)]
        self.assertEqual((never["chunks"], never["p50_ms"]), (1, None))
        self.close(never["share_runs"], [0.0, 0.5 / 88, 1 / 88])
        startup = self.one("routes", arm="decl", phase="startup", route="answer")
        self.assertEqual(startup["chunks"], 1)
        self.close(startup["share_runs"], [0.0, 0.0125, 0.025])

    def test_invariants(self):
        # Steady, 25-39 s: two streams a run. Stream 1 has no seed child
        # from 32 s to 33 s (host 1's prune); stream 0 has two from 34 s to
        # 35 s (host 3, lost), stream 1 two from 37 s (host 5).
        r = self.one("invariants", arm="decl", phase="steady")
        self.assertEqual((r["runs"], r["stream_s"], r["uncovered_s"], r["touched_s"]), (2, 60, 2.0, 2))
        self.assertAlmostEqual(r["uncovered"], 2 / 60)
        self.assertAlmostEqual(r["seed_children"], 2 * (15 + 1 + 14 + 3) / 60)
        self.assertEqual((r["ends_prune"], r["ends_lost"], r["ends_shed"], r["ends_switch"], r["ends_open"]),
                         (2, 2, 0, 0, 0))
        # The startup (0-4 s, 20-24 s): seed 2's stream 1 had no seed child
        # before 21 s, 6 of its seconds.
        s = self.one("invariants", arm="decl", phase="startup")
        self.assertEqual((s["stream_s"], s["uncovered_s"]), (40, 6.0))
        self.assertAlmostEqual(s["seed_children"], 34 / 40)
        self.close(s["uncovered_runs"], [0.0, 0.15, 0.3])
        # Past startup the stream-seconds have two phases (golden_cli's
        # latency event at 5 s, 5-19 s): a row of phase NULL pools them.
        pooled = self.one("invariants", arm="decl", phase=None)
        self.assertEqual((pooled["stream_s"], pooled["uncovered_s"], pooled["ends_prune"]), (120, 17.0, 2))
        self.assertAlmostEqual(pooled["seed_children"], (30 + 15 + 66) / 120)
        # A run without parents rows has no seed children to count: its I1
        # is NULL, not wholly uncovered.
        bare = self.one("invariants", arm="bare", phase="steady")
        self.assertEqual((bare["runs"], bare["unparented"], bare["stream_s"], bare["uncovered"],
                          bare["seed_children"], bare["uncovered_runs"]), (1, 1, 0, None, None, None))
        self.assertEqual(r["unparented"], 0)

    def test_late(self):
        got = {(r["class"], r["after"]): r for r in self.rows("late", arm="decl", phase="steady")}
        self.assertEqual(list(got), [("no_seed_child", "prune"), ("released", None), ("pushed_late", None),
                                     ("child_never", None), ("other", None)])
        late = 13
        want = {("no_seed_child", "prune"): (2, 0.0, 0, 300.0), ("released", None): (2, 1.0, 0, 500.0),
                ("pushed_late", None): (4, 1.5, 0, 450.0), ("child_never", None): (3, 1.0, 1, 700.0),
                ("other", None): (2, 1.0, 0, 260.0)}
        for key, (n, children, never, p50) in want.items():
            r = got[key]
            self.assertEqual((r["chunks"], r["children"], r["never"], r["p50_ms"]), (n, children, never, p50), key)
            self.assertAlmostEqual(r["late_share"], n / late)
            self.assertAlmostEqual(r["share"], n / 176)
        # child_never: 106 in both runs, seed 1's never-held 100 too.
        self.close(got[("child_never", None)]["share_runs"], [1 / 88, 1.5 / 88, 2 / 88])
        startup = {(r["class"], r["after"]): r["chunks"] for r in self.rows("late", arm="decl", phase="startup")}
        # Seed 2's chunk 1: its stream had no seed child before (after NULL).
        self.assertEqual(startup, {("no_seed_child", None): 1, ("pushed_late", None): 2})
        # The runs without parents rows have no classes.
        self.assertEqual(self.rows("late", arm="bare") + self.rows("late", arm="pp"), [])

    def test_selection(self):
        # The collapsed runs alone, past startup (the default phases).
        # The collapsed run records its stream count: no warning.
        warned = []
        got = coverage.report(forecast.Selection(exp=[SRC], names=NAMES, collapsed=True), ["latency"],
                              warn=warned.append)
        self.assertEqual({(r["arm"], r["phase"]) for r in got["latency"].records()}, {("main", "steady")})
        self.assertEqual(warned, [])
        with self.assertRaisesRegex(ValueError, "no section 'attribution'"):
            coverage.report(SEL, ["attribution"], warn=[].append)


class Grain(Fixture):
    def test_a_job_counts_in_each_of_its_labels_once(self):
        sel = forecast.Selection(exp=[SRC, RELABEL], names=NAMES, phases="all")
        got = {(r["arm"], r["cell"], r["phase"]): r for r in coverage.report(sel, ["latency"], warn=[].append)
               ["latency"].records()}
        # star-5's decl is SEL's, its seed 2 counted once though both
        # invocations name it there; star-5b's is decl's seed 1 alone.
        mine = self.one("latency", arm="decl", phase="steady")
        self.assertEqual(got[("decl", "star-5", "steady")], mine)
        b = got[("decl", "star-5b", "steady")]
        self.assertEqual((b["runs"], b["seeds"], b["chunks"], b["late"], b["never"], b["p99_ms"]),
                         (1, [1], JUDGED_STEADY, len(STEADY_LATE) + 1, 1, mine["p99_runs"][2]))

    def test_the_ppspp_sims_job(self):
        # decl's seed-1 holds to chunk 153, published at 39 s, a second
        # before the run's end: its judged chunks are decl's seed 1's, and
        # 26 censored (chunks 128-153) where decl's have 32.
        r = self.one("latency", arm="pp", phase="steady")
        self.assertEqual((r["runs"], r["chunks"], r["censored"], r["late"], r["never"], r["anomalies"]),
                         (1, JUDGED_STEADY, PP_CHUNKS - 128, len(STEADY_LATE) + 1, 1, 1))
        self.assertAlmostEqual(r["p99_ms"], 614)
        # Its parents are empty, as the ppspp sim writes them without hybrid
        # push: no I1, no classes.
        i = self.one("invariants", arm="pp", phase="steady")
        self.assertEqual((i["unparented"], i["stream_s"], i["uncovered_s"], i["seed_children"]), (1, 0, None, None))

    def test_the_warnings(self):
        # noarr's run wrote no arrivals; bare's and the ppspp job's have no
        # parents rows; of the five left only main's seed 2 (series_meta)
        # and the ppspp job (its row) record their stream count.
        self.assertEqual(self.warned, [
            f"coverage: 1 runs wrote no arrivals ({query.knobs_text('arrivals')} writes them): left out of every "
            "section",
            "coverage: 2 runs have no parents rows (no push trees): their I1 and their late chunks' classes are NULL",
            "coverage: 4 runs' stream count is inferred, not read (1 from asks, 3 from parents): neither their "
            "series_meta nor their row records it, and their interval and I1 take the count inferred"])
        got = {(r["arm"], r["collapsed"]): r["inferred_streams"] for r in self.rows("latency", phase="steady")}
        self.assertEqual(got, {("bare", False): 1, ("decl", False): 2, ("main", True): 0, ("pp", False): 0})
        self.assertFalse([r for rows in self.results.values() for r in rows if r["arm"] == "noarr"])

    def test_a_selection_without_arrivals_is_refused(self):
        sel = forecast.Selection(exp=[SRC], arm=["strims-sim-noarr"], names=NAMES)
        with self.assertRaisesRegex(query.Refused, "the selection has no arrivals"):
            coverage.report(sel, ["latency"], warn=[].append)

    def test_a_catalog_without_asks(self):
        # bare's run then has no source of its count: one stream, said so,
        # and the report still runs: with one stream, half its publications
        # come 0 ms after another, and its interval is 0.
        real = catalog.scoring_cols

        def without(columns, have=None):
            return real(columns, have={t: c for t, c in (have or {}).items() if t != "asks"})

        warned = []
        with mock.patch.object(catalog, "scoring_cols", without):
            got = coverage.report(SEL, ["latency"], warn=warned.append)
        self.assertIn("(3 from parents, 1 with one assumed)", warned[-1])
        bare = [r for r in got["latency"].records() if r["arm"] == "bare" and r["phase"] == "steady"]
        self.assertEqual([r["interval_runs"] for r in bare], [[0.0] * 3])


class Cli(Fixture):
    def cli(self, *args):
        env = golden_cli.environment(self.root)
        p = subprocess.run([sys.executable, str(golden_cli.SIMDB), "coverage-report", *args], env=env,
                           capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stderr)
        return p.stdout

    def test_its_tables_and_json(self):
        names = [f"--name={arm}={label}" for label, arm in NAMES.items()]
        text = self.cli("--exp", SRC, *names, "--section", "latency", "--section", "late")
        self.assertTrue(text.startswith("## latency\n"))
        self.assertIn("## late\n", text)
        self.assertIn("| decl ", text)
        got = json.loads(self.cli("--exp", SRC, *names, "--section", "routes", "--json"))
        self.assertEqual(sorted(got), ["routes", "warnings"])
        self.assertEqual((got["routes"], got["warnings"]), (self.results["routes"], self.warned))
        rest = json.loads(self.cli("--exp", SRC, *names, "--phases", "rest", "--section", "latency", "--json"))
        self.assertNotIn("startup", {r["phase"] for r in rest["latency"]})


if __name__ == "__main__":
    unittest.main()
