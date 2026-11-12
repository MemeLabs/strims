# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The trace-neutral check (`suite.py neutral`): a change that adds a log
or a metric to the sims changes no run's behaviour. Against its parent
commit every run keeps its trace and its reads, and every table, row and
series_meta stays the same byte for byte, but for what the change means
to move (`expect`).

  check(bases=(), head="HEAD", cells=None, seeds=SEEDS, expect=(),
        slots=None, mem_gib=None, log=None) -> Report
                 builds the head and each base (build), runs every cell at
                 every seed under each with every table and log on (run),
                 and compares each base's runs with the head's (compare);
                 Report.text() and .json() say what moved, .exit_code() is
                 0, 1 for a move nothing expects, 2 for a build or run that
                 failed
  pairs(bases, head) -> (head sha, [base sha])
                 the commits checked: `head` (TREE, ".", for a snapshot of
                 the working tree), against each of `bases`, by default
                 its parents; a merge commit against every parent, whatever
                 the bases
  cell_set(spec=DEFAULT_SET) -> [cell]
                 a set of SETS (AUDIT, the default, or VOCAB, every
                 writer's), a suite tier, or cell names, comma-separated
                 (SETS' cells, then the tiers'), each with LOGS on
  build(sha, log) -> (bin dir, build seconds or None when kept)
                 `sha`'s test binaries of both sims (BUILD), built once in
                 a detached worktree of their own with a CARGO_TARGET_DIR
                 of their own (target/neutral/bin/SHA12)
  run(bins, cells, seeds, slots, mem_gib, log) -> ({(sha, cell, seed):
        Run}, failures)
                 each binary dir's runs, kept by commit, cell, settings and
                 seed (target/neutral/runs/SHA12/), each run's tables and
                 its record: its row and its files' digests (manifest)
  compare(a, b, expect) -> [Move]
                 what moved from run `a` to run `b`
  manifest(dir) -> {name: {file, sha256, bytes, lines, zst?}}
                 every file of a run's tables dir by name, `.zst` left off,
                 hashed as its decompressed bytes

A move is named as `--expect` names it: TABLE (a table, or a file by its
stem: series_meta, a schema's table) for a change of its rows, order or
presence, TABLE.COL for one column's values, type or presence, row.KEY
for a field of the run's ROW, series_meta.KEY for one of series_meta's.
`expect` allows a move by its name or its table's (`row` allows every
field but trace and reads_hash, the behaviour itself, which only their
own names allow).

A table that differs is diagnosed streaming, a row at a time: with the
same row count, column by column at each row (TABLE.COL, its count and
its first rows); else as multisets of rows over the columns nothing
expects, hashed into partition files (rows only in each, their first
rows, and the columns near pairs of them differ in, TABLE), and where most
rows moved each column left out in turn, which finds one renumbered after
an insertion (an id) and the rows that moved besides it. Memory stays
flat whatever a table's size. A run whose trace moved has its tables'
differences listed, not diagnosed: they follow from it.

The runs go through suite.run_jobs (slots, a memory budget, the heaviest
first); a binary's runs are deterministic, so a run is kept and never run
again. A build's worktree and target dir go once its binaries are copied
out: cargo takes a unit as fresh by its sources' mtimes, so a target dir
fed by more than one tree can hand one commit's units to another.
Builds, runs and the scratch of diagnoses live in the checkout's
target/neutral (WORK); a commit's binaries and runs unused for PRUNE_DAYS
go.

stdlib alone (zstd included: compression.zstd).
"""

import collections
import concurrent.futures as cf
import contextlib
import csv
import dataclasses
import fcntl
import hashlib
import heapq
import itertools
import json
import os
import resource
import shutil
import subprocess
import sys
import tempfile
import time
from array import array
from compression import zstd
from pathlib import Path

from . import suite as lab

ROOT = lab.ROOT
WORK = ROOT / "target" / "neutral"
GiB = 1 << 30

# --- cells ------------------------------------------------------------------

SEEDS = (2004, 3033)


def _star(viewers, mbit, secs, **env):
    return {**lab.SS_STAR, "STRIMS_SIM_CAL_VIEWERS": str(viewers), "STRIMS_SIM_CAL_SEED_UP_MBIT": str(mbit),
            "STRIMS_SIM_CAL_SECS": str(secs), **env}


# The default cells: each kind of host, link and audience the logs record
# (overload, a halved seed, thin relays, joins, departures, greedy peers,
# churn, mobile links), cut short. star-31-5's overload writes the largest
# logs.
AUDIT = [
    lab.cell("star-31-5", lab.SIM, "calibration_star", _star(31, 5, 30), lab.MEASURE_MEM_GIB,
             "31 viewers behind a 5 Mbit/s seed: overload, the largest logs"),
    lab.cell("halve-15-10", lab.SIM, "calibration_star", _star(15, 10, 40, STRIMS_SIM_CAL_SEED_UP_SCALE="20:0.5"),
             2, "the seed's uplink halved at 20 s"),
    lab.cell("thin-15-10-trickle", lab.SIM, "calibration_star",
             _star(15, 10, 30, STRIMS_SIM_CAL_VIEWER_UP_MBIT="3", STRIMS_SIM_ARRIVALS="trickle"),
             2, "thin relays (3 Mbit/s viewer uplinks), the viewers trickling in"),
    lab.cell("star-15-10-end-abrupt", lab.SIM, "calibration_star",
             _star(15, 10, 40, STRIMS_SIM_ARRIVALS="trickle", STRIMS_SIM_DEPART="60:20:5:abrupt"),
             2, "an event's end: 60 % of the viewers leaving abruptly within 5 s of 20 s"),
    lab.cell("no-greedy", lab.PPSPP, "sim_job",
             {**lab.PP_CABLE, "SIM_PEERS": "50", "SIM_MBIT": "6", "SIM_SECS": "20", "SIM_GREEDY": "0"},
             2, "50 peers behind a cable seed, no greedy pulls"),
    lab.cell("grabbers", lab.PPSPP, "sim_job",
             {**lab.PP_CABLE, "SIM_PEERS": "30", "SIM_MBIT": "6", "SIM_SECS": "20", "SIM_GREEDY_KINDS": "grab",
              "SIM_GREEDY_SHARE": "20"},
             2, "a fifth of the peers grabbing"),
    lab.cell("churn", lab.PPSPP, "sim_job",
             {**lab.PP_CABLE, "SIM_PEERS": "50", "SIM_MBIT": "4", "SIM_SECS": "30", "SIM_CHURN": "120",
              "SIM_CHURN_AWAY": "5"},
             2, "peers leaving for 5 s and coming back"),
    lab.cell("wan-mobile", lab.PPSPP, "sim_job", {**lab.PP_AB, "SIM_SCENARIO": "WanMobile", "SIM_SECS": "30"},
             2, "mobile viewers over the WAN"),
    lab.cell("cable-end-abrupt", lab.PPSPP, "sim_job",
             {**lab.PP_CABLE, "SIM_PEERS": "50", "SIM_MBIT": "4", "SIM_SECS": "40", "SIM_ARRIVALS": "trickle",
              "SIM_DEPART": "60:20:5:abrupt"},
             2, "an event's end in the ppspp sim"),
]
# The vocabulary's cells: AUDIT's, and every writer the audit leaves off
# (the loop log, collusion, media, clock skew, hostile viewers, browsers,
# discovery and the WAN scenarios' hosts), so a change to how the sims
# name what they record shows on every name.
VOCAB = AUDIT + [
    lab.cell("loops-collude", lab.PPSPP, "sim_job",
             {**lab.PP_CABLE, "SIM_PEERS": "30", "SIM_MBIT": "6", "SIM_SECS": "20", "SIM_LOOPS": "1",
              "SIM_COLLUDE": "1"},
             2, "30 peers behind a cable seed, the loop log on, the greedy viewers colluding"),
    lab.cell("media-sign", lab.PPSPP, "sim_job",
             {**lab.PP_CABLE, "SIM_PEERS": "30", "SIM_SECS": "20", "SIM_MEDIA": "1500v", "SIM_SIGN": "u8,t200"},
             2, "a signed 1500 kbit/s VBR stream in the ppspp sim"),
    lab.cell("skew", lab.PPSPP, "sim_job",
             {**lab.PP_CABLE, "SIM_PEERS": "30", "SIM_MBIT": "6", "SIM_SECS": "20", "SIM_CLOCK_SKEW": "real"},
             2, "real-world clock skew in the ppspp sim"),
    lab.cell("cross-traffic", lab.PPSPP, "sim_job", {**lab.PP_AB, "SIM_SCENARIO": "CrossTraffic", "SIM_SECS": "20"},
             2, "uplinks shared with cross traffic"),
    lab.cell("super-peer-20s", lab.PPSPP, "sim_job",
             {**lab.PP_AB, "SIM_SCENARIO": "WanSuperPeer", "SIM_SECS": "20"}, 2, "a super-peer seed over the WAN"),
    lab.cell("star-media-sign", lab.SIM, "calibration_star",
             _star(15, 10, 30, STRIMS_SIM_MEDIA="1500v", STRIMS_SIM_SIGN="u8,t200"),
             2, "a signed 1500 kbit/s VBR stream on the real stack"),
    lab.cell("star-skew-liars", lab.SIM, "calibration_star",
             _star(15, 10, 30, STRIMS_SIM_CLOCK_SKEW="real", STRIMS_SIM_HOSTILE="liar:0.2"),
             2, "real-world clock skew, a fifth of the viewers liars"),
    lab.cell("org-browsers-free-riders", lab.SIM, "calibration_star",
             _star(15, 10, 30, STRIMS_SIM_DISCOVERY="1", STRIMS_SIM_RTC=lab.BROWSERS,
                   STRIMS_SIM_HOSTILE="free-rider:0.2"),
             2, "browsers in an organic network, a fifth of the viewers free riders"),
]
SETS = {"audit": AUDIT, "vocab": VOCAB}
DEFAULT_SET = "audit"

# Every table and log, by sim: the measure tier's logs (the forecast log,
# the request logs) and each node's bins; the output dir is each run's.
BIN_NODES = {lab.PPSPP: "SIM_BIN_NODES", lab.SIM: "STRIMS_SIM_BIN_NODES"}
LOGS = {sim: {**lab.MEASURE_LOGS[sim], BIN_NODES[sim]: "1"} for sim in (lab.PPSPP, lab.SIM)}


def cell_set(spec=DEFAULT_SET):
    """The cells `spec` names, each with LOGS on: a set of SETS, a suite
    tier, or cell names, comma-separated (looked up in SETS, then in the
    tiers, the first of a name). ValueError for a name of none."""
    spec = spec or DEFAULT_SET
    if spec in SETS:
        out = SETS[spec]
    elif spec in lab.TIERS:
        out = lab.TIERS[spec]
    else:
        known = {}
        for c in itertools.chain(*SETS.values(), *lab.TIERS.values()):
            known.setdefault(c["name"], c)
        names = [n.strip() for n in spec.split(",") if n.strip()]
        missing = [n for n in names if n not in known]
        if missing or not names:
            raise ValueError(f"no cell set, tier or cell {', '.join(missing) or repr(spec)} "
                             f"(sets: {', '.join(SETS)}; tiers: {', '.join(sorted(lab.TIERS))})")
        out = [known[n] for n in names]
    return [{**c, "env": {**c["env"], **LOGS[c["sim"]]}} for c in out]


# --- commits ----------------------------------------------------------------

# --head's name for the working tree.
TREE = "."


def git(*args, env=None, cwd=None):
    p = subprocess.run(["git", *args], cwd=cwd or ROOT, env=env, capture_output=True, text=True)
    if p.returncode:
        raise GitError(f"git {' '.join(args)}: {p.stderr.strip() or p.stdout.strip()}")
    return p.stdout.strip()


class GitError(Exception):
    pass


def resolve(ref):
    """`ref`'s commit, or for TREE the working tree's (snapshot)."""
    if ref == TREE:
        return snapshot()
    try:
        return git("rev-parse", "--verify", "--quiet", "--end-of-options", f"{ref}^{{commit}}")
    except GitError:
        raise GitError(f"no commit {ref!r}") from None


def snapshot():
    """The working tree as a commit on HEAD, untracked files but ignored
    ones in: HEAD itself when they are the same. Staged in an index of its
    own, so the checkout's index and files stay as they are; its author,
    committer and date are HEAD's, so the same tree gives the same commit
    (and its kept builds and runs)."""
    head = git("rev-parse", "HEAD")
    WORK.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=WORK, prefix="index-") as tmp:
        index = Path(tmp) / "index"
        real = Path(git("rev-parse", "--path-format=absolute", "--git-path", "index"))
        if real.exists():
            shutil.copyfile(real, index)
        env = {**os.environ, "GIT_INDEX_FILE": str(index)}
        git("add", "-A", env=env)
        tree = git("write-tree", env=env)
    if tree == git("rev-parse", "HEAD^{tree}"):
        return head
    who = git("log", "-1", "--format=%an%x00%ae%x00%cI", head).split("\0")
    env = {**os.environ, "GIT_AUTHOR_NAME": who[0], "GIT_AUTHOR_EMAIL": who[1], "GIT_AUTHOR_DATE": who[2],
           "GIT_COMMITTER_NAME": who[0], "GIT_COMMITTER_EMAIL": who[1], "GIT_COMMITTER_DATE": who[2]}
    return git("commit-tree", tree, "-p", head, "-m", f"neutral: the working tree on {head[:12]}", env=env)


def parents(sha):
    return git("rev-list", "--parents", "-n", "1", sha).split()[1:]


def pairs(bases, head="HEAD"):
    """The head's commit and the bases' it is checked against: `bases`
    (refs), else the head's parents; a merge commit is checked against
    each of its parents as well."""
    h = resolve(head)
    ps = parents(h)
    out = []
    for b in [*(resolve(r) for r in bases or ()), *(ps if len(ps) > 1 or not bases else ())]:
        if b not in out:
            out.append(b)
    if not out:
        raise GitError(f"{head} has no parent: name a base")
    return h, out


# --- builds -----------------------------------------------------------------

# Both sims' test binaries, in one cargo invocation, and each one's name in
# a bin dir (suite.run_jobs runs BIN_DIR/<cell's sim>).
BUILD = ["cargo", "test", "--release", "--locked", "-p", "strims-sim", "--test", "scenarios", "-p", "strims-ppspp",
         "--test", "sim", "--no-run", "--message-format=json"]
TARGETS = {lab.SIM: "scenarios", lab.PPSPP: "sim"}
PRUNE_DAYS = 7


class BuildError(Exception):
    pass


@contextlib.contextmanager
def locked(path):
    """An exclusive lock on `path`, across processes."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


def drop_tree(src):
    """Removes a build's worktree, registered or not (`worktree add
    --force` takes a path still registered once its dir is gone)."""
    if src.exists():
        with contextlib.suppress(GitError):
            git("worktree", "remove", "--force", "--force", str(src))
        shutil.rmtree(src, ignore_errors=True)


def build(sha, log=None):
    """`sha`'s bin dir (both sims' binaries and build.json) and the
    seconds its build took, None when it was kept: built in a detached
    worktree of the commit with a target dir of its own, both removed once
    the binaries are copied out. BuildError when cargo fails (its output in
    bin/SHA12.build.log)."""
    log = log or _log
    key = sha[:12]
    dest = WORK / "bin" / key
    with locked(WORK / "locks" / f"{key}.lock"):
        if (dest / "build.json").exists():
            os.utime(dest / "build.json")
            return dest, None
        src, cargo = WORK / "src" / key, WORK / "cargo" / key
        drop_tree(src)
        shutil.rmtree(cargo, ignore_errors=True)
        dest.parent.mkdir(parents=True, exist_ok=True)
        git("worktree", "add", "--force", "--detach", str(src), sha)
        errlog = WORK / "bin" / f"{key}.build.log"
        log(f"building {key} in {src.relative_to(ROOT)} (its own target dir)")
        t0 = time.monotonic()
        try:
            env = {**os.environ, "CARGO_TARGET_DIR": str(cargo), "CARGO_INCREMENTAL": "0"}
            with open(errlog, "w") as err:
                p = subprocess.run(BUILD, cwd=src, env=env, stdout=subprocess.PIPE, stderr=err, text=True)
            if p.returncode:
                tail = "\n".join(errlog.read_text(errors="replace").splitlines()[-12:])
                raise BuildError(f"{key}: cargo exited {p.returncode} ({errlog}):\n{tail}")
            exes = {}
            for line in p.stdout.splitlines():
                m = json.loads(line)
                if m.get("reason") == "compiler-artifact" and m.get("executable"):
                    exes[m["target"]["name"]] = m["executable"]
            tmp = dest.with_name(f"{key}.tmp")
            shutil.rmtree(tmp, ignore_errors=True)
            tmp.mkdir()
            files = {}
            for sim, target in TARGETS.items():
                if target not in exes:
                    raise BuildError(f"{key}: cargo built no {target} binary")
                shutil.copyfile(exes[target], tmp / sim)
                (tmp / sim).chmod(0o755)
                files[sim] = digest(tmp / sim)[0]
            secs = round(time.monotonic() - t0, 1)
            (tmp / "build.json").write_text(json.dumps({"sha": sha, "secs": secs, "files": files,
                                                        "at": time.strftime("%Y-%m-%dT%H:%M:%S%z")}) + "\n")
            shutil.rmtree(dest, ignore_errors=True)
            tmp.rename(dest)
        finally:
            drop_tree(src)
            shutil.rmtree(cargo, ignore_errors=True)
        log(f"built {key} in {secs:.0f} s")
        return dest, secs


def prune(days=PRUNE_DAYS):
    """Removes the binaries and runs of commits unused for `days`, and the
    worktrees and target dirs of builds no process is running (cut short)."""
    (WORK / "locks").mkdir(parents=True, exist_ok=True)
    for d in [*(WORK / "src").glob("*"), *(WORK / "cargo").glob("*")]:
        with open(WORK / "locks" / f"{d.name}.lock", "a") as f:
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                continue
            try:
                drop_tree(d) if d.parent.name == "src" else shutil.rmtree(d, ignore_errors=True)
            finally:
                fcntl.flock(f, fcntl.LOCK_UN)
    cut = time.time() - days * 86400
    for d in (WORK / "bin").glob("*") if (WORK / "bin").exists() else ():
        if d.is_dir() and (d / "build.json").exists() and (d / "build.json").stat().st_mtime < cut:
            shutil.rmtree(d, ignore_errors=True)
    for d in (WORK / "runs").glob("*") if (WORK / "runs").exists() else ():
        used = [f.stat().st_mtime for f in d.glob("*/run.json")]
        if d.is_dir() and (not used or max(used) < cut):
            shutil.rmtree(d, ignore_errors=True)


# --- runs -------------------------------------------------------------------


@dataclasses.dataclass
class Run:
    """A kept run: its dir (tables/ and run.json) and its record ({sha,
    cell, seed, sim, test, env, row, wall_s, rss_mb, files})."""
    dir: Path
    record: dict

    @property
    def row(self):
        return self.record["row"]

    @property
    def files(self):
        return self.record["files"]

    def path(self, name):
        return self.dir / "tables" / self.files[name]["file"]


def digest(path):
    """sha256, size and newlines of a file's bytes, a `.zst` one's
    decompressed, read a block at a time."""
    h = hashlib.sha256()
    n = lines = 0
    with open(path, "rb") as raw:
        f = zstd.ZstdFile(raw) if path.name.endswith(".zst") else raw
        while chunk := f.read(1 << 22):
            h.update(chunk)
            n += len(chunk)
            lines += chunk.count(b"\n")
    return h.hexdigest(), n, lines


def manifest(d):
    out = {}
    for f in sorted(d.iterdir()):
        if f.is_file():
            h, n, lines = digest(f)
            out[f.name.removesuffix(".zst")] = {"file": f.name, "sha256": h, "bytes": n, "lines": lines,
                                                **({"zst": f.stat().st_size} if f.name.endswith(".zst") else {})}
    return out


def env_key(c):
    return hashlib.sha1(json.dumps([c["sim"], c["test"], sorted(c["env"].items())]).encode()).hexdigest()[:10]


def run_dir(sha, c, seed):
    return WORK / "runs" / sha[:12] / f"{c['name']}.{seed}.{env_key(c)}"


def _alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        pass
    return True


def run(bins, cells, seeds, slots=None, mem_gib=None, log=None):
    """Each of `bins`' ({sha: bin dir}) runs of `cells` at `seeds`, kept
    ones read back: ({(sha, cell, seed): Run}, failures). A run writes into
    a dir of its own process's and moves into place once its record is
    written, so a run that failed or was cut short leaves nothing kept."""
    log = log or _log
    slots = slots or max(1, (os.cpu_count() or 2) - 1)
    mem_gib = mem_gib or default_mem_gib()
    out, jobs, failures = {}, [], []
    for sha, bin_dir in bins.items():
        top = WORK / "runs" / sha[:12]
        for stale in top.glob("*.tmp-*") if top.exists() else ():
            pid = stale.name.rsplit("-", 1)[1]
            if not pid.isdigit() or not _alive(int(pid)):
                shutil.rmtree(stale, ignore_errors=True)
        for c in cells:
            for s in seeds:
                d = run_dir(sha, c, s)
                if (d / "run.json").exists():
                    os.utime(d / "run.json")
                    out[(sha, c["name"], s)] = Run(d, json.loads((d / "run.json").read_text()))
                    continue
                tmp = d.with_name(f"{d.name}.tmp-{os.getpid()}")
                shutil.rmtree(tmp, ignore_errors=True)
                (tmp / "tables").mkdir(parents=True)
                jobs.append((sha, bin_dir, c, s, {lab.OUT_KNOB[c["sim"]]: str(tmp / "tables")}))
    if jobs:
        log(f"running {len(jobs)} runs ({len(out)} kept) under {slots} slots and {mem_gib:.0f} GiB")
    t0 = time.monotonic()
    done = 0
    for ended in lab.run_jobs(jobs, slots, mem_gib, lambda c: c["mem_gib"]):
        for (sha, _, c, s, extra), row, err in ended:
            done += 1
            tmp = Path(extra[lab.OUT_KNOB[c["sim"]]]).parent
            name = f"{c['name']} {s} at {sha[:12]}"
            if row is None:
                failures.append(f"{name}: {err}")
                shutil.rmtree(tmp, ignore_errors=True)
                log(f"run {done}/{len(jobs)}: {name} FAILED: {err}")
                continue
            rec = {"sha": sha, "cell": c["name"], "seed": s, "sim": c["sim"], "test": c["test"], "env": c["env"],
                   "row": row, "wall_s": row.get("_wall_s"), "rss_mb": row.get("_rss_mb"),
                   "files": manifest(tmp / "tables")}
            (tmp / "run.json").write_text(json.dumps(rec, sort_keys=True) + "\n")
            d = run_dir(sha, c, s)
            try:
                tmp.rename(d)
            except OSError:
                # Another invocation kept the same run first: it's this one.
                shutil.rmtree(tmp, ignore_errors=True)
                rec = json.loads((d / "run.json").read_text())
            out[(sha, c["name"], s)] = Run(d, rec)
            note = f" ({err})" if err else ""
            log(f"run {done}/{len(jobs)}: {name}: {rec['wall_s']:.0f} s, {rec['rss_mb']} MiB{note}, "
                f"{time.monotonic() - t0:.0f} s in")
    return out, failures


def default_mem_gib():
    """Three quarters of the memory available now."""
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemAvailable:"):
                return max(4.0, 0.75 * int(line.split()[1]) * 1024 / GiB)
    except OSError:
        pass
    return 16.0


# --- comparison ---------------------------------------------------------------

# A row's fields that say how the host ran it, not what the sim did.
HOST_KEYS = frozenset({"_wall_s", "_rss_mb", "wall_secs"})
# The fields that are the run's behaviour: `row` alone doesn't expect them.
BEHAVIOUR = ("trace", "reads_hash")
# The rows a move's detail keeps, and the rows a multiset diff reads back
# to pair (near pairs say which columns a moved row changed in).
SAMPLE = 5
PAIR_SAMPLE = 64
# A partition file's rows in a multiset diff, about: each is counted in
# memory alone.
PART_ROWS = 50_000
SPLIT_BUFFER = 1 << 20
# A rows move where more than this share of rows moved tries each column
# left out, while the tables' rows times their columns stay in this budget.
LEAVE_OUT_SHARE = 0.1
LEAVE_OUT_ROWS = 50_000_000
# The leading columns a sample row is shown by.
CONTEXT = 3


class Expect:
    """The moves `--expect` allows: TABLE (a table, a file's stem, `row`
    or `series_meta`) or TABLE.COL; comma-separated too. `row` covers each
    field but BEHAVIOUR's. Which names allowed a move is kept (unused)."""

    def __init__(self, names=()):
        self.names = {n.strip() for x in names for n in x.split(",") if n.strip()}
        self.used = set()

    def covers(self, table, col=None):
        """The name that allows the move, or None."""
        if col is not None and f"{table}.{col}" in self.names:
            return f"{table}.{col}"
        if table in self.names and not (table == "row" and col in BEHAVIOUR):
            return table
        return None

    def __call__(self, table, col=None):
        hit = self.covers(table, col)
        if hit:
            self.used.add(hit)
        return hit is not None

    def unused(self):
        return sorted(self.names - self.used)


@dataclasses.dataclass
class Move:
    """One thing that moved: its name (as `--expect` names it), its kind
    (value, new, gone, type, order, rows, bytes), whether it was expected,
    a line for people and its detail (counts, sample rows)."""
    name: str
    kind: str
    expected: bool
    text: str
    detail: dict = dataclasses.field(default_factory=dict)


def fmt(v, width=60):
    s = json.dumps(v, sort_keys=True) if not isinstance(v, str) else v
    return s if len(s) <= width else s[: width - 3] + "..."


def same(x, y):
    return json.dumps(x, sort_keys=True) == json.dumps(y, sort_keys=True)


def keyed_moves(table, a, b, expect, skip=frozenset()):
    """A JSON object's keys that moved (the row, series_meta)."""
    out = []
    for k in sorted((a.keys() | b.keys()) - skip):
        if k not in b:
            kind, text = "gone", f"{table}.{k}: gone (was {fmt(a[k])})"
        elif k not in a:
            kind, text = "new", f"{table}.{k}: new ({fmt(b[k])})"
        elif same(a[k], b[k]):
            continue
        else:
            kind, text = "value", f"{table}.{k}: {fmt(a[k], 30)} -> {fmt(b[k], 30)}"
        out.append(Move(f"{table}.{k}", kind, expect(table, k), text, {"base": a.get(k), "head": b.get(k)}))
    return out


def schema_moves(table, a, b, expect):
    """A table's schema file: its columns' types, presence and order."""
    ca = {c["name"]: c.get("type") for c in a.get("columns", [])}
    cb = {c["name"]: c.get("type") for c in b.get("columns", [])}
    out = []
    for c in ca:
        if c not in cb:
            out.append(Move(f"{table}.{c}", "gone", expect(table, c), f"{table}.{c}: column gone"))
        elif ca[c] != cb[c]:
            out.append(Move(f"{table}.{c}", "type", expect(table, c), f"{table}.{c}: type {ca[c]} -> {cb[c]}"))
    for c in cb:
        if c not in ca:
            out.append(Move(f"{table}.{c}", "new", expect(table, c), f"{table}.{c}: new column ({cb[c]})"))
    if [c for c in ca if c in cb] != [c for c in cb if c in ca]:
        out.append(Move(table, "order", expect(table), f"{table}: its columns in another order"))
    if not out:
        out.append(Move(table, "bytes", expect(table), f"{table}: its schema file's bytes differ"))
    return out


@contextlib.contextmanager
def lines(path):
    """A table's lines, as bytes, a `.zst` one decompressed as it's read."""
    with open(path, "rb") as raw:
        if path.name.endswith(".zst"):
            with zstd.ZstdFile(raw) as z:
                yield z
        else:
            yield raw


def fields(line):
    """A CSV line's fields, as bytes (the sims write no quotes; a line with
    one is parsed as CSV)."""
    line = line.rstrip(b"\r\n")
    if b'"' in line:
        return [f.encode() for f in next(csv.reader([line.decode(errors="replace")]))]
    return line.split(b",")


def header(path):
    with lines(path) as f:
        return [c.decode(errors="replace") for c in fields(next(f, b""))] if path.stat().st_size else []


def text(v):
    return None if v is None else v.decode(errors="replace")


def shown(v):
    """A field's value in a report's line: '' for an empty one, - for none
    (past a row's end)."""
    return "-" if v is None else v if v != "" else "''"


def context(fs, head):
    return " ".join(f"{c}={shown(text(v))}" for c, v in list(zip(head, fs))[:CONTEXT])


def positional(pa, pb, ha, hb, cols, keep=SAMPLE):
    """Two tables of one row count, compared at each row on `cols`: each
    column's differing rows and its first `keep`, and the row count."""
    ia, ib = [ha.index(c) for c in cols], [hb.index(c) for c in cols]
    layout = ha == hb
    counts, samples = collections.Counter(), collections.defaultdict(list)
    n = 0
    with lines(pa) as la, lines(pb) as lb:
        next(la, None)
        next(lb, None)
        for n, (x, y) in enumerate(zip(la, lb), 1):
            if layout and x == y:
                continue
            fx, fy = fields(x), fields(y)
            for c, i, j in zip(cols, ia, ib):
                vx = fx[i] if i < len(fx) else None
                vy = fy[j] if j < len(fy) else None
                if vx != vy:
                    counts[c] += 1
                    if len(samples[c]) < keep:
                        samples[c].append({"row": n - 1, "base": text(vx), "head": text(vy), "at": context(fx, ha)})
    return counts, samples, n


def projected(fs, idx):
    return [fs[i] if i < len(fs) else b"" for i in idx]


def first_divergence(pa, pb, ha, hb, cols):
    """The first row whose `cols` differ, and both sides' (None past a
    table's end)."""
    ia, ib = [ha.index(c) for c in cols], [hb.index(c) for c in cols]
    with lines(pa) as la, lines(pb) as lb:
        next(la, None)
        next(lb, None)
        for n, (x, y) in enumerate(itertools.zip_longest(la, lb)):
            fx = projected(fields(x), ia) if x is not None else None
            fy = projected(fields(y), ib) if y is not None else None
            if fx != fy:
                show = lambda fs: None if fs is None else dict(zip(cols, map(text, fs)))
                return {"row": n, "base": show(fx), "head": show(fy)}
    return None


def _split(path, idx, raw, out, parts):
    """Each row's hash and place into partition files by hash: an array of
    (hash, row) pairs each."""
    out.mkdir()
    # Unbuffered: the arrays are the buffers, SPLIT_BUFFER bytes between
    # them (a file's own would be io.DEFAULT_BUFFER_SIZE each).
    files = [open(out / str(p), "wb", buffering=0) for p in range(parts)]
    bufs = [array("Q") for _ in range(parts)]
    flush = max(64, SPLIT_BUFFER // 8 // parts)
    try:
        with lines(path) as f:
            next(f, None)
            for pos, line in enumerate(f):
                key = line.rstrip(b"\r\n") if raw else b"\x1f".join(projected(fields(line), idx))
                h = hash(key) & 0xFFFF_FFFF_FFFF_FFFF
                b = bufs[h % parts]
                b.append(h)
                b.append(pos)
                if len(b) >= flush:
                    b.tofile(files[h % parts])
                    del b[:]
        for b, f in zip(bufs, files):
            b.tofile(f)
    finally:
        for f in files:
            f.close()


def _part(path):
    a = array("Q")
    a.frombytes(path.read_bytes())
    return a


def multiset(pa, pb, ha, hb, cols, rows, keep=PAIR_SAMPLE):
    """Two tables' rows on `cols` as multisets: the number of rows only in
    each and the first `keep` of them by place, [base, head]. Rows are
    hashed into partition files of about PART_ROWS rows (`rows`, the
    larger table's), each counted on its own, so memory stays flat."""
    ia, ib = [ha.index(c) for c in cols], [hb.index(c) for c in cols]
    raw = ha == hb == cols
    parts = max(1, min(4096, rows // PART_ROWS + 1))
    WORK.mkdir(parents=True, exist_ok=True)
    counts, first = [0, 0], [[], []]
    with tempfile.TemporaryDirectory(dir=WORK, prefix="multiset-") as tmp:
        tmp = Path(tmp)
        _split(pa, ia, raw, tmp / "a", parts)
        _split(pb, ib, raw, tmp / "b", parts)
        for p in range(parts):
            arrs = (_part(tmp / "a" / str(p)), _part(tmp / "b" / str(p)))
            cnt = [collections.Counter(arr[0::2]) for arr in arrs]
            moved = {h for h, k in cnt[0].items() if cnt[1][h] != k} | {h for h in cnt[1] if h not in cnt[0]}
            if not moved:
                continue
            for side in (0, 1):
                places = collections.defaultdict(list)
                arr = arrs[side]
                for h, pos in zip(arr[0::2], arr[1::2]):
                    if h in moved:
                        places[h].append(pos)
                extra = []
                for h, ps in places.items():
                    k = cnt[side][h] - cnt[1 - side][h]
                    if k > 0:
                        counts[side] += k
                        extra.extend(ps[-k:])
                first[side] = heapq.nsmallest(keep, first[side] + extra)
    return counts, first


def fetch(path, places):
    """The rows at `places` (0 the first after the header), their fields."""
    want, out = set(places), {}
    last = max(want, default=-1)
    with lines(path) as f:
        next(f, None)
        for pos, line in enumerate(f):
            if pos > last:
                break
            if pos in want:
                out[pos] = fields(line)
    return out


def near_pairs(sa, sb, cols):
    """Rows only in the base paired with the head's nearest (the fewest
    differing columns, then the nearest place), where they differ in a
    third of the columns or fewer: [(base row, head row, columns)]."""
    out = []
    for pa_, ra in sa:
        best = None
        for pb_, rb in sb:
            d = [c for c in cols if ra.get(c) != rb.get(c)]
            key = (len(d), abs(pa_ - pb_))
            if best is None or key < best[0]:
                best = (key, pb_, d)
        if best and best[2] and len(best[2]) <= max(1, len(cols) // 3):
            out.append((pa_, best[1], best[2]))
    return out


def divergence_text(first, most=6):
    """A first divergence's lines: the columns that differ, base -> head,
    by the base row's leading columns; both rows' leading columns where
    more differ (than `most`, or half the columns), or a table ended."""
    if not first:
        return []
    b, h = first["base"], first["head"]
    row = lambda r: "(past the end)" if r is None else " ".join(f"{k}={shown(v)}" for k, v in list(r.items())[:8])
    if b is not None and h is not None:
        diff = [k for k in b if b[k] != h.get(k)]
        if len(diff) <= min(most, max(1, len(b) // 2)):
            ctx = " ".join(f"{k}={shown(v)}" for k, v in list(b.items())[:CONTEXT])
            return [f"first divergence, row {first['row']} ({ctx}): "
                    + ", ".join(f"{k} {shown(b[k])} -> {shown(h[k])}" for k in diff)]
    return [f"first divergence, row {first['row']} in base: {row(b)}",
            f"first divergence, row {first['row']} in head: {row(h)}"]


def table_moves(table, a, b, expect, diagnose=True):
    """A table's moves, from run `a` to run `b`."""
    pa, pb = a.path(f"{table}.csv"), b.path(f"{table}.csv")
    ha, hb = header(pa), header(pb)
    na, nb = max(0, a.files[f"{table}.csv"]["lines"] - 1), max(0, b.files[f"{table}.csv"]["lines"] - 1)
    out = [Move(f"{table}.{c}", "gone", expect(table, c), f"{table}.{c}: column gone") for c in ha if c not in hb]
    out += [Move(f"{table}.{c}", "new", expect(table, c), f"{table}.{c}: new column") for c in hb if c not in ha]
    common = [c for c in ha if c in hb]
    if common != [c for c in hb if c in ha]:
        out.append(Move(table, "order", expect(table), f"{table}: its columns in another order"))
    if not diagnose:
        out.append(Move(table, "rows" if na != nb else "bytes", expect(table),
                        f"{table}: differs, rows {na} -> {nb} (not diagnosed: the trace moved)",
                        {"rows": [na, nb]}))
        return out
    if na == nb:
        counts, samples, _ = positional(pa, pb, ha, hb, common)
        for c in common:
            if counts[c]:
                s = samples[c][0]
                out.append(Move(f"{table}.{c}", "value", expect(table, c),
                                f"{table}.{c}: {counts[c]} of {na} rows differ, first row {s['row']}: "
                                f"{shown(s['base'])} -> {shown(s['head'])} ({s['at']})",
                                {"rows": counts[c], "of": na, "sample": samples[c]}))
    else:
        # The columns nothing expects: a row counts as moved for them alone.
        checked = [c for c in common if not expect.covers(table, c)] or common
        (oa, ob), (fa, fb) = multiset(pa, pb, ha, hb, checked, max(na, nb))
        first = first_divergence(pa, pb, ha, hb, checked)
        # Most rows moved: a column renumbered after an insertion (an id)
        # moves every later row. The column whose leaving out moves the
        # fewest rows is set aside, and the rows that moved besides it are
        # shown.
        without = None
        if (len(checked) > 1 and min(oa, ob) > LEAVE_OUT_SHARE * min(na, nb)
                and (na + nb) * len(checked) <= LEAVE_OUT_ROWS):
            for c in checked:
                (xa, xb), _ = multiset(pa, pb, ha, hb, [x for x in checked if x != c], max(na, nb), keep=0)
                if without is None or xa + xb < without[1] + without[2]:
                    without = (c, xa, xb)
            if without[1] + without[2] > (oa + ob) // 2:
                without = None
            else:
                checked = [x for x in checked if x != without[0]]
                _, (fa, fb) = multiset(pa, pb, ha, hb, checked, max(na, nb))
        rows_a, rows_b = fetch(pa, fa), fetch(pb, fb)
        ia, ib = [ha.index(c) for c in checked], [hb.index(c) for c in checked]
        sa = [(p, dict(zip(checked, map(text, projected(rows_a[p], ia))))) for p in fa]
        sb = [(p, dict(zip(checked, map(text, projected(rows_b[p], ib))))) for p in fb]
        near = near_pairs(sa, sb, checked)
        cols = collections.Counter(c for _, _, d in near for c in d)
        hint = f"; near pairs differ in {', '.join(c for c, _ in cols.most_common(6))}" if cols else ""
        at = f", first divergence at row {first['row']}" if first else ""
        aside = (f"; without {without[0]}, {without[2]} only in head and {without[1]} only in base"
                 if without else "")
        out.append(Move(table, "rows", expect(table),
                        f"{table}: rows {na} -> {nb}, {ob} only in head and {oa} only in base{at}{aside}{hint}",
                        {"rows": [na, nb], "only_base": oa, "only_head": ob, "first": first,
                         "without": without and {"column": without[0], "only_base": without[1],
                                                 "only_head": without[2]},
                         "sample_base": [dict(row=p, **r) for p, r in sa[:SAMPLE]],
                         "sample_head": [dict(row=p, **r) for p, r in sb[:SAMPLE]],
                         "pairs": [{"base_row": x, "head_row": y, "columns": d} for x, y, d in near[:SAMPLE]],
                         "pair_columns": dict(cols.most_common())}))
    if not out:
        out.append(Move(table, "bytes", expect(table), f"{table}: its bytes differ, no row or column does"))
    return out


def compare(a, b, expect):
    """What moved from Run `a` to Run `b`: the row's fields, then each file
    that differs (by its decompressed digest), the tables diagnosed unless
    the trace moved."""
    out = keyed_moves("row", a.row, b.row, expect, HOST_KEYS)
    traced = any(m.name == "row.trace" for m in out)
    fa, fb = a.files, b.files
    for name in sorted(fa.keys() | fb.keys()):
        x, y = fa.get(name), fb.get(name)
        if x and y and x["sha256"] == y["sha256"]:
            continue
        table = name.split(".", 1)[0]
        if x is None or y is None:
            if name.endswith(".schema.json") and (f"{table}.csv" in fa) != (f"{table}.csv" in fb):
                continue
            kind = "new" if x is None else "gone"
            what = f"table, {(y or x)['lines'] - 1} rows" if name.endswith(".csv") else name
            out.append(Move(table if name.endswith((".csv", ".schema.json")) else name, kind,
                            expect(table) if name.endswith((".csv", ".schema.json")) else expect(name),
                            f"{table}: {kind} ({what})"))
        elif name.endswith(".csv"):
            out += table_moves(table, a, b, expect, diagnose=not traced)
        elif name.endswith(".schema.json"):
            out += schema_moves(table, json.loads(a.path(name).read_text()), json.loads(b.path(name).read_text()),
                                expect)
        elif name == "series_meta.json":
            out += keyed_moves("series_meta", json.loads(a.path(name).read_text()),
                               json.loads(b.path(name).read_text()), expect)
        else:
            out.append(Move(name, "bytes", expect(name), f"{name}: its bytes differ"))
    seen, kept = set(), []
    for m in out:
        if (m.name, m.kind) not in seen:
            seen.add((m.name, m.kind))
            kept.append(m)
    return kept


# --- the check ------------------------------------------------------------------


@dataclasses.dataclass
class RunDiff:
    cell: str
    seed: int
    base: Run | None
    head: Run | None
    moves: list
    error: str | None = None

    def unexpected(self):
        return [m for m in self.moves if not m.expected]


@dataclasses.dataclass
class Pair:
    base: str
    head: str
    runs: list

    def unexpected(self):
        return sum(len(r.unexpected()) for r in self.runs)


@dataclasses.dataclass
class Report:
    head: str
    pairs: list
    cells: list
    seeds: list
    expect: Expect
    builds: dict
    failures: list
    times: dict
    peak_mib: int

    def unexpected(self):
        return sum(p.unexpected() for p in self.pairs)

    def exit_code(self):
        if self.failures or any(r.error for p in self.pairs for r in p.runs):
            return 2
        return 1 if self.unexpected() else 0

    def suggested(self):
        """--expect names covering every move: each move's own, the row's
        fields as `row` past six of them."""
        names = sorted({m.name for p in self.pairs for r in p.runs for m in r.moves})
        row = [n for n in names if n.startswith("row.") and n not in ("row.trace", "row.reads_hash")]
        if len(row) > 6:
            names = [n for n in names if n not in row] + ["row"]
        return sorted(names)

    def largest_run(self):
        runs = [r for p in self.pairs for d in p.runs for r in (d.base, d.head) if r and r.record.get("rss_mb")]
        return max(runs, key=lambda r: r.record["rss_mb"], default=None)

    def text(self):
        out = []
        logs = {k: v for c in self.cells for k, v in LOGS[c["sim"]].items()}
        inherited = sorted(k for k in os.environ if k.startswith(("SIM_", "STRIMS_SIM_")))
        out.append(f"neutral: {len(self.cells)} cells x seeds {', '.join(map(str, self.seeds))}, every table and "
                   f"log on ({' '.join(f'{k}={v}' for k, v in sorted(logs.items()))})")
        if inherited:
            out.append(f"  from the environment, for every run: {' '.join(f'{k}={os.environ[k]}' for k in inherited)}")
        for f in self.failures:
            out.append(f"FAILED {f}")
        for p in self.pairs:
            same_n = sum(1 for r in p.runs if not r.moves and not r.error)
            out.append(f"\n{p.head[:12]} against {p.base[:12]}:")
            for r in p.runs:
                if r.error:
                    out.append(f"  {r.cell} {r.seed}: not compared: {r.error}")
                    continue
                row = r.head.row
                tr = f"trace {row.get('trace')}, reads {row.get('reads_hash')}"
                if not r.moves:
                    out.append(f"  {r.cell} {r.seed}: the same ({tr}, {len(r.head.files)} files)")
                    continue
                un = r.unexpected()
                moved = any(m.name in ("row.trace", "row.reads_hash") for m in r.moves)
                out.append(f"  {r.cell} {r.seed}: {len(r.moves)} moved, {len(un) or 'none'} unexpected"
                           f"{'' if moved else ' (' + tr + ' the same)'}")
                for m in un:
                    out.append(f"    UNEXPECTED {m.text}")
                    for s in m.detail.get("sample", [])[1:]:
                        out.append(f"        row {s['row']}: {shown(s['base'])} -> {shown(s['head'])} ({s['at']})")
                    for x in m.detail.get("pairs", [])[:3]:
                        out.append(f"        base row {x['base_row']} ~ head row {x['head_row']}: "
                                   f"{', '.join(x['columns'])}")
                    out += [f"        {line}" for line in divergence_text(m.detail.get("first"))]
                by = collections.defaultdict(list)
                for m in r.moves:
                    if m.expected:
                        by[m.name.split(".", 1)[0]].append(m)
                for table, ms in sorted(by.items()):
                    if len(ms) == 1:
                        out.append(f"    expected {ms[0].text}")
                        continue
                    shown = []
                    for m in ms[:6]:
                        what, _, how = m.text.partition(": ")
                        shown.append(f"{what.split('.', 1)[-1]} {how}".strip())
                    more = f"; and {len(ms) - 6} more" if len(ms) > 6 else ""
                    out.append(f"    expected {table}, {len(ms)} moved: {'; '.join(shown)}{more}")
            moved = len(p.runs) - same_n
            out.append(f"  {len(p.runs)} runs: {same_n} the same, {moved} moved, {p.unexpected()} unexpected moves")
        out.append("")
        if any(r.moves for p in self.pairs for r in p.runs):
            out.append(f"every move: --expect {' '.join(self.suggested())}")
        if self.expect.unused():
            out.append(f"--expect that allowed nothing: {' '.join(self.expect.unused())}")
        big = self.largest_run()
        if big:
            out.append(f"peak memory: this process {self.peak_mib} MiB; the largest run {big.record['rss_mb']} MiB "
                       f"({big.record['cell']} {big.record['seed']} at {big.record['sha'][:12]})")
        t = self.times
        builds = ", ".join(f"{k[:12]} " + ("kept" if v is None else f"{v:.0f} s") for k, v in self.builds.items())
        out.append(f"wall {t['total']:.0f} s: builds {t['builds']:.0f} s ({builds}), "
                   f"runs {t['runs']:.0f} s, compare {t['compare']:.0f} s")
        code = self.exit_code()
        out.append(f"result: {['neutral', 'UNEXPECTED MOVES', 'FAILED'][code]} (exit {code})")
        return "\n".join(out) + "\n"

    def json(self):
        def run_json(r):
            return {"cell": r.cell, "seed": r.seed, "error": r.error,
                    "trace": [x.row.get("trace") if x else None for x in (r.base, r.head)],
                    "reads_hash": [x.row.get("reads_hash") if x else None for x in (r.base, r.head)],
                    "dirs": [str(x.dir) if x else None for x in (r.base, r.head)],
                    "moves": [dataclasses.asdict(m) for m in r.moves]}

        return {"head": self.head, "cells": [c["name"] for c in self.cells], "seeds": list(self.seeds),
                "logs": {c["name"]: LOGS[c["sim"]] for c in self.cells}, "builds": self.builds,
                "failures": self.failures,
                "pairs": [{"base": p.base, "head": p.head, "unexpected": p.unexpected(),
                           "runs": [run_json(r) for r in p.runs]} for p in self.pairs],
                "expect": sorted(self.expect.names), "unused_expect": self.expect.unused(),
                "suggested_expect": self.suggested(), "times": self.times, "peak_mib": self.peak_mib,
                "unexpected": self.unexpected(), "exit": self.exit_code()}


def _log(s):
    print(s, file=sys.stderr, flush=True)


def check(bases=(), head="HEAD", cells=None, seeds=SEEDS, expect=(), slots=None, mem_gib=None, log=None):
    """The check, as the module docs say: a Report."""
    log = log or _log
    t0 = time.monotonic()
    cells = cell_set() if cells is None else cells
    expect = Expect(expect)
    prune()
    h, bs = pairs(bases, head)
    shas = [h, *(b for b in bs if b != h)]
    log(f"neutral: {h[:12]} ({head}) against {', '.join(b[:12] for b in bs)}")
    failures, bins, builds = [], {}, {}
    with cf.ThreadPoolExecutor(len(shas)) as ex:
        futs = {s: ex.submit(build, s, log) for s in shas}
        for s, f in futs.items():
            try:
                bins[s], builds[s] = f.result()
            except (BuildError, GitError) as e:
                failures.append(f"build of {s[:12]}: {e}")
    t_builds = time.monotonic() - t0
    runs, run_failures = ({}, []) if failures else run(bins, cells, seeds, slots, mem_gib, log)
    failures += run_failures
    t_runs = time.monotonic() - t0 - t_builds
    out = []
    for b in bs if not (failures and not runs) else ():
        diffs = []
        for c in cells:
            for s in seeds:
                ra, rb = runs.get((b, c["name"], s)), runs.get((h, c["name"], s))
                if ra is None or rb is None:
                    missing = ", ".join(x[:12] for x, r in ((b, ra), (h, rb)) if r is None)
                    diffs.append(RunDiff(c["name"], s, ra, rb, [], f"no run at {missing}"))
                    continue
                diffs.append(RunDiff(c["name"], s, ra, rb, compare(ra, rb, expect)))
        out.append(Pair(b, h, diffs))
    total = time.monotonic() - t0
    times = {"total": round(total, 1), "builds": round(t_builds, 1), "runs": round(t_runs, 1),
             "compare": round(total - t_builds - t_runs, 1)}
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss // 1024
    return Report(h, out, cells, list(seeds), expect, builds, failures, times, peak)
