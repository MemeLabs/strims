#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.coverage over a fixture catalog of its own: four strims-sim jobs
of golden_cli's job dirs (arm decl at seeds 1 and 2, its viewers there
from the start; arm main at seed 1 with the seed's uplink stepped down at
20 s and up at 32 s, and at seed 2 a collapse, its series_meta recording
its seed and its stream count), arm bare's without push parents, arm
noarr's without arrivals, and a ppspp sim job (arm pp: decl's seed-1
holds, but ending a second after its last publication, as the ppspp
sim's runs do, with empty parents, as it writes them without hybrid push,
and its row recording its stream count), each with the holds and push
parents the measure reads, folded
and exported by simdb.py into a local suite invocation's source, and a
second invocation's source that labels two of them again (decl's seed 1
in a cell of its own, star-5b; its seed 2 in star-5, as the first has
it), a third invocation's (MIX) with arm mix's run, decl's seed 1
with host 3 a dishonest viewer, a fourth's (LOG) with arm log's,
mix's run with a seed log (seed_decisions, seed_sends), a few holds
and push parents changed and a viewer's departure, its twin's (EARLY)
with arm early's, log's run with that viewer leaving half a second
sooner, a fifth's (UNIT)
with arm unit's, decl's seed 1 with its chunks published several of a
stream at once, and a sixth's (ONCE) with arm once's, one publication of
each stream, indexed by the catalog. Every run but unit's and once's
publishes the same chunks: two streams, a chunk of each every 250 ms from
20 s to 39.75 s of a 40 s run, so the last 4 s are censored.

  Prepare      the measure's per-run facts: the seed, the streams, the
               interval, the run's end, the phases, the censored chunks,
               the parents rows' order at one ms, the subscriptions
  Sections     each section's numbers on the fixture, worked by hand:
               latency's counts, shares, quantiles, anomalies and the
               runs' spread, `rest` only where two phases pool, collapsed
               runs apart; routes'; I1's uncovered time, seed children
               and subscription ends; late's classes
  Grain        a job counts in each cell and arm the selection labels it
               with, once in each; the ppspp sim's job: decl's seed 1's
               latency, its censored chunks, no I1 or classes without
               parents rows; the stream count read where a sim recorded
               it, a warning for every run whose count is inferred, and
               none failing where the catalog has no asks; a run without
               arrivals left out with a warning, and a selection of none
               with them refused
  Dishonest    arm mix's run, worked by hand: a chunk's first hold the
               earliest of a node but the seed and the dishonest viewers
               (its source a dishonest viewer's copy where it is one), the
               seed's children no dishonest viewer, a late chunk's
               dishonest viewers subscribed to the seed counted apart; the
               runs without dishonest viewers have none of either
  SeedLog      arm log's run, worked by hand: I2's chunks (the seed's
               sends of each to its children), I2 and I3 by phase, each
               late chunk's causes (every cause of the tree, the details
               of each) and the attribution section's rows; runs without a
               seed log, and a catalog without its tables, leave I2, I3
               and the attribution empty, with a warning; arm early's
               run, whose child departs by a chunk's due time and after
               another's
  Units        arm unit's run, worked by hand: its interval the gap
               between a stream's publications, not between its chunks,
               and the late chunks and the latency section by it
  Once         arm once's run: no interval, so its chunks, held or
               never, are neither late nor on time, unclassed and without
               causes, with a warning
  Cli        simdb.py coverage-report's tables and JSON, as a caller runs
               it, the seed log's sections among them, and a section the
               catalog can't answer yet

  python3 hack/simlab/tests/test_coverage.py
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))
sys.path.insert(0, str(HERE))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import golden_cli  # noqa: E402
import simlab  # noqa: E402
from simlab import catalog, coverage, forecast, query, sketch, store_host, verdict  # noqa: E402

SRC = "suite/wt/cv"
# (arm, seed): the run's shape (golden_cli's series).
RUNS = {("decl", 1): None, ("decl", 2): None, ("main", 1): "step", ("main", 2): "collapse"}
# The second invocation, whose jobs are SRC's labelled again.
RELABEL = "suite/wt/cv-relabel"
# The arm label of the job that ends a second after its last publication.
PP = "strims-sim-pp"
NAMES = {f"strims-sim-{arm}": arm for arm in ("decl", "main", "bare", "noarr")} | {PP: "pp"}
SEL = forecast.Selection(exp=[SRC], names=NAMES, phases="all")
SEED = 0
END_MS = golden_cli.SECS * 1000
CHUNKS = 160
# The ppspp job's: its last published at 39 s, a second before its end.
PP_CHUNKS = 154
# The source of arm mix's run, apart from SRC's: decl's seed 1 with a
# dishonest viewer, host 3 (viewer 2).
MIX = "suite/wt/cv-mix"
DISHONEST = 3
MIX_SEL = forecast.Selection(exp=[MIX], names={"strims-sim-mix": "mix"}, phases="all")
# The source of arm log's run, apart from the others: arm mix's run with a
# seed log (log_decisions, log_sends) and the changes log_arrivals and
# log_parents make, among them host 4 the seed's child of stream 0 from
# 33.7 s until it departs at 34.6 s, holding nothing.
LOG = "suite/wt/cv-log"
LOG_SEL = forecast.Selection(exp=[LOG], names={"strims-sim-log": "log"}, phases="all")
DEPARTED = 4
DEPART_MS = 34600
# The source of arm early's run: arm log's with host 4's departure at
# 34.1 s (EARLY_DEPART_MS), the seed losing it at 34.6 s all the same.
EARLY = "suite/wt/cv-early"
EARLY_SEL = forecast.Selection(exp=[EARLY], names={"strims-sim-early": "early"}, phases="all")
EARLY_DEPART_MS = 34100
# The source of arm unit's run, apart from the others: decl's seed 1 with
# its chunks published four at one ms (unit_pub) and unit_arrivals' holds.
UNIT = "suite/wt/cv-unit"
UNIT_SEL = forecast.Selection(exp=[UNIT], names={"strims-sim-unit": "unit"}, phases="all")
# The source of arm once's run, apart from the others: decl's seed 1 with
# one publication of each stream (once_arrivals), so no interval.
ONCE = "suite/wt/cv-once"
ONCE_SEL = forecast.Selection(exp=[ONCE], names={"strims-sim-once": "once"}, phases="all")
# The bytes of a written push and of a written answer in the seed log.
PUSH_BYTES, ANSWER_BYTES = 1000, 1500
# The warning of runs without a seed log, their count left to format.
SEED_LOG_WARNING = ("coverage: {} runs have no seed log (seed_decisions, seed_sends: "
                    f"{query.knobs_text('seed_sends')} writes them): their I2, I3 and late chunks' causes are NULL")


def pub(q):
    """Chunk q's publication: two streams (q mod 2), a chunk of each every
    250 ms from 20 s."""
    return 20000 + 250 * (q // 2)


ARRIVAL_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"), ("seq", "BIGINT"), ("how", "VARCHAR"),
                ("peer", "BIGINT"), ("parent", "BIGINT"), ("released_ms", "BIGINT"), ("incarnation", "BIGINT")]


def arrivals(seed):
    """The holds: the seed (0) publishes chunks 0-159; host 1, the seed's
    child, holds each 40 ms after by its push, and host 2, host 1's
    child, 80 ms after. But:

      2    host 1 holds it 400 ms after (startup: pushed late)
      1    at seed 2 host 1 holds it 1000 ms after, by an answer (startup:
           stream 1 has no seed child until 21 s)
      100  no host holds it at seed 1 (never)
      101  only host 2, 300 ms after, by the seed's answer (stream 1's
           seed child pruned it from 32 s to 33 s)
      102  host 1 600 ms after (pushed late)
      103  only host 3, 100 ms after, from no source it names (an
           anomaly, on time)
      104  host 1 500 ms after by the seed's answer, its push released at
           250 ms (released)
      106  only host 2, 700 ms after, by the seed's answer (its child
           never held it)
      108  host 1 260 ms after by the seed's answer, no release (other)
      114  host 1 300 ms after (pushed late with two seed children: host
           3 is one, from 34 s to 35 s, and never holds it)
      157  host 1 400 ms after (censored)
      158  no host holds it (censored)
    """
    rows = [(pub(q), SEED, -1, q, "own", -1, -1, None, 0) for q in range(CHUNKS)]
    special = {2: [(1, 400, "push", 0, None), (2, 440, "push", 1, None)],
               100: [] if seed == 1 else None,
               101: [(2, 300, "answer", 0, None)],
               102: [(1, 600, "push", 0, None), (2, 640, "push", 1, None)],
               103: [(3, 100, "other", -1, None)],
               104: [(1, 500, "answer", 0, 250), (2, 540, "push", 1, None)],
               106: [(2, 700, "answer", 0, None)],
               108: [(1, 260, "answer", 0, None), (2, 300, "push", 1, None)],
               114: [(1, 300, "push", 0, None), (2, 340, "push", 1, None)],
               157: [(1, 400, "push", 0, None)],
               158: []}
    if seed == 2:
        special[1] = [(1, 1000, "answer", 0, None), (2, 1040, "push", 1, None)]
    for q in range(CHUNKS):
        holds = special.get(q)
        if holds is None:
            holds = [(1, 40, "push", 0, None), (2, 80, "push", 1, None)]
        for node, after, how, peer, released in holds:
            rows.append((pub(q) + after, node, node - 1, q, how, peer, peer, None if released is None
                         else pub(q) + released, 0))
    return rows


