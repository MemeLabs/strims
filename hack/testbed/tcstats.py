#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""tc's qdisc statistics as run.sh records them, for the tools that read a
testbed run (hack/testbed/bench.py and twin.py, hack/capture/analyze.py).

run.sh's tc-<i>.log holds a sample a line, `<unix ms> <qdiscs>`: every
device's qdiscs as `tc -s -j qdisc show` prints them, each with its dev.
Older runs hold the node's eth0's qdiscs alone: as `tc -s -j qdisc show
dev eth0` prints them (with no dev), or, in runs older still, which
recorded no JSON, as `tc -s qdisc show dev eth0` prints them, its lines
joined. tc's text rounds what it prints: a size within 16 bytes of a KiB
multiple is `<n>Kb`, and one within a KiB of a MiB multiple `<n>Mb`
(iproute2's sprint_size); a rate is in SI units (`<n>Kbit`, `<n>Mbit`).
Its JSON has bytes, and rates in bytes a second.

A node's access link queue (uplink) is the modem's qdisc, 10:, in every
link class's tree. It is on eth0, or for a class with slots on ifbm, the
ifb that hack/testbed/links.sh moves the modem to, ahead of the slots.
The run's `links` file names the device (link_dev: what strims-wan's
testbed parser gave run.sh) and the link's class, which sets the queue's
kind (queue_kind).
"""

import json
import math
import os
import re
import sys
from typing import NamedTuple, Optional

SIZE_UNIT = {"": 1, "K": 1024, "M": 1024 ** 2, "G": 1024 ** 3}
RATE_UNIT = {"": 1, "K": 1e3, "M": 1e6, "G": 1e9, "T": 1e12}


class Qdisc(NamedTuple):
    """One qdisc's statistics: its kind and handle, its parent (None at the
    root), what it sent (bytes, packets), dropped and holds (bytes,
    packets), its shaping rate in bytes a second where it has one (tbf's
    rate, cake's bandwidth), what its shaper charges for what it holds
    (`held`, which over `rate` is how long it takes to drain), and its
    device where tc named it (a show of every device's, as qdisc-<i>.txt
    is)."""

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
    held: int
    dev: Optional[str] = None


# The bytes of a packet's link header ahead of its network header, which
# cake's overhead takes the place of (its `avg_hdr_offset`: Ethernet's).
HDR_OFFSET = 14


def cake_held(backlog, qlen, overhead, mpu, line):
    """What cake charges for `backlog` skb bytes in `qlen` packets, at their
    mean size: it counts its backlog as the skbs are, but shapes on each
    packet's network bytes and its overhead, at least its mpu, in ATM cells
    or PTM's 65 bytes for 64 (tc-cake(8); sch_cake's cake_overhead), so a
    dsl router's queue drains at 0.88 of its bandwidth in skb bytes."""
    if qlen <= 0:
        return backlog
    n = max(backlog / qlen - HDR_OFFSET + overhead, mpu)
    if line == "atm":
        n = math.ceil(n / 48) * 53
    elif line == "ptm":
        n += math.ceil(n / 64)
    return round(qlen * n)


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
HEAD = re.compile(r"qdisc (\S+) ([0-9a-f]+:)(?: dev (\S+))?(?: (root)| parent (\S+))?")
SENT = re.compile(r"Sent (\d+) bytes (\d+) pkt \(dropped (\d+), overlimits (\d+)")
BACKLOG = re.compile(r"backlog ([\d.]+[KMG]?b) (\d+)p")
SHAPE = re.compile(r" (?:rate|bandwidth) ([\d.]+[KMGT]?bit)")
CAKE = re.compile(r" (noatm|atm|ptm) overhead (-?\d+)(?: mpu (\d+))?")


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
        held = size(backlog[1])
        cake = CAKE.search(part[:sent.start()]) if head[1] == "cake" else None
        if cake and " raw" not in part[:sent.start()]:
            held = cake_held(held, int(backlog[2]), int(cake[2]), int(cake[3] or 0), cake[1])
        out.append(Qdisc(
            kind=head[1],
            handle=head[2],
            parent=None if head[4] else head[5],
            sent=int(sent[1]),
            packets=int(sent[2]),
            drops=int(sent[3]),
            overlimits=int(sent[4]),
            backlog=size(backlog[1]),
            qlen=int(backlog[2]),
            rate=rate(shape[1]) if shape else None,
            held=held,
            dev=head[3],
        ))
    return out


