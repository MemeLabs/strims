#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The quantile method in Python (simlab.sketch) and SQL (scoring.sql's
sketch_floor, sketch_quantile, sketch_quantiles) against strims-base's
sketch_cases.tsv, which strims_base::stats::sketch's tests hold Rust to:
the three give the same value, bit for bit, at the grid's edges and over
random floats and integers, and the same percentiles of the same values.

  Cases        sketch.floor and sketch_floor on every case, and the cases'
               percentiles by sketch.quantile, sketch_quantile and
               sketch_quantiles; the cases are what an exact reading (in
               fractions, from the grid's definition) gives
  Rank         sketch_quantile's rank, DuckDB's quantile_disc at p / 100 as
               a DECIMAL, is the integer nearest rank for every n to 1000
               and p 0 to 100, scalar and listed

  python3 hack/simlab/tests/test_sketch.py
  python3 hack/simlab/tests/test_sketch.py write   (the cases, again)
"""

import math
import random
import re
import struct
import sys
import unittest
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__" and sys.argv[1:] != ["write"]:
    bootstrap.engine(__file__)

from simlab import catalog, sketch  # noqa: E402

CASES = HACK.parent / "crates" / "strims-base" / "src" / "stats" / "sketch_cases.tsv"
# The percentiles each case of values is read at.
PS = (0, 1, 5, 7, 10, 25, 50, 55, 75, 90, 95, 99, 100)


def bits(x):
    return struct.unpack("<Q", struct.pack("<d", x))[0]


def unbits(b):
    return struct.unpack("<d", struct.pack("<Q", b))[0]


def exact(x):
    """x (an int or a finite float) read from the grid's definition, in
    fractions: the greatest of 0, ±2^e (1 + m/SUB) not above it."""
    x = Fraction(x)
    if x == 0:
        return x
    a = abs(x)
    e = a.numerator.bit_length() - a.denominator.bit_length()
    while Fraction(2) ** e > a:
        e -= 1
    while Fraction(2) ** (e + 1) <= a:
        e += 1
    step = Fraction(2) ** e / sketch.SUB
    lo = Fraction(2) ** e + math.floor((a - Fraction(2) ** e) / step) * step
    if x > 0:
        return lo
    return -(a if lo == a else lo + step)


def as_float(f):
    """A fraction as a float, one past the largest as an infinity."""
    try:
        return float(f)
    except OverflowError:
        return math.inf if f > 0 else -math.inf


def write():
    """sketch_cases.tsv: the floats and integers at the grid's edges and at
    random, read exactly, and lists of them with their percentiles PS."""
    rng = random.Random(2010)
    floats = []
    for field in (0, 1, 2, 3, 20, 500, 1000, 1013, 1020, 1022, 1023, 1024, 1025, 1026, 1030, 1032, 1040, 1052,
                  1053, 1075, 1100, 1500, 2000, 2045, 2046):
        subs = range(128) if field in (1023, 1032) else (0, 1, 2, 63, 64, 126, 127)
        for m in subs:
            lo = field << 52 | m << 45
            floats += [lo, lo + 1, lo + (1 << 45) - 1] + ([lo - 1] if lo else [])
    floats += [1, 2, 3, 0xFF, 0x100, 0x101, 0x1FF, 0x16D, (1 << 52) - 1]
    floats += [rng.getrandbits(63) for _ in range(600)]
    floats += [bits(rng.uniform(0, 2.0 ** rng.randrange(0, 22))) for _ in range(600)]
    floats = [b for b in floats if math.isfinite(unbits(b))]
    floats += [b | 1 << 63 for b in floats[::2]]
    ints = list(range(600))
    ints += [(1 << k) + d for k in range(8, 63) for d in (-2, -1, 0, 1, 2)]
    ints += [rng.getrandbits(rng.randrange(9, 54)) for _ in range(600)]
    ints += [rng.getrandbits(63) for _ in range(100)]
    ints += [-x for x in ints[::3] if x]
    lines = ["# strims_base::stats::sketch's cases, which its tests, simlab.sketch's and scoring.sql's sketch_*",
             "# (hack/simlab/tests/test_sketch.py, which writes them) all read the same:",
             "#   f <x> <floor>              a float and its reading, as their bits in hex",
             "#   i <x> <floor>              an integer and its reading",
             "#   qf <x,...> <reading,...>   floats (bits) and their percentiles' readings at",
             f"#                              p = {', '.join(map(str, PS))}",
             "#   qi <x,...> <reading,...>   integers and theirs"]
    for b in sorted(set(floats), key=lambda b: (b >> 63, b)):
        lines.append(f"f\t{b:016x}\t{bits(as_float(exact(unbits(b)))):016x}")
    for x in sorted(set(ints), key=lambda x: (x < 0, abs(x))):
        lines.append(f"i\t{x}\t{exact(x)}")
    for k in range(160):
        n = rng.choice([1, 2, 3, 4, 5, 7, 10, 16, 33, 64, 100])
        if k % 2:
            span = 2.0 ** rng.randrange(0, 20)
            v = [rng.uniform(-span / 4 if k % 3 == 0 else 0, span) for _ in range(n)]
            vs = sorted(v)
            r = [bits(float(exact(vs[sketch.rank(p, n) - 1]))) for p in PS]
            lines.append(f"qf\t{','.join(f'{bits(x):016x}' for x in v)}\t{','.join(f'{x:016x}' for x in r)}")
        else:
            span = 1 << rng.randrange(4, 40)
            v = [rng.randrange(-span // 4 if k % 3 == 0 else 0, span) for _ in range(n)]
            vs = sorted(v)
            r = [exact(vs[sketch.rank(p, n) - 1]) for p in PS]
            lines.append(f"qi\t{','.join(map(str, v))}\t{','.join(map(str, r))}")
    CASES.write_text("\n".join(lines) + "\n")
    print(f"wrote {CASES}: {len(lines)} lines")


def cases():
    """sketch_cases.tsv's lines, split: {kind: [fields]}."""
    out = {"f": [], "i": [], "qf": [], "qi": []}
    for line in CASES.read_text().splitlines():
        if line and not line.startswith("#"):
            kind, *rest = line.split("\t")
            out[kind].append(rest)
    return out


def sketch_sql():
    """scoring.sql's sketch_* macros, their constants filled in."""
    consts = catalog.scoring_consts()
    text = catalog.CONST.sub(lambda m: str(consts[m.group(1)]), catalog.SCORING.read_text())
    return [m.group(0) for m in re.finditer(r"^CREATE OR REPLACE MACRO sketch_\w+.*?;$", text, re.M | re.S)]


def connect():
    import duckdb

    conn = duckdb.connect()
    for s in sketch_sql():
        conn.execute(s)
    return conn


def same(a, b):
    """Equal to the bit (the sign of a zero, and a NaN, too)."""
    return bits(float(a)) == bits(float(b)) and type(a) is type(b)


class Cases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cases = cases()
        cls.conn = connect()

    def test_there_are_cases_of_each(self):
        self.assertEqual({k: len(v) > 1000 for k, v in self.cases.items() if k in "fi"}, {"f": True, "i": True})
        self.assertGreater(len(self.cases["qf"]) + len(self.cases["qi"]), 100)

    def test_python_reads_every_case(self):
        for x, f in self.cases["f"]:
            self.assertTrue(same(sketch.floor(unbits(int(x, 16))), unbits(int(f, 16))), x)
        for x, f in self.cases["i"]:
            self.assertEqual(sketch.floor(int(x)), int(f), x)
            self.assertIsInstance(sketch.floor(int(x)), int)

    def test_sql_reads_every_case(self):
        xs = [unbits(int(x, 16)) for x, _ in self.cases["f"]]
        got = self.conn.execute("SELECT list(sketch_floor(x) ORDER BY i) FROM (SELECT unnest(?::DOUBLE[]) x, "
                                "generate_subscripts(?::DOUBLE[], 1) i)", [xs, xs]).fetchone()[0]
        bad = [(x, g, f) for (x, f), g in zip(self.cases["f"], got) if bits(g) != int(f, 16)]
        self.assertEqual(bad, [])
        # Integers to 2^53, as BIGINTs: as the floats they are.
        ints = [(int(x), int(f)) for x, f in self.cases["i"] if abs(int(x)) <= 1 << 53]
        xs = [x for x, _ in ints]
        got = self.conn.execute("SELECT list(sketch_floor(x) ORDER BY i) FROM (SELECT unnest(?::BIGINT[]) x, "
                                "generate_subscripts(?::BIGINT[], 1) i)", [xs, xs]).fetchone()[0]
        self.assertEqual([(x, g) for (x, f), g in zip(ints, got) if g != f], [])
        self.assertEqual(self.conn.execute("SELECT sketch_floor(NULL), sketch_floor(-0.0::DOUBLE)::VARCHAR, "
                                           "sketch_floor('inf'::DOUBLE), sketch_floor('-inf'::DOUBLE), "
                                           "isnan(sketch_floor('nan'::DOUBLE))").fetchone(),
                         (None, "0.0", math.inf, -math.inf, True))

    def test_the_infinities_and_nan_read_as_they_are(self):
        self.assertEqual((sketch.floor(math.inf), sketch.floor(-math.inf)), (math.inf, -math.inf))
        self.assertTrue(math.isnan(sketch.floor(math.nan)))
        self.assertTrue(same(sketch.floor(-0.0), 0.0))
        self.assertEqual(sketch.floor(-1.7976931348623157e308), -math.inf)

    def test_percentiles_of_every_case(self):
        rows = []
        for kind in ("qf", "qi"):
            for v, wants in self.cases[kind]:
                if kind == "qf":
                    v, wants = [unbits(int(x, 16)) for x in v.split(",")], [unbits(int(x, 16)) for x in wants.split(",")]
                else:
                    v, wants = [int(x) for x in v.split(",")], [int(x) for x in wants.split(",")]
                for p, want in zip(PS, wants, strict=True):
                    self.assertTrue(same(sketch.quantile(v, p), want), (kind, p, v))
                    rows.append((p, [float(x) for x in v], float(want)))
        # sketch_quantile at each p over each case's values, as rows.
        conn = self.conn
        conn.execute("CREATE TEMP TABLE cases AS SELECT row_number() OVER () AS c, * "
                     "FROM (SELECT unnest(?, recursive := true))",
                     [[{"p": p, "xs": xs, "want": w} for p, xs, w in rows]])
        bad = conn.execute(f"""
            WITH v AS (SELECT c, p, want, unnest(xs) AS x FROM cases)
            SELECT c, p, any_value(want) AS want,
                   CASE p WHEN 0 THEN sketch_quantile(x, 0) WHEN 1 THEN sketch_quantile(x, 1)
                          WHEN 5 THEN sketch_quantile(x, 5) WHEN 7 THEN sketch_quantile(x, 7)
                          WHEN 10 THEN sketch_quantile(x, 10) WHEN 25 THEN sketch_quantile(x, 25)
                          WHEN 50 THEN sketch_quantile(x, 50) WHEN 55 THEN sketch_quantile(x, 55)
                          WHEN 75 THEN sketch_quantile(x, 75) WHEN 90 THEN sketch_quantile(x, 90)
                          WHEN 95 THEN sketch_quantile(x, 95) WHEN 99 THEN sketch_quantile(x, 99)
                          WHEN 100 THEN sketch_quantile(x, 100) END AS got,
                   list_position({list(PS)}, p) AS i, sketch_quantiles(x, {list(PS)}) AS listed
            FROM v GROUP BY c, p HAVING got IS DISTINCT FROM want OR listed[i] IS DISTINCT FROM want
        """).fetchall()
        self.assertEqual(bad, [])


class Rank(unittest.TestCase):
    def test_quantile_disc_ranks_in_integers(self):
        # Values 1..n, so the value read is its rank: every n to 1000 and
        # p from 0 to 100, through sketch_quantile's quantile_disc (the
        # grid holds every integer under 256, so the rank is read
        # directly) and sketch_quantiles'.
        conn = connect()
        ps = list(range(101))
        cols = ", ".join(f"quantile_disc(v, {p} * 0.01) AS p{p}" for p in ps)
        got = conn.execute(f"SELECT n, {cols}, quantile_disc(v, list_transform({ps}, lambda q: q * 0.01)) AS listed "
                           "FROM (SELECT n, unnest(range(1, n + 1)) AS v FROM range(1, 1001) t(n)) GROUP BY n "
                           "ORDER BY n").fetchall()
        self.assertEqual(len(got), 1000)
        bad = [(r[0], p) for r in got for p in ps
               if r[1 + p] != sketch.rank(p, r[0]) or r[-1][p] != sketch.rank(p, r[0])]
        self.assertEqual(bad, [])
        # The macros' bodies are those quantile_disc calls.
        text = "\n".join(sketch_sql())
        self.assertIn("sketch_floor(quantile_disc(x, p * 0.01))", text)
        self.assertIn("quantile_disc(x, list_transform(ps, lambda sk_p: sk_p * 0.01))", text)
        # Grid points read as themselves, so sketch_quantile's value is the
        # rank's: 1..n's p-th percentile at n to 256.
        got = conn.execute(f"SELECT n, sketch_quantiles(v, {ps}) FROM (SELECT n, unnest(range(1, n + 1)) AS v "
                           "FROM range(1, 257) t(n)) GROUP BY n").fetchall()
        self.assertEqual([(n, p) for n, q in got for p in ps if q[p] != sketch.rank(p, n)], [])

    def test_a_double_q_ranks_otherwise(self):
        # Why the macros take p / 100 as a DECIMAL: a DOUBLE 0.55 is a hair
        # above, and its rank of 100 values the 56th. (In one query DuckDB
        # takes the two calls for one aggregate, the first's.)
        conn = connect()
        one = lambda sql: conn.execute(f"SELECT {sql} FROM range(1, 101) t(v)").fetchone()[0]
        self.assertEqual((one("quantile_disc(v, 55 / 100)"), one("sketch_quantile(v, 55)")), (56, 55.0))


if __name__ == "__main__":
    if sys.argv[1:] == ["write"]:
        write()
    else:
        unittest.main()
