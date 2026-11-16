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

  tables(run_dir, binary=None, knobs=None) -> Path
                 the run's tables dir (<run_dir>/tables), written by the
                 testbed_tables binary (BINARY's, by default this
                 checkout's release build), `knobs` added to its twin's
  cells_of(envs) -> [str], cell_of(env) -> str
                 each env's cell (see above; cell_sql's keying)
  ingest(run_dirs, name, binary=None, arm="testbed", knobs=None,
         replace=False) -> (src, [job])
                 the runs' tables folded into the cache as source
                 testbed/<name>; each run a job of its knobs' cell
                 (cell_of's) and `arm`, its seed its place among
                 the runs of its cell (1, 2, ...), as a twin's seeds pair
                 with it
  twins(src, binary, name, seeds=8, arm="twin", env=None, kind="one",
        side_arm="one", replace=False) -> (src, [job])
                 each cell's twin of the runs testbed_side(src, kind,
                 side_arm) names (src a source's LIKE pattern) run at seeds
                 1..N (a scenarios test binary's calibration_star with the
                 cell's knobs and `env`'s), folded into the cache as
                 source testbed/<name>: the scoring macros then pair a
                 twin seed with a testbed run of its number (effects,
                 pairs: testbed/<name> arm `arm` against src arm testbed)
  cells_in(src_like, kind="one", arm="one") -> {cell: env}
                 a side's cells and their twins' knobs, from the catalog
                 (twin_cells refuses what no one twin stands for)
  tier_cells() -> [(kind, cell)]
                 the testbed tier: the robust tier's calibration_star cells
                 a testbed hosts on native nodes and the browsers tier's on
                 browsers (no cable jitter, rate limit or departures, at
                 most MAX_VIEWERS viewers), each honest
  tier_matrices(dest, repeats=2, ref="main") -> [path]
                 the tier as farm testbed matrices, one a cell, each naming
                 its cell, its arrivals' knob and each repeat's joins
                 (run_env: strims-sim's testbed::knobs_of the other way)

hack/sim-db/simdb.py testbed, twin and testbed-tier are its command line;
`simdb.py index` then takes a source in. A testbed source is a local
experiment (exp_id its source), its arm `arm`; a farm run of the tier's
matrices puts its runs in the cache as farm sources (`simdb.py pull`).
"""

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
BINARY = ROOT / "target" / "release" / "examples" / "testbed_tables"
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


def tables(run_dir, binary=None, knobs=None):
    """The run's tables, written into <run_dir>/tables by testbed_tables."""
    run_dir = Path(run_dir)
    out = run_dir / "tables"
    shutil.rmtree(out, ignore_errors=True)
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
    testbed_comparable(env) (`env` less TESTBED_FIXED, the job keys, the
    TESTBED_ settings and link specs of DEFAULT_LINK alone) and
    testbed_cell(env), over `tiers` (a table of tier, cell, test and env,
    as the catalog's raw.tier_cells): `<tier>:<cell>@<tree>` where the
    run's knobs are a calibration_star tier cell's env but for those
    (CELL_TIERS' first, else the first tier by name; the tier in the name,
    as two tiers hold cells of one name: robust's native star-3-10 and
    browsers') and it holds no TESTBED_ setting but its qdisc tree,
    `<tree>` that tree's last stage (TESTBED_QDISC: root-class): the
    cell's testbed variant, whose twin runs the cell's env
    with what the testbed sets (one clock, its buckets, its link class);
    else `tb-<hash>` of its knobs but the job keys. The index names every
    testbed job's cell by it (catalog.JOBS_SQL) and ingest, twins and the
    tests by cells_of, so a run's cell is one function's on every path."""
    t = "TEMP " if temp else ""
    fixed = "[" + ", ".join(store_host.q(k) for k in (*TESTBED_FIXED, *JOB_KNOBS)) + "]"
    jobs = "[" + ", ".join(store_host.q(k) for k in JOB_KNOBS) + "]"
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
    names it); returns the source and the jobs. An existing source is
    refused unless `replace` (its runs and seeds would go)."""
    src = f"testbed/{name}"
    refuse_existing(src, replace)
    jobs, seeds = [], {}
    for d in run_dirs:
        out = tables(d, binary, knobs)
        job = json.loads((out / "job.json").read_text())
        jobs.append({**job, "dir": str(out), "arm": arm, "at": None, "wall_s": None, "rss_mb": None})
    for j, c in zip(jobs, cells_of(j["env"] for j in jobs)):
        seeds[c] = seeds.get(c, 0) + 1
        j.update(cell=c, seed=seeds[c])
    export(src, jobs)
    return src, jobs


def refuse_existing(src, replace):
    """Refuses to write cache source `src` over one, unless `replace`."""
    if (store_host.cache_root() / "cache" / src).exists() and not replace:
        raise FileExistsError(f"{src} exists: its runs (and their seeds) would go; name another, or replace it")


def export(src, jobs):
    """`jobs` folded into a store and exported to the cache as `src`."""
    (store_host.cache_root() / "tmp").mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="testbed-", dir=store_host.cache_root() / "tmp"))
    try:
        db = work / "store.duckdb"
        store_host.fold(db, jobs, src)
        store_host.export(db, store_host.cache_root() / "cache" / src)
    finally:
        shutil.rmtree(work, ignore_errors=True)


def twin_cells(rows):
    """The cells a twin runs, {cell: env}, of testbed runs' (cell, env)
    rows: refused where a cell holds runs of two knobs (what the keying
    sets aside, two buckets, say), which one twin can't stand for, or a run
    holds a TESTBED_ setting other than its qdisc tree, which no twin knob
    takes (a refused link, a seed's loss, run.sh's late date stamps)."""
    out = {}
    for cell, env in rows:
        odd = sorted(k for k in env if k.startswith("TESTBED_") and k != "TESTBED_QDISC")
        if odd:
            raise ValueError(f"cell {cell}: {', '.join(odd)}, which no twin knob takes: no twin")
        if cell in out and out[cell] != env:
            diff = sorted(k for k in {*out[cell], *env} if out[cell].get(k) != env.get(k))
            raise ValueError(f"cell {cell} holds runs of two knobs ({', '.join(diff)}): no one twin")
        out[cell] = env
    return dict(sorted(out.items()))


def cells_in(src_like, kind="one", arm="one"):
    """The cells of the testbed runs a side names (testbed_side(src_like,
    kind, arm) over the catalog: a local ingest's testbed/<name>, a farm
    run's farm/<run>/%), {cell: env}, their twins' knobs (twin_cells); the
    catalog's, so `simdb.py index` first."""
    from . import catalog

    rows = catalog.lake(f"SELECT DISTINCT j.stored_cell AS cell, j.env FROM jobs j SEMI JOIN (SELECT unnest("
                        f"testbed_side({store_host.q(src_like)}, kind := {store_host.q(kind)}, "
                        f"arm := {store_host.q(arm)}), recursive := true)) s ON s.k = j.k ORDER BY 1;")
    envs = [r["env"] if isinstance(r["env"], dict) else dict(r["env"]) for r in rows]
    return twin_cells([(r["cell"], {k: v for k, v in e.items() if k not in JOB_KNOBS}) for r, e in zip(rows, envs)])


# The tables a twin writes beyond a sim job's: each node's bins and
# channels (node_bins, peer_samples), which a testbed run's tables hold and
# twin_runs_of reads (seed_wire_per_written, the channels' measures).
TWIN_TABLES = {"STRIMS_SIM_BIN_NODES": "1"}


def twins(src, binary, name, seeds=8, arm="twin", env=None, parallel=8, kind="one", side_arm="one",
          replace=False):
    """Each cell's twin of the testbed runs side (src, kind, side_arm)
    names (cells_in: `src` a source's LIKE pattern): strims-sim's
    calibration_star (the scenarios test binary `binary`) at seeds 1..N
    with the cell's knobs (its TESTBED_QDISC left out), TWIN_TABLES and
    `env`'s, one
    farm job's way each (STRIMS_SIM_JOB=1), their tables folded into the
    cache as source testbed/<name> (refused over one, unless `replace`),
    each a job of its cell, `arm` and seed. Returns the source and the
    jobs."""
    import concurrent.futures

    binary = Path(binary)
    with open(binary, "rb") as f:
        digest = hashlib.file_digest(f, "sha256").hexdigest()
    refuse_existing(f"testbed/{name}", replace)
    cells = cells_in(src, kind, side_arm)
    if not cells:
        raise ValueError(f"no testbed runs in {src} ({kind}, {side_arm}): simdb.py index takes a source in")
    work = Path(tempfile.mkdtemp(prefix="twins-", dir=ROOT / "target"))
    todo = []
    for cell, knobs in cells.items():
        base = {k: v for k, v in knobs.items() if not k.startswith("TESTBED_")}
        for seed in range(1, seeds + 1):
            todo.append((cell, {**base, **TWIN_TABLES, **(env or {}), "STRIMS_SIM_JOB": "1"}, seed))

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

    try:
        with concurrent.futures.ThreadPoolExecutor(parallel) as ex:
            jobs = list(ex.map(one, todo))
        out = f"testbed/{name}"
        export(out, jobs)
    finally:
        shutil.rmtree(work, ignore_errors=True)
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
    its links out, to the tenth of a second below (a node's --join-at
    takes seconds to the ms; rounding up could put a warm join on its
    burst's start)."""
    parts = arrivals.split(":")
    if parts == ["trickle"]:
        times = [rng.uniform(0, secs / 3) for _ in range(viewers)]
    elif parts[0] == "event" and len(parts) == 4:
        warm, at, window = float(parts[1]), float(parts[2]), float(parts[3])
        m = min(viewers, round(viewers * warm / 100))
        times = [rng.uniform(0, at) for _ in range(m)] + [at + rng.uniform(0, window) for _ in range(viewers - m)]
    else:
        raise ValueError(f"arrivals {arrivals}: the tier takes trickle and event profiles")
    return " ".join(f"{i}:{math.floor(t * 10) / 10:.1f}" for i, t in enumerate(sorted(times), start=1))


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