PARENT_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("stream", "BIGINT"), ("parent", "BIGINT"),
               ("reason", "VARCHAR")]


def parents(seed):
    """The push trees: host 1 the seed's child of both streams from the
    start (stream 1 from 21 s at seed 2), pruning stream 1 at 32 s and
    back at 33 s; host 2 host 1's child; host 3 the seed's child of
    stream 0 from 34 s, lost at 35 s; host 4 opening to the seed and shed
    at one ms (36 s), which ends no subscription; host 5 host 1's child of
    stream 1, shed by it and opening to the seed at one ms (37 s): the
    seed's child to the end."""
    rows = [(0 if seed == 1 else 21000, 1, 1, 0, "open"), (0, 1, 0, 0, "open"),
            (32000, 1, 1, -1, "prune"), (33000, 1, 1, 0, "open"),
            (0, 2, 0, 1, "open"), (0, 2, 1, 1, "open"),
            (34000, 3, 0, 0, "open"), (35000, 3, 0, -1, "lost"),
            (36000, 4, 0, -1, "shed"), (36000, 4, 0, 0, "open"),
            (0, 5, 1, 1, "open"), (37000, 5, 1, 0, "open"), (37000, 5, 1, -1, "shed")]
    return rows


def mix_arrivals():
    """Arm mix's holds: decl's seed-1 holds, where host 3 (DISHONEST) is
    still 103's only holder, and 112's first, 20 ms after by the seed's
    push; host 1 holds 112 400 ms after by host 3's answer, host 2 440 ms
    after by host 1's push. Host 3's rows name no viewer, as strims-sim
    writes a hostile host's."""
    rows = [r for r in arrivals(1) if r[3] != 112 or r[1] == SEED]
    rows += [(pub(112) + after, node, node - 1, 112, how, peer, peer, None, 0)
             for node, after, how, peer in ((DISHONEST, 20, "push", SEED), (1, 400, "answer", DISHONEST),
                                            (2, 440, "push", 1))]
    return [(r[0], r[1], -1 if r[1] == DISHONEST else r[2], *r[3:]) for r in rows]


def log_arrivals():
    """Arm log's holds: arm mix's, but 110 late: host 1 holds it 400 ms
    after by the seed's answer, host 2 440 ms after by host 1's push."""
    rows = [r for r in mix_arrivals() if r[3] != 110 or r[1] == SEED]
    return rows + [(pub(110) + after, node, node - 1, 110, how, peer, peer, None, 0)
                   for node, after, how, peer in ((1, 400, "answer", SEED), (2, 440, "push", 1))]


def log_parents():
    """Arm log's push trees: decl's seed-1 ones, but host 4's (none of
    its own here) and with host 3 the seed's child of stream 1 from 32.6 s
    to 33 s, host 1 pruning stream 0 at 33.7 s and back at 34.05 s, and
    host 4 the seed's child of stream 0 from 33.7 s, lost at its
    departure."""
    return [r for r in parents(1) if r[1] != DEPARTED] + [
        (32600, DISHONEST, 1, 0, "open"), (33000, DISHONEST, 1, -1, "lost"),
        (33700, 1, 0, -1, "prune"), (34050, 1, 0, 0, "open"),
        (33700, DEPARTED, 0, 0, "open"), (DEPART_MS, DEPARTED, 0, -1, "lost")]


def log_children(q):
    """The seed's children of chunk q's stream at its publication in arm
    log's run (log_parents), but the dishonest host 3."""
    t = pub(q)
    if q % 2 == 0:
        return [n for n, on in ((1, not 33700 <= t < 34050), (DEPARTED, 33700 <= t < DEPART_MS)) if on]
    return [n for n, on in ((1, not 32000 <= t < 33000), (5, t >= 37000)) if on]


DECISION_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("n", "BIGINT"), ("peer", "BIGINT"),
                 ("peer_incarnation", "BIGINT"), ("kind", "VARCHAR"), ("stream", "BIGINT"), ("seq", "BIGINT"),
                 ("why", "VARCHAR"), ("granted", "BOOLEAN"), ("subscribers", "BIGINT")]


def log_decisions():
    """Arm log's seed decisions (t_ms, peer, kind, stream, why, granted,
    subscribers): requests, I3's those declined with no subscriber, and
    the removals 101's uncovered stream (host 1's stream-1 subscription,
    0 s to its prune at 32 s) meets or doesn't."""
    rows = [
        (21000, 2, "request", 0, "seed_streams_held", False, 0),  # I3, startup
        (26000, 1, "request", 0, "seed_holder", True, 0),  # granted: none
        (30000, 1, "barred", None, "unserved", None, None),  # within host 1's subscription, before the next
        (31000, 1, "holder_left", None, "idle", None, None),  # its latest removal: 101's seed_end
        (31500, 1, "closed", 0, "spill", None, 0),  # later, but of stream 0
        (32100, 1, "barred", None, "silent", None, None),  # after its end
        (32200, 2, "request", 1, "seed_no_copy", False, 0),  # I3, after the end, before 101: declined
        (32300, DISHONEST, "request", 1, "seed_barred", False, 0),  # I3 of a dishonest viewer
        (32400, 2, "request", 1, "seed_no_copy", False, 1),  # a subscriber: none
        (38000, 2, "request", 1, "seed_no_copy", False, 0),  # I3, after the departure
    ]
    return [(t, SEED, n, peer, 0, kind, stream, None, why, granted, subs)
            for n, (t, peer, kind, stream, why, granted, subs) in enumerate(rows)]


SEND_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("n", "BIGINT"), ("peer", "BIGINT"),
             ("peer_incarnation", "BIGINT"), ("how", "VARCHAR"), ("stream", "BIGINT"), ("seq", "BIGINT"),
             ("queued_ms", "BIGINT"), ("fate", "VARCHAR"), ("bytes", "BIGINT"), ("cancel_ms", "BIGINT"),
             ("declared", "BOOLEAN"), ("subscribed", "BOOLEAN")]


def log_sends():
    """Arm log's seed sends: a push of each chunk to each of its children
    (log_children), queued at its publication and written 5 ms after, but
    (how, fate, queued_ms, t_ms, cancel_ms):

      2    none (startup: not written, none queued)
      100  stale, 500 ms after (not written)
      102  written 350 ms after (written late)
      104  cancelled at 255 ms, the CANCEL at 250, and an answer written
           at 490 ms, its request at 300 (not written, the answer alone
           written: I2 kept)
      106  cancelled at 715 ms, the CANCEL at 710, 10 ms after host 2's
           hold (not written; held at the cancel)
      108  written 10 ms after, and an answer at 240 ms (delivered late:
           host 1 holds it at 260)
      112  to host 4 alone, written 350 ms after (written late; host 4
           departs 600 ms after, past the 250 the chunk was due in)
      114  to host 4 written 3 ms after, 2 ms before host 1's (other:
           the first written push, host 4's, never held, and host 1,
           which holds it late, stays)

    and the answers to nodes no child: host 2's of 101 at 290 ms and of
    106 at 690, host 1's of 110 at 390 (110: pushed to host 4 alone,
    which departs 850 ms after, the chunk due in 250: other). An answer
    is ANSWER_BYTES, a written push PUSH_BYTES."""
    special = {(2, 1): [],
               (100, 1): [("push", "stale", 0, 500, None)],
               (102, 1): [("push", "written", 0, 350, None)],
               (104, 1): [("push", "cancelled", 0, 255, 250), ("answer", "written", 300, 490, None)],
               (106, 1): [("push", "cancelled", 0, 715, 710)],
               (108, 1): [("push", "written", 0, 10, None), ("answer", "written", 200, 240, None)],
               (112, DEPARTED): [("push", "written", 0, 350, None)],
               (114, DEPARTED): [("push", "written", 0, 3, None)]}
    sends = [(q, peer, *s) for q in range(CHUNKS) for peer in log_children(q)
             for s in special.get((q, peer), [("push", "written", 0, 5, None)])]
    sends += [(q, peer, "answer", "written", queued, t, None) for q, peer, queued, t in
              ((101, 2, 200, 290), (106, 2, 650, 690), (110, 1, 350, 390))]
    rows = sorted((pub(q) + t, peer, how, q, pub(q) + queued, fate, cancel) for q, peer, how, fate, queued, t, cancel
                  in sends)
    return [(t, SEED, n, peer, 0, how, q % 2, q, queued, fate,
             (PUSH_BYTES if how == "push" else ANSWER_BYTES) if fate == "written" else None,
             None if cancel is None else pub(q) + cancel, False, how == "push")
            for n, (t, peer, how, q, queued, fate, cancel) in enumerate(rows)]


