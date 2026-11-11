#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.forecast and simlab.regimes over a fixture catalog of their own:
four strims-sim jobs (arms decl and main, seeds 1 and 2) of golden_cli's
job dirs, each with the decision logs the study reads (arrivals, asks, and
main's haves) and parents, folded and exported by simdb.py into a local
suite invocation's source and indexed by the catalog:

  Prepare      the study's rows: the seed, the publishing interval, each
               asked row's path, supply, position and whether a rival but
               the seed declared it, the startup's rows not kept
  Sections     each section's numbers on the fixture's decisions, worked
               by hand: decompose's shares and rates, explained's
               leave-a-seed-out log loss, scores' pinball, WIS, coverage,
               PIT and D-calibration, baselines' points and skill,
               staleness' buckets, aligned's parent changes, selection's
               curse and regret (the seed among the rivals), bound's HAVE
               delay quantiles and main's outcomes
  Cli          simdb.py forecast-report's tables and JSON, as a caller runs
               it
  Hsmm         the forward-backward against enumerating every
               segmentation; a fit to two plain regimes finds them; dwell
               runs; lag classes
  Regimes      the fixture's sequences (lag class, pulls, trend), and the
               description's tables

  python3 hack/simlab/tests/test_forecast.py
"""

import json
import math
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
sys.path.insert(0, str(HERE))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import golden_cli  # noqa: E402
import simlab  # noqa: E402
from simlab import catalog, forecast, regimes, store_host  # noqa: E402

SRC = "suite/wt/fs"
ARMS = {"decl": "strims-sim-decl", "main": "strims-sim-main"}
NAMES = {label: arm for arm, label in ARMS.items()}
SEL = forecast.Selection(exp=[SRC], names=NAMES)
END_MS = 35500


def pub(q):
    """The fixture's publishing: a chunk every 100 ms from 25 s."""
    return 25000 + 100 * q


def arrivals():
    """Every node's holds: the seed (0) publishes chunks 0-99; host h of
    1-5 holds each 50 h ms after it by push, but host 3 never holds chunk 7,
    host 4 pulls the odd chunks (stream 1) 600 ms after, and host 5 holds
    chunk 25 at 300 ms."""
    rows = [(pub(q), 0, -1, q, "own", -1, -1, 0) for q in range(100)]
    for h in range(1, 6):
        for q in range(100):
            if (h, q) == (3, 7):
                continue
            t, how = pub(q) + 50 * h, "push"
            if h == 4 and q % 2:
                t, how = pub(q) + 600, "answer"
            if (h, q) == (5, 25):
                t = pub(q) + 300
            rows.append((t, h, h - 1, q, how, 0 if how == "push" else 2, 0, 0))
    return rows


ARRIVAL_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"), ("seq", "BIGINT"), ("how", "VARCHAR"),
                ("peer", "BIGINT"), ("parent", "BIGINT"), ("incarnation", "BIGINT")]
ASK_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("honest", "BOOLEAN"), ("ask", "BIGINT"), ("round", "VARCHAR"),
            ("peer", "BIGINT"), ("peer_honest", "BOOLEAN"), ("picked", "BOOLEAN"), ("declares", "BOOLEAN"),
            ("stream", "BIGINT"), ("seq", "BIGINT"), ("grace_ms", "BIGINT"), ("rtt_ms", "BIGINT"),
            ("predicted_ms", "BIGINT"), ("predicted_p90_ms", "BIGINT"), ("predicted_p99_ms", "BIGINT"),
            ("frontier_seq", "BIGINT"), ("hole_seq", "BIGINT"), ("state_age_ms", "BIGINT"), ("outcome", "VARCHAR"),
            ("reason", "VARCHAR"), ("t_outcome_ms", "BIGINT"), ("channel_open_ms", "BIGINT"), ("since_ms", "BIGINT"),
            ("peer_incarnation", "BIGINT")]


