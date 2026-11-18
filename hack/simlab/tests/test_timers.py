#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The timer bench's macros (scoring.sql's side_window_of,
timer_runs_of, timer_shares_of, timer_fit_of, timer_fit_spread_of,
alloc_steps_of and alloc_step_fit_of) over
a fixture of jobs, hosts, timers and seed decisions with known answers:
the shares of whole ms late by the hosts table's kind and by source, each
run weighed alike, over each run's window; a timer armed for a deadline
already past left out, one armed for now kept, a negative lateness read
as 0 ms, one with no lateness counted missing; the testbed's repeat
spread of each run's mean and p99 (sketch_quantile's) in a cell; the
fit's total-variation distance, and its mean and p99 checks within the
spread and a floor; a source the model keeps no rows of expected and any
other one-sided row failing; an empty or NULL side refused; and the
seed's allocation-check steps, on time or a tick late, with their fit
within the larger of 5 points and twice the difference's standard
error.

  python3 hack/simlab/tests/test_timers.py
"""

import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import catalog  # noqa: E402

MACROS = ("twin_window_of", "side_window_of", "timer_runs_of", "timer_shares_of", "timer_fit_of",
          "timer_fit_spread_of", "alloc_steps_of", "alloc_step_fit_of", "sketch_exp", "sketch_lo", "sketch_floor", "sketch_quantile")
S = 1_000_000


def connect():
    """An in-memory DuckDB with the macros and the fixture: a testbed run
    (k `tb`) of a Chromium seed (node 0) and a native viewer (node 1), its
    repeat (`tb3`, the same cell), a testbed run (`tb2`) of a Firefox seed,
    and a twin (`tw`) of the first; and for the steps a testbed run and a
    twin of 40 steps each (`tb4`, `tw2`); every stream 60 s."""
    import duckdb

    conn = duckdb.connect()
    conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, env MAP(VARCHAR, VARCHAR), stored_cell VARCHAR)")
    for k, src, cell in [("tb", "testbed/b", "cA"), ("tb3", "testbed/b", "cA"), ("tb2", "testbed/c", "cB"),
                         ("tw", "testbed/a", "tA"), ("tb4", "testbed/d", "cD"), ("tw2", "testbed/e", "tD"),
                         ("tw3", "testbed/f", "tD")]:
        conn.execute("INSERT INTO jobs VALUES (?, ?, MAP {'STRIMS_SIM_CAL_SECS': '60'}, ?)", [k, src, cell])
    conn.execute("CREATE TABLE hosts (k VARCHAR, src VARCHAR, node BIGINT, kind VARCHAR)")
    conn.executemany("INSERT INTO hosts VALUES (?, ?, ?, ?)", [
        ("tb", "testbed/b", 0, "chromium"), ("tb", "testbed/b", 1, "native"),
        ("tb2", "testbed/c", 0, "firefox"), ("tb2", "testbed/c", 1, "native"),
        ("tw", "testbed/a", 0, "chromium"), ("tw", "testbed/a", 1, "native"), ("tw", "testbed/a", 2, "native"),
        ("tb3", "testbed/b", 0, "chromium"), ("tb3", "testbed/b", 1, "native"),
        ("tb4", "testbed/d", 0, "chromium"), ("tw2", "testbed/e", 0, "chromium"), ("tw3", "testbed/f", 0, "chromium")])
    conn.execute("CREATE TABLE timers (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, n BIGINT, "
                 "source VARCHAR, armed_us BIGINT, deadline_us BIGINT, late_us BIGINT, requested_us BIGINT, "
                 "context VARCHAR)")
    rows = []

    def add(k, src, node, source, lates, requested=1000, t=10 * S):
        for i, late in enumerate(lates):
            rows.append((k, src, t + i * 10_000, node, len(rows), source, 0, 0, late, requested, "task"))

    # The testbed's Chromium seed: 0 ms twice (-1500 reads as 0, and one
    # armed for now, at 0 ms), 1 ms and 3+ ms (4 ms), and one with no
    # lateness, missing; one armed for a past deadline, left out; and one
    # before the window and one at its end, left out.
    add("tb", "testbed/b", 0, "node_task", [-1500, 1500, 4000])
    add("tb", "testbed/b", 0, "node_task", [300], requested=0, t=11 * S)
    add("tb", "testbed/b", 0, "node_task", [None], t=12 * S)
    add("tb", "testbed/b", 0, "node_task", [9000], requested=-1000, t=13 * S)
    add("tb", "testbed/b", 0, "node_task", [9000], t=10 * S - 10_000)
    add("tb", "testbed/b", 0, "node_task", [9000], t=60 * S)
    # Its native viewer: 2 ms twice; its record timer, the model's none.
    add("tb", "testbed/b", 1, "node_task", [2000, 2999])
    add("tb", "testbed/b", 0, "record", [2000])
    # The Firefox run: 1 ms, once a 4-ms delay; its native viewer 0 ms
    # eight times, a cell of more timers weighed as one run.
    add("tb2", "testbed/c", 0, "node_task", [1000])
    add("tb2", "testbed/c", 0, "node_task", [1200], requested=4000)
    add("tb2", "testbed/c", 1, "node_task", [0] * 8)
    # The twin's seed: 0, 0, 1 and 2 ms; its two viewers 2 ms, three
    # times; and a source the testbed lacks.
    add("tw", "testbed/a", 0, "node_task", [0, 0, 1000, 2000])
    add("tw", "testbed/a", 1, "node_task", [2000, 2000])
    add("tw", "testbed/a", 2, "node_task", [2000])
    add("tw", "testbed/a", 1, "writer", [0])
    # The first run's repeat: its seed 0, 1, 1 and 2 ms (mean 1, p99 2);
    # its viewer 2 ms twice.
    add("tb3", "testbed/b", 0, "node_task", [0, 1000, 1000, 2000])
    add("tb3", "testbed/b", 1, "node_task", [2000, 2000])
    conn.executemany("INSERT INTO timers VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", rows)
    conn.execute("CREATE TABLE seed_decisions (k VARCHAR, src VARCHAR, t_ms BIGINT, node BIGINT, n BIGINT, "
                 "kind VARCHAR, why VARCHAR, stream BIGINT)")
    decisions = []

    def checks(k, src, node, times):
        for t in times:
            decisions.append((k, src, t, node, len(decisions), "window", "check", 0))

    # The testbed's seed: steps that end from 10 s of 1000, 1000, 1099,
    # 1001, 1100 and 3000 ms (a check that didn't run), one that ends
    # before; a first-wave window and a holder's check row, which aren't
    # checks.
    checks("tb", "testbed/b", 0, [8_000, 9_500, 10_500, 11_500, 12_599, 13_600, 14_700, 17_700])
    decisions.append(("tb", "testbed/b", 12_000, 0, len(decisions), "window", "first_wave", 0))
    decisions.append(("tb", "testbed/b", 12_100, 0, len(decisions), "check", None, 0))
    # The twin's: every step 1000 ms.
    checks("tw", "testbed/a", 0, [10_000, 11_000, 12_000, 13_000, 14_000])
    # 40 steps: the testbed's half slipped, the twin's none.
    checks("tb4", "testbed/d", 0, [10_000 + sum(1100 if i % 2 else 1000 for i in range(j)) for j in range(41)])
    checks("tw2", "testbed/e", 0, [10_000 + 1000 * j for j in range(41)])
    # A twin's 40 of which every fourth slipped.
    checks("tw3", "testbed/f", 0, [10_000 + sum(1100 if i % 4 == 3 else 1000 for i in range(j)) for j in range(41)])
    conn.executemany("INSERT INTO seed_decisions VALUES (?, ?, ?, ?, ?, ?, ?, ?)", decisions)
    for s in catalog.scoring_statements(MACROS, {"timers", "hosts", "seed_decisions", "jobs.env",
                                                 "jobs.stored_cell"}):
        conn.execute(s)
    return conn


class Timers(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.conn = connect()

    def q(self, sql, *args):
        return self.conn.execute(sql, args).fetchall()

    def test_the_macros_load(self):
        names = {r[0] for r in self.q("SELECT DISTINCT function_name FROM duckdb_functions() "
                                       "WHERE function_name IN ?", list(MACROS))}
        self.assertEqual(names, set(MACROS))

    def test_shares_are_by_whole_ms_kind_and_source(self):
        got = self.q("SELECT kind, source, runs, nodes, timers, missing, late_ms0, late_ms1, late_ms2, late_ms3, "
                     "round(mean_ms, 6) FROM timer_shares_of([{'k': 'tb'}]) ORDER BY kind, source")
        self.assertEqual(got, [("chromium", "node_task", 1, 1, 4, 1, 0.5, 0.25, 0.0, 0.25, 1.45),
                               ("chromium", "record", 1, 1, 1, 0, 0.0, 0.0, 1.0, 0.0, 2.0),
                               ("native", "node_task", 1, 1, 2, 0, 0.0, 0.0, 1.0, 0.0, 2.4995)])

    def test_each_run_weighs_alike(self):
        # The native viewers: 2 ms twice in one run, 0 ms eight times in
        # the other: half each, not a fifth.
        got = self.q("SELECT runs, nodes, timers, late_ms0, late_ms2, round(mean_ms, 6) "
                     "FROM timer_shares_of([{'k': 'tb'}, {'k': 'tb2'}]) WHERE kind = 'native'")
        self.assertEqual(got, [(2, 2, 10, 0.5, 0.5, 1.24975)])
        got = self.q("SELECT runs, nodes, timers FROM timer_shares_of([{'k': 'tw'}]) "
                     "WHERE kind = 'native' AND source = 'node_task'")
        self.assertEqual(got, [(1, 2, 3)])
        # The p99 is sketch_quantile's: the nearest rank, 2999 µs, at its
        # bucket's lower bound, 2992.
        got = self.q("SELECT p99_ms FROM timer_shares_of([{'k': 'tb'}]) WHERE kind = 'native'")
        self.assertEqual(got, [(2.992,)])

    def test_a_cells_repeats_spread_its_mean_and_p99(self):
        # The Chromium seed's means 1.45 and 1 ms, p99s 4 and 2, in one
        # cell; one run alone has none.
        got = self.q("SELECT runs, round(mean_ms, 6), p99_ms, round(mean_spread_ms, 6), p99_spread_ms "
                     "FROM timer_shares_of([{'k': 'tb'}, {'k': 'tb3'}]) "
                     "WHERE kind = 'chromium' AND source = 'node_task'")
        self.assertEqual(got, [(2, 1.225, 3.0, 0.45, 2.0)])
        got = self.q("SELECT mean_spread_ms, p99_spread_ms FROM timer_shares_of([{'k': 'tb'}, {'k': 'tb2'}]) "
                     "WHERE kind = 'native' AND source = 'node_task'")
        self.assertEqual(got, [(None, None)])
        got = self.q("SELECT k, cell, round(mean_ms, 6), p99_ms FROM timer_runs_of([{'k': 'tb'}, {'k': 'tb3'}]) "
                     "WHERE kind = 'chromium' AND source = 'node_task' ORDER BY k")
        self.assertEqual(got, [("tb", "cA", 1.45, 4.0), ("tb3", "cA", 1.0, 2.0)])

    def test_a_window_and_delays_split_them(self):
        got = self.q("SELECT kind, timers FROM timer_shares_of([{'k': 'tb'}], hi_s := 10.015) "
                     "WHERE source = 'node_task' ORDER BY kind")
        self.assertEqual(got, [("chromium", 2), ("native", 2)])
        got = self.q("SELECT delay_ms, timers FROM timer_shares_of([{'k': 'tb2'}], by_delay := true) "
                     "WHERE kind = 'firefox' ORDER BY delay_ms")
        self.assertEqual(got, [(1, 1), (4, 1)])

    def test_an_empty_side_is_refused(self):
        # A literal [], an empty list, and a src that names no job.
        nobody = "(SELECT list({'k': k}) FROM jobs WHERE src = 'testbed/typo')"
        for side in ["[]", "[{'k': 'none'}][:0]", nobody]:
            with self.assertRaisesRegex(Exception, "timer_runs_of: a side of no runs"):
                self.q(f"FROM timer_shares_of({side})")
            with self.assertRaisesRegex(Exception, "alloc_steps_of: a side of no runs"):
                self.q(f"FROM alloc_steps_of({side})")

    def test_the_fit_is_the_shares_total_variation_and_moments(self):
        got = self.q("SELECT kind, source, a_timers, b_timers, a_ms0, b_ms0, a_ms2, b_ms2, tv, "
                     "round(mean_diff_ms, 6), round(p99_diff_ms, 6), pass "
                     "FROM timer_fit_of([{'k': 'tw'}], [{'k': 'tb'}]) ORDER BY kind, source")
        # Chromium's node task: [0.5, 0.25, 0.25, 0] against [0.5, 0.25,
        # 0, 0.25], means 0.75 and 1.45 ms, p99s 2 and 4; the native
        # viewer's 2 ms on both sides, means 2 and 2.4995 ms, p99s 2 and
        # 2.992.
        self.assertEqual(got, [
            ("chromium", "node_task", 4, 4, 0.5, 0.5, 0.25, 0.0, 0.25, 0.7, 2.0, False),
            ("chromium", "record", None, 1, None, 0.0, None, 1.0, None, None, None, None),
            ("native", "node_task", 3, 2, 0.0, 0.0, 1.0, 1.0, 0.0, 0.4995, 0.992, False),
            ("native", "writer", 1, None, 1.0, None, 0.0, None, None, None, None, False)])
        # One testbed run has no spread: the bounds are the floors.
        got = self.q("SELECT kind, mean_bound, p99_bound FROM timer_fit_of([{'k': 'tw'}], [{'k': 'tb'}]) "
                     "WHERE source = 'node_task' ORDER BY kind")
        self.assertEqual(got, [("chromium", 0.1, 1.0), ("native", 0.1, 1.0)])
        # Each check within its bound passes, and each past it fails.
        fit = ("SELECT kind, pass FROM timer_fit_of([{'k': 'tw'}], [{'k': 'tb'}], bound := ?, mean_floor := ?, "
               "p99_floor := ?) WHERE source = 'node_task' ORDER BY kind")
        self.assertEqual(self.q(fit, 0.25, 0.7, 2.0), [("chromium", True), ("native", True)])
        self.assertEqual(self.q(fit, 0.24, 0.7, 2.0), [("chromium", False), ("native", True)])
        self.assertEqual(self.q(fit, 0.25, 0.69, 2.0), [("chromium", False), ("native", True)])
        self.assertEqual(self.q(fit, 0.25, 0.7, 1.99), [("chromium", False), ("native", True)])
        self.assertEqual(self.q(fit, 0.25, 0.49, 2.0), [("chromium", False), ("native", False)])
        self.assertEqual(self.q(fit, 0.25, 0.7, 0.99), [("chromium", False), ("native", False)])
        # The testbed's repeats widen the bounds by their spread: 0.45 and
        # 2 ms past the floors, so a mean 0.475 off and a p99 1 off pass.
        got = self.q("SELECT round(mean_bound, 6), p99_bound, round(mean_diff_ms, 6), p99_diff_ms, "
                     "abs(mean_diff_ms) <= mean_bound AND abs(p99_diff_ms) <= p99_bound "
                     "FROM timer_fit_of([{'k': 'tw'}], [{'k': 'tb'}, {'k': 'tb3'}]) "
                     "WHERE kind = 'chromium' AND source = 'node_task'")
        self.assertEqual(got, [(0.55, 3.0, 0.475, 1.0, True)])
        # A held-out score: one testbed run, the spread its cell's repeats'.
        got = self.q("SELECT round(mean_bound, 6), p99_bound FROM timer_fit_spread_of([{'k': 'tw'}], [{'k': 'tb'}], "
                     "[{'k': 'tb'}, {'k': 'tb3'}]) WHERE kind = 'chromium' AND source = 'node_task'")
        self.assertEqual(got, [(0.55, 3.0)])
        # 0 ms against 2 ms: every share's difference counts.
        got = self.q("SELECT tv FROM timer_fit_of([{'k': 'tw'}], [{'k': 'tb2'}]) "
                     "WHERE kind = 'native' AND source = 'node_task'")
        self.assertEqual(got, [(1.0,)])

    def test_unmodelled_sources_are_named(self):
        got = self.q("SELECT source, pass FROM timer_fit_of([{'k': 'tw'}], [{'k': 'tb'}], unmodelled := 'pacer') "
                     "WHERE b_runs IS NULL OR a_runs IS NULL ORDER BY source")
        self.assertEqual(got, [("record", False), ("writer", False)])

    def test_delays_pair_by_delay(self):
        got = self.q("SELECT delay_ms, a_timers, b_timers FROM timer_fit_of([{'k': 'tb2'}], [{'k': 'tb2'}], "
                     "by_delay := true) WHERE kind = 'firefox' ORDER BY delay_ms")
        self.assertEqual(got, [(1, 1, 1), (4, 1, 1)])

    def test_the_seeds_checks_step_on_time_or_a_tick_late(self):
        got = self.q("SELECT kind, runs, on_time, slipped, others, slipped_share FROM alloc_steps_of([{'k': 'tb'}])")
        self.assertEqual(got, [("chromium", 1, 3, 2, 1, 0.4)])
        # 0 of 4 against 2 of 5: se sqrt(0.4 0.6 / 5), 0.219; 0.4 is within
        # twice it, so few steps pass.
        got = self.q("SELECT kind, a_steps, b_steps, a_share, b_share, diff, round(se, 4), round(lo, 4), "
                     "round(hi, 4), round(allowance, 4), pass FROM alloc_step_fit_of([{'k': 'tw'}], [{'k': 'tb'}])")
        self.assertEqual(got, [("chromium", 4, 5, 0.0, 0.4, 0.4, 0.2191, -0.0382, 0.8382, 0.4382, True)])
        # 0 of 40 against 20 of 40: se 0.079; 0.5 is past twice it, and
        # past a bound of 0.4, within one of 0.5.
        got = self.q("SELECT a_steps, b_steps, diff, round(se, 4), round(allowance, 4), pass "
                     "FROM alloc_step_fit_of([{'k': 'tw2'}], [{'k': 'tb4'}])")
        self.assertEqual(got, [(40, 40, 0.5, 0.0791, 0.1581, False)])
        fit = "SELECT pass FROM alloc_step_fit_of([{'k': 'tw2'}], [{'k': 'tb4'}], bound := ?)"
        self.assertEqual(self.q(fit, 0.4), [(False,)])
        self.assertEqual(self.q(fit, 0.5), [(True,)])
        # Both sides' variance counts: 10 of 40 against 20 of 40, se
        # sqrt(0.25 0.75 / 40 + 0.25 / 40).
        got = self.q("SELECT a_share, b_share, round(se, 4) FROM alloc_step_fit_of([{'k': 'tw3'}], [{'k': 'tb4'}])")
        self.assertEqual(got, [(0.25, 0.5, 0.1046)])
        # A side's share is its slipped over its steps, pooled: 2 of 5
        # and 20 of 40 are 22 of 45.
        got = self.q("SELECT on_time, slipped, round(slipped_share, 4) FROM alloc_steps_of([{'k': 'tb'}, {'k': 'tb4'}])")
        self.assertEqual(got, [(23, 22, 0.4889)])
        # A kind one side lacks fails.
        got = self.q("SELECT kind, pass FROM alloc_step_fit_of([{'k': 'tw'}], [{'k': 'tb2'}])")
        self.assertEqual(got, [("chromium", False)])


if __name__ == "__main__":
    unittest.main()
