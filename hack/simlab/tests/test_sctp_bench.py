#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The SCTP bench's macros (scoring.sql's sctp_stages_of, sctp_logged_side
and sctp_fit_of) over a fixture with known answers: testbed runs with the
SCTP log on (`tb1`, `tb2`) and off (`tc1`, `tc2`), and two twin runs
(`tw1`, `tw2`). Each reading from its source over each run's window (the
wire's delays, trains at each gap, spells and send gaps, the bins' drops,
the writes' buffered amounts, the log's SACKs, resends a drop and
spacing), each judged against the testbed side its source allows, a
testbed run of the other kind refused there; and the pass: delays and
buffered amounts within the log-off runs' range widened by their floors,
both quantiles; the counts' and the send gaps' total variation within the
plan's bound or the testbed's own, the larger; resends a drop within
the logged runs' range widened by counting noise, the widest of the runs
at one ratio at an end, whichever run has it; the resend spacing
unruled; a ruled reading either side lacks failing; and a side of no runs
refused. Over the fixture as a catalog without the SCTP log and the
writes, and without the packets too (Lacking), the readings of what it
holds score as over every table, the others unscored with a caveat. Over
runs whose scans and aggregates run on threads (Repeats), 20 queries of
the fit give the same bits.

  python3 hack/simlab/tests/test_sctp_bench.py
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

MACROS = ("sketch_exp", "sketch_lo", "sketch_floor", "sketch_quantile", "compensated_sum", "twin_window_of",
          "side_window_of", "bench_lacking", "bench_logs", "packets_or_none", "link_writes_or_none", "sctp_events_or_none",
          "entry_folds_of", "sctp_stages_of", "sctp_logged_side", "sctp_fit_of")
HAVE = {"packets", "hosts", "bins", "sctp_events", "link_writes", "jobs.sim", "jobs.env"}
T = 20_000_000
TB, TC, TW = [{"k": "tb1"}, {"k": "tb2"}], [{"k": "tc1"}, {"k": "tc2"}], [{"k": "tw1"}, {"k": "tw2"}]
RUNS = (("tb1", "testbed/on", "testbed"), ("tb2", "testbed/on", "testbed"), ("tc1", "testbed/off", "testbed"),
        ("tc2", "testbed/off", "testbed"), ("tw1", "testbed/on-twin", "strims-sim"),
        ("tw2", "testbed/on-twin", "strims-sim"))
LOGGED, WIRED = {"tb1", "tb2", "tw1", "tw2"}, {"tc1", "tc2", "tw1", "tw2"}
G100, G400 = "3 packets a send opportunity, 100 µs", "3 packets a send opportunity, 400 µs"
GAPS = "12 send gaps"
EVENT = ("k", "src", "node", "peer", "link", "n", "t_us", "dir", "what", "tsn_lo", "tsn_hi", "chunks",
         "data_bytes", "packet_bytes", "cum_tsn", "gaps", "cwnd_bytes", "flight_bytes", "estimated")


