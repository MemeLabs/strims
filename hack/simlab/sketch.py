# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The quantile method of the sims and the lab: the integer nearest rank,
read through a log sketch. strims_base::stats::sketch's docs say what it
is; this is the same in Python, and scoring.sql's sketch_floor,
sketch_quantile and sketch_quantiles in SQL, value for value
(strims-base's sketch_cases.tsv, which all three check: test_sketch).

  rank(p, n) -> int        the p-th percentile's rank (from 1) of n values
  floor(x) -> int | float  x read through the sketch
  quantile(v, p)           the p-th percentile of v, read through it

p is an integer percentile, 0 to 100. The rank is max(1, ceil(p*n/100)),
in integers. A value reads as the greatest point of the grid not above it:
each doubling splits into 2**SUB_BITS buckets, a positive value read at
its bucket's lower bound (never above it, at most 1/129 under, integers
under 256 exactly), zero as itself, a negative one as its magnitude rounded
up to the grid, negated. An int reads as an int (the integer formula, the
grid's points from 256 up being integers), a float as a float; NaN and the
infinities as they are.
"""

import math

# Each doubling splits into 2**SUB_BITS buckets.
SUB_BITS = 7
SUB = 1 << SUB_BITS


def rank(p, n):
    """The rank (from 1) of the p-th percentile (an integer, 0 to 100, more
    reading as 100) of n values."""
    return max(1, (min(p, 100) * n + 99) // 100)


def _int_bounds(a):
    """The bounds [lo, hi) of the bucket of a non-negative int a."""
    drop = max(0, a.bit_length() - SUB_BITS - 1)
    lo = a >> drop << drop
    return lo, lo + (1 << drop)


def floor(x):
    """x (an int or a float) read through the sketch."""
    if isinstance(x, int):
        lo, hi = _int_bounds(abs(x))
        return lo if x >= 0 else -(lo if lo == -x else hi)
    x = float(x)
    if not math.isfinite(x):
        return x
    if x == 0:
        return 0.0
    m, e = math.frexp(abs(x))
    k = math.floor(m * 2 * SUB)
    lo = math.ldexp(k, e - SUB_BITS - 1)
    if x > 0:
        return lo
    if lo == -x:
        return x
    try:
        return -math.ldexp(k + 1, e - SUB_BITS - 1)
    except OverflowError:
        return -math.inf


def quantile(v, p):
    """The p-th percentile (an integer, 0 to 100) of the values v, read
    through the sketch; None for none."""
    v = sorted(v)
    return floor(v[rank(p, len(v)) - 1]) if v else None