def unit_pub(q):
    """Chunk q's publication in arm unit's run: two streams (q mod 2), two
    chunks of each, four at one ms, every 500 ms from 20 s."""
    return 20000 + 500 * (q // 4)


def unit_arrivals():
    """Arm unit's holds: the seed (0) publishes chunks 0-159 (unit_pub);
    host 1, the seed's child, holds each 40 ms after by its push, and host
    2, host 1's child, 80 ms after. But, of the publication at 30 s
    (steady), host 1 alone holds them, by the seed's push:

      80  no host holds it (never)
      81  300 ms after (on time: within its stream's 500 ms)
      82  500 ms after (on time: late is more than the interval)
      83  501 ms after (pushed late)
    """
    special = {80: [], 81: [(1, 300)], 82: [(1, 500)], 83: [(1, 501)]}
    rows = [(unit_pub(q), SEED, -1, q, "own", -1, -1, None, 0) for q in range(CHUNKS)]
    for q in range(CHUNKS):
        for node, after in special.get(q, [(1, 40), (2, 80)]):
            rows.append((unit_pub(q) + after, node, node - 1, q, "push", node - 1, node - 1, None, 0))
    return rows


def once_arrivals():
    """Arm once's holds: the seed (0) publishes chunks 0-3, two of each
    stream, all at 20 s and nothing after; host 1, the seed's child, holds
    0-2 40 ms after by its push, and host 2, host 1's child, 80 ms after;
    no host holds 3."""
    rows = [(20000, SEED, -1, q, "own", -1, -1, None, 0) for q in range(4)]
    return rows + [(20000 + after, node, node - 1, q, "push", node - 1, node - 1, None, 0)
                   for q in range(3) for node, after in ((1, 40), (2, 80))]


ASK_COLS = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("honest", "BOOLEAN"), ("ask", "BIGINT"), ("round", "VARCHAR"),
            ("peer", "BIGINT"), ("peer_honest", "BOOLEAN"), ("picked", "BOOLEAN"), ("stream", "BIGINT"),
            ("seq", "BIGINT")]


def build(root):
    """The fixture's cache under `root` (target/sim-db), indexed, and an
    empty git repository (its main checkout). Arm bare's run has no push
    parents (its streams are its asks'), arm noarr's no arrivals."""
    sm = golden_cli.simdb_test().simdb
    os.environ["SIM_DB_ROOT"] = str(root / "target" / "sim-db")
    jobs = []
    for (arm, seed), shape in [*RUNS.items(), (("bare", 1), None), (("noarr", 1), None)]:
        label = f"strims-sim-{arm}"
        d = root / "jobs" / f"{arm}-{seed}"
        job = (SRC, f"{label}:star-5:{seed}", "strims-sim", "calibration_star", "star-5", label, seed,
               golden_cli.SS_ENV, golden_cli.T0, shape)
        rec = golden_cli.job_dir(d, job)
        if arm != "noarr":
            golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, arrivals(seed))
        if arm == "bare":
            for f in ("parents.csv", "parents.schema.json"):
                (d / f).unlink()
            golden_cli.write_table(d, "asks", ASK_COLS, [(26000, 1, True, 0, "flight", 0, True, True, 1, 81)])
        else:
            golden_cli.write_table(d, "parents", PARENT_COLS, parents(seed))
        if (arm, seed) == ("main", 2):
            meta = json.loads((d / "series_meta.json").read_text())
            (d / "series_meta.json").write_text(json.dumps(meta | {"seed": SEED, "streams": 2}))
        jobs.append(rec)
    d = root / "jobs" / "pp-1"
    rec = golden_cli.job_dir(d, (SRC, f"{PP}:star-5:1", "strims-sim", "calibration_star", "star-5", PP, 1,
                                 golden_cli.STAR_ENV,
                                 golden_cli.T0, None))
    rec["row"]["streams"] = 2
    golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, [r for r in arrivals(1) if r[3] < PP_CHUNKS])
    golden_cli.write_table(d, "parents", PARENT_COLS, [])
    jobs.append(rec)
    db = root / "store.duckdb"
    sm.fold(db, jobs, SRC)
    sm.export(db, root / "target" / "sim-db" / "cache" / SRC)
    db.unlink()
    # Arm mix's run: decl's seed 1 with host 3 hostile in its viewers and
    # its series_meta, which records its seed and stream count; arm log's
    # too, with its seed log and host 4's departure.
    for arm, src, holds, trees in (("mix", MIX, mix_arrivals(), parents(1)),
                                   ("log", LOG, log_arrivals(), log_parents()),
                                   ("early", EARLY, log_arrivals(), log_parents())):
        d = root / "jobs" / f"{arm}-1"
        rec = golden_cli.job_dir(d, (src, f"strims-sim-{arm}:star-5:1", "strims-sim", "calibration_star", "star-5",
                                     f"strims-sim-{arm}", 1, golden_cli.SS_ENV, golden_cli.T0, None))
        golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, holds)
        golden_cli.write_table(d, "parents", PARENT_COLS, trees)
        head, *lines = (d / "viewers.csv").read_text().splitlines()
        cols = head.split(",")
        viewers = [line.split(",") for line in lines]
        for v in viewers:
            if v[cols.index("host")] == str(DISHONEST):
                v[cols.index("hostile")] = "true"
        (d / "viewers.csv").write_text("\n".join([head, *(",".join(v) for v in viewers)]) + "\n")
        meta = json.loads((d / "series_meta.json").read_text())
        (d / "series_meta.json").write_text(json.dumps(meta | {"seed": SEED, "streams": 2,
                                                               "hostile": [DISHONEST - 1]}))
        if arm in ("log", "early"):
            golden_cli.write_table(d, "seed_decisions", DECISION_COLS, log_decisions())
            golden_cli.write_table(d, "seed_sends", SEND_COLS, log_sends())
            head, *lines = (d / "events.csv").read_text().splitlines()
            gone = DEPART_MS if arm == "log" else EARLY_DEPART_MS
            lines.append(f"{gone * 1000},depart,{DEPARTED},-1,1,abrupt")
            lines.sort(key=lambda line: int(line.split(",")[0]))
            (d / "events.csv").write_text("\n".join([head, *lines]) + "\n")
        sm.fold(db, [rec], src)
        sm.export(db, root / "target" / "sim-db" / "cache" / src)
        db.unlink()
    # Arm unit's run: decl's seed 1 with unit_arrivals' holds, its
    # series_meta recording its seed and stream count.
    d = root / "jobs" / "unit-1"
    rec = golden_cli.job_dir(d, (UNIT, "strims-sim-unit:star-5:1", "strims-sim", "calibration_star", "star-5",
                                 "strims-sim-unit", 1, golden_cli.SS_ENV, golden_cli.T0, None))
    golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, unit_arrivals())
    golden_cli.write_table(d, "parents", PARENT_COLS, parents(1))
    meta = json.loads((d / "series_meta.json").read_text())
    (d / "series_meta.json").write_text(json.dumps(meta | {"seed": SEED, "streams": 2}))
    sm.fold(db, [rec], UNIT)
    sm.export(db, root / "target" / "sim-db" / "cache" / UNIT)
    db.unlink()
    # Arm once's run: decl's seed 1 with once_arrivals' holds, its
    # series_meta recording its seed and stream count.
    d = root / "jobs" / "once-1"
    rec = golden_cli.job_dir(d, (ONCE, "strims-sim-once:star-5:1", "strims-sim", "calibration_star", "star-5",
                                 "strims-sim-once", 1, golden_cli.SS_ENV, golden_cli.T0, None))
    golden_cli.write_table(d, "arrivals", ARRIVAL_COLS, once_arrivals())
    golden_cli.write_table(d, "parents", PARENT_COLS, parents(1))
    meta = json.loads((d / "series_meta.json").read_text())
    (d / "series_meta.json").write_text(json.dumps(meta | {"seed": SEED, "streams": 2}))
    sm.fold(db, [rec], ONCE)
    sm.export(db, root / "target" / "sim-db" / "cache" / ONCE)
    db.unlink()
    label = "strims-sim-decl"
    golden_cli.relabel(root, RELABEL, [
        (f"{label}:star-5:{seed}", "strims-sim", "calibration_star", cell, label, seed, golden_cli.SS_ENV)
        for seed, cell in ((1, "star-5b"), (2, "star-5"))])
    subprocess.run(["git", "init", "-q", str(root)], check=True)


class Fixture(unittest.TestCase):
    """The fixture under a scratch root, indexed once; the main checkout
    is the root."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-coverage-")
        cls.root = Path(cls.tmp.name)
        saved = os.environ.get("SIM_DB_ROOT")
        try:
            build(cls.root)
            with cls.patched(cls):
                catalog.index(2, lambda e: None)
                cls.warned = []
                cls.results = {s: r.records() for s, r in coverage.report(SEL, warn=cls.warned.append).items()}
        finally:
            os.environ.pop("SIM_DB_ROOT", None)
            if saved is not None:
                os.environ["SIM_DB_ROOT"] = saved

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def patched(self):
        stack = mock.patch.multiple(store_host, main_checkout=lambda: self.root)
        env = mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.root / "target" / "sim-db"), "TZ": "UTC"})
        sm = mock.patch.object(simlab.simdb(), "main_checkout", lambda: self.root)

        class Both:
            def __enter__(s):
                for p in (stack, env, sm):
                    p.start()

            def __exit__(s, *exc):
                for p in (sm, env, stack):
                    p.stop()

        return Both()

    def setUp(self):
        p = self.patched()
        p.__enter__()
        self.addCleanup(p.__exit__, None, None, None)

    def rows(self, section, **want):
        return [r for r in self.results[section] if all(r.get(k) == v for k, v in want.items())]

    def one(self, section, **want):
        got = self.rows(section, **want)
        self.assertEqual(len(got), 1, f"{section} {want}: {got}")
        return got[0]

    def close(self, got, want):
        """`got`, a list of numbers, is `want` to rounding."""
        self.assertEqual(len(got), len(want), got)
        for g, w in zip(got, want):
            self.assertAlmostEqual(g, w)


def p99(xs):
    """sketch_quantile's p99 of `xs` (simlab.sketch's)."""
    return sketch.quantile(xs, 99)


def spread(*xs):
    """The runs' spread of `xs`, one a run: the least, the median (the
    sketch's) and the most."""
    return [min(xs), sketch.quantile(xs, 50), max(xs)]