def connect(lacking=()):
    """An in-memory DuckDB with the macros, and the fixture of 60 s streams,
    a seed (node 0) sending to one viewer (node 1) on link 0; without the
    tables `lacking`, its macros then made as a reader of a catalog of the
    others makes them (made).

    - The wire (log-off testbed runs and the twin's): from 20 s, three
      packets of DATA 50 µs apart (the twin's four), a SACK alone 20 µs
      after, one more 30 µs after that, and one 10 ms on: trains of 3, 1
      and 1 at either gap; one lost; each arriving 20 ms after its offer
      (the twin's 30), a testbed run's 5 ms egress delay (hosts.delay_us)
      among them, each carried packet leaving its queue (sent_us) 1 ms
      after it; send gaps of 50, 50, 100, 9800 and 5000 µs (the twin's 50,
      50, 50, 100, 9750 and 5000).
    - The bins: the seed's drops, 10 and 12 off, 4 and 5 on, 8 and 10 in
      the twin, and 99 past the window.
    - The writes: three unheld to the channel, a held one, a TCP one and
      one past the stream's end.
    - The log (logged testbed runs and the twin's): three first sends, a
      SACK of them, one more send, its resend and a resend of it with a new
      chunk; the viewer SACKs two packets and one (the twin's two and two),
      and one more packet it never SACKs."""
    import duckdb

    conn = duckdb.connect()
    conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, sim VARCHAR, env MAP(VARCHAR, VARCHAR))")
    for k, src, sim in RUNS:
        # A logged run's knobs name its log's hosts, as its twin's do.
        knobs = {"STRIMS_SIM_CAL_SECS": "60", **({"STRIMS_SIM_TRACE_SCTP": "hosts:0,1"} if k in LOGGED else {})}
        conn.execute("INSERT INTO jobs VALUES (?, ?, ?, MAP(?, ?))", (k, src, sim, list(knobs), list(knobs.values())))
    conn.execute("CREATE TABLE link_writes (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                 "link BIGINT, kind VARCHAR, off BIGINT, n BIGINT, bytes BIGINT, queued_us BIGINT, "
                 "done_us BIGINT, unsent_before BIGINT, wait VARCHAR)")
    conn.execute(f"CREATE TABLE sctp_events ({', '.join(c + (' VARCHAR' if c in ('k', 'src', 'dir', 'what') else ' BOOLEAN' if c == 'estimated' else ' BIGINT') for c in EVENT)})")
    conn.execute("CREATE TABLE packets (k VARCHAR, src VARCHAR, node BIGINT, n BIGINT, t_us BIGINT, arrived_us BIGINT, "
                 "lost VARCHAR, peer BIGINT, kind VARCHAR, ip_bytes BIGINT, sent_us BIGINT)")
    conn.execute("CREATE TABLE bins (k VARCHAR, src VARCHAR, t_ms BIGINT, seed_drops BIGINT)")
    conn.execute("CREATE TABLE hosts (k VARCHAR, src VARCHAR, node BIGINT, delay_us BIGINT)")
    buffered = {"tc1": [4096, 8192, 32768], "tc2": [4096, 16384, 32768], "tw1": [2048, 4096, 8192],
                "tw2": [4096, 4096, 8192], "tb1": [65536] * 3, "tb2": [65536] * 3}
    drops = {"tb1": 4, "tb2": 5, "tc1": 10, "tc2": 12, "tw1": 8, "tw2": 10}
    for k, src, _ in RUNS:
        twin = k.startswith("tw")
        # The seed's egress delay, 5 ms, between its offer (t_us) and its
        # queue: the arrival holds it.
        conn.execute("INSERT INTO hosts VALUES (?, ?, 0, 5000)", (k, src))
        conn.execute("INSERT INTO bins VALUES (?, ?, 30000, ?), (?, ?, 61000, 99)", (k, src, drops[k], k, src))
        writes = [(T + 1000 * i, b, "none", "webrtc") for i, b in enumerate(buffered[k])]
        writes += [(T + 5000, 65536, "channel", "webrtc"), (T + 6000, 65536, "none", "tcp"),
                   (61_000_000, 65536, "none", "webrtc")]
        for n, (t, b, wait, kind) in enumerate(writes):
            conn.execute("INSERT INTO link_writes VALUES (?, ?, ?, 0, 1, 0, ?, ?, ?, 1000, ?, ?, ?, ?)",
                         (k, src, t, kind, 1000 * n, n, t - 100, t + 10, b, wait))
        if k in WIRED:
            # The testbed's 20 ms from its offer, the twin's 30.
            delay = 30_000 if twin else 20_000
            train = 4 if twin else 3
            wire = [(T + 50 * i, "data") for i in range(train)]
            wire += [(T + 50 * train + 20, "sack"), (T + 50 * train + 50, "data"), (T + 10_000, "data")]
            wire += [(T + 15_000, "lost"), (1000, "data")]
            for n, (t, kind) in enumerate(sorted(wire)):
                lost = "queue" if kind == "lost" else None
                arrived = None if lost else t + delay
                sent = None if lost else t + (0 if twin else 6000)
                conn.execute("INSERT INTO packets VALUES (?, ?, 0, ?, ?, ?, ?, 1, ?, 1000, ?)",
                             (k, src, 10 * n, t, arrived, lost, "data" if kind == "lost" else kind, sent))
        if k in LOGGED:
            seed = [(T + 250 * i, "out", "data", i, i, 1) for i in range(3)]
            seed += [(T + 30_000, "in", "sack", None, None, 0), (T + 30_100, "out", "data", 3, 3, 1),
                     (T + 50_000, "out", "rtx", 3, 3, 1), (T + 60_000, "out", "rtx", 3, 4, 2),
                     (1000, "out", "data", None, None, 0)]
            sacked = [2, 2] if twin else [2, 1]
            peer, tsn, t = [], 0, T + 20_000
            for count in sacked:
                for _ in range(count):
                    peer.append((t, "in", "data", tsn, tsn, 1))
                    tsn, t = tsn + 1, t + 250
                peer.append((t - 200, "out", "sack", None, None, 0))
            peer.append((T + 50_100, "in", "data", tsn, tsn, 1))
            for node, other, rows in ((0, 1, seed), (1, 0, peer)):
                for n, (t, d, what, lo, hi, chunks) in enumerate(sorted(rows, key=lambda r: r[0])):
                    cum = 2 if what == "sack" else None
                    conn.execute(f"INSERT INTO sctp_events VALUES ({', '.join('?' * len(EVENT))})",
                                 (k, src, node, other, 0, 10 * n, t, d, what, lo, hi, chunks, 1000 * chunks,
                                  28 + 1016 * chunks, cum, 0 if cum is not None else None, 60_000,
                                  1016 * chunks if d == "out" or cum is not None else None, False))
    for t in lacking:
        conn.execute(f"DROP TABLE {t}")
    if lacking:
        made(conn)
        return conn
    for s in catalog.scoring_statements(MACROS, HAVE):
        conn.execute(s)
    return conn


def made(conn):
    """MACROS made over `conn`'s tables as a reader of a catalog of them
    makes them: catalog.needed's, its fallback for each one needing a table
    they lack."""
    have = {}
    for t, c in conn.execute("SELECT table_name, column_name FROM duckdb_columns()").fetchall():
        have.setdefault(t, set()).add(c)
    text = catalog.needed(catalog.scoring_text(catalog.scoring_consts()), have)
    for m in catalog.MACRO.finditer(text):
        if m.group(2) in MACROS:
            conn.execute(text[m.start():catalog.STATEMENT_END.search(text, m.end()).end()])


