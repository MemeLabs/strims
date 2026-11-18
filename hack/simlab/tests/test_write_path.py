#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The write path bench's macros (scoring.sql's write_stages_of and
write_fit_of) over a fixture of two testbed runs and a twin with known
answers: each stage's p50 and p90 at the seed over each run's window, a
write's read found by its bytes, the testbed's first packet less its
egress delay, and the pass: within the testbed's runs' range widened by
the floor, the holds' and lows' counts within it widened by counting
noise, a stage one side lacks failing, and a side of no runs refused;
and over a twin's packets that share their µs, a write's first packet
the least n of them, whatever the packets table's row order.

  python3 hack/simlab/tests/test_write_path.py
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

MACROS = ("sketch_exp", "sketch_lo", "sketch_floor", "sketch_quantile", "twin_window_of", "side_window_of",
          "write_stages_of", "write_fit_of")
HAVE = {"link_writes", "link_reads", "link_samples", "seed_sends", "packets", "hosts", "jobs.sim", "jobs.env"}
T = 20_000_000
TB, TW = [{"k": "tb1"}, {"k": "tb2"}], [{"k": "tw"}]


def connect():
    """An in-memory DuckDB with the macros, and the fixture: testbed runs
    `tb1` and `tb2` and a twin `tw` of 60 s streams, each a seed (node 0)
    writing to one viewer (node 1) on link 0."""
    import duckdb

    conn = duckdb.connect()
    conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, sim VARCHAR, env MAP(VARCHAR, VARCHAR))")
    for k, src, sim in (("tb1", "testbed/b", "testbed"), ("tb2", "testbed/b", "testbed"),
                        ("tw", "testbed/b-twin", "strims-sim")):
        conn.execute("INSERT INTO jobs VALUES (?, ?, ?, MAP {'STRIMS_SIM_CAL_SECS': '60'})", (k, src, sim))
    conn.execute("CREATE TABLE link_writes (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                 "link BIGINT, kind VARCHAR, off BIGINT, n BIGINT, bytes BIGINT, queued_us BIGINT, "
                 "done_us BIGINT, unsent_before BIGINT, wait VARCHAR)")
    conn.execute("CREATE TABLE link_reads (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                 "link BIGINT, kind VARCHAR, off BIGINT, n BIGINT, bytes BIGINT, taken_us BIGINT)")
    conn.execute("CREATE TABLE link_samples (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                 "link BIGINT, source VARCHAR, cause VARCHAR, buffered_bytes BIGINT)")
    conn.execute("CREATE TABLE seed_sends (k VARCHAR, src VARCHAR, t_ms BIGINT, node BIGINT, peer BIGINT, "
                 "how VARCHAR, fate VARCHAR, queued_ms BIGINT)")
    conn.execute("CREATE TABLE packets (k VARCHAR, src VARCHAR, node BIGINT, n BIGINT, t_us BIGINT, "
                 "arrived_us BIGINT, lost VARCHAR, peer BIGINT, kind VARCHAR)")
    conn.execute("CREATE TABLE hosts (k VARCHAR, src VARCHAR, node BIGINT, delay_us BIGINT)")
    for k, src in (("tb1", "testbed/b"), ("tb2", "testbed/b"), ("tw", "testbed/b-twin")):
        tb = src == "testbed/b"
        held = 50_000 if k == "tb1" else 70_000 if k == "tb2" else 60_000
        # 20 s in: a write into an empty channel, unheld (1 ms), read 30 ms
        # after; then one held for its cap, read 10 ms after, then one by
        # the host, read 4 ms after; and a write past the stream's end.
        writes = [
            (T, 0, 1000, T - 500, T + 1000, 0, "none"),
            (T + 10_000, 1000, 2000, T + 9_000, T + 10_000 + held, 30_000, "channel"),
            (T + 100_000, 3000, 500, T + 100_000, T + 106_000, 4000, "host"),
            (61_000_000, 3500, 100, 61_000_000, 61_000_100, 0, "none"),
        ]
        for n, (b, off, nb, q, d, before, wait) in enumerate(writes):
            conn.execute("INSERT INTO link_writes VALUES (?, ?, ?, 0, 1, 0, 'webrtc', ?, ?, ?, ?, ?, ?, ?)",
                         (k, src, b, off, n, nb, q, d, before, wait))
        # The viewer's reads: the first write's bytes in two messages (the
        # second holds its last byte), then each other write's whole.
        reads = [(T + 20_000, 0, 600), (T + 31_000, 600, 400), (T + 20_000 + held, 1000, 2000),
                 (T + 110_000, 3000, 500), (61_000_200, 3500, 100)]
        for n, (at, off, nb) in enumerate(reads):
            conn.execute("INSERT INTO link_reads VALUES (?, ?, ?, 1, 0, 0, 'webrtc', ?, ?, ?, ?)",
                         (k, src, at, off, n, nb, at + 2_000))
        # Its lows: 0 and 24 kB (the twin's 20 kB); one before the window.
        lows = [0, 24_000] if tb else [0, 20_000]
        for i, b in enumerate(lows):
            conn.execute("INSERT INTO link_samples VALUES (?, ?, ?, 0, 1, 0, 'webrtc', 'low', ?)",
                         (k, src, T + 60_000 + i, b))
        conn.execute("INSERT INTO link_samples VALUES (?, ?, 1000, 0, 1, 0, 'webrtc', 'low', 7)", (k, src))
        conn.execute("INSERT INTO seed_sends VALUES (?, ?, 20000, 0, 1, 'push', 'written', 19990)", (k, src))
        # The first data packet after the first write's done: offered at
        # +3 ms on the testbed (its t_us its capture's plus the 1 ms egress
        # delay), at +2 ms in the twin; it arrives 20 ms after.
        offered = T + 1000 + (3_000 if tb else 2_000)
        conn.execute("INSERT INTO packets VALUES (?, ?, 0, 0, ?, ?, NULL, 1, 'data')",
                     (k, src, offered + (1000 if tb else 0), offered + 20_000))
        conn.execute("INSERT INTO hosts VALUES (?, ?, 0, ?)", (k, src, 1000 if tb else 0))
    for s in catalog.scoring_statements(MACROS, HAVE):
        conn.execute(s)
    return conn


