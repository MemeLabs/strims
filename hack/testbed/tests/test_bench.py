#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""bench.phases and benchsuite.py phases over a fixture of a seed and its
links' records, probes and samples with known answers: each phase's
one-way delays, probe round trips, buffered amounts and windows, a
sample's buffered amount and cwnd each in their own column whichever the
sample holds (the testbed's data channel only its buffered amount, the
window model's both, a TCP socket only its cwnd); a link not asked for
and a sample outside every phase left out.

  python3 hack/testbed/tests/test_bench.py
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TESTBED = HERE.parent
sys.path.insert(0, str(TESTBED))

import bench  # noqa: E402


def rec(frm, at_ms, owd_ms, k=0):
    return {"ev": "r", "from": frm, "k": k, "at": at_ms * 1000, "got": (at_ms + owd_ms) * 1000}


def sample(to, t_ms, **amounts):
    return {"ev": "s", "to": to, "t": t_ms * 1000, **amounts}


def seed(samples):
    # Probes' echoes from links 1 and 3 (3 not asked for), and samples.
    return [rec(1, 1000, 40, k=2), rec(3, 1000, 900, k=2), rec(1, 9000, 60, k=2)] + samples


def child(owd_ms):
    return [rec(0, 2000, owd_ms), rec(0, 10000, owd_ms + 10), rec(0, 20000, 999)]


TESTBED_DC = seed([sample(1, 1000, buffered=8192), sample(1, 2000, buffered=16384),
                   sample(1, 9000, buffered=4096), sample(3, 1000, buffered=999999)])
MODEL_DC = seed([sample(1, 1000, buffered=2048, cwnd=65536, in_flight=1, upq=0),
                 sample(1, 9000, buffered=1024, cwnd=131072, in_flight=1, upq=0)])
TCP = seed([sample(1, 1000, unsent=0, unacked=0, cwnd=32768, srtt=1),
            sample(1, 9000, unsent=0, unacked=0, cwnd=49152, srtt=1)])
EDGES = [0, 8, 16]


class Phases(unittest.TestCase):
    def test_testbed_channel(self):
        got = bench.phases({0: TESTBED_DC, 1: child(30), 3: child(500)}, EDGES, [1])
        self.assertEqual([(lo, hi) for lo, hi, _ in got], [(0, 8), (8, 16)])
        first, second = got[0][2], got[1][2]
        self.assertEqual(first, {"owd": [30.0], "rtt": [40.0], "buffered": [8.0, 16.0], "cwnd": []})
        self.assertEqual(second, {"owd": [40.0], "rtt": [60.0], "buffered": [4.0], "cwnd": []})

    def test_model_channel(self):
        got = bench.phases({0: MODEL_DC, 1: child(30)}, EDGES, [1])
        self.assertEqual(got[0][2]["buffered"], [2.0])
        self.assertEqual(got[0][2]["cwnd"], [64.0])
        self.assertEqual(got[1][2]["buffered"], [1.0])
        self.assertEqual(got[1][2]["cwnd"], [128.0])

    def test_tcp(self):
        got = bench.phases({0: TCP, 1: child(30)}, EDGES, [1])
        self.assertEqual([m["cwnd"] for _, _, m in got], [[32.0], [48.0]])
        self.assertEqual([m["buffered"] for _, _, m in got], [[], []])


class Cli(unittest.TestCase):
    def test_columns(self):
        with tempfile.TemporaryDirectory() as d:
            run = Path(d, "r1", "case")
            for sub, evs in (("", TESTBED_DC), ("arm", MODEL_DC)):
                (run / sub).mkdir(parents=True, exist_ok=True)
                for i, node in ((0, evs), (1, child(30))):
                    (run / sub / f"node-{i}.jsonl").write_text("".join(json.dumps(e) + "\n" for e in node))
            out = subprocess.run([sys.executable, str(TESTBED / "benchsuite.py"), "phases", "--root", d,
                                  "--edges", "0,8,16", "--links", "1", "case", "arm"],
                                 capture_output=True, text=True, check=True).stdout.splitlines()
        cells = lambda line: [c.split() for c in line.split(" | ")[1:]]
        self.assertEqual(cells(out[1]), [["owd", "p50/p90", "rtt", "p50/p90", "buffered", "p50/max",
                                          "cwnd", "p50/max"]] * 2)
        # The testbed's channel: its buffered amount, no window; the
        # model's: both, each in its own column.
        self.assertEqual(cells(out[2]), [["30.0/30.0", "40.0/40.0", "16.0/16.0", "-/-"],
                                         ["30.0/30.0", "40.0/40.0", "2.0/2.0", "64.0/64.0"]])
        self.assertEqual(cells(out[3]), [["40.0/40.0", "60.0/60.0", "4.0/4.0", "-/-"],
                                         ["40.0/40.0", "60.0/60.0", "1.0/1.0", "128/128"]])


if __name__ == "__main__":
    unittest.main()
