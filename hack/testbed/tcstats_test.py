#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""tcstats' parsers on tc's output (iproute2 6.19, the testbed image's),
captured from the link classes' trees on a dummy device with a queue
standing.

    python3 hack/testbed/tcstats_test.py
"""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import tcstats  # noqa: E402

# A tbf root holding 2 KiB, which tc prints as 2Kb, joined as run.sh's
# tc-0.log has it; the clsact after it holds 0b.
TBF_LINE = (
    "1794840471604 qdisc tbf 10: root refcnt 2 rate 1Mbit burst 3028b lat 776ms   Sent 4096 bytes 4 pkt "
    "(dropped 0, overlimits 6 requeues 0)   backlog 2Kb 2p requeues 0 qdisc clsact ffff: parent ffff:fff1   "
    "Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)   backlog 0b 0p requeues 0 "
)
# The same tbf a moment later, as text and as JSON.
TBF_TEXT = """qdisc tbf 10: root refcnt 2 rate 1Mbit burst 3028b lat 776ms
 Sent 3072 bytes 3 pkt (dropped 0, overlimits 5 requeues 0)
 backlog 3Kb 3p requeues 0
qdisc clsact ffff: parent ffff:fff1
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
"""
TBF_JSON = (
    '[{"kind":"tbf","handle":"10:","root":true,"refcnt":2,"options":{"rate":125000,"burst":3028,"lat":775776},'
    '"bytes":3072,"packets":3,"drops":0,"overlimits":5,"requeues":0,"backlog":3072,"qlen":3},'
    '{"kind":"clsact","handle":"ffff:","parent":"ffff:fff1","options":{},"bytes":0,"packets":0,"drops":0,'
    '"overlimits":0,"requeues":0,"backlog":0,"qlen":0}]'
)
# cable's tbf, its peak and size table, over DOCSIS-PIE.
CABLE = """qdisc tbf 10: root refcnt 2 rate 1Mbit burst 15625b peakrate 100Mbit minburst 3028b lat 675ms
 Sent 6168 bytes 6 pkt (dropped 2, overlimits 4 requeues 0)
 backlog 3084b 3p requeues 0
qdisc pie 20: parent 10:1 limit 10000p target 10ms tupdate 16ms alpha 2 beta 20
 Sent 6168 bytes 6 pkt (dropped 2, overlimits 0 requeues 0)
 backlog 3084b 3p requeues 0
  prob 0 delay 0us
  pkts_in 6 overlimit 0 dropped 0 maxq 1 ecn_mark 0
"""
# A cake router at the root; its tins' table repeats a backlog.
CAKE = """qdisc cake 10: root refcnt 2 bandwidth 950Kbit diffserv3 triple-isolate nonat nowash no-ack-filter split-gso rtt 100ms noatm overhead 18 mpu 64
 Sent 1024 bytes 1 pkt (dropped 0, overlimits 6 requeues 0)
 backlog 5Kb 5p requeues 0
 memory used: 11520b of 4Mb
 capacity estimate: 950Kbit
                   Bulk  Best Effort        Voice
  backlog            0b          5Kb           0b
  pkts                0            6            0