def ask(t, node, n, rnd, peer, picked, seq, grace, p, frontier, hole, age, outcome):
    p50, p90, p99 = p or (None, None, None)
    return (t, node, True, n, rnd, peer, True, picked, rnd == "flight", seq % 2, seq, grace if picked else None, 20,
            p50, p90, p99, frontier, hole, age, outcome if picked else None, None, t + 30 if picked else None, 0, 0, 0)


def decl_asks(seed):
    """The decisions (module docs: Sections): D0 a speculative ask of host
    2 for unpublished chunk 12 past its frontier, too early, beside host
    3; D1 a flight ask of host 4 for its declared hole 20, which only the
    seed (a rival) held; D2 one of host 5 for chunk 25 between its hole
    and frontier, answered at seed 1 and not at seed 2; D3 one in the
    startup; D4 one of host 3 for chunk 7 behind its hole, which it never
    held."""
    return [
        ask(26000, 1, 0, "spec", 2, True, 12, 100, (26250, 26400, 26600), 9, None, 40, "not_yet"),
        ask(26000, 1, 0, "spec", 3, False, 12, 100, (26350, 26500, 26700), 11, None, 60, None),
        ask(27000, 1, 1, "flight", 4, True, 20, 0, None, 25, 20, 100, "not_yet"),
        ask(27000, 1, 1, "flight", 2, False, 20, 0, (27100, 27150, 27200), 22, None, 30, None),
        ask(27000, 1, 1, "flight", 0, False, 20, 0, (27000, 27000, 27000), 30, None, 10, None),
        ask(28000, 1, 2, "flight", 5, True, 25, 0, (27700, 27800, 27900), 30, 22, 300,
            "answered" if seed == 1 else "not_yet"),
        ask(2000, 1, 3, "spec", 2, True, 3, 100, (2100, 2200, 2300), 1, None, 50, "not_yet"),
        ask(29000, 2, 4, "flight", 3, True, 7, 0, (25800, 25900, 26000), 40, 10, 600, "not_yet"),
    ]


MAIN_ASK_COLS = [c for c in ASK_COLS if c[0] not in ("grace_ms", "frontier_seq", "hole_seq", "state_age_ms")] + [
    ("timeout_ms", "BIGINT"), ("have_ms", "BIGINT"), ("released_ms", "BIGINT")]


def main_asks():
    """Main's two requests: host 2's answer of chunk 12, host 4's expiry
    of chunk 20."""
    rows = [(26000, 1, True, 0, "flight", 2, True, True, True, 0, 12, 20, None, None, None, "answered", None, 26340,
             0, 0, 0, 500, 26310, None),
            (26000, 1, True, 0, "flight", 3, True, False, True, 0, 12, 20, 26400, 26500, None, None, None, None,
             0, 0, 0, None, 26360, None),
            (27000, 1, True, 1, "flight", 4, True, True, True, 0, 20, 20, None, None, None, "expired", "timeout", 27500,
             0, 0, 0, 500, 27210, None)]
    return rows


HAVE_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("honest", "BOOLEAN"), ("peer", "BIGINT"),
             ("peer_honest", "BOOLEAN"), ("stream", "BIGINT"), ("seq", "BIGINT"), ("channel_open_ms", "BIGINT"),
             ("peer_incarnation", "BIGINT")]
DELAYS = (10, 20, 30, 40, 100)


def haves():
    """Host 1's HAVEs from host 2 of chunks 10-14, DELAYS after its holds."""
    return [(pub(q) + 100 + d, 1, True, 2, True, q % 2, q, 0, 0) for q, d in zip(range(10, 15), DELAYS)]


PARENT_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("stream", "BIGINT"), ("parent", "BIGINT"),
               ("reason", "VARCHAR")]
# Every host's push parent is the seed from the start; host 4 switches
# stream 0's at 26.5 s, host 3 stream 1's at 28.5 s.
PARENTS = sorted([(0, h, s, 0, "open") for h in range(1, 6) for s in (0, 1)]
                 + [(26500, 4, 0, 1, "switch"), (28500, 3, 1, 2, "switch")])