# Every run's steady late chunks but 100 (seed 1's never), with their
# latencies, and the steady chunks held 40 ms after.
STEADY_LATE = {101: 300, 102: 600, 104: 500, 106: 700, 108: 260, 114: 300}
JUDGED_STEADY = 88


class Prepare(Fixture):
    def test_the_runs_facts(self):
        with forecast.connected() as c:
            coverage.prepare(c, SEL)
            jobs = c.execute("SELECT arm, seed, collapsed, seed_node, seed_from, streams, streams_from, end_ms, "
                             "interval_ms, parents_rows FROM cv_jobs ORDER BY ALL").fetchall()
            phases = c.execute("SELECT arm, seed, phase, list(sec ORDER BY sec) FROM cv_phase "
                               "JOIN cv_jobs USING (k, src) WHERE arm IN ('decl', 'main') GROUP BY ALL "
                               "ORDER BY ALL").fetchall()
            chunks = c.execute("SELECT judged, count(*), min(pub_ms), max(pub_ms) FROM cv_chunks "
                               "WHERE arm = 'decl' AND seed = 1 GROUP BY ALL ORDER BY ALL").fetchall()
        # The seed (main's seed-2 series_meta records it, the others'
        # arrivals give it) and the run's end (its last second, 39, ends at
        # 40 s); two streams, read from main's seed-2 series_meta and the
        # ppspp job's row, else from the parents, or from bare's asks; 250
        # ms within a stream; 13 parents rows, none for bare and the ppspp
        # job. noarr's run, without arrivals, is left out.
        rows = len(parents(1))
        self.assertEqual(jobs, [("bare", 1, False, SEED, "arrivals", 2, "asks", END_MS, 250.0, 0),
                                ("decl", 1, False, SEED, "arrivals", 2, "parents", END_MS, 250.0, rows),
                                ("decl", 2, False, SEED, "arrivals", 2, "parents", END_MS, 250.0, rows),
                                ("main", 1, False, SEED, "arrivals", 2, "parents", END_MS, 250.0, rows),
                                ("main", 2, True, SEED, "series_meta", 2, "series_meta", END_MS, 250.0, rows),
                                ("pp", 1, False, SEED, "arrivals", 2, "row", END_MS, 250.0, 0)])
        # golden_cli's latency event at 5 s holds 5-19 s; main's seed 1
        # steps down at 20 s and up at 32 s.
        span = lambda a, b: list(range(a, b))  # noqa: E731
        self.assertEqual(phases[:3], [("decl", 1, "after_disruption", span(5, 20)),
                                      ("decl", 1, "startup", span(0, 5) + span(20, 25)),
                                      ("decl", 1, "steady", span(25, 40))])
        self.assertEqual([p[2:] for p in phases if p[:2] == ("main", 1)],
                         [("after_disruption", span(5, 20)), ("after_down", span(20, 32)),
                          ("after_up", span(32, 40)), ("startup", span(0, 5))])
        # A chunk published past 35.75 s (40 s less the latency budget's
        # most) is censored.
        self.assertEqual(verdict.DELIVERY_MAX_MS, 4250)
        self.assertEqual(chunks, [(False, 32, 36000, 39750), (True, 128, 20000, 35750)])

    def test_parents_at_one_ms_and_the_subscriptions(self):
        with forecast.connected() as c:
            coverage.prepare(c, SEL)
            subs = c.execute("SELECT node, stream, open_ms, close_ms, end_reason FROM cv_subs "
                             "JOIN cv_jobs USING (k, src) WHERE arm = 'decl' AND seed = 1 ORDER BY ALL").fetchall()
        # Host 4 had no parent at 36 s: its open came first, then the shed,
        # a subscription of no length. Host 5 had host 1: shed, then the
        # open, the seed's child to the end (end_reason NULL).
        self.assertEqual(subs, [(1, 0, 0, END_MS, None), (1, 1, 0, 32000, "prune"), (1, 1, 33000, END_MS, None),
                                (3, 0, 34000, 35000, "lost"), (5, 1, 37000, END_MS, None)])

    def test_the_definitions_are_the_catalogs_macros(self):
        # Each with its comment and parameters, as describe gives them.
        got = {r["name"]: r for r in catalog.scoring_docs()}
        want = {"coverage_runs_of": "(jobs)", "coverage_ask_streams_of": "(jobs)", "coverage_seed_logs_of": "(jobs)",
                "seed_subscriptions_in": "(runs)"}
        for name in ("seed_children", "coverage_chunks", "seed_cover", "seed_writes", "seed_declines",
                     "coverage_causes"):
            want |= {f"{name}_in": "(runs)", f"{name}_of": "(jobs)"}
        for name, params in want.items():
            self.assertEqual((got[name]["kind"], got[name]["keys"]), ("table macro", params), name)
            self.assertTrue(got[name]["description"], name)
        self.assertIn(f"published at least {verdict.DELIVERY_MAX_MS} ms",
                      got["coverage_chunks_in"]["description"])
        # The vocabulary's names, filled in; the fallback's columns line
        # isn't its doc.
        self.assertIn("a 'switch' first, an 'open' after", got["seed_subscriptions_in"]["description"])
        self.assertIn("every node but the dishonest viewers", got["seed_children_in"]["description"])
        self.assertNotIn("else none", got["coverage_ask_streams_of"]["description"])
        self.assertIn("I1, the invariant that every stream a viewer watches", got["seed_cover_in"]["description"])
        for name in ("coverage_seed_logs_of", "seed_writes_in", "seed_declines_in", "coverage_causes_in"):
            self.assertNotIn("else none", got[name]["description"], name)
        self.assertIn("I2, the invariant that the seed writes every chunk", got["seed_writes_in"]["description"])
        self.assertIn("I3, the invariant that the seed never declines", got["seed_declines_in"]["description"])
        self.assertIn("of kind 'depart' or 'crash'", got["coverage_causes_in"]["description"])