qdisc clsact ffff: parent ffff:fff1
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
"""
CAKE_JSON = (
    '[{"kind":"cake","handle":"10:","root":true,"refcnt":2,"options":{"bandwidth":118750,"diffserv":"diffserv3"},'
    '"bytes":1024,"packets":1,"drops":0,"overlimits":6,"requeues":0,"backlog":5120,"qlen":5,'
    '"tins":[{"backlog_bytes":5120}]}]'
)


class Parse(unittest.TestCase):
    def test_a_backlog_in_kib_reads_as_its_bytes(self):
        t, rest = TBF_LINE.split(" ", 1)
        qs = tcstats.parse(rest)
        tbf = tcstats.find(qs, "10:", "tbf")
        self.assertEqual((tbf.backlog, tbf.qlen, tbf.drops, tbf.sent), (2048, 2, 0, 4096))
        self.assertEqual(tbf.rate, 125000)
        self.assertIsNone(tbf.parent)
        self.assertEqual(tcstats.find(qs, "ffff:").backlog, 0)

    def test_sizes_and_rates_take_their_units(self):
        self.assertEqual(tcstats.size("1072218b"), 1072218)
        self.assertEqual(tcstats.size("3Kb"), 3072)
        self.assertEqual(tcstats.size("2Mb"), 2 * 1024 * 1024)
        self.assertEqual(tcstats.rate("8Kbit"), 1000)
        self.assertEqual(tcstats.rate("9846Kbit"), 9846000 / 8)
        self.assertEqual(tcstats.rate("100Mbit"), 12.5e6)
        with self.assertRaises(ValueError):
            tcstats.size("3K")

    def test_text_and_json_agree(self):
        self.assertEqual(tcstats.parse(TBF_TEXT), tcstats.parse(TBF_JSON))
        self.assertEqual(tcstats.parse(CAKE)[0], tcstats.parse(CAKE_JSON)[0])

    def test_a_child_has_its_parent_and_the_root_its_rate(self):
        tbf, pie = tcstats.parse(CABLE)
        self.assertEqual((tbf.kind, tbf.handle, tbf.parent, tbf.rate, tbf.drops, tbf.backlog), ("tbf", "10:", None, 125000, 2, 3084))
        self.assertEqual((pie.kind, pie.handle, pie.parent, pie.rate), ("pie", "20:", "10:1", None))

    def test_cake_has_its_bandwidth_and_root_backlog(self):
        cake = tcstats.parse(CAKE)[0]
        self.assertEqual((cake.kind, cake.backlog, cake.qlen, cake.rate), ("cake", 5120, 5, 118750))

    def test_samples_skip_a_torn_line(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "tc-0.log")
            with open(p, "w") as f:
                f.write(f"1794840471700 {TBF_JSON}\n")
                f.write(TBF_LINE + "\n")
                f.write('1794840471800 [{"kind":"tbf","handle"\n')
                f.write("1794840471900 qdisc tbf 10: root refcnt 2 rate 1Mbit burst\n")
            got = tcstats.samples(p)
            self.assertEqual([t for t, _ in got], [1794840471604, 1794840471700])
            self.assertEqual([tcstats.find(qs, "10:").backlog for _, qs in got], [2048, 3072])


# The trace class's tree: tracelink's program drops in the prio and holds
# packets in the fq.
TRACE_JSON = (
    '[{"kind":"prio","handle":"10:","root":true,"refcnt":2,"options":{"bands":2},"bytes":572980,"packets":566,'
    '"drops":893,"overlimits":0,"requeues":0,"backlog":22308,"qlen":22},'
    '{"kind":"fq","handle":"20:","parent":"10:1","options":{"limit":100000},"bytes":572980,"packets":566,'
    '"drops":0,"overlimits":0,"requeues":0,"backlog":22308,"qlen":22}]'
)
# run.sh's tree before the ifb: netem at the root, the tbf its child.
NETEM_ROOT = """qdisc netem 1: root refcnt 2 limit 100000 delay 20ms
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
qdisc tbf 10: parent 1:1 rate 10Mbit burst 3028b lat 99.9ms
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
"""


# qdisc-<i>.txt: every device's qdiscs in detail (`tc -s -d qdisc show`),
# eth0's root with its size table, the delay's ifb and the slow downlink's.
ALL_DEVICES = """qdisc noqueue 0: dev lo root refcnt 2
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
qdisc tbf 10: dev eth0 root refcnt 2 rate 10Mbit burst 3028b/1 mpu 0b lat 21.6ms linklayer ethernet
 overhead 4 mpu 64 mtu 2048 tsize 2048
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
qdisc clsact ffff: dev eth0 parent ffff:fff1
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
qdisc netem 8001: dev ifbd root refcnt 2 limit 100000 delay 20ms
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
qdisc tbf 857b: dev ifbs root refcnt 2 rate 1Mbit burst 3028b/1 mpu 0b lat 75.8ms linklayer ethernet
 Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)
 backlog 0b 0p requeues 0