def parse_json(text):
    """The qdiscs in tc's JSON (`tc -s -j qdisc show`)."""
    out = []
    for q in json.loads(text):
        opts = q.get("options") or {}
        r = opts.get("rate", opts.get("bandwidth"))
        held = q.get("backlog", 0)
        if q["kind"] == "cake" and not opts.get("raw"):
            held = cake_held(held, q.get("qlen", 0), opts.get("overhead", 0), opts.get("mpu", 0), opts.get("atm"))
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
            held=held,
            dev=q.get("dev"),
        ))
    return out


def parse(text):
    """The qdiscs in tc's JSON or its text."""
    text = text.strip()
    return parse_json(text) if text.startswith("[") else parse_text(text)


def samples(path):
    """tc-<i>.log's samples, [(unix ms, [Qdisc])], in file order, which is
    time order: a run from before run.sh took its stamps from bash's clock
    has some up to 900 ms late (`date +%s%3N` under uutils), so the stamps
    may run backwards and a reader mustn't sort by them. A line cut short
    (the run's end) is skipped."""
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
    return out


def find(qdiscs, handle, kind=None, dev=None):
    """The qdisc with `handle` (of `kind` and on `dev`, if given), or
    None."""
    for q in qdiscs:
        if q.handle == handle and kind in (None, q.kind) and dev in (None, q.dev):
            return q
    return None


def links_line(run, node=0):
    """Node `node`'s line of the run's `links` file (`node <i> <spec>
    <up_mbit> <delay_ms> <queue_ms> [<modem_dev>]`), its words; None for a
    run from before run.sh wrote it."""
    p = os.path.join(run, "links")
    if not os.path.exists(p):
        return None
    with open(p) as f:
        for line in f:
            w = line.split()
            if len(w) >= 3 and w[0] == "node" and w[1] == str(node):
                return w
    raise LookupError(f"{p}: no node {node}")


def link_spec(run, node=0):
    """Node `node`'s access link spec (hack/testbed/links.sh), from the
    run's `links` file; tbf for a run from before run.sh wrote it, when
    every link was one."""
    w = links_line(run, node)
    return w[2] if w else "tbf"


def link_dev(run, node=0):
    """The device whose root is node `node`'s modem, its qdisc 10:, as the
    run's `links` file names it (strims-wan's testbed parser's LINK_DEV:
    ifbm for a link with slots, eth0 otherwise); None for a run from before
    run.sh wrote it, whose samples name no other."""
    w = links_line(run, node)
    return w[6] if w and len(w) >= 7 else None


def queue_kind(spec):
    """The kind of the modem's qdisc 10: (link_dev's root), the root of
    every class's modem and the qdisc that holds its queue, for link
    `spec`: a cake router's cake; a
    trace's prio, over the fq that tracelink's program holds packets in
    (its drops count in the prio); else the class's tbf, which counts what
    the qdisc under it holds and drops (cable's PIE, a router's
    fq_codel)."""
    opts = dict(kv.partition("=")[::2] for kv in spec.split(",")[1:])
    if opts.get("router") == "cake":
        return "cake"
    if spec.split(",")[0] == "trace":
        return "prio"
    return "tbf"


def uplink(run, node=0):
    """Node `node`'s access link queue over a run: its link spec and
    tc-<node>.log's samples of the qdisc that holds the queue,
    [(unix ms, Qdisc)] (queue_kind), on its modem's device (link_dev; a
    sample that names no device, an older run's of eth0 alone, and any
    sample of a run whose `links` names none, are taken as they come). Its
    `rate` drains it, where it has one: a trace's prio has none. Raises
    LookupError, naming the link, when the log has samples but none of that
    qdisc."""
    spec = link_spec(run, node)
    kind = queue_kind(spec)
    dev = link_dev(run, node)
    p = os.path.join(run, f"tc-{node}.log")
    got = samples(p) if os.path.exists(p) else []

    def modem(qs):
        return next((q for q in qs if q.handle == "10:" and q.kind == kind
                     and (dev is None or q.dev in (None, dev))), None)

    out = [(t, q) for t, qs in got if (q := modem(qs))]
    if got and not out:
        on = f" on {dev}" if dev else ""
        raise LookupError(f"{p}: node {node}'s {spec} link has no {kind} 10:{on} in its samples")
    return spec, out


def note(msg):
    """Says on stderr what a tool can't read from a run."""
    print(f"NOTE: {msg}", file=sys.stderr)
