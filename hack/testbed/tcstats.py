#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""tc's qdisc statistics as run.sh records them, for the tools that read a
testbed run (hack/testbed/bench.py and twin.py, hack/capture/analyze.py).

run.sh's tc-<i>.log holds a sample a line, `<unix ms> <qdiscs>`: the
qdiscs of the node's eth0 as `tc -s -j qdisc show dev eth0` prints them,
or, in runs before it recorded JSON, as `tc -s qdisc show dev eth0` does,
its lines joined. tc's text rounds what it prints: a size within 16 bytes
of a KiB multiple is `<n>Kb`, within a KiB of a MiB multiple `<n>Mb`
(iproute2's sprint_size), and a rate is in SI units (`<n>Kbit`,
`<n>Mbit`); its JSON has bytes, and rates in bytes a second.
"""

import json
import re
from typing import NamedTuple, Optional

SIZE_UNIT = {"": 1, "K": 1024, "M": 1024 ** 2, "G": 1024 ** 3}
RATE_UNIT = {"": 1, "K": 1e3, "M": 1e6, "G": 1e9, "T": 1e12}


class Qdisc(NamedTuple):
    """One qdisc's statistics: its kind and handle, its parent (None at the
    root), what it sent (bytes, packets), dropped and holds (bytes,
    packets), and its shaping rate in bytes a second where it has one
    (tbf's rate, cake's bandwidth)."""

    kind: str
    handle: str
    parent: Optional[str]
    sent: int
    packets: int
    drops: int
    overlimits: int
    backlog: int
    qlen: int
    rate: Optional[float]


def size(s):
    """A size as tc's text prints it (`1024b`, `3Kb`, `2Mb`), in bytes."""
    m = re.fullmatch(r"([\d.]+)([KMG]?)b", s)
    if not m:
        raise ValueError(f"not a tc size: {s!r}")
    return round(float(m[1]) * SIZE_UNIT[m[2]])


def rate(s):
    """A rate as tc's text prints it (`8Kbit`, `9846Kbit`, `10Mbit`), in
    bytes a second."""
    m = re.fullmatch(r"([\d.]+)([KMGT]?)bit", s)
    if not m:
        raise ValueError(f"not a tc rate: {s!r}")
    return float(m[1]) * RATE_UNIT[m[2]] / 8


# One qdisc in tc's text: its header, then its stats lines.
HEAD = re.compile(r"qdisc (\S+) ([0-9a-f]+:)(?: (root)| parent (\S+))?")
SENT = re.compile(r"Sent (\d+) bytes (\d+) pkt \(dropped (\d+), overlimits (\d+)")
BACKLOG = re.compile(r"backlog ([\d.]+[KMG]?b) (\d+)p")
SHAPE = re.compile(r" (?:rate|bandwidth) ([\d.]+[KMGT]?bit)")


def parse_text(text):
    """The qdiscs in tc's text (`tc -s qdisc show`), its lines joined or
    not."""
    starts = [m.start() for m in re.finditer(r"(?:^|(?<=\s))qdisc \S+ [0-9a-f]+:", text)]
    out = []
    for a, b in zip(starts, starts[1:] + [len(text)]):
        part = text[a:b]
        head = HEAD.match(part)
        sent = SENT.search(part)
        backlog = BACKLOG.search(part)
        if not (head and sent and backlog):
            raise ValueError(f"not tc's statistics: {part[:80]!r}")
        # The shaping rate is on the header line, ahead of the stats.
        shape = SHAPE.search(part[:sent.start()])
        out.append(Qdisc(
            kind=head[1],
            handle=head[2],
            parent=None if head[3] else head[4],
            sent=int(sent[1]),
            packets=int(sent[2]),
            drops=int(sent[3]),
            overlimits=int(sent[4]),
            backlog=size(backlog[1]),
            qlen=int(backlog[2]),
            rate=rate(shape[1]) if shape else None,
        ))
    return out


def parse_json(text):
    """The qdiscs in tc's JSON (`tc -s -j qdisc show`)."""
    out = []
    for q in json.loads(text):
        opts = q.get("options") or {}
        r = opts.get("rate", opts.get("bandwidth"))
        out.append(Qdisc(
            kind=q["kind"],
            handle=q["handle"],
            parent=None if q.get("root") else q.get("parent"),
            sent=q.get("bytes", 0),
            packets=q.get("packets", 0),
            drops=q.get("drops", 0),
            overlimits=q.get("overlimits", 0),
            backlog=q.get("backlog", 0),
            qlen=q.get("qlen", 0),
            rate=float(r) if isinstance(r, (int, float)) else None,
        ))
    return out


def parse(text):
    """The qdiscs in tc's JSON or its text."""
    text = text.strip()
    return parse_json(text) if text.startswith("[") else parse_text(text)


def samples(path):
    """tc-<i>.log's samples, [(unix ms, [Qdisc])], in time order; a line
    cut short (the run's end) is skipped."""
    out = []
    with open(path) as f:
        for line in f:
            t, _, rest = line.strip().partition(" ")
            if not t.isdigit() or not rest:
                continue
            try:
                out.append((int(t), parse(rest)))
            except (ValueError, KeyError):
                continue
    out.sort(key=lambda s: s[0])
    return out


def find(qdiscs, handle, kind=None):
    """The qdisc with `handle` (and of `kind`, if given), or None."""
    for q in qdiscs:
        if q.handle == handle and (kind is None or q.kind == kind):
            return q
    return None
