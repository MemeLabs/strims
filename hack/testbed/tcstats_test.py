#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""tcstats and its readers on tc's output as iproute2 6.19 (the testbed
image's) prints it. testdata/tc holds captures (testdata/tc/capture.sh) of
each link class's tree as run.sh builds a node's, at 100 kbit/s under a
UDP flood so its queue stands and drops: `tc -s qdisc show dev <modem's>`
(<class>.txt: eth0's, or ifbm's for a class with slots), its JSON
(<class>.json), every device's JSON (<class>.every.json, tc-<i>.log's
form) and every device's detailed text (<class>.all, qdisc-<i>.txt's
form); tbf-kib, a tbf at 1 Mbit/s whose
3 KiB backlog tc's text prints as 3Kb; and dsl-cake-flood.json, a dsl
router's cake at 1 Mbit/s 2 s into a flood of 1400-byte payloads.

    python3 hack/testbed/tcstats_test.py
"""

import contextlib
import io
import os
import sys
import tempfile
import unittest
from pathlib import Path

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "capture"))
import bench  # noqa: E402
import tcstats  # noqa: E402

DATA = os.path.join(HERE, "testdata", "tc")
# Each capture's spec, as the run's `links` file names it.
CLASSES = {
    "tbf": "tbf",
    "cable": "cable",
    "cable_aqm_fifo": "cable,aqm=fifo",
    "dsl": "dsl",
    "dsl_line_vdsl": "dsl,line=vdsl",
    "fiber": "fiber",
    "cable_router_fq_codel": "cable,router=fq_codel",
    "dsl_router_cake": "dsl,router=cake",
    "trace": "trace,file=slow.us,format=us",
}


def fixture(name):
    with open(os.path.join(DATA, name)) as f:
        return f.read()


def run_dir(d, links, lines, qdisc=None):
    """A run's files: its `links` line(s), tc-0.log's (stamp, text) lines
    and qdisc-0.txt."""
    if links is not None:
        with open(os.path.join(d, "links"), "w") as f:
            f.write(links)
    with open(os.path.join(d, "tc-0.log"), "w") as f:
        for t, text in lines:
            f.write(f"{t} {' '.join(text.split())}\n")
    if qdisc is not None:
        with open(os.path.join(d, "qdisc-0.txt"), "w") as f:
            f.write(qdisc)
    return d


class Parse(unittest.TestCase):
    def test_every_class_reads_alike_from_its_text_and_its_json(self):
        for name, spec in CLASSES.items():
            with self.subTest(name):
                text = tcstats.parse(fixture(f"{name}.txt"))
                self.assertEqual(text, tcstats.parse(fixture(f"{name}.json")))
                q = tcstats.find(text, "10:")
                self.assertEqual((q.kind, q.parent), (tcstats.queue_kind(spec), None))
                self.assertGreater(q.qlen, 0)
                self.assertGreater(q.drops, 0)
                self.assertGreaterEqual(q.held, q.backlog)

    def test_each_class_has_its_tree(self):
        kinds = {name: [(q.kind, q.handle, q.parent) for q in tcstats.parse(fixture(f"{name}.json"))]
                 for name in CLASSES}
        self.assertEqual(kinds["cable"][:2], [("tbf", "10:", None), ("pie", "20:", "10:1")])
        self.assertEqual(kinds["cable_router_fq_codel"][:2], [("tbf", "10:", None), ("fq_codel", "20:", "10:1")])
        self.assertEqual(kinds["trace"][:2], [("prio", "10:", None), ("fq", "20:", "10:1")])
        self.assertEqual(kinds["dsl_router_cake"][0], ("cake", "10:", None))
        # The shaping rates: VDSL2's 64/65 of 100 kbit/s, a router's 95 %.
        rates = {name: tcstats.find(tcstats.parse(fixture(f"{name}.json")), "10:").rate for name in CLASSES}
        self.assertEqual((rates["tbf"], rates["dsl_line_vdsl"], rates["cable_router_fq_codel"]), (12500, 12250, 11875))
        self.assertEqual((rates["dsl_router_cake"], rates["trace"]), (11875, None))
        # A tbf over a qdisc holds what the qdisc does.
        tbf, child = tcstats.parse(fixture("cable_router_fq_codel.json"))[:2]
        self.assertEqual((tbf.backlog, tbf.qlen, tbf.drops), (child.backlog, child.qlen, child.drops))

    def test_every_device_s_detailed_qdiscs_name_their_device(self):
        for name, spec in CLASSES.items():
            with self.subTest(name):
                dev = tcstats.modem_dev(spec)
                for form in ("all", "every.json"):
                    qs = tcstats.parse(fixture(f"{name}.{form}"))
                    devs = {"lo", "eth0", "ifbd"} | ({"ifbm"} if dev == "ifbm" else set())
                    self.assertEqual({q.dev for q in qs}, devs)
                    q = tcstats.find(qs, "10:", dev=dev)
                    self.assertIsNone(q.parent)
                    self.assertEqual(q.kind, tcstats.queue_kind(spec))
                    self.assertEqual([x for x in qs if x.handle == "10:"], [q])
        # A class with slots: eth0's root is the prio that marks and hands
        # the shaper on ifbm what the delay returns.
        qs = tcstats.parse(fixture("cable.every.json"))
        self.assertEqual([(q.kind, q.handle) for q in qs if q.dev == "eth0"][:2], [("prio", "1:"), ("netem", "30:")])
        # -d adds a class's size table, which leaves the stats alone.
        qs = tcstats.parse_text(fixture("cable.all"))
        self.assertIn("overhead 4 mpu 64 mtu 2047 tsize 2048", fixture("cable.all"))
        self.assertEqual(tcstats.find(qs, "10:").rate, 12500)

    def test_a_backlog_in_kib_reads_as_its_bytes(self):
        text, json_ = tcstats.parse(fixture("tbf-kib.txt")), tcstats.parse(fixture("tbf-kib.json"))
        self.assertIn("backlog 3Kb 3p", fixture("tbf-kib.txt"))
        self.assertEqual(text, json_)
        self.assertEqual(tcstats.find(text, "10:", "tbf").backlog, 3072)
        # tc's joined text, as old runs' tc-0.log has it: the clsact after
        # the tbf holds 0b, which a lazy regex once read for the tbf's.
        joined = " ".join(fixture("tbf-kib.txt").split())
        self.assertEqual(tcstats.find(tcstats.parse(joined), "10:").backlog, 3072)

    def test_sizes_and_rates_take_their_units(self):
        self.assertEqual(tcstats.size("1072218b"), 1072218)
        self.assertEqual(tcstats.size("3Kb"), 3072)
        self.assertEqual(tcstats.size("2Mb"), 2 * 1024 * 1024)
        self.assertEqual(tcstats.rate("8Kbit"), 1000)
        self.assertEqual(tcstats.rate("9846Kbit"), 9846000 / 8)
        self.assertEqual(tcstats.rate("100Mbit"), 12.5e6)
        with self.assertRaises(ValueError):
            tcstats.size("3K")

    def test_cake_holds_what_its_shaper_charges(self):
        # A dsl router's cake under a flood of 1442-byte skbs (1428 IP
        # bytes): each costs 1428 + 40 in ATM cells, 31 of 53 bytes, so its
        # queue drains at 1442 / 1643 of its bandwidth in skb bytes (0.879
        # measured over 10 s).
        cake = tcstats.find(tcstats.parse(fixture("dsl-cake-flood.json")), "10:", "cake")
        self.assertEqual(cake.backlog, cake.qlen * 1442)
        self.assertEqual(cake.held, cake.qlen * 1643)
        # Its text has the same options as its JSON.
        self.assertEqual(tcstats.parse(fixture("dsl_router_cake.txt")), tcstats.parse(fixture("dsl_router_cake.json")))
        self.assertGreater(tcstats.find(tcstats.parse(fixture("dsl_router_cake.txt")), "10:").held, cake.backlog // cake.qlen)


class Samples(unittest.TestCase):
    def test_samples_keep_file_order_and_skip_a_torn_line(self):
        a, b = fixture("tbf.json").strip(), fixture("tbf-kib.json").strip()
        with tempfile.TemporaryDirectory() as d:
            # The second's stamp is late (date's lost leading zero), the
            # third's line text, the fourth's torn.
            run_dir(d, None, [(1794840471700, a), (1794840471604, b), (1794840471800, fixture("tbf-kib.txt")),
                              (1794840471900, '[{"kind":"tbf","handle"')])
            got = tcstats.samples(os.path.join(d, "tc-0.log"))
            self.assertEqual([t for t, _ in got], [1794840471700, 1794840471604, 1794840471800])
            self.assertEqual([tcstats.find(qs, "10:").backlog for _, qs in got], [29808, 3072, 3072])


class Uplink(unittest.TestCase):
    def test_the_queue_is_eth0s_root_of_the_class_kind(self):
        for spec, kind in (("tbf", "tbf"), ("cable", "tbf"), ("cable,aqm=fifo,slot=4-4", "tbf"),
                           ("cable,router=fq_codel", "tbf"), ("dsl,line=vdsl,router=cake", "cake"),
                           ("fiber", "tbf"), ("trace,file=up.mm,limit=30000", "prio")):
            self.assertEqual(tcstats.queue_kind(spec), kind, spec)

    def test_each_class_s_queue_is_read_from_its_links_line(self):
        for name, spec in CLASSES.items():
            # tc-<i>.log's every device, and an older run's one device.
            for form in ("every.json", "json"):
                with self.subTest(f"{name}.{form}"), tempfile.TemporaryDirectory() as d:
                    run_dir(d, f"node 0 {spec} 10 20 100\nnode 1 tbf 20 10 100\n", [(1000, fixture(f"{name}.{form}"))])
                    got_spec, got = tcstats.uplink(d, 0)
                    self.assertEqual((got_spec, got[0][1].kind), (spec, tcstats.queue_kind(spec)))
                    if form == "every.json":
                        self.assertEqual(got[0][1].dev, tcstats.modem_dev(spec))
                    self.assertEqual(tcstats.link_spec(d, 1), "tbf")

    def test_the_modem_is_on_ifbm_with_slots(self):
        for spec, dev in (("tbf", "eth0"), ("cable", "ifbm"), ("cable,slot=off", "eth0"),
                          ("cable,aqm=fifo", "ifbm"), ("tbf,slot=1-3", "ifbm"), ("dsl", "eth0"),
                          ("fiber,slot=off", "eth0"), ("trace,file=x,slot=2", "ifbm")):
            self.assertEqual(tcstats.modem_dev(spec), dev, spec)
        # A slotted class's 10: on a device that isn't its modem's isn't its
        # queue.
        with tempfile.TemporaryDirectory() as d:
            qs = fixture("tbf.every.json").replace('"dev":"eth0"', '"dev":"ifbx"')
            run_dir(d, "node 0 tbf 10 20 100\n", [(1000, qs)])
            with self.assertRaisesRegex(LookupError, "tbf link has no tbf 10: on eth0"):
                tcstats.uplink(d, 0)

    def test_a_log_without_the_class_s_queue_is_an_error(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 cable,router=cake 10 20 100\n", [(1000, fixture("tbf.json"))])
            with self.assertRaisesRegex(LookupError, "cable,router=cake link has no cake 10: on ifbm"):
                tcstats.uplink(d, 0)

    def test_a_run_before_the_links_file_had_a_tbf(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, None, [(1000, fixture("tbf-kib.json"))])
            spec, got = tcstats.uplink(d, 0)
            self.assertEqual((spec, got[0][1].backlog), ("tbf", 3072))


class Readers(unittest.TestCase):
    """What the tools make of a run, and what they say where they can't."""

    def test_upq_times_a_queue_by_its_rate_and_a_trace_s_has_none(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 dsl,router=cake 0.1 20 100\n", [(2000, fixture("dsl_router_cake.json"))])
            q = tcstats.find(tcstats.parse(fixture("dsl_router_cake.json")), "10:")
            self.assertEqual(bench.tc_queue(d, 1000), [(1_000_000, q.held / q.rate * 1000)])
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 trace,file=slow.us 0.1 20 100\n", [(2000, fixture("trace.json"))])
            with contextlib.redirect_stderr(io.StringIO()) as err:
                self.assertEqual(bench.tc_queue(d, 1000), [])
            self.assertIn("NOTE: ", err.getvalue())
            self.assertIn("has no rate", err.getvalue())
            # Its drops are the prio's.
            prio = tcstats.find(tcstats.parse(fixture("trace.json")), "10:", "prio")
            self.assertGreater(prio.drops, 0)
            self.assertEqual(bench.tc_drops(d), prio.drops)

    def test_a_reader_says_so_where_the_log_lacks_the_class_s_queue(self):
        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 cable,router=cake 10 20 100\n", [(1000, fixture("tbf.json"))])
            with contextlib.redirect_stderr(io.StringIO()) as err:
                self.assertIsNone(bench.tc_drops(d))
                self.assertEqual(bench.tc_queue(d, 0), [])
            self.assertEqual(err.getvalue().count("NOTE: "), 2)

    def test_analyze_s_delays_say_so_for_a_trace(self):
        import analyze

        with tempfile.TemporaryDirectory() as d:
            run_dir(d, "node 0 trace,file=slow.us 0.1 20 100\n", [(2000, fixture("trace.json"))])
            with contextlib.redirect_stderr(io.StringIO()) as err:
                self.assertEqual(analyze.tc_queue(Path(d)), [])
            self.assertIn("no delay fields", err.getvalue())
            run_dir(d, "node 0 cable 0.1 20 100\n", [(2000, fixture("cable.json"))])
            q = tcstats.find(tcstats.parse(fixture("cable.json")), "10:")
            self.assertEqual(analyze.tc_queue(Path(d)), [(2000, 1000 * q.held / q.rate)])

    def test_the_bench_takes_frames_where_the_class_s_qdisc_is_the_root(self):
        netem_root = fixture("tbf.txt").replace("qdisc tbf 10: root", "qdisc tbf 10: parent 1:1")
        netem_root = ("qdisc netem 1: root refcnt 2 limit 100000 delay 20ms\n"
                      " Sent 0 bytes 0 pkt (dropped 0, overlimits 0 requeues 0)\n backlog 0b 0p requeues 0\n"
                      + netem_root)
        with tempfile.TemporaryDirectory() as d:
            for name in CLASSES:
                self.assertTrue(bench.frames(run_dir(d, None, [], qdisc=fixture(f"{name}.all"))), name)
                self.assertTrue(bench.frames(run_dir(d, None, [], qdisc=fixture(f"{name}.txt"))), name)
            self.assertFalse(bench.frames(run_dir(d, None, [], qdisc=netem_root)))
            # A file cut short takes its header lines' word for it, and says
            # so: every class's, in the one-device form and in every
            # device's, which run.sh writes now.
            for name in CLASSES:
                for form in ("txt", "all"):
                    text = fixture(f"{name}.{form}")
                    if "\nqdisc " not in text[text.index(" 10: "):]:
                        continue  # the modem's device holds the root alone
                    # Cut in the qdisc after the root, past its header.
                    second = text.index("\nqdisc ", text.index(" 10: ")) + 1
                    cut = text[:text.index("\n", second) + 1] + " Sent 1"
                    with self.subTest(f"{name}.{form} cut"), contextlib.redirect_stderr(io.StringIO()) as err:
                        self.assertTrue(bench.frames(run_dir(d, None, [], qdisc=cut)))
                        self.assertIn("NOTE: ", err.getvalue())
            cut = netem_root[:netem_root.index("qdisc tbf") + 40]
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertFalse(bench.frames(run_dir(d, None, [], qdisc=cut)))


if __name__ == "__main__":
    unittest.main()