"""


def run_dir(d, links, lines, qdisc=None):
    if links is not None:
        with open(os.path.join(d, "links"), "w") as f:
            f.write(links)
    with open(os.path.join(d, "tc-0.log"), "w") as f:
        for t, text in lines:
            f.write(f"{t} {text}\n")
    if qdisc is not None:
        with open(os.path.join(d, "qdisc-0.txt"), "w") as f:
            f.write(qdisc)
    return d


class Uplink(unittest.TestCase):
    def test_the_queue_is_eth0s_root_of_the_class_kind(self):
        for spec, kind in (("tbf", "tbf"), ("cable", "tbf"), ("cable,aqm=fifo,slot=4-4", "tbf"),
                           ("cable,router=fq_codel", "tbf"), ("dsl,line=vdsl,router=cake", "cake"),
                           ("fiber", "tbf"), ("trace,file=up.mm,limit=30000", "prio")):
            self.assertEqual(tcstats.queue_kind(spec), kind, spec)

    def test_a_class_s_queue_is_read_from_its_links_line(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 cable,router=cake 10 20 100\nnode 1 tbf 20 10 100\n", [(1000, CAKE_JSON)])
            spec, got = tcstats.uplink(d, 0)
            self.assertEqual(spec, "cable,router=cake")
            self.assertEqual([(t, q.kind, q.backlog, q.rate) for t, q in got], [(1000, "cake", 5120, 118750)])
            self.assertEqual(tcstats.link_spec(d, 1), "tbf")

    def test_a_trace_s_queue_has_its_drops_and_no_rate(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 trace,file=up.mm 10 20 100\n", [(1000, TRACE_JSON)])
            spec, got = tcstats.uplink(d, 0)
            q = got[0][1]
            self.assertEqual((q.kind, q.drops, q.backlog, q.rate), ("prio", 893, 22308, None))

    def test_a_log_without_the_class_s_queue_is_an_error(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 cable,router=cake 10 20 100\n", [(1000, TBF_JSON)])
            with self.assertRaisesRegex(LookupError, "cable,router=cake link has no cake 10:"):
                tcstats.uplink(d, 0)

    def test_a_run_before_the_links_file_had_a_tbf(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, None, [(1000, TBF_JSON)])
            spec, got = tcstats.uplink(d, 0)
            self.assertEqual((spec, got[0][1].backlog), ("tbf", 3072))

    def test_the_bench_takes_frames_where_the_class_s_qdisc_is_the_root(self):
        import bench

        with tempfile.TemporaryDirectory() as d:
            self.assertTrue(bench.frames(run_dir(d, None, [], qdisc=CAKE)))
            self.assertTrue(bench.frames(run_dir(d, None, [], qdisc=CABLE)))
            self.assertFalse(bench.frames(run_dir(d, None, [], qdisc=NETEM_ROOT)))
            self.assertTrue(bench.frames(run_dir(d, None, [], qdisc=ALL_DEVICES)))

    def test_every_device_s_qdiscs_name_their_device(self):
        qs = tcstats.parse_text(ALL_DEVICES)
        self.assertEqual([(q.dev, q.kind, q.handle, q.parent) for q in qs], [
            ("lo", "noqueue", "0:", None), ("eth0", "tbf", "10:", None), ("eth0", "clsact", "ffff:", "ffff:fff1"),
            ("ifbd", "netem", "8001:", None), ("ifbs", "tbf", "857b:", None)])
        self.assertEqual(tcstats.find(qs, "857b:", dev="ifbs").rate, 125000)
        self.assertIsNone(tcstats.find(qs, "10:", dev="ifbs"))


if __name__ == "__main__":
    unittest.main()