class Sections(Fixture):
    def test_latency(self):
        r = self.one("latency", arm="decl", phase="steady")
        # Two runs of 88 judged chunks (32 censored each); seed 1's chunk
        # 100 never held; the late chunks; 103 from no named source.
        self.assertEqual((r["collapsed"], r["runs"], r["seeds"], r["interval_runs"], r["chunks"], r["censored"]),
                         (False, 2, [1, 2], [250.0] * 3, 2 * JUDGED_STEADY, 64))
        self.assertEqual((r["held"], r["never"], r["late"], r["anomalies"]), (175, 1, 2 * len(STEADY_LATE) + 1, 2))
        self.assertAlmostEqual(r["never_share"], 1 / 176)
        self.assertAlmostEqual(r["late_share"], 13 / 176)
        held = [100] * 2 + [ms for ms in STEADY_LATE.values()] * 2
        held += [40] * (175 - len(held))
        self.assertEqual((r["p50_ms"], r["p90_ms"], r["max_ms"]), (40.0, 40.0, 700))
        self.assertAlmostEqual(r["p99_ms"], p99(held))
        # The 174th of 175 (ceil(0.99 x 175)): 700.
        self.assertAlmostEqual(r["p99_ms"], 700)
        # Each run's: seed 1 7 late of 88 and a never, seed 2 6 late; seed
        # 1 holds 87 (80 at 40 ms), seed 2 88.
        self.close(r["late_runs"], spread(6 / 88, 7 / 88))
        self.close(r["never_runs"], spread(0.0, 1 / 88))
        one = [100] + list(STEADY_LATE.values())
        lo, hi = p99(one + [40] * 81), p99(one + [40] * 80)
        self.close(r["p99_runs"], spread(lo, hi))
        self.close(r["p99_runs"], [700, 700, 700])
        # The startup's: chunk 2 at 400 ms in both runs, chunk 1 at 1000 ms
        # at seed 2.
        s = self.one("latency", arm="decl", phase="startup")
        self.assertEqual((s["chunks"], s["censored"], s["late"], s["max_ms"]), (80, 0, 3, 1000))
        self.assertAlmostEqual(s["p99_ms"], p99([40] * 77 + [400, 400, 1000]))
        self.close(s["late_runs"], spread(0.025, 0.05))

    def test_a_pooled_row_of_two_phases_and_collapsed_runs_apart(self):
        # decl's chunks are startup and steady alone: no pooled row. main's
        # seed 1 has two phases past startup (down at 20 s, up at 32 s),
        # pooled in a row of phase NULL, its collapsed seed 2 one.
        self.assertEqual([r["phase"] for r in self.rows("latency", arm="decl")], ["startup", "steady"])
        main = [(r["collapsed"], r["phase"], r["chunks"], r["censored"], r["late"], r["never"])
                for r in self.rows("latency", arm="main")]
        self.assertEqual(main, [(False, "after_down", 96, 0, 1, 0), (False, "after_up", 32, 32, 7, 1),
                                (False, None, 128, 32, 8, 1),
                                (True, "startup", 40, 0, 2, 0), (True, "steady", 88, 32, 6, 0)])

    def test_routes(self):
        got = {(r["route"], r["source"]): r for r in self.rows("routes", arm="decl", phase="steady")}
        self.assertEqual(sorted(got, key=str),
                         [("answer", "seed"), ("never", None), ("other", "unnamed"), ("push", "seed")])
        push, answer = got[("push", "seed")], got[("answer", "seed")]
        # Pushes: 82 and 83 of 88, 102 and 114 late; answers: 101, 104,
        # 106 and 108 in both runs.
        self.assertEqual((push["chunks"], push["late"], push["max_ms"]), (165, 4, 600))
        self.assertAlmostEqual(push["share"], 165 / 176)
        self.assertAlmostEqual(push["p99_ms"], p99([40] * 161 + [300, 300, 600, 600]))
        self.close(push["share_runs"], spread(82 / 88, 83 / 88))
        # Answers 260, 300, 500 and 700 ms twice: the p50 is the 4th, 300.
        self.assertEqual((answer["chunks"], answer["late"], answer["p50_ms"], answer["p99_ms"]), (8, 8, 300.0, 700.0))
        other = got[("other", "unnamed")]
        self.assertEqual((other["chunks"], other["late"], other["p50_ms"]), (2, 0, 100.0))
        # The never-held chunk is seed 1's alone: seed 2's share is 0.
        never = got[("never", None)]
        self.assertEqual((never["chunks"], never["p50_ms"]), (1, None))
        self.close(never["share_runs"], spread(0.0, 1 / 88))
        startup = self.one("routes", arm="decl", phase="startup", route="answer")
        self.assertEqual(startup["chunks"], 1)
        self.close(startup["share_runs"], spread(0.0, 0.025))

    def test_invariants(self):
        # Steady, 25-39 s: two streams a run. Stream 1 has no seed child
        # from 32 s to 33 s (host 1's prune); stream 0 has two from 34 s to
        # 35 s (host 3, lost), stream 1 two from 37 s (host 5).
        r = self.one("invariants", arm="decl", phase="steady")
        self.assertEqual((r["runs"], r["stream_s"], r["uncovered_s"], r["touched_s"]), (2, 60, 2.0, 2))
        self.assertAlmostEqual(r["uncovered"], 2 / 60)
        self.assertAlmostEqual(r["seed_children"], 2 * (15 + 1 + 14 + 3) / 60)
        self.assertEqual((r["ends_prune"], r["ends_lost"], r["ends_shed"], r["ends_switch"], r["ends_open"]),
                         (2, 2, 0, 0, 0))
        # The startup (0-4 s, 20-24 s): seed 2's stream 1 had no seed child
        # before 21 s, 6 of its seconds.
        s = self.one("invariants", arm="decl", phase="startup")
        self.assertEqual((s["stream_s"], s["uncovered_s"]), (40, 6.0))
        self.assertAlmostEqual(s["seed_children"], 34 / 40)
        self.close(s["uncovered_runs"], spread(0.0, 0.3))
        # Past startup the stream-seconds have two phases (golden_cli's
        # latency event at 5 s, 5-19 s): a row of phase NULL pools them.
        pooled = self.one("invariants", arm="decl", phase=None)
        self.assertEqual((pooled["stream_s"], pooled["uncovered_s"], pooled["ends_prune"]), (120, 17.0, 2))
        self.assertAlmostEqual(pooled["seed_children"], (30 + 15 + 66) / 120)
        # A run without parents rows has no seed children to count: its I1
        # is NULL, not wholly uncovered.
        bare = self.one("invariants", arm="bare", phase="steady")
        self.assertEqual((bare["runs"], bare["unparented"], bare["stream_s"], bare["uncovered"],
                          bare["seed_children"], bare["uncovered_runs"]), (1, 1, 0, None, None, None))
        self.assertEqual(r["unparented"], 0)

    def test_late(self):
        got = {(r["class"], r["after"]): r for r in self.rows("late", arm="decl", phase="steady")}
        self.assertEqual(list(got), [("no_seed_child", "prune"), ("released", None), ("pushed_late", None),
                                     ("child_never", None), ("other", None)])
        late = 13
        want = {("no_seed_child", "prune"): (2, 0.0, 0, 300.0), ("released", None): (2, 1.0, 0, 500.0),
                ("pushed_late", None): (4, 1.5, 0, 300.0), ("child_never", None): (3, 1.0, 1, 700.0),
                ("other", None): (2, 1.0, 0, 260.0)}
        for key, (n, children, never, p50) in want.items():
            r = got[key]
            self.assertEqual((r["chunks"], r["children"], r["never"], r["p50_ms"]), (n, children, never, p50), key)
            self.assertAlmostEqual(r["late_share"], n / late)
            self.assertAlmostEqual(r["share"], n / 176)
        # child_never: 106 in both runs, seed 1's never-held 100 too.
        self.close(got[("child_never", None)]["share_runs"], spread(1 / 88, 2 / 88))
        startup = {(r["class"], r["after"]): r["chunks"] for r in self.rows("late", arm="decl", phase="startup")}
        # Seed 2's chunk 1: its stream had no seed child before (after NULL).
        self.assertEqual(startup, {("no_seed_child", None): 1, ("pushed_late", None): 2})
        # The runs without parents rows have no classes.
        self.assertEqual(self.rows("late", arm="bare") + self.rows("late", arm="pp"), [])

    def test_selection(self):
        # The collapsed runs alone, past startup (the default phases).
        # The collapsed run records its stream count: no warning.
        warned = []
        got = coverage.report(forecast.Selection(exp=[SRC], names=NAMES, collapsed=True), ["latency"],
                              warn=warned.append)
        self.assertEqual({(r["arm"], r["phase"]) for r in got["latency"].records()}, {("main", "steady")})
        self.assertEqual(warned, [])
        with self.assertRaisesRegex(ValueError, "no section 'mechanisms'"):
            coverage.report(SEL, ["mechanisms"], warn=[].append)


class Grain(Fixture):
    def test_a_job_counts_in_each_of_its_labels_once(self):
        sel = forecast.Selection(exp=[SRC, RELABEL], names=NAMES, phases="all")
        got = {(r["arm"], r["cell"], r["phase"]): r for r in coverage.report(sel, ["latency"], warn=[].append)
               ["latency"].records()}
        # star-5's decl is SEL's, its seed 2 counted once though both
        # invocations name it there; star-5b's is decl's seed 1 alone.
        mine = self.one("latency", arm="decl", phase="steady")
        self.assertEqual(got[("decl", "star-5", "steady")], mine)
        b = got[("decl", "star-5b", "steady")]
        self.assertEqual((b["runs"], b["seeds"], b["chunks"], b["late"], b["never"], b["p99_ms"]),
                         (1, [1], JUDGED_STEADY, len(STEADY_LATE) + 1, 1, mine["p99_runs"][2]))

    def test_a_job_that_ends_a_second_after_its_last_publication(self):
        # decl's seed-1 holds to chunk 153, published at 39 s, a second
        # before the run's end: its judged chunks are decl's seed 1's, and
        # 26 censored (chunks 128-153) where decl's have 32.
        r = self.one("latency", arm="pp", phase="steady")
        self.assertEqual((r["runs"], r["chunks"], r["censored"], r["late"], r["never"], r["anomalies"]),
                         (1, JUDGED_STEADY, PP_CHUNKS - 128, len(STEADY_LATE) + 1, 1, 1))
        # The 88th of 88: 700.
        self.assertAlmostEqual(r["p99_ms"], 700)
        # Its parents are empty, as the ppspp sim writes them without hybrid
        # push: no I1, no classes.
        i = self.one("invariants", arm="pp", phase="steady")
        self.assertEqual((i["unparented"], i["stream_s"], i["uncovered_s"], i["seed_children"]), (1, 0, None, None))

    def test_the_warnings(self):
        # noarr's run wrote no arrivals; bare's and the ppspp job's have no
        # parents rows; none of the six left has a seed log; of them only
        # main's seed 2 (series_meta) and the ppspp job (its row) record
        # their stream count.
        self.assertEqual(self.warned, [
            f"coverage: 1 runs wrote no arrivals ({query.knobs_text('arrivals')} writes them): left out of every "
            "section",
            "coverage: 2 runs have no parents rows (no push trees): their I1, I2 and their late chunks' classes and "
            "causes are NULL", SEED_LOG_WARNING.format(6),
            "coverage: 4 runs' stream count is inferred, not read (1 from asks, 3 from parents): neither their "
            "series_meta nor their row records it, and their interval and I1 take the count inferred"])
        got = {(r["arm"], r["collapsed"]): r["inferred_streams"] for r in self.rows("latency", phase="steady")}
        self.assertEqual(got, {("bare", False): 1, ("decl", False): 2, ("main", True): 0, ("pp", False): 0})
        self.assertFalse([r for rows in self.results.values() for r in rows if r["arm"] == "noarr"])

    def test_a_selection_without_arrivals_is_refused(self):
        sel = forecast.Selection(exp=[SRC], arm=["strims-sim-noarr"], names=NAMES)
        with self.assertRaisesRegex(query.Refused, "the selection has no arrivals"):
            coverage.report(sel, ["latency"], warn=[].append)

    def test_a_catalog_without_asks(self):
        # bare's run then has no source of its count: one stream, said so,
        # and the report still runs: with one stream, its chunks are
        # published two at one ms, and its interval is still 250 ms.
        real = catalog.scoring_cols

        def without(columns, have=None):
            return real(columns, have={t: c for t, c in (have or {}).items() if t != "asks"})

        warned = []
        with mock.patch.object(catalog, "scoring_cols", without):
            got = coverage.report(SEL, ["latency"], warn=warned.append)
        self.assertIn("(3 from parents, 1 with one assumed)", warned[-1])
        bare = [r for r in got["latency"].records() if r["arm"] == "bare" and r["phase"] == "steady"]
        self.assertEqual([r["interval_runs"] for r in bare], [[250.0] * 3])