class SctpBench(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.conn = connect()

    def q(self, sql, *args):
        return self.conn.execute(sql, args).fetchall()

    def readings(self, k):
        return {r: (src, n, p50, p90) for r, src, n, p50, p90 in self.q(
            "SELECT reading, source, n, p50, p90 FROM sctp_stages_of([{'k': ?}]) ORDER BY reading", k)}

    def dist(self, k, reading):
        got = self.q("SELECT dist FROM sctp_stages_of([{'k': ?}]) WHERE reading = ?", k, reading)
        return {d["v"]: round(d["share"], 6) for d in got[0][0]}

    def fit(self, *cols, a=TW, b=TB, t=TC, **opts):
        args = "".join(f", {o} := {v}" for o, v in opts.items())
        return {r[0]: r[1:] for r in self.q(
            f"SELECT reading, {', '.join(cols)} FROM sctp_fit_of(?, ?, ?{args})", a, b, t)}

    def rolled_back(self, *statements, t=TC, **opts):
        """The fit's rows with `statements` applied, then undone."""
        self.conn.execute("BEGIN")
        try:
            for s, *args in statements:
                self.conn.execute(s, args)
            return self.fit("a_runs", "b_runs", "a_p50", "a_p90", "tv", "tv_bound", "pass", t=t, **opts)
        finally:
            self.conn.execute("ROLLBACK")

    def test_the_macros_load(self):
        names = {r[0] for r in self.q("SELECT DISTINCT function_name FROM duckdb_functions() "
                                       "WHERE function_name LIKE 'sctp_%'")}
        self.assertEqual(names, {"sctp_stages_of", "sctp_logged_side", "sctp_fit_of", "sctp_events_or_none"})

    def test_a_log_off_runs_readings(self):
        got = self.readings("tc1")
        # The window's packets alone, a lost one with no delay.
        self.assertEqual(got["1 one-way delay"], ("wire", 5, 20, 20))
        # The SACK alone between breaks the train, though 30 µs on; the lost
        # packet went to the uplink all the same, a train of its own. The
        # same trains at either gap.
        for gap in (G100, G400):
            self.assertEqual(got[gap], ("wire", 4, 1, 3))
            self.assertEqual(self.dist("tc1", gap), {1: 0.75, 3: 0.25})
        # One busy spell of the window's six packets, 15 ms, and its bytes.
        src, n, p50, _ = got["4 drain"]
        self.assertEqual((src, n), ("wire", 1))
        self.assertAlmostEqual(p50, 15, delta=0.1)
        self.assertAlmostEqual(got["5 drain rate"][2], 6000 / 15, delta=4)
        # The window's drops, a run's one value.
        self.assertEqual(got["6 queue drops"], ("wire", 10, 10, 10))
        # The unheld writes to the channel in the window: not the held, the
        # TCP or the late one.
        self.assertEqual(got["2 buffered at write"], ("writes", 3, 8192, 32768))
        # Every gap from a packet of DATA to the next, past the SACK alone,
        # in bins doubling from 50 µs.
        self.assertEqual(got[GAPS][:2], ("wire", 5))
        self.assertEqual(self.dist("tc1", GAPS), {50: 0.4, 100: 0.2, 3200: 0.2, 6400: 0.2})
        self.assertFalse({r for r in got if got[r][0] == "log"})

    def test_a_logged_runs_readings(self):
        got = self.readings("tb1")
        # SACKs of two packets and one; the last packet's segment, SACKed
        # by none, left out; each SACK 50 µs after its last packet.
        self.assertEqual(got["7 packets a SACK"], ("log", 2, 1, 2))
        self.assertEqual(self.dist("tw1", "7 packets a SACK"), {2: 1.0})
        src, n, p50, _ = got["8 SACK delay"]
        self.assertEqual((src, n), ("log", 2))
        self.assertAlmostEqual(p50, 0.05, delta=0.001)
        # Two chunks resent (a resend with a new chunk counts its resent
        # one alone) over the run's 4 drops.
        self.assertEqual(got["9 resends a drop"], ("log", 2, 0.5, 0.5))
        # The second resend of TSN 3 came with no SACK since the first.
        self.assertEqual(got["10 resend spacing"], ("log", 1, 0, 0))
        self.assertEqual(got["11 flight at sends"][:2], ("log", 6))
        # A logged run without a capture has no wire readings, its bins'
        # drops all the same.
        self.assertNotIn("1 one-way delay", got)
        self.assertNotIn(GAPS, got)
        self.assertEqual(got["6 queue drops"][1], 4)

    def test_a_window_of_its_own(self):
        got = self.q("SELECT n FROM sctp_stages_of([{'k': 'tc1'}], lo_s := 20, hi_s := 20.0001) "
                     "WHERE reading = ?", G100)
        self.assertEqual(got, [(1,)])
        got = self.q("SELECT reading, n FROM sctp_stages_of([{'k': 'tb1'}], lo_s := 20, hi_s := 20.001) "
                     "WHERE reading IN ('7 packets a SACK', '11 flight at sends') ORDER BY reading")
        self.assertEqual(got, [("11 flight at sends", 3)])
        # The window's first packet of DATA has no gap before it.
        got = self.q("SELECT n FROM sctp_stages_of([{'k': 'tc1'}], lo_s := 20.0001) WHERE reading = ?", GAPS)
        self.assertEqual(got, [(3,)])

    def test_a_fold_of_the_entry_has_its_egress_delay_taken_off(self):
        # tc1 folded as before t_us was the offer: each packet's t_us its
        # entry, 5 ms later, which a carried one leaves 1 ms after; the
        # window's first packet then entered past 20 s.
        conn = connect()
        conn.execute("UPDATE packets SET t_us = t_us + 5000 WHERE k = 'tc1'")
        self.assertEqual(conn.execute("SELECT k, delay_us FROM entry_folds_of(?, 0)", (TB + TC + TW,)).fetchall(),
                         [("tc1", 5000)])
        for lo, hi in ((20, -1), (20, 20.0001), (20.0001, -1)):
            sql = f"FROM sctp_stages_of([{{'k': 'tc1'}}], lo_s := {lo}, hi_s := {hi}) ORDER BY reading"
            self.assertEqual(dicts(conn, sql), dicts(self.conn, sql))
        self.assertEqual(dicts(conn, "SELECT p50 FROM sctp_stages_of([{'k': 'tc1'}]) "
                                     "WHERE reading = '1 one-way delay'"), [{"p50": 20}])

    def test_trains_at_each_gap(self):
        # A packet 250 µs after the one 10 ms on: its own train at 100 µs,
        # that one's second at 400.
        self.conn.execute("BEGIN")
        try:
            self.conn.execute("INSERT INTO packets SELECT x.* REPLACE (x.n + 5 AS n, x.t_us + 250 AS t_us, "
                              "x.arrived_us + 250 AS arrived_us) FROM packets x WHERE x.k = 'tc1' AND x.t_us = ?",
                              (T + 10_000,))
            self.assertEqual(self.dist("tc1", G100), {1: 0.8, 3: 0.2})
            self.assertEqual(self.dist("tc1", G400), {1: 0.5, 2: 0.25, 3: 0.25})
            # Gaps of their own take readings of their own names.
            got = self.q("SELECT reading, n FROM sctp_stages_of([{'k': 'tc1'}], short_gap_us := 300, "
                         "long_gap_us := 12000) WHERE reading LIKE '3 %' ORDER BY reading")
            self.assertEqual(got, [("3 packets a send opportunity, 12000 µs", 2),
                                   ("3 packets a send opportunity, 300 µs", 4)])
        finally:
            self.conn.execute("ROLLBACK")

    def test_send_gaps_take_every_gap(self):
        # A gap under the floor shares the first bin, one past the busy
        # spell's idle time has its own: no gap is cut.
        self.conn.execute("BEGIN")
        try:
            self.conn.execute("INSERT INTO packets SELECT x.* REPLACE (x.n + 5 AS n, x.t_us + 10 AS t_us, "
                              "x.arrived_us + 10 AS arrived_us) FROM packets x WHERE x.k = 'tc1' AND x.t_us = ?",
                              (T + 10_000,))
            self.conn.execute("INSERT INTO packets SELECT x.* REPLACE (x.n + 100 AS n, x.t_us + 45_000 AS t_us, "
                              "x.t_us + 60_000 AS arrived_us, NULL AS lost) FROM packets x "
                              "WHERE x.k = 'tc1' AND x.t_us = ?", (T + 15_000,))
            want = {0: 1, 50: 2, 100: 1, 3200: 1, 6400: 1, 25600: 1}
            self.assertEqual(self.dist("tc1", GAPS), {v: round(c / 7, 6) for v, c in want.items()})
            # Its p50 and p90 read the gaps themselves, in µs.
            src, n, p50, p90 = self.readings("tc1")[GAPS]
            self.assertEqual((src, n), ("wire", 7))
            self.assertAlmostEqual(p50, 100, delta=1)
            self.assertAlmostEqual(p90, 45_000, delta=450)
        finally:
            self.conn.execute("ROLLBACK")

    def test_each_reading_against_its_side(self):
        got = self.fit("source", "against", "perturbed", "b_runs", "b_ns", "pass")
        self.assertEqual(got["1 one-way delay"][:4], ("wire", "log off", False, 2))
        self.assertEqual(got["2 buffered at write"][:3], ("writes", "log off", False))
        for r in (G100, G400, GAPS):
            self.assertEqual(got[r][:4], ("wire", "log off", False, 2), r)
        self.assertEqual(got["6 queue drops"][4], [10, 12])
        self.assertEqual(got["7 packets a SACK"][:3], ("log", "log on", True))
        self.assertEqual(got["9 resends a drop"][:4], ("log", "log on", True, 2))
        for r in ("4 drain", "5 drain rate", "6 queue drops", "8 SACK delay", "10 resend spacing",
                  "11 flight at sends"):
            self.assertIsNone(got[r][-1], r)
        # In the readings' order, the gaps in theirs.
        self.assertEqual(list(got)[:4], ["1 one-way delay", "2 buffered at write", G100, G400])
        self.assertEqual(list(got)[-1], GAPS)

    def test_a_side_of_the_other_kind_is_refused(self):
        import duckdb

        # Logged runs as the log-off side, log-off runs as the logged one.
        with self.assertRaisesRegex(duckdb.Error, r"sctp_fit_of: t_side holds testbed runs with the SCTP log on "
                                                  r"\(TRACE_SCTP\): tb1, tb2"):
            self.fit("pass", t=TB)
        with self.assertRaisesRegex(duckdb.Error, r"sctp_fit_of: b_side holds testbed runs with the SCTP log off "
                                                  r"\(TRACE_SCTP\): tc2$"):
            self.fit("pass", b=TB + [{"k": "tc2"}])
        # A sim's runs stand on either side; an empty knob is no log.
        self.assertIs(self.fit("pass", b=TW, t=TW)["1 one-way delay"][0], True)
        self.conn.execute("BEGIN")
        try:
            self.conn.execute("UPDATE jobs SET env = MAP {'STRIMS_SIM_CAL_SECS': '60', 'STRIMS_SIM_TRACE_SCTP': ' '} "
                              "WHERE k = 'tc1'")
            self.assertIn("1 one-way delay", self.fit("pass"))
        finally:
            self.conn.execute("ROLLBACK")

    def test_the_delay_and_the_buffered_amount(self):
        got = self.fit("b_p50s", "a_p50", "a_p90", "pass")
        # 30 ms against 20 and 20, widened by 1: no pass.
        self.assertEqual(got["1 one-way delay"], ([20, 20], 30, 30, False))
        # The twin's runs' p50s 4096 and 4096, p90s 8192 and 8192, their
        # medians; its p50 at the range's widened edge, its p90 past it.
        self.assertEqual(got["2 buffered at write"], ([8192, 16384], 4096, 8192, False))
        self.assertIs(self.fit("pass", floor_bytes=24576)["2 buffered at write"][0], True)
        # The model's median over its runs.
        got = self.rolled_back(("UPDATE link_writes SET unsent_before = 4 * unsent_before WHERE k = 'tw2'",))
        self.assertEqual(got["2 buffered at write"][2:4], (10240, 20480))

    def test_the_delays_floor(self):
        # 21 ms against 20 and 20: within the 1 ms floor, not within half.
        late = ("UPDATE packets SET arrived_us = t_us + 21_000 WHERE k LIKE 'tw%' AND lost IS NULL",)
        got = self.rolled_back(late)
        self.assertEqual(got["1 one-way delay"][2:4], (21, 21))
        self.assertIs(got["1 one-way delay"][-1], True)
        self.assertIs(self.rolled_back(late, floor_ms=0.5)["1 one-way delay"][-1], False)

    def test_both_quantiles_rule(self):
        # The twin's p90 within, its p50 out: no pass, either rule.
        got = self.rolled_back(("UPDATE link_writes SET unsent_before = 32768 WHERE k LIKE 'tw%'",))
        self.assertEqual(got["2 buffered at write"][2:4], (32768, 32768))
        self.assertIs(got["2 buffered at write"][-1], False)
        got = self.rolled_back(("UPDATE packets SET arrived_us = t_us + 10_000 WHERE k LIKE 'tw%' AND n < 50",),
                               ("UPDATE packets SET arrived_us = t_us + 20_000 WHERE k LIKE 'tw%' AND n >= 50 "
                                "AND lost IS NULL",))
        self.assertEqual(got["1 one-way delay"][2:4], (10, 20))
        self.assertIs(got["1 one-way delay"][-1], False)
        # Its p50 within, its p90 out: no pass.
        got = self.rolled_back(("UPDATE packets SET arrived_us = t_us + 20_000 WHERE k LIKE 'tw%' AND n < 60",),
                               ("UPDATE packets SET arrived_us = t_us + 30_000 WHERE k LIKE 'tw%' AND n >= 60 "
                                "AND lost IS NULL",))
        self.assertEqual(got["1 one-way delay"][2:4], (20, 30))
        self.assertIs(got["1 one-way delay"][-1], False)
        got = self.rolled_back(("UPDATE link_writes SET unsent_before = 12288 WHERE k LIKE 'tw%' AND n < 2",),
                               ("UPDATE link_writes SET unsent_before = 65536 WHERE k LIKE 'tw%' AND n = 2",))
        self.assertEqual(got["2 buffered at write"][2:4], (12288, 65536))
        self.assertIs(got["2 buffered at write"][-1], False)

    def refused(self, *statements, cols=("pass",), **sides):
        """The fit's error with `statements` applied, then undone."""
        import duckdb

        self.conn.execute("BEGIN")
        try:
            for s, *args in statements:
                self.conn.execute(s, args)
            with self.assertRaises(duckdb.Error) as e:
                self.fit(*cols, **sides)
            return str(e.exception)
        finally:
            self.conn.execute("ROLLBACK")

    def test_a_ruled_reading_one_side_lacks_fails(self):
        # The model's packets all before the window: its wire readings have
        # none to read there.
        got = self.rolled_back(("UPDATE packets SET t_us = 1000 + n WHERE k LIKE 'tw%'",))
        self.assertEqual((got["1 one-way delay"][0], got["1 one-way delay"][-1]), (None, False))
        for r in (G100, G400, GAPS):
            self.assertIs(got[r][-1], False, r)
            self.assertIsNone(got[r][4], r)
        got = self.rolled_back(("DELETE FROM link_writes WHERE k LIKE 'tc%'",))
        self.assertEqual((got["2 buffered at write"][1], got["2 buffered at write"][-1]), (None, False))
        # A count's tv against a side without it is no number, and fails.
        got = self.rolled_back(("DELETE FROM sctp_events WHERE k LIKE 'tw%' AND node = 1",))
        self.assertIsNone(got["7 packets a SACK"][4])
        self.assertIs(got["7 packets a SACK"][-1], False)
        # Resends a drop, the model's runs without a drop or the testbed's
        # without a log.
        got = self.rolled_back(("UPDATE bins SET seed_drops = 0 WHERE k LIKE 'tw%'",))
        self.assertIsNone(got["9 resends a drop"][0])
        self.assertIs(got["9 resends a drop"][-1], False)
        got = self.rolled_back(("DELETE FROM sctp_events WHERE k LIKE 'tb%'",))
        self.assertIsNone(got["9 resends a drop"][1])
        self.assertIs(got["9 resends a drop"][-1], False)

    def test_a_model_side_without_a_readings_table_is_refused(self):
        # A twin without the packet log has no side of the wire readings:
        # the fit names the table, the knob and the runs, whatever columns
        # are asked for.
        for cols in (("pass",), ("reading", "source")):
            err = self.refused(("DELETE FROM packets WHERE k LIKE 'tw%'",), cols=cols)
            self.assertRegex(err, r"sctp_fit_of: a_side's runs tw1, tw2 hold no packets rows at node 0 "
                                  r"\(STRIMS_SIM_PACKET_LOG\), which 1 one-way delay, 3 packets a send opportunity, "
                                  r"100 µs, 3 packets a send opportunity, 400 µs, 4 drain, 5 drain rate, 12 send gaps "
                                  r"read: their model side is missing, not failed")
        # None at the node the readings read, none of their writes or of
        # their SCTP log.
        self.assertIn("runs tw1, tw2 hold no packets rows at node 0",
                      self.refused(("UPDATE packets SET node = 1 WHERE k LIKE 'tw%'",)))
        self.assertRegex(self.refused(("DELETE FROM link_writes WHERE k LIKE 'tw%'",)),
                         r"runs tw1, tw2 hold no link_writes rows at node 0 \(STRIMS_SIM_LINK_LOG\), which 2 buffered "
                         r"at write read:")
        self.assertRegex(self.refused(("DELETE FROM sctp_events WHERE k LIKE 'tw%'",)),
                         r"runs tw1, tw2 hold no sctp_events rows at node 0 \(STRIMS_SIM_TRACE_SCTP\), which 7 "
                         r"packets a SACK, 8 SACK delay, 9 resends a drop, 10 resend spacing, 11 flight at sends read:")
        # Two tables, each named.
        err = self.refused(("DELETE FROM packets WHERE k LIKE 'tw%'",), ("DELETE FROM link_writes WHERE k LIKE 'tw%'",))
        self.assertRegex(err, r"runs tw1, tw2 hold no link_writes rows at node 0 \(STRIMS_SIM_LINK_LOG\), which 2 "
                              r"buffered at write read, and tw1, tw2 hold no packets rows at node 0 "
                              r"\(STRIMS_SIM_PACKET_LOG\)")
        # A testbed's runs as the model's side: run.sh's setting.
        self.assertRegex(self.refused(cols=("pass",), a=TB),
                         r"sctp_fit_of: a_side's runs tb1, tb2 hold no packets rows at node 0 \(PCAP=1\), which 1 "
                         r"one-way delay")

    def test_a_side_of_runs_with_a_table_and_without_reads_those_with(self):
        # The log-off cell's twin beside the logged one's (one source's
        # twins of the SCTP matrices' log axis): the log's readings are the
        # logged twin's runs', the wire's both.
        for s in (("DELETE FROM sctp_events WHERE k = 'tw2'",), ("DELETE FROM packets WHERE k = 'tw2'",),
                  ("DELETE FROM link_writes WHERE k = 'tw2'",)):
            got = self.rolled_back(s)
            table = s[0].split()[2]
            reading = {"sctp_events": "7 packets a SACK", "packets": "1 one-way delay",
                       "link_writes": "2 buffered at write"}[table]
            self.assertEqual(got[reading][0], 1, table)

    def test_a_table_neither_side_holds_has_no_reading(self):
        got = self.rolled_back(("DELETE FROM link_writes",))
        self.assertNotIn("2 buffered at write", got)
        self.assertIn("1 one-way delay", got)

    def test_the_counts_total_variation(self):
        got = self.fit("tv", "tv_bound", "b_tv", "pass")
        # A train of four where the testbed's are three: a quarter of the
        # shares move, past the plan's 0.1; the testbed's own runs agree.
        for r in (G100, G400):
            tv, bound, own, ok = got[r]
            self.assertAlmostEqual(tv, 0.25)
            self.assertAlmostEqual(bound, 0.1)
            self.assertAlmostEqual(own, 0)
            self.assertIs(ok, False)
        self.assertIs(self.fit("pass", max_tv=0.3)[G100][0], True)
        # SACKs of two and two against two and one.
        tv, _, _, ok = got["7 packets a SACK"]
        self.assertAlmostEqual(tv, 0.5)
        self.assertIs(ok, False)
        self.assertIs(self.fit("pass", max_tv=0.6)["7 packets a SACK"][0], True)
        # The send gaps: half the twin's at 50 µs against two fifths, a
        # sixth at each other against a fifth.
        tv, bound, own, _ = got[GAPS]
        self.assertAlmostEqual(tv, 0.1)
        self.assertAlmostEqual(bound, 0.1)
        self.assertIs(self.fit("pass", max_tv=0.09)[GAPS][0], False)
        self.assertIs(self.fit("pass", max_tv=0.11)[GAPS][0], True)

    def test_the_resend_spacing_is_unruled(self):
        tv, bound, own, ok = self.fit("tv", "tv_bound", "b_tv", "pass")["10 resend spacing"]
        self.assertAlmostEqual(tv, 0)
        self.assertAlmostEqual(own, 0)
        self.assertIsNone(bound)
        self.assertIsNone(ok)

    def test_a_train_is_uncapped(self):
        self.conn.execute("BEGIN")
        try:
            self.conn.execute("INSERT INTO packets SELECT k, src, 0, 1000 + i, ? + 10 * i, ? + 10 * i + 20000, NULL, "
                              "1, 'data', 1000, ? + 10 * i + 6000 FROM range(12) r(i), "
                              "(SELECT DISTINCT k, src FROM packets WHERE k = 'tc1')", (T + 30_000,) * 3)
            got = self.q("SELECT p90, dist FROM sctp_stages_of([{'k': 'tc1'}]) WHERE reading = ?", G100)
            self.assertEqual(got[0][0], 12)
            self.assertIn(12, {d["v"] for d in got[0][1]})
        finally:
            self.conn.execute("ROLLBACK")

    def test_the_testbeds_own_spread_past_the_plans_bound_widens_it(self):
        # tc2's train of four, and a third run tc3 as tc1: its pairs differ
        # by a quarter, 0 and a quarter; b_tv takes the largest, and the
        # bound takes it past the plan's 0.1. The twin's trains of four, a
        # sixth from the pooled testbed's, are as near as its repeats are,
        # and pass.
        self.conn.execute("BEGIN")
        try:
            for s in ("INSERT INTO packets SELECT x.* REPLACE (25 AS n, x.t_us + 25 AS t_us, "
                      "x.arrived_us + 25 AS arrived_us) FROM packets x WHERE x.k = 'tc2' AND x.n = 20",
                      "INSERT INTO jobs SELECT 'tc3', src, sim, env FROM jobs WHERE k = 'tc1'",
                      "INSERT INTO packets SELECT x.* REPLACE ('tc3' AS k) FROM packets x WHERE x.k = 'tc1'",
                      "INSERT INTO bins SELECT x.* REPLACE ('tc3' AS k) FROM bins x WHERE x.k = 'tc1'"):
                self.conn.execute(s)
            got = self.fit("b_runs", "tv", "tv_bound", "b_tv", "pass", t=TC + [{"k": "tc3"}])
        finally:
            self.conn.execute("ROLLBACK")
        runs, tv, bound, own, ok = got[G100]
        self.assertEqual(runs, 3)
        self.assertAlmostEqual(own, 0.25)
        self.assertAlmostEqual(bound, 0.25)
        self.assertAlmostEqual(tv, 1 / 6)
        self.assertIs(ok, True)

    def test_resends_a_drop_within_the_range_and_counting_noise(self):
        # The logged runs' 100 resends over 50 drops and 120 over 50 (2 and
        # 2.4): the twin passes from 2 - 2·2·√(1/100 + 1/50) ≈ 1.31 to
        # 2.4 + 2·2.4·√(1/120 + 1/50) ≈ 3.21.
        for twin, want in ((3.0, True), (1.4, True), (3.4, False), (1.2, False)):
            statements = [("UPDATE bins SET seed_drops = 50 WHERE t_ms = 30000",)]
            for k, n in (("tb1", 100), ("tb2", 120), ("tw1", round(50 * twin)), ("tw2", round(50 * twin))):
                statements.append(("INSERT INTO sctp_events SELECT x.* REPLACE (x.n + 100 * (r.range + 1) AS n, "
                                   "x.t_us + 10 * (r.range + 1) AS t_us) FROM sctp_events x, range(?) r "
                                   "WHERE x.k = ? AND x.what = 'rtx' AND x.chunks = 1", n - 2, k))
            got = self.rolled_back(*statements)
            self.assertAlmostEqual(got["9 resends a drop"][2], twin, delta=0.02)
            self.assertIs(got["9 resends a drop"][-1], want, twin)

    def test_resends_a_drop_at_a_tied_end_widen_by_its_widest_noise(self):
        # The logged runs at one ratio, 2: 100 resends over 50 drops
        # (noise 2·√(1/100 + 1/50) ≈ 0.346) and 50 over 25 (≈ 0.490), so
        # both are each end and the wider noise widens it, from 1.02 to
        # 2.98, whichever run has which counts and whatever the rows'
        # order; the narrower's would fail the twin at 1.2 and at 2.8.
        def counts(k, resends, drops):
            return [("UPDATE bins SET seed_drops = ? WHERE t_ms = 30000 AND k = ?", drops, k),
                    ("INSERT INTO sctp_events SELECT x.* REPLACE (x.n + 100 * (r.range + 1) AS n, x.t_us + 10 * "
                     "(r.range + 1) AS t_us) FROM sctp_events x, range(?) r WHERE x.k = ? AND x.what = 'rtx' "
                     "AND x.chunks = 1", resends - 2, k)]

        reorder = [("CREATE OR REPLACE TABLE sctp_events AS FROM sctp_events ORDER BY hash(k, n) DESC",),
                   ("CREATE OR REPLACE TABLE bins AS FROM bins ORDER BY k DESC",)]
        got = {}
        for twin in (1.2, 2.8):
            tw = [s for k in ("tw1", "tw2") for s in counts(k, round(50 * twin), 50)]
            for wide in ("tb1", "tb2"):
                narrow = "tb2" if wide == "tb1" else "tb1"
                tb = counts(wide, 50, 25) + counts(narrow, 100, 50)
                for name, order in (("as made", []), ("reordered", reorder)):
                    row = self.rolled_back(*tw, *tb, *order)["9 resends a drop"]
                    got[twin, wide, name] = (round(row[2], 3), row[-1])
        self.maxDiff = None
        self.assertEqual(got, {(twin, wide, name): (twin, True) for twin in (1.2, 2.8) for wide in ("tb1", "tb2")
                               for name in ("as made", "reordered")})

    def test_a_run_of_no_resends_reads_none(self):
        # The twin's runs resend nothing: 0 a drop, which fails against the
        # logged runs' 0.4 and 0.5 once their counts are many; so does a
        # run that sent no DATA in the window at all.
        many = ("INSERT INTO sctp_events SELECT x.* REPLACE (x.n + 100 * (r.range + 1) AS n, x.t_us + 10 * "
                "(r.range + 1) AS t_us) FROM sctp_events x, range(20) r WHERE x.k IN ('tb1', 'tb2') "
                "AND x.what = 'rtx' AND x.chunks = 1",)
        drops = ("UPDATE bins SET seed_drops = 50 WHERE t_ms = 30000",)
        for gone in ("what = 'rtx'", "node = 0 AND dir = 'out'"):
            got = self.rolled_back(drops, many, (f"DELETE FROM sctp_events WHERE k LIKE 'tw%' AND {gone}",))
            self.assertEqual(got["9 resends a drop"][0], 2, gone)
            self.assertEqual(got["9 resends a drop"][2], 0, gone)
            self.assertIs(got["9 resends a drop"][-1], False, gone)

    def test_a_side_of_no_runs_is_refused(self):
        import duckdb

        with self.assertRaisesRegex(duckdb.Error, "sctp_stages_of: a side of no runs"):
            self.q("SELECT * FROM sctp_fit_of([], ?, ?)", TB, TC)
        with self.assertRaisesRegex(duckdb.Error, "sctp_stages_of: a side of no runs"):
            self.q("SELECT * FROM sctp_fit_of(?, ?, [])", TW, TB)


# The readings of each table a catalog can lack, and their caveat there.
WIRE = ("1 one-way delay", G100, G400, "4 drain", "5 drain rate", GAPS)
LOG = ("7 packets a SACK", "8 SACK delay", "9 resends a drop", "10 resend spacing", "11 flight at sends")
NO_PACKETS = "this catalog holds no packets (a testbed run's PCAP=1, a sim's STRIMS_SIM_PACKET_LOG)"
NO_WRITES = "this catalog holds no link_writes (a testbed run's LINK_LOG, a sim's STRIMS_SIM_LINK_LOG)"
NO_LOG = "this catalog holds no sctp_events (a testbed run's TRACE_SCTP, a sim's STRIMS_SIM_TRACE_SCTP)"


def dicts(conn, sql, *args):
    cur = conn.execute(sql, args)
    names = [d[0] for d in cur.description]
    return [dict(zip(names, r)) for r in cur.fetchall()]


class Lacking(unittest.TestCase):
    """The fixture as a catalog without the SCTP log and the writes
    (sctp_events and link_writes: a testbed tier's, its runs PCAP=1 but
    neither TRACE_SCTP nor LINK_LOG), and without the wire's packets too,
    its macros made as a reader of it makes them: the readings of the
    tables it holds score as over every table, and the others come out
    unscored, a row of no values whose caveat names the table and its
    settings; a model side holding none of a table the catalog holds is
    still refused."""

    @classmethod
    def setUpClass(cls):
        cls.full = connect()
        cls.conn = connect(lacking=("sctp_events", "link_writes"))
        cls.bare = connect(lacking=("sctp_events", "link_writes", "packets"))

    def fit(self, conn):
        return dicts(conn, "FROM sctp_fit_of(?, ?, ?)", TW, TB, TC)

    def test_the_fit_scores_what_the_catalog_holds(self):
        full = {r["reading"]: r for r in self.fit(self.full)}
        named = {"reading", "source", "against", "perturbed", "caveat"}
        for conn, lacks in ((self.conn, {"2 buffered at write": NO_WRITES, **dict.fromkeys(LOG, NO_LOG)}),
                            (self.bare, {"2 buffered at write": NO_WRITES, **dict.fromkeys(LOG, NO_LOG),
                                         **dict.fromkeys(WIRE, NO_PACKETS)})):
            got = self.fit(conn)
            # Every reading, in the readings' order.
            self.assertEqual([r["reading"] for r in got], list(full))
            for r in got:
                if r["reading"] not in lacks:
                    self.assertEqual(r, full[r["reading"]], r["reading"])
                    continue
                want = full[r["reading"]]
                self.assertEqual({c: r[c] for c in named},
                                 {**{c: want[c] for c in named}, "caveat": lacks[r["reading"]]}, r["reading"])
                # No values, and no pass.
                self.assertEqual({c: v for c, v in r.items() if c not in named and v is not None}, {}, r["reading"])
        # The ruled ones it scores have a pass, and none a caveat.
        got = {r["reading"]: (r["pass"], r["caveat"]) for r in self.fit(self.conn)}
        self.assertEqual((got["1 one-way delay"], got[G100], got["6 queue drops"]),
                         ((False, None), (False, None), (None, None)))

    def test_each_run_has_a_row_of_each_reading_it_cant_score(self):
        lacks = {"2 buffered at write": ("writes", NO_WRITES), **dict.fromkeys(LOG, ("log", NO_LOG))}
        for k in ("tc1", "tb1", "tw1"):
            full = {r["reading"]: r for r in dicts(self.full, "FROM sctp_stages_of([{'k': ?}])", k)}
            got = {r["reading"]: r for r in dicts(self.conn, "FROM sctp_stages_of([{'k': ?}])", k)}
            self.assertEqual(set(got), set(full) | set(lacks), k)
            for reading, r in got.items():
                if reading not in lacks:
                    self.assertEqual(r, full[reading], (k, reading))
                    continue
                self.assertEqual((r["k"], r["source"], r["caveat"]), (k, *lacks[reading]), reading)
                self.assertEqual((r["n"], r["p50"], r["p90"], r["dist"]), (None,) * 4, reading)
        # The trains at gaps of a call's own, scored or not.
        sql = ("SELECT reading, caveat FROM sctp_stages_of([{'k': 'tc1'}], short_gap_us := 300, long_gap_us := 300) "
               "WHERE reading LIKE '3 %'")
        self.assertEqual(self.conn.execute(sql).fetchall(), [("3 packets a send opportunity, 300 µs", None)])
        self.assertEqual(self.bare.execute(sql).fetchall(), [("3 packets a send opportunity, 300 µs", NO_PACKETS)])

    def test_a_model_side_without_a_table_the_catalog_holds_is_refused(self):
        import duckdb

        self.conn.execute("BEGIN")
        try:
            self.conn.execute("DELETE FROM packets WHERE k LIKE 'tw%'")
            with self.assertRaisesRegex(duckdb.Error, r"sctp_fit_of: a_side's runs tw1, tw2 hold no packets rows at "
                                                      r"node 0 \(STRIMS_SIM_PACKET_LOG\), which 1 one-way delay, .*, "
                                                      r"12 send gaps read: their model side is missing, not failed$"):
                self.fit(self.conn)
        finally:
            self.conn.execute("ROLLBACK")
        with self.assertRaisesRegex(duckdb.Error, "sctp_stages_of: a side of no runs"):
            self.bare.execute("FROM sctp_fit_of([], ?, ?)", (TB, TC)).fetchall()


class Repeats(unittest.TestCase):
    """The fit over three testbed runs and three twin runs of 50,000
    packets each, more than a row group's (122,880) in all, so its scans
    and aggregates run on threads: each reading's every column the same
    bits in each of 20 queries, the total variation distances and their
    bounds too, whose sums of floats once added in the order the threads
    met their rows (three or five distinct tv of 20 at a train's gaps)."""

    PACKETS = 50_000

    @classmethod
    def setUpClass(cls):
        import duckdb

        conn = cls.conn = duckdb.connect()
        conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, sim VARCHAR, env MAP(VARCHAR, VARCHAR))")
        for i in range(1, 4):
            for k, src, sim in ((f"tc{i}", "testbed/off", "testbed"), (f"tw{i}", "testbed/on-twin", "strims-sim")):
                conn.execute("INSERT INTO jobs VALUES (?, ?, ?, MAP {'STRIMS_SIM_CAL_SECS': '60'})", (k, src, sim))
        conn.execute("CREATE TABLE hosts AS SELECT k, src, 0::BIGINT AS node, 0::BIGINT AS delay_us FROM jobs")
        conn.execute("CREATE TABLE bins AS SELECT k, src, 30000::BIGINT AS t_ms, 3::BIGINT AS seed_drops FROM jobs")
        conn.execute("CREATE TABLE link_writes (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, peer BIGINT, "
                     "link BIGINT, kind VARCHAR, off BIGINT, n BIGINT, bytes BIGINT, queued_us BIGINT, "
                     "done_us BIGINT, unsent_before BIGINT, wait VARCHAR)")
        conn.execute(f"CREATE TABLE sctp_events ({', '.join(c + (' VARCHAR' if c in ('k', 'src', 'dir', 'what') else ' BOOLEAN' if c == 'estimated' else ' BIGINT') for c in EVENT)})")
        # Each run's packets of DATA to the viewer from 20 s, their gaps a
        # hash of the run and the packet's: mostly 20-169 µs, one in seven
        # 2-5 ms, so the trains and the send gaps take many values, of
        # shares no power of two's.
        conn.execute(f"""
            CREATE TABLE packets AS
            WITH g AS (
                SELECT j.k, j.src, r.i,
                       CASE WHEN hash(j.k, r.i) % 7 = 0 THEN 2000 + hash(j.k, r.i, 1) % 3000
                            ELSE 20 + hash(j.k, r.i, 2) % 150 END AS gap
                FROM jobs j, range({cls.PACKETS}) r(i))
            SELECT k, src, 0::BIGINT AS node, i::BIGINT AS n,
                   (20000000 + sum(gap) OVER (PARTITION BY k ORDER BY i))::BIGINT AS t_us,
                   NULL::BIGINT AS arrived_us, NULL::VARCHAR AS lost, 1::BIGINT AS peer, 'data' AS kind,
                   1200::BIGINT AS ip_bytes, NULL::BIGINT AS sent_us
            FROM g""")
        conn.execute("UPDATE packets SET arrived_us = t_us + 20000 + hash(k, n, 3) % 1000, sent_us = t_us")
        for s in catalog.scoring_statements(MACROS, HAVE):
            conn.execute(s)

    def test_every_query_gives_the_same_bits(self):
        tw = [{"k": f"tw{i}"} for i in range(1, 4)]
        tc = [{"k": f"tc{i}"} for i in range(1, 4)]
        # repr: a float's every bit.
        got = {repr(dicts(self.conn, "FROM sctp_fit_of(?, ?, ?)", tw, tw, tc)) for _ in range(20)}
        self.assertEqual(len(got), 1)
        rows = {r["reading"]: r for r in dicts(self.conn, "FROM sctp_fit_of(?, ?, ?)", tw, tw, tc)}
        for r in (G100, G400, GAPS):
            self.assertGreater(rows[r]["tv"], 0, r)
            self.assertGreater(rows[r]["b_tv"], 0, r)


if __name__ == "__main__":
    unittest.main()