class WritePath(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.conn = connect()

    def q(self, sql, *args):
        return self.conn.execute(sql, args).fetchall()

    def stages(self, k):
        return {s: (n, p50, p90) for s, n, p50, p90 in self.q(
            "SELECT stage, n, p50, p90 FROM write_stages_of([{'k': ?}]) ORDER BY stage", k)}

    def fit(self, stage, a=TW, b=TB):
        return self.q("SELECT a_n, pass50, pass90, pass_n, pass FROM write_fit_of(?, ?) WHERE stage = ?",
                      a, b, stage)

    def counts(self, k, stage, n):
        """Gives run k n rows of a counted stage, at the times its first
        one has."""
        if stage == "6 low reading":
            self.conn.execute("INSERT INTO link_samples SELECT k, src, t_us, node, peer, link, source, cause, "
                              "buffered_bytes FROM link_samples, range(?) WHERE k = ? AND cause = 'low' "
                              "AND t_us = ?", (n - 2, k, T + 60_000))
            return
        wait = {"3 held: channel": "channel", "4 held: host": "host"}[stage]
        self.conn.execute("INSERT INTO link_writes SELECT x.* REPLACE (x.off + 10_000 * (r.range + 1) AS off) "
                          "FROM link_writes x, range(?) r WHERE x.k = ? AND x.wait = ?", (n - 1, k, wait))

    def test_the_macros_load(self):
        names = {r[0] for r in self.q("SELECT DISTINCT function_name FROM duckdb_functions() "
                                       "WHERE function_name IN ('write_stages_of', 'write_fit_of')")}
        self.assertEqual(names, {"write_stages_of", "write_fit_of"})

    def test_a_runs_stages(self):
        got = self.stages("tb1")
        self.assertEqual(got["1 push: runner"], (1, 10, 10))
        self.assertFalse({"1a", "1b", "1c"} & {s.split(" ")[0] for s in got})
        # The window's writes alone: 20 s to the stream's end (60 s).
        self.assertEqual(got["2 driver queue"], (3, 0.5, 1))
        self.assertEqual(got["3 held: channel"], (1, 50, 50))
        self.assertEqual(got["4 held: host"], (1, 6, 6))
        self.assertEqual(got["5 unheld"], (1, 1, 1))
        self.assertEqual(got["6 low reading"][0], 2)
        # A write's read is the one holding its last byte.
        self.assertEqual(got["7 done to read"], (3, 10, 30))
        # The testbed's first packet less its egress delay.
        self.assertEqual(got["7a empty: to first packet"], (1, 3, 3))
        self.assertEqual(got["7b empty: packet to arrival"], (1, 20, 20))
        self.assertEqual(got["7c empty: arrival to read"], (1, 7, 7))
        self.assertEqual(got["8 read to take"], (3, 2, 2))
        self.assertEqual(self.stages("tw")["7a empty: to first packet"], (1, 2, 2))
        # A window of its own.
        got = self.q("SELECT n FROM write_stages_of([{'k': 'tb1'}], lo_s := 20, hi_s := 20.05) "
                     "WHERE stage = '2 driver queue'")
        self.assertEqual(got, [(2,)])

    def test_the_twin_passes_within_the_testbeds_range(self):
        got = {r[0]: r[1:] for r in self.q(
            "SELECT stage, b_p50s, a_p50, pass50, pass90, b_ns, a_n, pass_n, pass FROM write_fit_of(?, ?)", TW, TB)}
        # 60 ms between the testbed's 50 and 70, as many holds.
        self.assertEqual(got["3 held: channel"], ([50, 70], 60, True, True, [1, 1], 1, True, True))
        # 2 ms against 3 and 3, widened by 0.2: no pass.
        self.assertEqual(got["7a empty: to first packet"][2:4], (False, False))
        self.assertFalse(got["7a empty: to first packet"][-1])
        # The twin's lows read 0 and 20 kB against 0 and 24 kB: its p90
        # fails with no floor for bytes, and passes with one.
        self.assertEqual(got["6 low reading"][3], False)
        floored = self.q("SELECT pass90 FROM write_fit_of(?, ?, floor_bytes := 4096) "
                         "WHERE stage = '6 low reading'", TW, TB)
        self.assertEqual(floored, [(True,)])

    def test_the_holds_and_lows_count_within_the_repeats_and_counting_noise(self):
        # The testbed's runs 100 and 110 a stage: the twin's count passes
        # within 100 - 2√100 = 80 and 110 + 2√110 ≈ 131, and fails past
        # them, its times as the testbed's.
        for stage in ("3 held: channel", "4 held: host", "6 low reading"):
            for twin, want in ((125, True), (85, True), (135, False), (75, False)):
                self.conn.execute("BEGIN")
                try:
                    self.counts("tb1", stage, 100)
                    self.counts("tb2", stage, 110)
                    self.counts("tw", stage, twin)
                    got = self.fit(stage)
                    floored = self.q("SELECT pass_n FROM write_fit_of(?, ?, floor_bytes := 4096) WHERE stage = ?",
                                     TW, TB, stage)
                    self.assertEqual(got[0][0], twin, stage)
                    self.assertEqual(floored, [(want,)], (stage, twin))
                    if stage != "6 low reading":
                        self.assertEqual(got[0][1:], (True, True, want, want), (stage, twin))
                finally:
                    self.conn.execute("ROLLBACK")

    def test_a_stage_one_side_lacks_fails(self):
        self.conn.execute("BEGIN")
        try:
            self.conn.execute("DELETE FROM link_writes WHERE k = 'tw' AND wait = 'host'")
            got = self.q("SELECT a_runs, b_runs, pass FROM write_fit_of(?, ?) WHERE stage = '4 held: host'", TW, TB)
            self.assertEqual(got, [(None, 2, False)])
            self.conn.execute("DELETE FROM link_writes WHERE k IN ('tb1', 'tb2') AND wait = 'channel'")
            got = self.q("SELECT a_runs, b_runs, pass FROM write_fit_of(?, ?) WHERE stage = '3 held: channel'",
                         TW, TB)
            self.assertEqual(got, [(1, None, False)])
        finally:
            self.conn.execute("ROLLBACK")

    def refused(self, *statements, cols="pass", a=TW, b=TB):
        """write_fit_of's error with `statements` applied, then undone."""
        import duckdb

        self.conn.execute("BEGIN")
        try:
            for s in statements:
                self.conn.execute(s)
            with self.assertRaises(duckdb.Error) as e:
                self.q(f"SELECT {cols} FROM write_fit_of(?, ?)", a, b)
            return str(e.exception)
        finally:
            self.conn.execute("ROLLBACK")

    def test_a_model_side_without_a_stages_table_is_refused(self):
        # A twin without the packet log has no side of the first packet's
        # stages: the fit names the table, the knob, the run and the
        # stages, whatever columns are asked for.
        for cols in ("pass", "stage"):
            self.assertRegex(self.refused("DELETE FROM packets WHERE k = 'tw'", cols=cols),
                             r"write_fit_of: a_side's runs tw hold no packets rows at node 0 "
                             r"\(STRIMS_SIM_PACKET_LOG\), which 7a empty: to first packet, 7b empty: packet to "
                             r"arrival, 7c empty: arrival to read read: their model side is missing, not failed")
        # The peer's reads of the node's writes.
        self.assertRegex(self.refused("UPDATE link_reads SET peer = 2 WHERE k = 'tw'"),
                         r"runs tw hold no link_reads rows at node 0 \(STRIMS_SIM_LINK_LOG\), which 7 done to read, "
                         r"7a empty: to first packet, 7b empty: packet to arrival, 7c empty: arrival to read, 8 read "
                         r"to take read:")
        self.assertRegex(self.refused("DELETE FROM seed_sends WHERE k = 'tw'"),
                         r"runs tw hold no seed_sends rows at node 0 \(STRIMS_SIM_SPEC\), which 1 push: runner read:")
        self.assertRegex(self.refused("DELETE FROM link_samples WHERE k = 'tw'"),
                         r"runs tw hold no link_samples rows at node 0 \(STRIMS_SIM_LINK_LOG\), which 6 low reading "
                         r"read:")

    def test_a_side_of_runs_with_a_table_and_without_reads_those_with(self):
        # A second twin run whose packets the cache lacks: the first
        # packet's stages are the other's alone.
        self.conn.execute("BEGIN")
        try:
            self.conn.execute("INSERT INTO jobs SELECT 'tw2', src, sim, env FROM jobs WHERE k = 'tw'")
            for t in ("link_writes", "link_reads", "link_samples", "seed_sends", "hosts"):
                self.conn.execute(f"INSERT INTO {t} SELECT x.* REPLACE ('tw2' AS k) FROM {t} x WHERE x.k = 'tw'")
            got = self.q("SELECT stage, a_runs FROM write_fit_of(?, ?) WHERE stage IN ('7a empty: to first packet', "
                         "'2 driver queue') ORDER BY stage", TW + [{"k": "tw2"}], TB)
            self.assertEqual(got, [("2 driver queue", 2), ("7a empty: to first packet", 1)])
        finally:
            self.conn.execute("ROLLBACK")

    def test_a_testbed_side_as_the_models_names_runshs_setting(self):
        err = self.refused("DELETE FROM packets WHERE k = 'tb1'", "DELETE FROM packets WHERE k = 'tb2'",
                           a=TB, b=TW)
        self.assertRegex(err, r"write_fit_of: a_side's runs tb1, tb2 hold no packets rows at node 0 \(PCAP=1\)")

    def test_a_stages_table_the_testbed_lacks_fails_or_reads_nothing(self):
        # The testbed's runs without packets: their stages 7a-c are the
        # model's alone, and fail; without them on either side, no stage.
        self.conn.execute("BEGIN")
        try:
            self.conn.execute("DELETE FROM packets WHERE k IN ('tb1', 'tb2')")
            got = self.q("SELECT stage, a_runs, b_runs, pass FROM write_fit_of(?, ?) WHERE stage LIKE '7a%'", TW, TB)
            self.assertEqual(got, [("7a empty: to first packet", 1, None, False)])
            self.conn.execute("DELETE FROM packets")
            got = self.q("SELECT stage FROM write_fit_of(?, ?) WHERE stage LIKE '7_ %'", TW, TB)
            self.assertEqual(got, [])
        finally:
            self.conn.execute("ROLLBACK")

    def test_a_side_of_no_runs_is_refused(self):
        import duckdb

        with self.assertRaisesRegex(duckdb.Error, "write_stages_of: a side of no runs"):
            self.q("SELECT * FROM write_fit_of([], ?)", TB)


# A twin's tie: each write into the empty channel is followed, 2 ms after
# its done, by TIED data packets to the viewer that its uplink took at one
# µs, the first (the least n) arriving 20 ms after and the others at
# TIED_AFTER_MS, out of n order.
TIED = 8
TIED_AFTER_MS = (20, 24, 21, 26, 22, 27, 23, 25)
TIED_WRITES = 5


def tied():
    """An in-memory DuckDB with the macros and an empty packets table,
    and a twin run `tie` of a 60 s stream: its seed (node 0) writes
    TIED_WRITES times into an empty data channel to its viewer (node 1)
    on link 0, each done 1 ms after it began and read 30 ms after that;
    `packets_src` holds each write's TIED packets, which share their µs,
    for a test to make `packets` of in an order of its own."""
    import duckdb

    conn = duckdb.connect()
    conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, sim VARCHAR, env MAP(VARCHAR, VARCHAR))")
    conn.execute("INSERT INTO jobs VALUES ('tie', 'testbed/b-twin', 'strims-sim', MAP {'STRIMS_SIM_CAL_SECS': '60'})")
    conn.execute("CREATE TABLE link_writes (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                 "link BIGINT, kind VARCHAR, off BIGINT, n BIGINT, bytes BIGINT, queued_us BIGINT, "
                 "done_us BIGINT, unsent_before BIGINT, wait VARCHAR)")
    conn.execute("CREATE TABLE link_reads (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                 "link BIGINT, kind VARCHAR, off BIGINT, n BIGINT, bytes BIGINT, taken_us BIGINT)")
    conn.execute("CREATE TABLE link_samples (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                 "link BIGINT, source VARCHAR, cause VARCHAR, buffered_bytes BIGINT)")
    conn.execute("CREATE TABLE seed_sends (k VARCHAR, src VARCHAR, t_ms BIGINT, node BIGINT, peer BIGINT, "
                 "how VARCHAR, fate VARCHAR, queued_ms BIGINT)")
    conn.execute("CREATE TABLE hosts (k VARCHAR, src VARCHAR, node BIGINT, delay_us BIGINT)")
    conn.execute("INSERT INTO hosts VALUES ('tie', 'testbed/b-twin', 0, 0)")
    conn.execute("CREATE TABLE packets_src (k VARCHAR, src VARCHAR, node BIGINT, n BIGINT, t_us BIGINT, "
                 "arrived_us BIGINT, lost VARCHAR, peer BIGINT, kind VARCHAR)")
    for w in range(TIED_WRITES):
        t = T + 100_000 * w
        conn.execute("INSERT INTO link_writes VALUES ('tie', 'testbed/b-twin', ?, 0, 1, 0, 'webrtc', ?, ?, 1000, ?, "
                     "?, 0, 'none')", (t, 1000 * w, w, t, t + 1000))
        conn.execute("INSERT INTO link_reads VALUES ('tie', 'testbed/b-twin', ?, 1, 0, 0, 'webrtc', ?, ?, 1000, ?)",
                     (t + 31_000, 1000 * w, w, t + 32_000))
        offered = t + 3000
        for i, after in enumerate(TIED_AFTER_MS):
            conn.execute("INSERT INTO packets_src VALUES ('tie', 'testbed/b-twin', 0, ?, ?, ?, NULL, 1, ?)",
                         (TIED * w + i, offered, offered + 1000 * after, "data" if i else "data_sack"))
    conn.execute("CREATE TABLE packets AS FROM packets_src LIMIT 0")
    for s in catalog.scoring_statements(MACROS, HAVE):
        conn.execute(s)
    return conn