class Dishonest(Fixture):
    """Arm mix's run (MIX's source): decl's seed 1 with host 3 a dishonest
    viewer, 103's only holder and 112's first, and the seed's child of
    stream 0 from 34 s to 35 s."""

    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        with cls.patched(cls):
            cls.mix_warned = []
            cls.mix = {s: r.records() for s, r in coverage.report(MIX_SEL, warn=cls.mix_warned.append).items()}

    def mixed(self, section, **want):
        got = [r for r in self.mix[section] if all(r.get(k) == v for k, v in want.items())]
        self.assertEqual(len(got), 1, f"{section} {want}: {got}")
        return got[0]

    def test_the_chunks_and_the_subscriptions(self):
        with forecast.connected() as c:
            coverage.prepare(c, MIX_SEL)
            chunks = c.execute("SELECT seq, first_node, latency_ms, route, source, late, children, dishonest_children, "
                               "class, after FROM cv_chunks WHERE seq IN (103, 110, 112, 114) ORDER BY seq").fetchall()
            subs = c.execute("SELECT node, stream, open_ms, close_ms, end_reason FROM cv_subs ORDER BY ALL").fetchall()
            every = c.execute("SELECT node, stream, open_ms, close_ms, end_reason, dishonest "
                              "FROM seed_subscriptions_in((SELECT list(r) FROM cv_runs r)) ORDER BY ALL").fetchall()
        self.assertEqual(chunks, [
            # Host 3 held it alone: never held, so late; stream 1 had no
            # seed child at 32.75 s (host 1's prune), nor a dishonest one.
            (103, None, None, "never", None, True, 0, 0, "no_seed_child", "prune"),
            # Held as in decl's runs.
            (110, 1, 40, "push", "seed", False, None, None, None, None),
            # Host 1's hold by host 3's answer, late: host 1 the seed's child
            # of stream 0 then, host 3 (from 34 s) the dishonest one.
            (112, 1, 400, "answer", "dishonest", True, 1, 1, "other", None),
            # Pushed late, with one seed child (decl's two but host 3).
            (114, 1, 300, "push", "seed", True, 1, 1, "pushed_late", None)])
        # decl's seed-1 subscriptions but host 3's, the dishonest one.
        self.assertEqual(subs, [(1, 0, 0, END_MS, None), (1, 1, 0, 32000, "prune"), (1, 1, 33000, END_MS, None),
                                (5, 1, 37000, END_MS, None)])
        self.assertEqual(every, [(1, 0, 0, END_MS, None, False), (1, 1, 0, 32000, "prune", False),
                                 (1, 1, 33000, END_MS, None, False), (3, 0, 34000, 35000, "lost", True),
                                 (5, 1, 37000, END_MS, None, False)])

    def test_latency_and_routes(self):
        # decl's seed 1 (7 late of 88, 100 never held, 103 its anomaly)
        # but 103 never held and 112 held 400 ms after: 86 held, 79 of
        # them 40 ms after.
        r = self.mixed("latency", phase="steady")
        self.assertEqual((r["runs"], r["chunks"], r["censored"], r["held"], r["never"], r["late"], r["anomalies"]),
                         (1, JUDGED_STEADY, 32, 86, 2, 9, 0))
        self.assertAlmostEqual(r["late_share"], 9 / JUDGED_STEADY)
        self.assertAlmostEqual(r["never_share"], 2 / JUDGED_STEADY)
        held = [40] * 79 + list(STEADY_LATE.values()) + [400]
        self.assertEqual((r["p50_ms"], r["p90_ms"], r["max_ms"]), (40.0, 40.0, 700))
        self.assertAlmostEqual(r["p99_ms"], p99(held))
        # decl's seed 1 pushes 82 and answers 4 (101, 104, 106 and 108);
        # 112's answer is host 3's.
        got = {(x["route"], x["source"]): (x["chunks"], x["late"], x["p50_ms"])
               for x in self.mix["routes"] if x["phase"] == "steady"}
        self.assertEqual(got, {("push", "seed"): (81, 2, 40.0), ("answer", "seed"): (4, 4, 300.0),
                               ("answer", "dishonest"): (1, 1, 400.0), ("never", None): (2, 2, None)})

    def test_invariants(self):
        # Steady: stream 1 uncovered from 32 s to 33 s, as in decl's seed
        # 1; the seed's children 15 + 14 + 3 s, host 3's second on stream 0
        # not among them, nor its loss among the ends.
        r = self.mixed("invariants", phase="steady")
        self.assertEqual((r["runs"], r["stream_s"], r["uncovered_s"], r["touched_s"]), (1, 30, 1.0, 1))
        self.assertAlmostEqual(r["seed_children"], (15 + 14 + 3) / 30)
        self.assertEqual((r["ends_prune"], r["ends_lost"]), (1, 0))

    def test_late(self):
        got = {(x["class"], x["after"]): x for x in self.mix["late"] if x["phase"] == "steady"}
        # (chunks, children, dishonest_children, never, p50): 101 and 103
        # with no seed child; 104 released; 102 and 114 pushed late; 100
        # and 106 never held by host 1; 108 and 112 held otherwise.
        want = {("no_seed_child", "prune"): (2, 0.0, 0.0, 1, 300.0), ("released", None): (1, 1.0, 0.0, 0, 500.0),
                ("pushed_late", None): (2, 1.0, 0.5, 0, 300.0), ("child_never", None): (2, 1.0, 0.0, 1, 700.0),
                ("other", None): (2, 1.0, 0.5, 0, 260.0)}
        self.assertEqual(list(got), list(want))
        for key, w in want.items():
            x = got[key]
            self.assertEqual((x["chunks"], x["children"], x["dishonest_children"], x["never"], x["p50_ms"]), w, key)
            self.assertAlmostEqual(x["late_share"], w[0] / 9)
            self.assertAlmostEqual(x["share"], w[0] / JUDGED_STEADY)

    def test_runs_without_dishonest_viewers(self):
        # SEL's runs have none: no first copy is a dishonest viewer's, and
        # no late chunk has one subscribed to the seed.
        self.assertNotIn("dishonest", {x["source"] for x in self.results["routes"]})
        self.assertEqual({x["dishonest_children"] for x in self.results["late"]}, {0.0})
        # Arm mix's run has no seed log, alone among the warnings.
        self.assertEqual(self.mix_warned, [SEED_LOG_WARNING.format(1)])


