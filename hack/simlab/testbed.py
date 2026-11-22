# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Testbed runs in the lake: hack/testbed/run.sh's out dirs, each read as a
run of its twin's cell and folded into the cache as a local suite
invocation's jobs are, so the catalog's tables, verdicts, scoring macros
and views read a testbed run as they read a sim run.

strims-sim's testbed module does the reading (its testbed_tables example):
it writes a run's tables with the sim's own table writer, from the
records the testbed's nodes log and the run's captures, and its
`job.json` names the run's key, its sim (`testbed`), its test
(`calibration_star`), its twin's knobs (its env) and its row. A run's
cell is its knobs' (cell_sql, one keying on every path: the index names
every testbed job's by it, ingest and twins by cells_of): the testbed
variant of the tier cell (suite.TIERS) whose env its knobs are but for
what a testbed run always sets and a tier's cells don't (TESTBED_FIXED:
one clock, its tbf's buckets, honest viewers; links of run.sh's default
class, DEFAULT_LINK), `<cell>@<tree>` with the qdisc tree that shaped it
(TESTBED_QDISC's last stage), its twin that cell's env with those; a run
of no tier cell, or with another TESTBED_ setting, takes a cell named for
its knobs (`tb-<hash>`).

  tables(run_dir, out, binary=None, knobs=None) -> Path
                 the run's tables, written into `out` (a dir not there
                 yet, outside the run dir: fresh_out) by the
                 testbed_tables binary (BINARY's, by default this
                 checkout's release build, where cargo writes it:
                 $CARGO_TARGET_DIR, else target/), `knobs` added to its twin's
  fresh_out(run_dir, out) -> Path
                 `out`, refused inside the run dir or where it exists
  work_dir() -> Path
                 (a context) a dir of the fold's own under the cache's
                 tmp/, removed after
  cells_of(envs) -> [str], cell_of(env) -> str
                 each env's cell (see above; cell_sql's keying)
  ingest(run_dirs, name, binary=None, arm="testbed", knobs=None,
         replace=False) -> (src, [job])
                 the runs' tables folded into the cache as source
                 testbed/<name>; each run a job of its knobs' cell
                 (cell_of's) and `arm`, its seed its place among
                 the runs of its cell (1, 2, ...), as a twin's seeds pair
                 with it, `run` its run dir
  twins(src, binary, name, seeds=8, arm="twin", env=None, kind="one",
        side_arm="one", replace=False, packet_hosts="seed") -> (src, [job])
                 each cell's twin of the runs testbed_side(src, kind,
                 side_arm) names (src a source's LIKE pattern) run at seeds
                 1..N (a scenarios test binary's calibration_star with the
                 cell's knobs and `env`'s), folded into the cache as
                 source testbed/<name>: the scoring macros then pair a
                 twin seed with a testbed run of its number (effects,
                 pairs: testbed/<name> arm `arm` against src arm testbed)
  cells_in(src_like, kind="one", arm="one", packet_hosts="seed")
                 -> {cell: env}
                 a side's cells and their twins' knobs, from the catalog
                 (twin_cells refuses what no one twin stands for), a cell
                 whose runs hold packets (PCAP=1's) logging those of
                 `packet_hosts` (logged_hosts: the seed, all, or host
                 numbers) that they hold (PACKET_LOG, packet_log)
  tier_cells() -> [(kind, cell)]
                 the testbed tier: the robust tier's calibration_star cells
                 a testbed hosts on native nodes and the browsers tier's on
                 browsers (no cable jitter, rate limit or departures, at
                 most MAX_VIEWERS viewers), each honest
  tier_matrices(dest, repeats=2, ref="main") -> [path]
                 the tier as farm testbed matrices, one a cell, each naming
                 its cell, its arrivals' knob and each repeat's joins
                 (run_env: strims-sim's testbed::knobs_of the other way)
  criterion(tb_like, tw_like, kind="one", paired=False,
            budget_stat="mean") -> [row]
                 the meter search's resume criterion (scoring.sql's
                 twin_criterion_of) of a testbed source's runs against
                 their cells' twins, per group and measure and per cell;
                 criterion_text and criterion_met read its rows

A fold only reads a run dir: the dir is its maker's (a run.sh caller's,
another session's, a farm run's local copy under target/sim-farm/*/runs),
and a tables/ in it its maker's too. The fold writes each run's tables in
a dir of its own under the cache's tmp/ (work_dir), folds them from
there and removes it, so a run dir comes out of a fold as it went in.

hack/sim-db/simdb.py testbed, twin, testbed-tier and criterion are its
command line; `simdb.py index` then takes a source in. A testbed source
is a local experiment (exp_id its source), its arm `arm`; a farm run of
the tier's matrices puts its runs in the cache as farm sources
(`simdb.py pull`).
"""

import contextlib
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from . import store_host

ROOT = Path(__file__).resolve().parents[2]
# Where `cargo build --release -p strims-sim --example testbed_tables`
# run in this checkout writes it: $CARGO_TARGET_DIR's (relative to the
# checkout) where set, as cargo reads it, else the checkout's target/.
BINARY = (ROOT / os.environ.get("CARGO_TARGET_DIR", "target")) / "release" / "examples" / "testbed_tables"
# What a testbed run's knobs hold that its tier cell's env doesn't, or
# holds otherwise: one clock (a cell draws real skew), its tbf's buckets
# (a cell has none), honest viewers (a tier's mix or off), the qdisc tree
# and the settings no twin knob takes (TESTBED_*); and the job keys.
TESTBED_FIXED = ("STRIMS_SIM_CLOCK_SKEW", "STRIMS_SIM_CAL_SEED_BURST", "STRIMS_SIM_CAL_VIEWER_BURST",
                 "STRIMS_SIM_HOSTILE")
# The access link class a run.sh node takes unless SEED_LINK, VIEWER_LINK
# or LINKS name another, which a run's knobs name (its links file) and a
# tier cell's don't: as the buckets, it isn't the cell's to differ in.
DEFAULT_LINK = "tbf"
JOB_KNOBS = ("STRIMS_SIM_JOB", "STRIMS_SIM_SERIES", "STRIMS_SIM_SEED", "STRIMS_SIM_OUT")
# What a testbed run recorded for its twin, not configured: its competing
# uploads' starts (each iperf3 report's timestamp, ms of stream), which
# differ by tens of ms from run to run, and the ms of its second its stream
# started at (EPOCH), its twin's epoch phase, where its nodes' grids on Unix
# time sit against the stream's writes. The keying sets them aside; a
# cell's twin takes the starts' median over its runs, and the epoch where
# its runs agree on one (a twin of one run's, as a joins-paired twin is),
# else none, each twin seed drawing its own (twin_cells). And what each
# packet of its Chromium nodes took of their network thread (COST, µs:
# strims-sim's testbed::thread_cost), its host's, which its CPU model (CPU,
# run.sh's `cpu`, no twin knob) sets.
EPOCH = "STRIMS_SIM_CAL_EPOCH_MS"
COST = "STRIMS_SIM_CAL_PACKET_COST_US"
CPU = "STRIMS_SIM_CAL_CPU"
MEASURED = ("STRIMS_SIM_CAL_COMPETE_MS", EPOCH, COST, CPU)


def fresh_out(run_dir, out):
    """`out` as a reader of `run_dir` may write it: refused inside the run
    dir, which its maker owns, or where it is there already (the reader
    writes into no dir it didn't make)."""
    run_dir, out = Path(run_dir), Path(out)
    if out.resolve().is_relative_to(run_dir.resolve()):
        raise ValueError(f"{out}: inside the run dir {run_dir}, which a fold only reads")
    if out.exists():
        raise FileExistsError(f"{out} exists: a run's tables go into a dir of their own")
    return out


@contextlib.contextmanager
def work_dir(prefix="testbed-"):
    """A dir of the fold's own under the cache's tmp/, removed after: the
    runs' tables and the store a fold writes on its way to the cache."""
    tmp = store_host.cache_root() / "tmp"
    tmp.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix=prefix, dir=tmp))
    try:
        yield work
    finally:
        shutil.rmtree(work, ignore_errors=True)


def tables(run_dir, out, binary=None, knobs=None):
    """The run's tables, written into `out` (fresh_out's) by testbed_tables,
    which only reads the run dir."""
    run_dir = Path(run_dir)
    out = fresh_out(run_dir, out)
    binary = Path(binary or BINARY)
    if not binary.exists():
        raise FileNotFoundError(f"no testbed_tables at {binary}: cargo build --release -p strims-sim "
                                f"--example testbed_tables")
    args = [str(binary), str(run_dir), str(out)]
    for k, v in sorted((knobs or {}).items()):
        args += ["--knob", f"{k}={v}"]
    p = subprocess.run(args, capture_output=True, text=True)
    if p.returncode:
        raise RuntimeError(f"{run_dir}: {p.stderr.strip()}")
    return out


# The tiers whose cells a run's knobs name first: the testbed tier's own
# (tier_cells), then the rest by name. Two tiers can hold one env under two
# names (peers' star-15 is robust's star-15-10's).
CELL_TIERS = ("robust", "browsers")


def cell_sql(tiers, temp=False):
    """The keying of a testbed run's cell, as SQL: the macros
    testbed_comparable(env) (`env` less TESTBED_FIXED, the job keys, what
    it MEASURED, the TESTBED_ settings and link specs of DEFAULT_LINK
    alone) and
    testbed_cell(env), over `tiers` (a table of tier, cell, test and env,
    as the catalog's raw.tier_cells): `<tier>:<cell>@<tree>` where the
    run's knobs are a calibration_star tier cell's env but for those
    (CELL_TIERS' first, else the first tier by name; the tier in the name,
    as two tiers hold cells of one name: robust's native star-3-10 and
    browsers') and it holds no TESTBED_ setting but its qdisc tree,
    `<tree>` that tree's last stage (TESTBED_QDISC: root-class): the
    cell's testbed variant, whose twin runs the cell's env
    with what the testbed sets (one clock, its buckets, its link class);
    else `tb-<hash>` of its knobs but the job keys and what it MEASURED. The index names every
    testbed job's cell by it (catalog.JOBS_SQL) and ingest, twins and the
    tests by cells_of, so a run's cell is one function's on every path."""
    t = "TEMP " if temp else ""
    fixed = "[" + ", ".join(store_host.q(k) for k in (*TESTBED_FIXED, *JOB_KNOBS, *MEASURED)) + "]"
    jobs = "[" + ", ".join(store_host.q(k) for k in (*JOB_KNOBS, *MEASURED)) + "]"
    link = store_host.q(DEFAULT_LINK)
    order = " ".join(f"WHEN {store_host.q(x)} THEN {i}" for i, x in enumerate(CELL_TIERS))
    return f"""
CREATE OR REPLACE {t}MACRO testbed_comparable(env) AS map_from_entries(list_filter(map_entries(env), lambda e: NOT (
    list_contains({fixed}, e.key) OR starts_with(e.key, 'TESTBED_')
    OR (list_contains(['STRIMS_SIM_CAL_SEED_LINK', 'STRIMS_SIM_CAL_VIEWER_LINK'], e.key) AND e.value = {link})
    OR (e.key = 'STRIMS_SIM_CAL_LINKS' AND list_bool_and(list_transform(string_split(trim(e.value), ' '),
                                                                        lambda x: split_part(x, ':', 2) = {link}))))));
CREATE OR REPLACE {t}MACRO testbed_cell(knobs) AS coalesce(
    (SELECT arg_min(c.tier || ':' || c.cell, (CASE c.tier {order} ELSE {len(CELL_TIERS)} END, c.tier))
     FROM {tiers} c
     WHERE c.test = 'calibration_star'
       AND list_sort(map_entries(testbed_comparable(c.env))) = list_sort(map_entries(testbed_comparable(knobs)))
       AND NOT list_bool_or(list_transform(map_keys(knobs), lambda x: starts_with(x, 'TESTBED_') AND x <> 'TESTBED_QDISC')))
        || '@' || regexp_extract(knobs['TESTBED_QDISC'], '[^>]*$'),
    'tb-' || sha256(array_to_string(list_sort(list_transform(
        list_filter(map_entries(knobs), lambda e: NOT list_contains({jobs}, e.key)),
        lambda e: e.key || '=' || e.value)), chr(10)))[1:8]);
"""


def map_sql(env):
    """`env` as a SQL MAP(VARCHAR, VARCHAR) literal."""
    if not env:
        return "MAP {}::MAP(VARCHAR, VARCHAR)"
    return "MAP {" + ", ".join(f"{store_host.q(k)}: {store_host.q(v)}" for k, v in sorted(env.items())) + "}"


def tier_rows():
    """suite.TIERS' cells, as the catalog's raw.tier_cells holds them."""
    from . import suite

    return [{"tier": t, "cell": c["name"], "test": c.get("test"), "env": {str(a): str(b) for a, b in c["env"].items()}}
            for t, cells in suite.TIERS.items() for c in cells]


def tiers_sql(name, temp=False):
    """suite.TIERS' cells (tier_rows) as table `name`, for cell_sql."""
    tiers = " UNION ALL ".join(f"SELECT {store_host.q(r['tier'])} AS tier, {store_host.q(r['cell'])} AS cell, "
                               f"{store_host.q(r['test'] or '')} AS test, {map_sql(r['env'])} AS env"
                               for r in tier_rows())
    return f"CREATE OR REPLACE {'TEMP ' if temp else ''}TABLE {name} AS {tiers};"


def cells_of(envs):
    """Each of `envs`' cell, by cell_sql's keying over suite.TIERS."""
    envs = list(envs)
    if not envs:
        return []
    rows = " UNION ALL ".join(f"SELECT {i} AS i, {map_sql(e)} AS env" for i, e in enumerate(envs))
    got = store_host.duck(f"{tiers_sql('tiers', temp=True)}\n{cell_sql('tiers', temp=True)}\n"
                          f"SELECT i, testbed_cell(env) AS cell FROM ({rows}) ORDER BY i;")
    return [r["cell"] for r in got]


def cell_of(env):
    """`env`'s cell (cells_of)."""
    return cells_of([env])[0]


def ingest(run_dirs, name, binary=None, arm="testbed", knobs=None, replace=False):
    """The runs folded into the cache as source testbed/<name> (see the
    module docs), each a job of its knobs' cell (cells_of, as the index
    names it), its tables written in the fold's work_dir; returns the
    source and the jobs, each with `run`, its run dir. An existing source
    is refused unless `replace` (its runs and seeds would go)."""
    src = f"testbed/{name}"
    refuse_existing(src, replace)
    jobs, seeds = [], {}
    with work_dir() as work:
        for i, d in enumerate(run_dirs):
            out = tables(d, work / str(i), binary, knobs)
            job = json.loads((out / "job.json").read_text())
            jobs.append({**job, "dir": str(out), "run": str(d), "arm": arm, "at": None, "wall_s": None,
                         "rss_mb": None})
        for j, c in zip(jobs, cells_of(j["env"] for j in jobs)):
            seeds[c] = seeds.get(c, 0) + 1
            j.update(cell=c, seed=seeds[c])
        export(src, jobs)
    for j in jobs:
        del j["dir"]
    return src, jobs


def refuse_existing(src, replace):
    """Refuses to write cache source `src` over one, unless `replace`."""
    if (store_host.cache_root() / "cache" / src).exists() and not replace:
        raise FileExistsError(f"{src} exists: its runs (and their seeds) would go; name another, or replace it")


def export(src, jobs):
    """`jobs` folded into a store in a work_dir and exported to the cache
    as `src`."""
    with work_dir() as work:
        db = work / "store.duckdb"
        store_host.fold(db, jobs, src)
        store_host.export(db, store_host.cache_root() / "cache" / src)


def twin_cells(rows):
    """The cells a twin runs, {cell: env}, of testbed runs' (cell, env)
    rows: refused where a cell holds runs of two knobs (what the keying
    sets aside, two buckets, say), which one twin can't stand for, or a run
    holds a TESTBED_ setting other than its qdisc tree, which no twin knob
    takes (a refused link, a seed's loss, run.sh's late date stamps). What
    the runs MEASURED is each cell's median over them (median_measured),
    but EPOCH, the runs' one where every run holds the same, else left
    out, COST (twin_cost, by the runs' CPU), where every run holds one,
    and CPU, which only says whose COST it is. `rows` come in each cell's
    run order (1, 2, ...)."""
    out, measured, runs = {}, {}, {}
    for cell, env in rows:
        runs[cell] = runs.get(cell, 0) + 1
        odd = sorted(k for k in env if k.startswith("TESTBED_") and k != "TESTBED_QDISC")
        if odd:
            raise ValueError(f"cell {cell}: {', '.join(odd)}, which no twin knob takes: no twin")
        env = dict(env)
        got = {k: env.pop(k) for k in MEASURED if k in env}
        if cell in out and out[cell] != env:
            diff = sorted(k for k in {*out[cell], *env} if out[cell].get(k) != env.get(k))
            raise ValueError(f"cell {cell} holds runs of two knobs ({', '.join(diff)}): no one twin")
        out[cell] = env
        for k, v in got.items():
            measured.setdefault(cell, {}).setdefault(k, []).append(v)
    for cell, ks in measured.items():
        for k, values in ks.items():
            if k == COST:
                if len(values) == runs[cell]:
                    cpus = ks.get(CPU, [])
                    out[cell][k] = twin_cost(values, cpus if len(cpus) == runs[cell] else [])
            elif k == CPU:
                pass
            elif k != EPOCH:
                out[cell][k] = median_measured(cell, k, values)
            elif len(values) == runs[cell] and len(set(values)) == 1:
                out[cell][k] = values[0]
    return dict(sorted(out.items()))


def twin_cost(values, cpus):
    """A cell's twin's COST of its runs' (`values`, in run order) by their
    hosts' CPU models (`cpus`, each run's CPU, or none where a run's fold
    recorded none): the costs' median, to a tenth of a µs, where every run
    names one model, which sets the cost; else each run's, joined by
    spaces, a twin seed taking the one of the run it pairs with
    (seed_knobs), so the twin's seeds run on the runs' hosts' costs in their
    shares, as a joins-paired twin runs on its run's. A cell with a run of
    no model pairs too: one model's runs read 12.1-16.0 µs (tier r6's
    star-15-10-event, all on c7a's EPYC 9R14) where the models' own costs
    lie 12.3, 15.4 and 18.1, so no bound on the costs alone tells one
    model's runs from two models', and a seed taking its run's cost runs on
    a host a run had where two models' median runs on neither."""
    if len(cpus) == len(values) and len(set(cpus)) == 1:
        costs = sorted(float(v) for v in values)
        mid = len(costs) // 2
        med = costs[mid] if len(costs) % 2 else (costs[mid - 1] + costs[mid]) / 2
        return f"{med:.1f}"
    return " ".join(values)


def seed_knobs(knobs, seed):
    """A cell's twin knobs (twin_cells') for its twin seed `seed`: a COST of
    its runs' each (twin_cost) the one of the run the seed pairs with, run
    (seed - 1) mod their count in run order, as a twin seed pairs with the
    testbed run of its number."""
    costs = knobs.get(COST, "").split()
    if len(costs) < 2:
        return knobs
    return {**knobs, COST: costs[(seed - 1) % len(costs)]}


def median_measured(cell, knob, values):
    """The median of a MEASURED knob's `<ms>:<secs> ...` values over a
    cell's runs, each upload's ms alone (the middle one, or the two middle
    ones' mean, to the ms); refused where the runs hold two counts of
    uploads or two lengths."""
    runs = [[p.split(":", 1) for p in v.split()] for v in values]
    if len({len(r) for r in runs}) != 1 or len({tuple(s for _, s in r) for r in runs}) != 1:
        raise ValueError(f"cell {cell}: its runs' {knob} hold two sets of uploads ({', '.join(values)}): no one twin")
    out = []
    for i, (_, secs) in enumerate(runs[0]):
        ms = sorted(float(r[i][0]) for r in runs)
        mid = len(ms) // 2
        med = ms[mid] if len(ms) % 2 else (ms[mid - 1] + ms[mid]) / 2
        out.append(f"{round(med)}:{secs}")
    return " ".join(out)


def cells_in(src_like, kind="one", arm="one", packet_hosts="seed"):
    """The cells of the testbed runs a side names (testbed_side(src_like,
    kind, arm) over the catalog: a local ingest's testbed/<name>, a farm
    run's farm/<run>/%), {cell: env}, their twins' knobs (twin_cells), each
    with PACKET_LOG naming those of `packet_hosts` (logged_hosts) whose
    packets its runs hold (the macro packet_hosts_of), where any; the
    catalog's, so `simdb.py index` first."""
    from . import catalog

    side = (f"testbed_side({store_host.q(src_like)}, kind := {store_host.q(kind)}, "
            f"arm := {store_host.q(arm)})")
    rows = catalog.lake(f"SELECT DISTINCT j.stored_cell AS cell, j.seed, j.env FROM jobs j SEMI JOIN (SELECT "
                        f"unnest({side}, recursive := true)) s ON s.k = j.k ORDER BY 1, 2;")
    envs = [r["env"] if isinstance(r["env"], dict) else dict(r["env"]) for r in rows]
    out = twin_cells([(r["cell"], {k: v for k, v in e.items() if k not in JOB_KNOBS}) for r, e in zip(rows, envs)])
    if out:
        for r in catalog.lake(f"SELECT cell, hosts FROM packet_hosts_of({side}) ORDER BY cell;"):
            if hosts := logged_hosts(r["hosts"], packet_hosts):
                out[r["cell"]][PACKET_LOG] = packet_log(hosts)
    return out


# The knob a twin logs the packets its hosts offered their uplinks by
# (`packets`; strims-sim's packet_log_hosts), which a testbed run with
# PCAP=1 holds of every node and the wire fits compare (sctp_fit_of,
# write_fit_of, at their at_node). PCAP is no knob of a run's (a capture
# would key its cell apart), so a twin's hosts are among those whose
# packets its side holds.
PACKET_LOG = "STRIMS_SIM_PACKET_LOG"
# The host the wire fits read by default (their at_node): the first
# swarm's seed, the testbed's node 0 and its twin's host 0. A twin logs its
# packets alone unless asked for more: the log is held to the run's end,
# and every host of star-31-5's 30 s held 1.17 M rows, 180 MB more at the
# peak (fields.toml's packets), for each of twins' parallel runs.
SEED_HOST = 0


def logged_hosts(held, want="seed"):
    """The hosts of `held` (those whose packets a cell's runs hold) that
    `want` names: `seed` (SEED_HOST), `all`, or host numbers; in order."""
    if want == "all":
        names = set(held)
    elif want == "seed":
        names = {SEED_HOST}
    else:
        names = {int(h) for h in want}
    return sorted(h for h in held if h in names)


def packet_log(hosts):
    """PACKET_LOG's value naming `hosts`, their numbers joined by commas: a
    lone host 1 named twice, as `1` alone reads as every host."""
    v = ",".join(str(h) for h in sorted(set(hosts)))
    return "1,1" if v == "1" else v


# The tables a twin writes beyond a sim job's: each node's bins and
# channels (node_bins, peer_samples), which a testbed run's tables hold and
# twin_runs_of reads (seed_wire_per_written, the channels' measures).
TWIN_TABLES = {"STRIMS_SIM_BIN_NODES": "1"}


def twins(src, binary, name, seeds=8, arm="twin", env=None, parallel=8, kind="one", side_arm="one",
          replace=False, packet_hosts="seed"):
    """Each cell's twin of the testbed runs side (src, kind, side_arm)
    names (cells_in: `src` a source's LIKE pattern): strims-sim's
    calibration_star (the scenarios test binary `binary`) at seeds 1..N
    with the cell's knobs (cells_in's, a COST of its runs' each the seed's
    run's (seed_knobs), the packet log of those of
    `packet_hosts` whose packets the cell's runs hold among them; its
    TESTBED_QDISC left out), TWIN_TABLES and `env`'s, one farm job's way
    each (STRIMS_SIM_JOB=1) into a dir of work_dir's, their tables folded
    into the cache as source testbed/<name> (refused over one, unless
    `replace`), each a job of its cell, `arm` and seed. Returns the source
    and the jobs."""
    import concurrent.futures

    binary = Path(binary)
    with open(binary, "rb") as f:
        digest = hashlib.file_digest(f, "sha256").hexdigest()
    refuse_existing(f"testbed/{name}", replace)
    cells = cells_in(src, kind, side_arm, packet_hosts)
    if not cells:
        raise ValueError(f"no testbed runs in {src} ({kind}, {side_arm}): simdb.py index takes a source in")
    todo = []
    for cell, knobs in cells.items():
        base = {k: v for k, v in knobs.items() if not k.startswith("TESTBED_")}
        for seed in range(1, seeds + 1):
            todo.append((cell, {**seed_knobs(base, seed), **TWIN_TABLES, **(env or {}), "STRIMS_SIM_JOB": "1"}, seed))

    def one(t):
        cell, e, seed = t
        key = json.dumps([digest, sorted(e.items()), seed])
        k = "tw-" + hashlib.sha256(key.encode()).hexdigest()[:16]
        out = work / k
        run = {**e, "STRIMS_SIM_SEED": str(seed), "STRIMS_SIM_OUT": str(out)}
        p = subprocess.run([str(binary), "--exact", "calibration_star", "--include-ignored", "--nocapture",
                            "--test-threads", "1"], env={**_base_env(), **run}, capture_output=True, text=True)
        rows = [line[line.find("ROW {") + 4:] for line in p.stdout.splitlines() if "ROW {" in line]
        if p.returncode or len(rows) != 1:
            raise RuntimeError(f"{cell} seed {seed}: exit {p.returncode}\n{p.stderr[-2000:]}")
        return {"k": k, "dir": str(out), "sim": "strims-sim", "test": "calibration_star", "cell": cell, "arm": arm,
                "seed": seed, "env": e, "row": json.loads(rows[0]), "binary_sha256": digest}

    with work_dir("twins-") as work:
        with concurrent.futures.ThreadPoolExecutor(parallel) as ex:
            jobs = list(ex.map(one, todo))
        out = f"testbed/{name}"
        export(out, jobs)
    return out, jobs


def _base_env():
    """The environment a twin runs in: PATH and HOME alone, as a farm job's."""
    import os

    return {k: os.environ[k] for k in ("PATH", "HOME") if k in os.environ}


# --- the testbed tier

# The knobs a tier cell may hold that the testbed can't stand for: a cable
# path's jitter, a node's rate limit, departures (run.sh's nodes stay to the
# stream's end). A cell with one isn't the tier's.
UNHOSTED = ("STRIMS_SIM_CAL_JITTER", "STRIMS_SIM_CAL_RATE_LIMIT_PCT", "STRIMS_SIM_DEPART")
# The most viewers a tier cell has: native nodes (32 containers fit one
# host easily), and browsers (the testbed skill's 16-node full mesh).
MAX_VIEWERS = {"native": 31, "browsers": 15}
# Each kind's host, by viewers: about half idle at the testbed skill's
# per-node CPU (native nodes 0.01-0.07 core; a browser 0.1-0.55 by peers).
HOST_CORES = {"native": ((15, 4), (31, 8)), "browsers": ((3, 4), (7, 8), (15, 16))}


def tier_cells():
    """The testbed tier's cells, (kind, cell): the robust tier's
    calibration_star cells for native nodes and the browsers tier's for
    browsers, the testbed can host (UNHOSTED, MAX_VIEWERS), each honest
    (STRIMS_SIM_HOSTILE=off), as their twins run it."""
    from . import suite

    out = []
    for kind, tier in (("native", "robust"), ("browsers", "browsers")):
        for c in suite.TIERS[tier]:
            env = c["env"]
            if c["test"] != "calibration_star" or any(k in env for k in UNHOSTED):
                continue
            if int(env.get("STRIMS_SIM_CAL_VIEWERS", "3")) > MAX_VIEWERS[kind]:
                continue
            out.append((kind, {**c, "env": {**env, "STRIMS_SIM_HOSTILE": "off"}}))
    return out


def joins_of(arrivals, viewers, secs, rng):
    """JOINS for an arrivals profile (crates/strims-sim's arrivals.rs, its
    knob's form): each viewer's join, in seconds after the stream's start,
    as the twin draws them (trickle: uniform within the stream's first
    third; event:<warm %>:<at>:<window>: the warm share uniform before
    `at`, the rest uniform within `window` from it), the k-th soonest
    viewer k's, so the topology's order is the arrivals' as the twin lays
    its links out, to the ms below, as the twin's draws are and a node's
    --join-at takes them (rounding up could put a warm join on its
    burst's start). A join falls anywhere in the runner's 100 ms tick
    (strims-ppspp's TICK_INTERVAL), as a deployed viewer's does: to the
    tenth of a second every join would sit on its grid, and each
    node's tick, set from its first channel, would land at one place in
    every later joiner's link-up (strims-sim's testbed module keys such
    a run apart, TESTBED_JOINS)."""
    parts = arrivals.split(":")
    if parts == ["trickle"]:
        times = [rng.uniform(0, secs / 3) for _ in range(viewers)]
    elif parts[0] == "event" and len(parts) == 4:
        warm, at, window = float(parts[1]), float(parts[2]), float(parts[3])
        m = min(viewers, round(viewers * warm / 100))
        times = [rng.uniform(0, at) for _ in range(m)] + [at + rng.uniform(0, window) for _ in range(viewers - m)]
    else:
        raise ValueError(f"arrivals {arrivals}: the tier takes trickle and event profiles")
    return " ".join(f"{i}:{math.floor(t * 1000) / 1000:.3f}" for i, t in enumerate(sorted(times), start=1))


def num(x):
    """A number as run.sh's settings take it: 5, 2.5."""
    return f"{x:g}"


def run_env(kind, cell):
    """run.sh's settings for a tier cell ((kind, cell) of tier_cells), the
    knobs testbed_tables adds to the run's (its arrivals), and the topology
    file (topologies/<name>, relative to the matrices)."""
    env = cell["env"]
    viewers = int(env["STRIMS_SIM_CAL_VIEWERS"])
    mbit = float(env.get("STRIMS_SIM_CAL_SEED_UP_MBIT", "10"))
    out = {"VIEWERS": str(viewers), "SEED_UP_MBIT": num(mbit), "SECS": env.get("STRIMS_SIM_CAL_SECS", "180"),
           "PCAP": "1"}
    if "STRIMS_SIM_CAL_VIEWER_UP_MBIT" in env:
        out["VIEWER_UP_MBIT"] = env["STRIMS_SIM_CAL_VIEWER_UP_MBIT"]
    if "STRIMS_SIM_CAL_SEED_UP_SCALE" in env:
        out["SEED_UP_STEPS"] = " ".join(f"{a}:{num(float(f) * mbit)}" for a, f in
                                        (p.split(":") for p in env["STRIMS_SIM_CAL_SEED_UP_SCALE"].split()))
    if "STRIMS_SIM_CHUNK" in env:
        args = f"--chunk {env['STRIMS_SIM_CHUNK']} --munro {env['STRIMS_SIM_MUNRO']}"
        out["NODE_ARGS"] = args
    mesh = env.get("STRIMS_SIM_CAL_MESH")
    topology = {"full": f"full-v{viewers}", "mesh6": f"mesh6-v{viewers}", None: None}[mesh]
    if kind == "browsers":
        stacks = env["STRIMS_SIM_RTC"].split(",")
        out["KINDS"] = " ".join(stacks[i % len(stacks)] for i in range(viewers + 1))
        out["BROWSER_LINKS"] = "webrtc"
        if "NODE_ARGS" in out:
            out["BROWSER_ARGS"] = out["NODE_ARGS"]
    knobs = {k: env[k] for k in ("STRIMS_SIM_ARRIVALS",) if k in env}
    return out, knobs, topology


def cores(kind, viewers):
    return next(c for v, c in HOST_CORES[kind] if viewers <= v)


def run_sh_stamps():
    """The clock run.sh says it stamps from (the STAMPS its settings get):
    `clock`, or `date` (uutils' late one), whose runs key apart."""
    m = re.search(r'echo "STAMPS=(\w+)"', (ROOT / "hack" / "testbed" / "run.sh").read_text())
    return m.group(1) if m else "date"


def tier_matrices(dest, repeats=2, ref="main"):
    """The testbed tier as farm testbed matrices in `dest`, one a cell
    (testbed-<kind>-<cell>.toml), each naming its tier cell (`cell`) and its
    arrivals' knob (`knobs`), each repeat its own draw of a trickle's or an
    event's joins ([repeat_env] JOINS: random.Random(cell, kind, repeat)),
    at the arm `ref`; returns their paths. A twin's seed r pairs with
    repeat r - 1's run. While run.sh stamps from `date` (run_sh_stamps),
    each one's header says its runs key apart."""
    import random

    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    matrices = Path(__file__).resolve().parents[1] / "sim-farm" / "matrices"
    out = []
    for kind, cell in tier_cells():
        env, knobs, topology = run_env(kind, cell)
        viewers = int(env["VIEWERS"])
        lines = [f"# The testbed tier's {cell['name']} ({kind}): {cell['why']}, honest, against its twin",
                 "# (simlab.testbed.tier_matrices writes it).", "",
                 *(["# Not yet: run.sh writes STAMPS=date, so every run keys apart (TESTBED_STAMPS) and has no",
                    "# twin until run.sh stamps from its clock (STAMPS=clock). Don't run it on the farm before then.",
                    ""] if run_sh_stamps() != "clock" else []),
                 'kind = "testbed"', f"repeats = {repeats}", f"host_cores = {cores(kind, viewers)}",
                 f'cell = "{cell["name"]}"']
        builds = ["cargo build --release --locked -p strims-sim --example testbed_node --example testbed_tables"]
        arts = {"NODE_BIN": "$CARGO_TARGET_DIR/release/examples/testbed_node",
                "TABLES_BIN": "$CARGO_TARGET_DIR/release/examples/testbed_tables"}
        if kind == "browsers":
            builds += ["cargo build --release --locked -p strims-testbed --example testbed_browser",
                       "export PATH=$HOME/.nvm/versions/node/v26.11.1/bin:$PATH && pnpm install --frozen-lockfile "
                       "--offline && pnpm exec webpack --mode production --env TARGET=wasm-testbed"]
            arts |= {"BROWSER_BIN": "$CARGO_TARGET_DIR/release/examples/testbed_browser",
                     "WEB_BUNDLE": "dist/wasm-testbed"}
        lines += ["", "[build]", "commands = [" + ", ".join(json.dumps(b) for b in builds) + "]", "", "[artifacts]"]
        lines += [f'{k} = "{v}"' for k, v in arts.items()]
        if topology:
            if not (matrices / "topologies" / topology).exists():
                raise FileNotFoundError(f"no topology {topology} in {matrices / 'topologies'}")
            rel = os.path.relpath(matrices / "topologies" / topology, dest.resolve())
            lines += ["", "[files]", f'TOPOLOGY = "{rel}"']
        lines += ["", "[arms]", f'main = "{ref}"', "", "[env]"]
        lines += [f"{k} = {json.dumps(v)}" for k, v in env.items()]
        if knobs:
            lines += ["", "[knobs]"] + [f"{k} = {json.dumps(v)}" for k, v in knobs.items()]
            arrivals = knobs["STRIMS_SIM_ARRIVALS"]
            draws = [joins_of(arrivals, viewers, float(env["SECS"]), random.Random(f"{cell['name']}:{kind}:{r}"))
                     for r in range(repeats)]
            lines += ["", "[repeat_env]", "JOINS = [" + ", ".join(json.dumps(d) for d in draws) + "]"]
        path = dest / f"testbed-{kind}-{cell['name']}.toml"
        path.write_text("\n".join(lines) + "\n")
        out.append(path)
    return out


BUDGET_STATS = ("mean", "median")

# What the criterion scores, a summary row each: every group of a kind's
# cells and every measure.
CRITERION_GROUPS = ("all", "arrivals", "at once")
CRITERION_MEASURES = ("truth_bps", "seed_upload", "seed_budget")


def criterion(tb_like, tw_like, kind="one", paired=False, budget_stat="mean"):
    """The meter search's resume criterion (crates/PLAN.md) over the
    catalog: scoring.sql's twin_criterion_of of the testbed runs
    testbed_side(tb_like, kind) names against the twin runs of their cells
    whose source matches the LIKE pattern tw_like, with `paired` each
    testbed run against its own joins-paired twins where its cell has them,
    and seed_budget's statistic `budget_stat` (BUDGET_STATS). The criterion
    is judged a kind at a time, so `kind` is native, browsers or one, never
    any. Its rows as dicts: a group's and measure's (cell None) first, then
    a cell's and measure's."""
    from . import catalog

    if kind == "any":
        raise ValueError("criterion: kind any mixes native and browser runs; the criterion is judged per kind")
    if budget_stat not in BUDGET_STATS:
        raise ValueError(f"budget_stat {budget_stat} is none of {', '.join(BUDGET_STATS)}")
    q = store_host.q
    twin = (f"SELECT list({{'cell': stored_cell, 'seed': seed, 'k': k, 'src': src}} ORDER BY stored_cell, src, seed) "
            f"FROM jobs WHERE sim <> 'testbed' AND src LIKE {q(tw_like)} "
            f"AND list_contains((SELECT list_transform(side, x -> x.cell) FROM b), stored_cell)")
    none = (f"error('criterion: no twin run in ' || {q(tw_like)} || ' of the cells of the testbed runs in ' "
            f"|| {q(tb_like)})")
    return catalog.lake(
        f"WITH b AS (SELECT testbed_side({q(tb_like)}, kind := {q(kind)}) AS side), a AS ({twin}) "
        f"SELECT * FROM twin_criterion_of(CASE WHEN (SELECT * FROM a) IS NULL THEN {none} ELSE (SELECT * FROM a) END, "
        "(SELECT side FROM b), "
        f"paired := {'true' if paired else 'false'}, budget_stat := {q(budget_stat)}) "
        f"ORDER BY cell NULLS FIRST, list_position({list(CRITERION_GROUPS)}, grp), "
        f"list_position({list(CRITERION_MEASURES)}, measure), cell;")


def criterion_failures(rows):
    """Why criterion's rows don't meet it, a reason a group and measure
    that fails: no summary row or no cells (the criterion needs every
    group and measure scored), or a bar not met. Empty when it's met."""
    summary = {(r["grp"], r["measure"]): r for r in rows if r["cell"] is None}
    out = []
    for g in CRITERION_GROUPS:
        for m in CRITERION_MEASURES:
            r = summary.get((g, m))
            if r is None or not r["cells"]:
                out.append(f"{g} {m} (no cells)")
            elif not r["pass"]:
                out.append(f"{g} {m}")
    return out


def criterion_met(rows):
    """Whether criterion's rows meet it: every group and measure scored,
    each passing."""
    return not criterion_failures(rows)


def criterion_text(rows):
    """criterion's rows as text: a line a group and measure (its readings,
    each bar's pass), a line a cell and measure (each side's runs, the
    statistic of each and their ratio), and whether they meet it."""
    def num(x, d=3):
        return "-" if x is None else f"{x:.{d}f}"

    def bar(ok):
        return "" if ok is None else ("ok" if ok else "FAIL")

    def table(head, lines):
        widths = [max(len(str(c)) for c in col) for col in zip(head, *lines)]
        return ["  ".join(str(c).ljust(w) for c, w in zip(line, widths)).rstrip() for line in (head, *lines)]

    summary = [r for r in rows if r["cell"] is None]
    out = table(("group", "measure", "stat", "runs tb/twin", "ratio", "", "in band", "", "rank_corr", "",
                 "floor tb vs twin", "floor_p", "", "pass"),
                [(r["grp"], r["measure"], r["stat"], f"{r['tb_runs']}/{r['twin_runs']}", num(r["ratio"]),
                  bar(r["ratio_pass"]), f"{r['in_band']}/{r['cells']}", bar(r["band_pass"]), num(r["rank_corr"]),
                  bar(r["rank_pass"]),
                  "" if r["floor_tb"] is None else
                  f"{r['floor_tb']}/{r['floor_tb_runs']} vs {r['floor_twin']}/{r['floor_twin_runs']}",
                  num(r["floor_p"]), bar(r["floor_pass"]), "yes" if r["pass"] else "no") for r in summary])
    out += [f"  {r['grp']} {r['measure']}: {r['caveat']}" for r in summary if r["caveat"]]
    out += [f"  {r['grp']} {r['measure']}: {r['tb_runs_unpaired']} testbed runs without twins of their own, "
            "left out" for r in summary if r["tb_runs_unpaired"]]
    out += [""] + table(("cell", "group", "measure", "stat", "runs tb/twin", "unpaired", "tb", "twin", "ratio",
                         "b_quantile"),
                        [(r["cell"], r["grp"], r["measure"], r["stat"], f"{r['tb_runs']}/{r['twin_runs']}",
                          r["tb_runs_unpaired"] or "", num(r["tb_value"], 4), num(r["twin_value"], 4),
                          num(r["ratio"]), num(r["b_quantile"]))
                         for r in rows if r["cell"] is not None])
    failed = criterion_failures(rows)
    out += ["", "criterion not met: " + ", ".join(failed) if failed else "criterion met"]
    return "\n".join(out) + "\n"