def build(root):
    """The fixture's cache under `root` (target/sim-db), indexed, and an
    empty git repository (its main checkout)."""
    sm = golden_cli.simdb_test().simdb
    os.environ["SIM_DB_ROOT"] = str(root / "target" / "sim-db")
    jobs = []
    for arm, label in ARMS.items():
        for seed in (1, 2):
            k = f"{label}:star-5:{seed}"
            d = root / "jobs" / f"{arm}-{seed}"
            job = (SRC, k, "strims-sim", "calibration_star", "star-5", label, seed, golden_cli.SS_ENV,
                   golden_cli.T0, None)
            rec = golden_cli.job_dir(d, job)
            golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, arrivals())
            golden_cli.write_table(d, "parents", PARENT_COLS, PARENTS)
            if arm == "decl":
                golden_cli.write_table(d, "asks", ASK_COLS, decl_asks(seed))
            else:
                golden_cli.write_table(d, "asks", MAIN_ASK_COLS, main_asks())
                golden_cli.write_table(d, "haves", HAVE_COLS, haves())
            jobs.append(rec)
    db = root / "store.duckdb"
    sm.fold(db, jobs, SRC)
    sm.export(db, root / "target" / "sim-db" / "cache" / SRC)
    db.unlink()
    subprocess.run(["git", "init", "-q", str(root)], check=True)