class SeedLog(Fixture):
    """Arm log's run (LOG's source): arm mix's with a seed log, host 3 the
    dishonest child of stream 1 from 32.6 s to 33 s, host 1 off stream 0
    from 33.7 s to 34.05 s, and host 4 the seed's child of stream 0 from
    33.7 s, departing at 34.6 s (after_departure from 35 s); the late
    chunks 2 (startup) and 100-114 (steady) each worked by hand."""

    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        with cls.patched(cls):
            cls.log_warned = []
            cls.log = {s: r.records() for s, r in coverage.report(LOG_SEL, warn=cls.log_warned.append).items()}
            with forecast.connected() as c:
                coverage.prepare(c, LOG_SEL)
                cls.writes = {r[0]: r[1:] for r in c.execute(
                    "SELECT seq, children, sends, written, cancelled, fates, cancel_ms, push_ms, push_wait_ms, "
                    "push_held_ms, unwritten FROM cv_writes ORDER BY seq").fetchall()}
                cls.causes = {r[0]: r[1:] for r in c.execute(
                    "SELECT seq, cause, matches, last_end, seed_end, seed_why, declined, fates, held_at_cancel, "
                    "push_wait_ms, delivery_ms, answer_bytes, push_bytes, pushes FROM cv_causes ORDER BY seq"
                ).fetchall()}

    def logged(self, section, **want):
        return [r for r in self.log[section] if all(r.get(k) == v for k, v in want.items())]

    def test_the_writes(self):
        # (children, sends, written, cancelled, fates, cancel_ms, push_ms,
        # push_wait_ms, push_held_ms, unwritten): host 1 the child but at
        # 110 and 112 (host 4) and 114 (both); 101 has none (I1's).
        self.assertEqual({q: self.writes[q] for q in (2, 100, 101, 104, 106, 108, 110, 112, 114)}, {
            2: (1, 0, 0, 0, [], None, None, None, None, True),
            100: (1, 1, 0, 0, ["push stale"], None, None, None, None, True),
            101: (0, 0, 0, 0, [], None, None, None, None, None),
            104: (1, 2, 1, 1, ["answer written", "push cancelled"], pub(104) + 250, None, None, None, False),
            106: (1, 1, 0, 1, ["push cancelled"], pub(106) + 710, None, None, None, False),
            108: (1, 2, 2, 0, ["answer written", "push written"], None, pub(108) + 10, 10, pub(108) + 260, False),
            # Host 4 never holds a chunk.
            110: (1, 1, 1, 0, ["push written"], None, pub(110) + 5, 5, None, False),
            112: (1, 1, 1, 0, ["push written"], None, pub(112) + 350, 350, None, False),
            # The first written is host 4's, which never holds it: host 1's,
            # 2 ms later, it holds 300 ms after.
            114: (2, 2, 2, 0, ["push written"], None, pub(114) + 3, 3, None, False)})
        # The judged chunks 0-127 (a run of 40 s, the last 4.25 s
        # censored), of which 97, 99, 101 and 103 have no child (stream 1,
        # 32-33 s); 2 and 100 alone break I2 (106's push was cancelled,
        # 104's child answered).
        self.assertEqual(sorted(self.writes), list(range(128)))
        self.assertEqual([q for q, w in self.writes.items() if w[0] == 0], [97, 99, 101, 103])
        self.assertEqual([q for q, w in self.writes.items() if w[-1]], [2, 100])

    def test_the_invariants(self):
        # (child_chunks, unwritten, unwritten_share, declined,
        # declined_rules, declined_dishonest): startup is 0-4 s and 20-24 s
        # (chunks 0-39), steady 25-34 s (40-119, 4 without a child),
        # after_departure 35-39 s (120-127 judged); the pooled row (NULL)
        # pools those two and after_disruption (5-19 s, no chunk).
        got = {r["phase"]: (r["child_chunks"], r["unwritten"], r["unwritten_share"], r["declined"],
                            r["declined_rules"], r["declined_dishonest"], r["unlogged"])
               for r in self.log["invariants"]}
        self.assertEqual(got, {
            "after_disruption": (0, None, None, 0, None, 0, 0),
            "startup": (40, 1, 1 / 40, 1, {"seed_streams_held": 1}, 0, 0),
            "steady": (76, 1, 1 / 76, 1, {"seed_no_copy": 1}, 1, 0),
            "after_departure": (8, 0, 0.0, 1, {"seed_no_copy": 1}, 0, 0),
            None: (84, 1, 1 / 84, 2, {"seed_no_copy": 2}, 1, 0)})
        steady = self.logged("invariants", phase="steady")[0]
        self.close(steady["unwritten_runs"], spread(1 / 76))
        self.assertEqual(self.log_warned, [])

    def test_the_causes(self):
        # (cause, matches, last_end, seed_end, seed_why, declined, fates,
        # held_at_cancel, push_wait_ms, delivery_ms, answer_bytes,
        # push_bytes, pushes), the bytes those written in the second
        # before the first written push (log_sends): 102's 101's answer,
        # 98's and 105's pushes; 108's 101's and 104's answers, 102's,
        # 105's, 107's and 109's pushes; 110's 108's answer and push too;
        # 112's 104's, 108's, 106's and 110's answers, 108's, 109's,
        # 110's, 111's, 113's, 114's two and 115's pushes; 114's (host
        # 4's at 3 ms) those answers, 107's, 108's, 109's, 110's, 111's and
        # 113's pushes.
        nw, a = "not_written", ANSWER_BYTES
        self.assertEqual(self.causes, {
            2: (nw, [nw], None, None, None, None, [], None, None, None, None, None, None),
            100: (nw, [nw], None, None, None, None, ["push stale"], None, None, None, None, None, None),
            # The stream's last subscription to the seed ended by host 1's
            # prune at 32 s; the seed's latest removal of host 1 within it
            # its holder_left (its closed is stream 0's, its later bar
            # after the end); host 2's request declined at 32.2 s.
            101: ("uncovered", ["uncovered"], "prune", "holder_left", "idle", True, [], None, None, None, None, None,
                  None),
            102: ("written_late", ["written_late"], None, None, None, None, ["push written"], None, 350, 250, a,
                  2 * PUSH_BYTES, 2),
            103: ("defectors_only", ["defectors_only"], None, None, None, None, [], None, None, None, None, None,
                  None),
            # No push written (it was cancelled at 250 ms, before host 1's
            # hold at 500), though answered.
            104: (nw, [nw], None, None, None, None, ["answer written", "push cancelled"], False, None, None, None, None,
                  None),
            # Cancelled 10 ms after host 2's hold.
            106: (nw, [nw], None, None, None, None, ["push cancelled"], True, None, None, None, None, None),
            108: ("delivered_late", ["delivered_late"], None, None, None, None, ["answer written", "push written"],
                  None, 10, 250, 2 * a, 4 * PUSH_BYTES, 4),
            # Pushed to host 4 alone, which never holds it and departs
            # 850 ms after (112: 600), past the 250 ms it was due in: not
            # why it was late.
            110: ("other", ["other"], None, None, None, None, ["push written"], None, 5, None, 3 * a,
                  5 * PUSH_BYTES, 5),
            112: ("written_late", ["written_late"], None, None, None, None, ["push written"], None, 350,
                  None, 4 * a, 8 * PUSH_BYTES, 8),
            # Its first written push host 4's, never held: no cause.
            114: ("other", ["other"], None, None, None, None, ["push written"], None, 3, None, 4 * a, 6 * PUSH_BYTES,
                  6)})

    def test_attribution(self):
        # Steady: 10 late of 80 judged chunks, a row each cause and each
        # part of one.
        got = [(r["cause"], r["breakdown"], r["value"], r["chunks"]) for r in self.logged("attribution",
                                                                                         phase="steady")]
        self.assertEqual(got, [
            ("uncovered", None, None, 1), ("uncovered", "last_end", "prune", 1),
            ("uncovered", "seed_end", "holder_left idle", 1), ("uncovered", "declined", "true", 1),
            ("defectors_only", None, None, 1),
            ("not_written", None, None, 3), ("not_written", "fates", "answer written, push cancelled", 1),
            ("not_written", "fates", "push cancelled", 1), ("not_written", "fates", "push stale", 1),
            ("not_written", "held_at_cancel", "false", 1), ("not_written", "held_at_cancel", "true", 1),
            ("written_late", None, None, 2), ("delivered_late", None, None, 1),
            ("other", None, None, 2), ("other", "fates", "push written", 2)])
        for r in self.logged("attribution", phase="steady"):
            self.assertAlmostEqual(r["late_share"], r["chunks"] / 10)
            self.assertAlmostEqual(r["share"], r["chunks"] / 80)
            self.close(r["late_share_runs"], spread(r["chunks"] / 10))
        row = lambda cause, breakdown=None: self.logged(  # noqa: E731
            "attribution", phase="steady", cause=cause, breakdown=breakdown)[0]
        # The first written push's wait and its child's hold after it, and
        # the second's sends before it (test_the_causes): 102 and 112
        # written late; 108 delivered late; 110 pushed to host 4 alone,
        # and 114 to host 4 first (two children), which never holds them.
        late, delivered, other = row("written_late"), row("delivered_late"), row("other")
        self.assertEqual((late["children"], late["wait_ms"], late["delivery_ms"]),
                         (1.0, sketch.quantile([350, 350], 50), sketch.quantile([250], 50)))
        self.assertEqual((late["answer_bytes"], late["push_bytes"], late["pushes"]), (2.5 * ANSWER_BYTES, 5000.0, 5.0))
        self.assertEqual((delivered["children"], delivered["wait_ms"], delivered["delivery_ms"]),
                         (1.0, sketch.quantile([10], 50), sketch.quantile([250], 50)))
        self.assertEqual((delivered["answer_bytes"], delivered["push_bytes"], delivered["pushes"]),
                         (2.0 * ANSWER_BYTES, 4000.0, 4.0))
        self.assertEqual((other["children"], other["wait_ms"], other["delivery_ms"], other["answer_bytes"],
                          other["push_bytes"], other["pushes"]),
                         (1.5, sketch.quantile([5, 3], 50), None, 3.5 * ANSWER_BYTES, 5500.0, 5.5))
        self.assertEqual([row(c)["wait_ms"] for c in ("uncovered", "not_written")], [None] * 2)
        # Startup: chunk 2, never pushed. The pooled row (NULL) pools
        # steady and after_departure (no late chunk), 88 judged.
        self.assertEqual([(r["cause"], r["breakdown"], r["value"], r["chunks"])
                          for r in self.logged("attribution", phase="startup")],
                         [("not_written", None, None, 1), ("not_written", "fates", "none", 1)])
        pooled = self.logged("attribution", phase=None)
        self.assertEqual([(r["cause"], r["breakdown"], r["value"], r["chunks"]) for r in pooled], got)
        self.assertAlmostEqual(pooled[0]["share"], 1 / 88)
        self.assertEqual(self.logged("attribution", phase="after_departure"), [])

    def test_a_child_departs_by_the_chunks_due_time(self):
        # Arm early's: host 4 departs at 34.1 s, 350 ms after 110's
        # publication (due in 250: not why it was late) and 100 ms after
        # 112's (departed, beside written late); 114's other child, host 1,
        # stays.
        with forecast.connected() as c:
            coverage.prepare(c, EARLY_SEL)
            got ={r[0]: (r[1], r[2]) for r in c.execute(
                "SELECT seq, cause, matches FROM cv_causes WHERE seq IN (110, 112, 114) ORDER BY seq").fetchall()}
        self.assertEqual(got, {110: ("other", ["other"]), 112: ("written_late", ["written_late", "departed"]),
                               114: ("other", ["other"])})

    def test_runs_without_a_seed_log(self):
        # SEL's runs have none: their I2 and I3 are empty, NULL, and they
        # are left out of the attribution.
        for r in self.results["invariants"]:
            self.assertEqual((r["unlogged"], r["child_chunks"], r["unwritten"], r["unwritten_share"], r["declined"],
                              r["declined_rules"], r["declined_dishonest"]), (r["runs"], 0, None, None, None, None,
                                                                               None), (r["arm"], r["phase"]))
        self.assertEqual(self.results["attribution"], [])

    def test_a_catalog_without_the_seed_log(self):
        # The seed log's macros give no rows: arm log's run is as one
        # without a seed log, and the report runs.
        real = catalog.scoring_cols

        def without(columns, have=None):
            return real(columns, have={t: c for t, c in (have or {}).items()
                                       if t not in ("seed_decisions", "seed_sends")})

        warned = []
        with mock.patch.object(catalog, "scoring_cols", without):
            got = coverage.report(LOG_SEL, ["invariants", "attribution"], warn=warned.append)
        self.assertEqual(warned, [SEED_LOG_WARNING.format(1)])
        steady = [r for r in got["invariants"].records() if r["phase"] == "steady"]
        self.assertEqual([(r["unlogged"], r["child_chunks"], r["unwritten"], r["declined"]) for r in steady],
                         [(1, 0, None, None)])
        self.assertEqual(got["attribution"].records(), [])


