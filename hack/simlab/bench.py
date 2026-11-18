# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Component benches in the lake: a scripted stimulus on the real stack and
in the model, compared directly over the tables both sides write.

The shaper bench replays a testbed run's offered packets through each
node's access link in the model (strims-sim's shaper_replay example over
strims_wan::replay): the model's queue and shaper fed the very packets the
testbed's modem was, open loop. Each run has two replays: of every packet
offered (env REPLAY_OFFERS=offered), whose drops the bench scores, and of
those alone the testbed's link carried (REPLAY_OFFERS=carried), whose
departures it scores. A replay is a job of sim `strims-sim`, test
`shaper_replay`, its source run's cell (testbed.cell_of: the cell the
source's own ingest names), arm `replay` (`replay:<variant>` for another;
the carried replay's arm that and `:carried`), its seed its source's place
among its cell's runs (as testbed.ingest numbers them), and env its
source's knobs with SOURCE_K, the source's key, which pairs them per packet
(scoring.sql's shaper_fit_of). A trace link has no replay yet.

  REPLAY_BINARY  where `cargo build --release -p strims-sim --example
                 shaper_replay` run in this checkout writes it
  OFFERS         the replays' kinds, REPLAY_OFFERS's values
  replay_tables(run_dir, out, binary, offers="offered", nodes=None, seed=1,
                knobs=None) -> job
                 one run's replay written into `out`: shaper_replay's
                 tables and its job.json, which it returns
  replays(run_dirs, name, binary=None, arm="replay", nodes=None, seed=1,
          knobs=None, replace=False) -> (src, [job])
                 each run's two replays folded into the cache as source
                 testbed/<name>, their tables in <run_dir>/replay and
                 <run_dir>/replay-carried (the run's own tables,
                 testbed.tables', stay in <run_dir>/tables)

hack/sim-db/simdb.py replay is its command line; `simdb.py index` then takes
the source in, and shaper_fit_of(replay side, testbed side) scores it.
"""

import hashlib
import json
import shutil
import subprocess
from pathlib import Path

from . import testbed

REPLAY_BINARY = testbed.BINARY.with_name("shaper_replay")
# What a replay's env holds beside its source's knobs: its source's key and
# the nodes it replayed, which are no cell's.
REPLAY_KNOBS = ("SOURCE_K", "REPLAY_NODES", "REPLAY_OFFERS")
OFFERS = ("offered", "carried")


def replay_tables(run_dir, out, binary, offers="offered", nodes=None, seed=1, knobs=None):
    """Run `run_dir`'s replay of `offers` (OFFERS) into `out`
    (shaper_replay's tables and job.json); returns its job.json."""
    out = Path(out)
    shutil.rmtree(out, ignore_errors=True)
    args = [str(binary), str(run_dir), str(out), "--offers", offers, "--seed", str(seed)]
    for n in nodes or ():
        args += ["--node", str(n)]
    for k, v in sorted((knobs or {}).items()):
        args += ["--knob", f"{k}={v}"]
    p = subprocess.run(args, capture_output=True, text=True)
    if p.returncode:
        raise RuntimeError(f"{run_dir}: {p.stderr.strip()}")
    return json.loads((out / "job.json").read_text())


def replays(run_dirs, name, binary=None, arm="replay", nodes=None, seed=1, knobs=None, replace=False):
    """Each of `run_dirs`' two replays folded into the cache as source
    testbed/<name> (see the module docs): each one's key `rp-` and a digest
    of the binary, its source's key, the nodes, the seed, the arm and what
    it offers, so a replay by other code keys apart. An existing source is
    refused unless `replace`. Returns the source and the jobs, a run's
    offered replay before its carried."""
    src = f"testbed/{name}"
    testbed.refuse_existing(src, replace)
    binary = Path(binary or REPLAY_BINARY)
    if not binary.exists():
        raise FileNotFoundError(f"no shaper_replay at {binary}: cargo build --release -p strims-sim "
                                f"--example shaper_replay")
    with open(binary, "rb") as f:
        digest = hashlib.file_digest(f, "sha256").hexdigest()
    jobs = []
    for d in run_dirs:
        d = Path(d)
        for offers in OFFERS:
            out = d / ("replay" if offers == "offered" else f"replay-{offers}")
            job = replay_tables(d, out, binary, offers, nodes, seed, knobs)
            a = arm if offers == "offered" else f"{arm}:{offers}"
            job["k"] = "rp-" + hashlib.sha256(json.dumps([digest, job["env"]["SOURCE_K"], sorted(nodes or []), seed,
                                                          a]).encode()).hexdigest()[:16]
            jobs.append({**job, "dir": str(out), "arm": a, "at": None, "wall_s": None, "rss_mb": None,
                         "binary_sha256": digest})
    # A run's place among its cell's runs, as its ingest numbered it: both
    # its replays take it.
    runs = jobs[::len(OFFERS)]
    envs = [{k: v for k, v in j["env"].items() if k not in REPLAY_KNOBS} for j in runs]
    seeds = {}
    for i, c in enumerate(testbed.cells_of(envs)):
        seeds[c] = seeds.get(c, 0) + 1
        for j in jobs[i * len(OFFERS):(i + 1) * len(OFFERS)]:
            j.update(cell=c, seed=seeds[c])
    testbed.export(src, jobs)
    return src, jobs