class Fixture(unittest.TestCase):
    """The fixture under a scratch root, indexed once; the main checkout
    is the root."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-forecast-")
        cls.root = Path(cls.tmp.name)
        saved = os.environ.get("SIM_DB_ROOT")
        try:
            build(cls.root)
            with cls.patched(cls):
                catalog.index(2, lambda e: None)
                cls.results = {s: r.records() for s, r in forecast.report(SEL).items()}
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


class Prepare(Fixture):
    def test_the_asked_rows_and_their_structure(self):
        with forecast.connected() as c:
            forecast.prepare(c, SEL)
            seed = c.execute("SELECT DISTINCT node FROM fs_seed").fetchall()
            rate = c.execute("SELECT DISTINCT ms_per_seq FROM fs_rate").fetchall()
            end = c.execute("SELECT DISTINCT end_ms FROM fs_jobs").fetchall()
            got = c.execute("SELECT seq, round, path, supply, position, elsewhere, kept, phase FROM fs_asks "
                            "WHERE arm = 'decl' AND seed = 1 ORDER BY t_ms").fetchall()
        self.assertEqual((seed, rate, end), ([(0,)], [(100.0,)], [(END_MS,)]))
        # D1's rival host 2 declares chunk 20 (its frontier 22, no hole);
        # the seed, the other rival, doesn't count; D0's rival host 3 is
        # behind it.
        no, yes = "not_elsewhere", "declared_elsewhere"
        self.assertEqual(got, [
            (3, "spec", "push", "unpublished", "past", no, False, "startup"),
            (12, "spec", "push", "unpublished", "past", no, True, "steady"),
            (20, "flight", "push", "seed_only", "hole", yes, True, "steady"),
            (25, "flight", "push", "spread", "declared", no, True, "steady"),
            (7, "flight", "never", "spread", "behind", no, True, "steady"),
        ])


class Sections(Fixture):
    def test_runs(self):
        # No run collapsed; each seed's kept asks are D0, D1, D2 and D4,
        # all but D2 at seed 1 failing.
        r = self.one("runs", arm="decl")
        self.assertEqual((r["collapsed"], r["runs"], r["seeds"], r["asks"], r["asks_per_run"]),
                         (False, 2, [1, 2], 8, [4.0, 4.0, 4.0]))
        self.assertEqual((r["fail_rate"], r["flight_share"], r["seed_only"], r["hole_behind"]),
                         (7 / 8, 6 / 8, 2 / 8, 4 / 8))
        with forecast.connected() as c:
            forecast.prepare(c, forecast.Selection(exp=[SRC], collapsed=True))
            self.assertEqual(c.execute("SELECT count(*) FROM fs_jobs").fetchall(), [(0,)])

    def test_decompose(self):
        hole = self.one("decompose", arm="decl", round="flight", factor="position", level="hole")
        self.assertEqual((hole["n"], hole["share"], hole["fail_rate"], hole["fail_share"]), (2, 2 / 6, 1.0, 2 / 5))
        declared = self.one("decompose", arm="decl", round="flight", factor="position", level="declared")
        self.assertEqual((declared["fail_rate"], declared["err_p50_ms"]), (0.5, 100.0))
        never = self.one("decompose", arm="decl", round="flight", factor="path", level="never")
        self.assertEqual((never["n"], never["held"], never["err_p50_ms"]), (2, 0, None))
        # The held rows' errors: D2's 100 ms twice, nothing else held with
        # a prediction in the flight round: no variance.
        self.assertIsNone(declared["eta2"])
        spec = self.one("decompose", arm="decl", round="spec", factor="supply", level="unpublished")
        self.assertEqual((spec["n"], spec["fail_rate"], spec["not_yet_rate"]), (2, 1.0, 1.0))

    def test_explained_leaves_a_seed_out(self):
        # The flight round's rows with a prediction: D2 (answered at seed 1,
        # not at 2) and D4 (failed at both).
        eps = forecast.EPS
        clip = lambda p: min(1 - eps, max(eps, p))  # noqa: E731
        # base: seed 1's rate from seed 2 (2 of 2), seed 2's from seed 1
        # (1 of 2), each class pulled toward it by a row.
        b1, b2 = 2 / 2, 1 / 2
        r1, r2 = clip((3 - 1 + b1) / (4 - 2 + 1)), clip((3 - 2 + b2) / (4 - 2 + 1))
        base = -(math.log(r1) + math.log(1 - r1) + 2 * math.log(r2)) / 4
        d1, d2 = clip((1 - 0 + b1) / 2), clip((1 - 1 + b2) / 2)
        h1, h2 = clip((2 - 1 + b1) / 2), clip((2 - 1 + b2) / 2)
        pos = -(math.log(1 - d1) + math.log(d2) + math.log(h1) + math.log(h2)) / 4
        got = {r["pred"]: r for r in self.rows("explained", arm="decl", round="flight")}
        self.assertAlmostEqual(got["base"]["log_loss"], base)
        self.assertAlmostEqual(got["position"]["log_loss"], pos)
        self.assertAlmostEqual(got["position"]["r2"], 1 - pos / base)
        self.assertEqual(got["base"]["fail_rate"], 3 / 4)
        self.assertIn("model", got)

    def test_scores(self):
        d0 = self.one("scores", arm="decl", round="spec", rows="asked", phase="steady")
        self.assertEqual(d0["n"], 2)
        self.assertAlmostEqual(d0["pinball_p50"], 50 * 0.5 / 1000)
        self.assertAlmostEqual(d0["pinball_p90"], 100 * 0.1 / 1000)
        self.assertAlmostEqual(d0["pinball_p99"], 300 * 0.01 / 1000)
        self.assertAlmostEqual(d0["wis"], 2 * (0.025 + 0.01 + 0.003) / 3)
        self.assertEqual((d0["cover_p50"], d0["cover_p90"], d0["cover_p99"]), (0.0, 1.0, 1.0))
        self.assertEqual([d0[f"pit_{i}"] for i in range(4)], [0.0, 1.0, 0.0, 0.0])
        self.assertAlmostEqual(d0["dcal"], 0.25 / 0.5 + 0.36 / 0.4 + 0.0081 / 0.09 + 0.0001 / 0.01)
        self.assertEqual((d0["err_p50_ms"], d0["spread_p90_ms"], d0["spread_p99_ms"], d0["no_spread"]),
                         (50.0, 150.0, 350.0, 0.0))
        # Every row: the rival (host 3) held it at its median.
        every = self.one("scores", arm="decl", round="spec", rows="every")
        self.assertEqual((every["n"], every["cover_p50"]), (4, 0.5))
        # A hold never seen is censored at the run's last hold: its loss a
        # lower bound.
        flight = self.one("scores", arm="decl", round="flight", rows="asked", phase="steady")
        self.assertEqual((flight["n"], flight["never"]), (4, 0.5))
        self.assertAlmostEqual(flight["pinball_p50"], ((27800 - 27700) * 0.5 + (END_MS - 25800) * 0.5) / 2000)

    def test_baselines(self):
        # D2's frontier 30 was held at 28250, published at 28000: both
        # forecasters put chunk 25 at 27750, 50 ms early, and their other
        # seed's residual moves them onto the hold.
        for base in ("persistence", "cadence"):
            r = self.one("baselines", arm="decl", round="flight", base=base)
            self.assertEqual((r["n"], r["pinball_p50"], r["err_p50_ms"]), (2, 0.0, 0.0))
            self.assertAlmostEqual(r["model_pinball_p50"], 100 * 0.5 / 1000)
            self.assertIsNone(r["skill_p50"])
            spec = self.one("baselines", arm="decl", round="spec", base=base)
            self.assertEqual(spec["err_p50_ms"], 0.0)

    def test_staleness(self):
        ages = {r["bucket"]: r["n"] for r in self.rows("staleness", arm="decl", round="flight", axis="state_age")}
        self.assertEqual(ages, {"03 >=100ms": 2, "04 >=250ms": 2, "05 >=500ms": 2})
        since = {r["bucket"]: r["fail_rate"]
                 for r in self.rows("staleness", arm="decl", round="flight", axis="since_parent_change")}
        self.assertEqual(since, {"01 >=0s": 1.0, "04 >=10s": 0.5})

    def test_aligned(self):
        # D1 and D4 came 0.5 s after their peers' parent changes; startup's
        # D3 counts here too, but no peer of its changed parents.
        got = self.rows("aligned", arm="decl", kind="parent_change")
        self.assertEqual([(r["off_s"], r["n"], r["fail_rate"]) for r in got], [(0, 4, 1.0)])

    def test_selection(self):
        spec = self.one("selection", arm="decl", round="spec")
        self.assertEqual((spec["asks"], spec["rivals"], spec["err_pick_ms"], spec["err_rivals_ms"], spec["curse_ms"],
                          spec["pick_missed"], spec["regret"]), (2, 1.0, 50.0, 0.0, 50.0, 1.0, 0.0))
        flight = self.one("selection", arm="decl", round="flight")
        self.assertEqual((flight["asks"], flight["rivals"], flight["err_pick_ms"], flight["pick_missed"],
                          flight["regret"], flight["regret_peer"], flight["regret_seed_only"]),
                         (2, 2.0, None, 1.0, 1.0, 0.0, 1.0))

    def test_bound(self):
        have = self.one("bound", arm="main", what="have delay")
        self.assertEqual((have["n"], have["q_ms"]), (10, [30.0, 100.0, 100.0]))
        asks = self.one("bound", arm="main", what="ask flight")
        self.assertEqual((asks["n"], asks["answered"], asks["expired"], asks["q_ms"]), (4, 0.5, 0.5, [340.0] * 3))
        flight = self.one("bound", arm="decl", what="ask flight")
        self.assertEqual((flight["n"], flight["answered"], flight["not_yet"]), (6, 1 / 6, 5 / 6))
        self.assertEqual((flight["failed"], flight["failed_seed_min"], flight["failed_seed_max"]), (5 / 6, 2 / 3, 1.0))
        self.assertEqual((asks["failed"], asks["failed_seed_min"], asks["failed_seed_max"]), (0.5, 0.5, 0.5))


class Cli(Fixture):
    def cli(self, *args):
        env = golden_cli.environment(self.root)
        p = subprocess.run([sys.executable, str(golden_cli.SIMDB), "forecast-report", *args], env=env,
                           capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stderr)
        return p.stdout

    def test_its_tables_and_json(self):
        names = [f"--name={arm}={label}" for arm, label in ARMS.items()]
        text = self.cli("--exp", SRC, *names, "--section", "decompose", "--section", "bound")
        self.assertTrue(text.startswith("## decompose\n"))
        self.assertIn("## bound\n", text)
        self.assertIn("| decl ", text)
        got = json.loads(self.cli("--exp", SRC, *names, "--section", "bound", "--json"))
        self.assertEqual(sorted(got), ["bound"])
        self.assertEqual(got["bound"], self.results["bound"])


class Hsmm(unittest.TestCase):
    def test_forward_backward_against_every_segmentation(self):
        rng = random.Random(3)
        m = regimes.init_model(2, 3, rng)
        m.dur = [regimes.normalized([rng.random() for _ in range(3)]) for _ in range(2)]
        m.a = [regimes.normalized([rng.random() for _ in range(2)]) for _ in range(2)]
        m.pi = regimes.normalized([rng.random(), rng.random()])
        seq = regimes.Sequence("k", "a", "c", 1, 1, 0, 0, [(0, 1, 0), (2, 0, 1), None, (1, 1, 2), (3, 0, 1)],
                               [0, 1, 0, 1, 1], ["steady"] * 5)
        e = regimes.emissions(m, seq)
        T, total, occ = len(seq.obs), 0.0, [[0.0, 0.0] for _ in seq.obs]

        def walk(t, segs):
            nonlocal total
            if t == T:
                p = m.pi[segs[0][0]]
                at = 0
                for i, (j, d) in enumerate(segs):
                    p *= m.a[segs[i - 1][0]][j] if i else 1
                    p *= sum(m.dur[j][d - 1:]) if i == len(segs) - 1 else m.dur[j][d - 1]
                    p *= math.exp(sum(e[s][j] for s in range(at, at + d)))
                    at += d
                total += p
                at = 0
                for j, d in segs:
                    for s in range(at, at + d):
                        occ[s][j] += p
                    at += d
                return
            for j in range(2):
                for d in range(1, 4):
                    if t + d <= T:
                        walk(t + d, segs + [(j, d)])

        walk(0, [])
        ll, gamma, _, _, _ = regimes.forward_backward(m, seq)
        self.assertAlmostEqual(ll, math.log(total), places=12)
        for g, o in zip(gamma, occ):
            for j in range(2):
                self.assertAlmostEqual(g[j], o[j] / total, places=12)

    def test_a_fit_finds_two_plain_regimes(self):
        rng = random.Random(1)
        seqs = []
        for i in range(30):
            obs, state = [], rng.random() < 0.5
            while len(obs) < 60:
                for _ in range(rng.randint(3, 8)):
                    obs.append((0, 0, 1) if state else (3, 1, 1))
                state = not state
            seqs.append(regimes.Sequence(f"k{i}", "a", "c", 1, 1, 0, 0, obs[:60], [0] * 60, ["steady"] * 60))
        m = regimes.fit(seqs, k=2, iters=10, d=10)
        self.assertEqual(sorted(m.names), ["on_path_fresh", "pulled_late"])
        steady = m.names.index("on_path_fresh")
        self.assertGreater(m.emit[steady][0][0][0], 0.95)
        self.assertGreater(m.emit[1 - steady][0][0][3], 0.95)
        self.assertGreater(m.ll[-1], m.ll[0])
        path = regimes.decode(m, seqs[0])
        want = [steady if o[0] == 0 else 1 - steady for o in seqs[0].obs]
        self.assertEqual(path, want)

    def test_a_fits_states_match_their_permutation(self):
        m = regimes.init_model(4, 5, random.Random(2))
        perm = [2, 0, 3, 1]
        other = regimes.Model([m.pi[i] for i in perm], m.a, m.dur, [m.emit[i] for i in perm])
        self.assertEqual(regimes.matched(m, other), {j: perm[j] for j in range(4)})
        self.assertEqual(regimes.distance(m, 2, other, 0), 0)

    def test_the_learning_curve(self):
        rng = random.Random(4)
        seqs = []
        for i in range(60):
            obs = [(rng.choice((0, 0, 1)), int(i % 3 == 0), 1) for _ in range(30)]
            seqs.append(regimes.Sequence(f"k{i}", "a", "c", 1 + i % 8, i, 0, 0, obs, [i % 2] * 30, ["steady"] * 30))
        small = regimes.subsample(seqs, 10)
        self.assertEqual(small, regimes.subsample(list(reversed(seqs)), 10))
        self.assertEqual(small, regimes.subsample(seqs, 20)[:10])
        m = regimes.fit(seqs[:5], iters=1)
        self.assertAlmostEqual(regimes.per_observation(m, seqs[:2]),
                               sum(regimes.loglik(m, s) for s in seqs[:2]) / 60)
        self.assertAlmostEqual(regimes.loglik(m, seqs[0]), regimes.forward_backward(m, seqs[0])[0])
        rows = regimes.learning_curve(seqs, [10, 40], iters=2, workers=2, everything=True)
        held = sum(s.seed in regimes.HELD_OUT_SEEDS for s in seqs)
        self.assertEqual([r["sequences"] for r in rows], [10, 40, 60])
        self.assertEqual(rows[1]["pulled_fresh"], 0)
        self.assertEqual((rows[2]["in_sample"], "both_halves" in rows[2]), (True, False))
        self.assertTrue(all(r["held_out_ll"] < 0 for r in rows))
        self.assertGreater(held, 0)

    def test_dwell_runs_and_lag_classes(self):
        self.assertEqual(regimes.runs_of([0, 0, 1, 1, 1, 0]), [(0, 0, 2, True), (1, 2, 3, False), (0, 5, 1, True)])
        self.assertEqual([regimes.lag_class(x) for x in (None, 0, 250, 251, 750, 2500, 2501)], [3, 0, 0, 1, 1, 2, 3])


class Regimes(Fixture):
    def test_the_fixtures_sequences(self):
        with forecast.connected() as c:
            forecast.prepare(c, SEL)
            seqs = {(s.arm, s.seed, s.peer, s.stream): s for s in regimes.sequences(c, None)}
        self.assertEqual(len(seqs), 2 * 2 * 5 * 2)
        pulled = seqs[("decl", 1, 4, 1)]
        self.assertEqual((pulled.t0, len(pulled.obs)), (25, 10))
        self.assertEqual(set(pulled.obs), {(1, 1, 1)})
        never = seqs[("decl", 1, 3, 1)]
        self.assertEqual(never.obs[:2], [(3, 0, 1), (0, 0, 0)])
        self.assertEqual(seqs[("decl", 1, 5, 1)].obs[2:4], [(1, 0, 2), (0, 0, 0)])
        self.assertEqual(set(never.swarm), {0})
        self.assertEqual(set(never.phase), {"steady"})

    def test_the_description(self):
        tables, model = regimes.regimes(SEL, sample=1, decode_sample=2, iters=2)
        self.assertEqual(set(tables), {"states", "occupancy", "dwell", "transitions", "events", "swarm", "stability",
                                       "seed_spread", "availability", "by_receiver"})
        self.assertEqual(len(model.names), regimes.K)
        self.assertEqual({r["state"] for r in tables["states"]}, set(regimes.STATE_ORDER))
        occ = [r for r in tables["occupancy"] if r["arm"] == "decl"]
        self.assertEqual({r["phase"] for r in occ}, {"rest"})
        self.assertAlmostEqual(sum(occ[0][n] for n in regimes.STATE_ORDER), 1.0)
        self.assertTrue(any(r["pred"] == "state" for r in tables["availability"]))


if __name__ == "__main__":
    unittest.main()