class Units(Fixture):
    """Arm unit's run (UNIT's source): decl's seed 1 with two chunks of
    each stream published at one ms, every 500 ms."""

    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        with cls.patched(cls):
            cls.unit_warned = []
            cls.unit = {s: r.records() for s, r in coverage.report(UNIT_SEL, ["latency", "late"],
                                                                   warn=cls.unit_warned.append).items()}

    def united(self, section, **want):
        got = [r for r in self.unit[section] if all(r.get(k) == v for k, v in want.items())]
        self.assertEqual(len(got), 1, f"{section} {want}: {got}")
        return got[0]

    def test_the_interval_and_the_chunks(self):
        with forecast.connected() as c:
            coverage.prepare(c, UNIT_SEL)
            jobs = c.execute("SELECT streams, streams_from, interval_ms FROM cv_jobs").fetchall()
            chunks = c.execute("SELECT seq, pub_ms, latency_ms, late, children, class FROM cv_chunks "
                               "WHERE seq BETWEEN 80 AND 83 ORDER BY seq").fetchall()
        # Each stream's 80 chunks are 40 publications of two, 500 ms apart:
        # half its consecutive chunks come 0 ms apart, its publications 500
        # ms, its interval.
        self.assertEqual(jobs, [(2, "series_meta", 500.0)])
        # Late is held more than 500 ms after: 80 never, 83 at 501 ms, each
        # with host 1 the seed's child of its stream at 30 s.
        self.assertEqual(chunks, [(80, 30000, None, True, 1, "child_never"),
                                  (81, 30000, 300, False, None, None),
                                  (82, 30000, 500, False, None, None),
                                  (83, 30000, 501, True, 1, "pushed_late")])

    def test_latency_and_late(self):
        # Steady: 22 publications of four judged (25-35.5 s), 88 chunks, and
        # 8 censored (36-39.5 s), 32; 80 never held, 80 and 83 late, the
        # other 84 held 40 ms after.
        r = self.united("latency", phase="steady")
        self.assertEqual((r["runs"], r["chunks"], r["censored"], r["held"], r["never"], r["late"],
                          r["interval_runs"]), (1, JUDGED_STEADY, 32, 87, 1, 2, [500.0] * 3))
        self.assertAlmostEqual(r["late_share"], 2 / JUDGED_STEADY)
        self.assertEqual((r["p50_ms"], r["max_ms"]), (40.0, 501))
        self.assertAlmostEqual(r["p99_ms"], p99([40] * 84 + [300, 500, 501]))
        # The startup's 10 publications (20-24.5 s), each chunk held 40 ms
        # after: none late.
        s = self.united("latency", phase="startup")
        self.assertEqual((s["chunks"], s["censored"], s["late"], s["max_ms"]), (40, 0, 0, 40))
        got = {(x["phase"], x["class"]): x["chunks"] for x in self.unit["late"]}
        self.assertEqual(got, {("steady", "child_never"): 1, ("steady", "pushed_late"): 1})
        # Its stream count is recorded, it has parents, and neither section
        # reads the seed log: no warning.
        self.assertEqual(self.unit_warned, [])


class Once(Fixture):
    """Arm once's run (ONCE's source): one publication of each stream, so
    its interval is unknown."""

    WARNING = ("coverage: 1 runs have no publication interval (each stream published at one ms): their chunks are "
               "neither late nor on time, left out of the late share and of the late and attribution sections")

    def test_a_chunk_of_no_interval_is_neither_late_nor_classed(self):
        with forecast.connected() as c:
            said = coverage.prepare(c, ONCE_SEL)
            jobs = c.execute("SELECT streams, interval_ms, parents_rows > 0 FROM cv_jobs").fetchall()
            chunks = c.execute("SELECT seq, pub_ms, latency_ms, route, judged, late, children, class FROM cv_chunks "
                               "ORDER BY seq").fetchall()
        self.assertEqual(jobs, [(2, None, True)])
        self.assertIn(self.WARNING, said)
        # Held 40 ms after, or never held, judged, with a seed child: late
        # NULL, no class; the never-held one never all the same.
        self.assertEqual(chunks, [(q, 20000, 40, "push", True, None, None, None) for q in range(3)]
                         + [(3, 20000, None, "never", True, None, None, None)])
        warned = []
        got = {s: r.records() for s, r in coverage.report(ONCE_SEL, ["latency", "late", "attribution"],
                                                          warn=warned.append).items()}
        (r,) = got["latency"]
        self.assertEqual((r["phase"], r["chunks"], r["held"], r["never"], r["late"], r["late_share"]),
                         ("startup", 4, 3, 1, 0, None))
        self.assertAlmostEqual(r["never_share"], 0.25)
        self.assertEqual((got["late"], got["attribution"]), ([], []))
        self.assertIn(self.WARNING, warned)


class Cli(Fixture):
    def cli(self, *args):
        env = golden_cli.environment(self.root)
        p = subprocess.run([sys.executable, str(golden_cli.SIMDB), "coverage-report", *args], env=env,
                           capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stderr)
        return p.stdout

    def test_its_tables_and_json(self):
        names = [f"--name={arm}={label}" for label, arm in NAMES.items()]
        text = self.cli("--exp", SRC, *names, "--section", "latency", "--section", "late")
        self.assertTrue(text.startswith("## latency\n"))
        self.assertIn("## late\n", text)
        self.assertIn("| decl ", text)
        got = json.loads(self.cli("--exp", SRC, *names, "--section", "routes", "--json"))
        self.assertEqual(sorted(got), ["routes", "warnings"])
        # The routes read no seed log: no warning of one.
        self.assertEqual((got["routes"], got["warnings"]),
                         (self.results["routes"], [w for w in self.warned if w != SEED_LOG_WARNING.format(6)]))
        rest = json.loads(self.cli("--exp", SRC, *names, "--phases", "rest", "--section", "latency", "--json"))
        self.assertNotIn("startup", {r["phase"] for r in rest["latency"]})

    def test_the_seed_log_sections(self):
        # Arm log's: a map's entries as key=value in the text, a JSON
        # object in --json.
        args = ("--exp", LOG, "--name=log=strims-sim-log", "--section", "invariants", "--section", "attribution")
        text = self.cli(*args)
        self.assertIn("## attribution\n", text)
        self.assertIn("| seed_no_copy=1 ", text)
        got = json.loads(self.cli(*args, "--json"))
        want = coverage.report(LOG_SEL, ["invariants", "attribution"], warn=[].append)
        self.assertEqual(got, {**{s: r.records() for s, r in want.items()}, "warnings": []})


if __name__ == "__main__":
    unittest.main()