class Ties(unittest.TestCase):
    """Packets that share their µs: a write's first packet is the least n
    of those at the first µs at or after its done, so the stages read the
    same however the table's rows are ordered, and every time."""

    # The row orders the packets table is made in.
    ORDERS = {"n": "n", "n descending": "n DESC", "arrival": "arrived_us, n",
              **{f"shuffle {s}": f"hash(n + {s})" for s in (1, 2, 3)}}

    @classmethod
    def setUpClass(cls):
        cls.conn = tied()

    def stages(self):
        return {s: (n, p50, p90) for s, n, p50, p90 in self.conn.execute(
            "SELECT stage, n, p50, p90 FROM write_stages_of([{'k': 'tie'}]) WHERE stage LIKE '7%' ORDER BY stage")
            .fetchall()}

    def test_a_writes_first_packet_is_the_least_n_at_its_first_offer(self):
        # Its 2 ms to the packets, the least n's 20 ms to arrive and 8 to
        # the read (30 after the done): the one answer of five queries in
        # each order.
        want = {"7 done to read": (TIED_WRITES, 30, 30), "7a empty: to first packet": (TIED_WRITES, 2, 2),
                "7b empty: packet to arrival": (TIED_WRITES, 20, 20),
                "7c empty: arrival to read": (TIED_WRITES, 8, 8)}
        got = {}
        for name, order in self.ORDERS.items():
            self.conn.execute(f"CREATE OR REPLACE TABLE packets AS FROM packets_src ORDER BY {order}")
            got[name] = {tuple(sorted(self.stages().items())) for _ in range(5)}
        self.assertEqual(got, {name: {tuple(sorted(want.items()))} for name in self.ORDERS})


if __name__ == "__main__":
    unittest.main()
