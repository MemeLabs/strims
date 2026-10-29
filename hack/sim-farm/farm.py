#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Runs the sims' trial matrices, and the testbed's, on EC2 spot instances.

  hack/sim-farm/farm.py build  MATRIX...
  hack/sim-farm/farm.py run    MATRIX... [--max-vcpus N] [--families F,...]
                               [--regions R,...] [--max-region-vcpus N]
                               [--max-instance-cores N] [--work-per-boot K]
                               [--hours H] [--verify N] [--job-timeout MIN]
                               [--keep] [--local [--slots N]]
  hack/sim-farm/farm.py estimate MATRIX... [run's options]
  hack/sim-farm/farm.py status [MATRIX] [--regions R,...]
  hack/sim-farm/farm.py down   [MATRIX] [--regions R,...]
  hack/sim-farm/farm.py report MATRIX [--where KEY=VALUE ...] [--group KEY,...]

A matrix (matrices/*.toml) names tests that run one config from their
environment and print one `ROW {json}` line: the ppspp sim's `sim_job`,
or a strims-sim sweep (target "scenarios") under STRIMS_SIM_JOB=1, which
runs the one seed STRIMS_SIM_SEED names. It names the arms to run them at
(git refs), the environment every job shares, and the axes whose product
gives the jobs:

  package = "strims-ppspp"      # the tests' crate,
  target = "sim"                # integration test target
  test = "sim_job"              # and test, or a list of tests (an axis)
  slot_mem_gib = 3              # the memory one job needs
  [arms]    name = "git ref"
  [env]     SIM_X = "fixed"
  [axes]    SIM_Y = ["a", "b"]
  [canary]  SIM_SECS = "2"      # overrides for the canary job (below)
  [report]  columns = ["field:mean,max", "num/den:ratio"]
            group = ["arm", "SIM_Y"]   # default: arm, test if there are
                                       # several, every axis but the seed
  [[derive]]                    # jobs that wait for another job's row:
  when = { SIM_Y = ["b"] }      # the jobs it selects
  from = { SIM_Y = "a" }        # wait for the job with this env instead,
  set = { SIM_Z = "field:round" }  # and take SIM_Z from its row

A [[derive]] table's jobs each wait for the job of the same arm, test
and environment but for `from`, which is a job of the matrix whether or
not the axes give it. They run once it has a row, with each variable of
`set` from a field of that row: as it is, or with `:round` to the nearest
integer, halves away from zero as Rust's f64::round does. A job matches
at most one table, and a job waited on waits on none. A derived job's id
hashes its environment and its table, not the values it takes: the job it
waits on fixes those.

An arm's source is the tree of what its binary is built from (Cargo.toml,
Cargo.lock, rust-toolchain.toml and crates/ at its commit), so commits
elsewhere change nothing. build checks an arm's commit out in a worktree
under target/sim-farm/src, builds the test's release binary into the
shared target directory, and keeps it as
target/sim-farm/bin/<crate>-<target>-<source>. Arms build from commits,
never from the working tree.

run builds what's missing, then keeps spot instances busy until every job
has a row. A matrix's state is target/sim-farm/<matrix>/: rows.jsonl (a
row per finished job, with its arm, commit, environment, host and its
zone, wall and CPU time and peak RSS), failures.jsonl, mismatches.jsonl
and farm.log. A job's id hashes its arm's source and its environment, so
a rerun skips the jobs that have rows, and an arm whose source changed
runs again.

run takes several matrices at once: one queue holds their jobs, and one
set of instances runs them. A job two matrices share (the same source and
environment) runs once, and its row goes to each under that matrix's arm;
a row one of them already has is copied into the others. The run is named
for its matrices, their names sorted and joined with "+" (past EC2's
256-character tag value, the first's, "+<n>more-" and a hash of them all),
and keeps its own files (the list of its matrices, the canary's trace, SSH
host keys, --local's worker) in target/sim-farm/<run>/, which for one
matrix is that matrix's state. Each matrix's farm.log gets the whole log.

A test job is expected to take the mean wall of the latest 64 earlier
rows of its test and environment, seeds aside, here in any matrix
(another arm's too; code and instance types change what a job takes);
else of its matrix's cell, the rows whose environment is the matrix's
[env] but for its axes and what [[derive]] sets (another value of an
axis), else whose environment has the matrix's [env] among other
variables (a knob it doesn't name); else, once some have run, the mean of
its matrix's jobs this run; else a minute. target/sim-farm/walls.jsonl
holds every test row's test, environment, wall and time, and each run
first takes in what the matrices' rows.jsonl have added since
(walls-read.json keeps how far). The queue runs the longest jobs first,
and before them those no earlier row times, which may be long, so a run
ends on short jobs; jobs expected alike keep their matrices' order.

- Test jobs' instances run one job per physical core, fewer if memory
  binds (the largest slot_mem_gib of the run's matrices each, and 4 GiB
  for the system). The candidates are the x86 types of --families from
  8xlarge up to --max-instance-cores (64) cores, in each of the run's
  regions (AWS, below) in the zones its VPC's public subnets cover. A pool
  is a type in a zone, and the pools of every region are ranked as one,
  by spot price per slot it would fill, then by the quota's vCPUs per slot
  (an SMT core costs two): the cheapest pools anywhere fill first, and a
  run goes on to dearer ones, in its region or another, only as the
  cheaper ones are full or out of capacity.
- A run launches slots for its work, not its jobs. An instance's boot
  (launch to its canary passed, most of a minute: this run's mean, else
  that of the last runs in costs.jsonl) is billed and runs nothing, so a
  slot is worth launching only for enough work. With B the boot and k
  --work-per-boot (2), the run wants as many new slots as the work left
  (the queued and waiting jobs' expected walls, and half theirs for the
  jobs out), less what its instances will have done (1 + k)B from now (a
  booting one's slots from when it should be ready), would keep busy for
  kB each: a new instance spends at most 1/(1 + k) of its time booting.
  It launches no more slots than jobs, and no instance of twice the slots
  it wants or more unless it has none.
- Each region has a spot quota of its own. A region's budget is its quota
  less the spot vCPUs it counts there outside this run's instances (those
  shutting down count too), at most --max-region-vcpus; the run's
  instances in every region together have at most --max-vcpus. No pool
  takes more than half of what the run may have in its region. A region's
  budget is read again every 2 minutes while the run wants more
  instances; a quota error there holds it at what the run has in it until
  then, and the run launches in the others meanwhile. A pool out of
  capacity, or that took an instance back (a spot interruption), is
  skipped for 10 minutes.
- Instances are one-time spot instances, tagged strims-sim-farm=<run>,
  that terminate when they shut down. Each shuts itself down after
  --hours (6), and its worker powers it off after --idle-minutes (15) with
  nothing to do and no word from the controller, so a controller that
  dies costs little. `down` terminates them; `status` and `down` given a
  matrix act on every run that includes it. Both look in every region the
  run could have used (AWS, below).
- An instance boots on a thread of its own from its launch, waiting on
  no sync: it polls for its address until it's running, then for SSH,
  uploads the worker and the binaries, starts the worker, and a new
  instance of test jobs then runs the canary (polled a second apart): the
  first matrix's first job that waits on none, with its [canary]
  overrides, also run here. A different trace means the binaries don't
  reproduce across hosts, and the run stops. The instance logs its boot's
  steps when it's ready: running with an address, SSH answering, its
  worker started, ready, each after its launch.
- Every 15 s the controller syncs each instance over SSH (worker.py
  sync), at once when an instance's boot ends, and again at once each one
  with room once the jobs it finished are taken off. A sync fills the
  instance's free slots, and queues jobs behind them while they're short
  (45 s or less expected): up to 45 s of expected work per slot (three
  syncs' worth), but no more work than the instance's share of what's
  left (a long job queued behind others would start late, and could end
  the run late). It stores the rows that came back (fsync), and acks them
  on the next sync.
- An instance that is gone, has a spot interruption notice, or fails SSH
  for 3 minutes is terminated, and its jobs go back to the head of the
  queue. Runs are deterministic, so a reissued job must print the same
  row: a job whose row arrives twice is compared on (trace, frames,
  bytes), and --verify N (2) runs N jobs of each matrix twice on purpose,
  on different instances. A job that fails twice is recorded and dropped.
- A job that waits goes to the head of the queue once the job it waits
  on has a row, and fails for good when that job does. Waiting jobs count
  towards the instances the run wants, and keep them from being retired.
- Once the queue is empty, instances with nothing left to run are
  terminated; until then they take the next matrix's jobs. However the run
  ends (done, ^C, SIGTERM or SIGHUP, or an error), it terminates the
  instances it has, each in its region, then finds its tag in every region
  it could have used and terminates what that turns up: an instance whose
  launch never answered. A signal during a launch waits for the launch to
  answer, so its instance is tracked, and a second signal doesn't cut the
  cleanup short. With --keep an interrupted run leaves its instances; a
  rerun of the same matrices, in any order, finds them by tag in those
  regions and adopts their jobs.

--local runs the jobs here instead, --slots at a time (default half the
cores), with no AWS: for small matrices and for measuring one job.
Testbed jobs run here one at a time.

What a run costs is its instances' spot prices, plus each one's public
IPv4 address and root volume, from its launch call to its terminate call,
a minute at least, as EC2 bills them. A test run logs its estimate before
it launches: its jobs and expected work, the instances it would launch
for that in the cheapest pools if each had capacity, and its wall time
and cost: the model's (a boot, then the work over their slots or the
longest job, whichever is longer, and two syncs, for those instances),
times the median of what the last 5 runs took over their model's, which
covers what it leaves out (the syncs' latency, waves of jobs, the end).
`estimate` prints that for a run's matrices and options without building
or launching anything: run it with --regions and without to see what
more regions buy. Once it's done a run logs what it took (launch to
finish), what it cost, per 1,000 jobs and as instances booting, running
jobs (their walls) and idle, and each region's mean boot, and adds that
and its estimate as a line of target/sim-farm/costs.jsonl.

A testbed matrix (kind = "testbed") runs hack/testbed/run.sh as a black
box: `run.sh <out dir>` with the job's environment, real nodes in Docker
containers in real time. Its jobs are its arms by its axes by `repeats`:

  kind = "testbed"
  repeats = 3                   # job ids hash the repeat; more adds jobs
  host_cores = 4                # an instance's physical cores, exactly,
  host_mem_gib = 8              # and at least this memory (default 0)
  sources = ["crates", ...]     # an arm's source (default as for tests;
                                # hack/testbed is always in it)
  paired = true                 # the default (below)
  [build]     commands = ["cargo build --release ..."]
  [artifacts] NODE_BIN = "$CARGO_TARGET_DIR/release/examples/testbed_node"
  [files]     TOPOLOGY = "mesh3"
  [arms] [env] [axes] [report]  as above; [report] over the row below

build runs the commands with bash in the arm's worktree, with
CARGO_TARGET_DIR at target/sim-farm/cargo (in the shared target/, an old
arm's build would replace the example run.sh finds by default),
CARGO_INCREMENTAL=0, and STRIMS_ROOT at this checkout. It packs the
worktree's hack/testbed and each artifact (a file or a directory at a
path in the worktree, where $VAR and ${VAR} take the build's environment)
into the arm's bundle, target/sim-farm/bin/testbed-<hash>.tar.gz, which
hashes the arm's source, the commands and the artifacts. A host runs
run.sh from the bundle's hack/testbed with each artifact's variable set
to its path there, and each [files] variable (a file relative to the
matrix, the same for every arm, whose path may name the job's variables
as ${VAR}: "topologies/mesh6-v${VIEWERS}") to the host's copy. A job's id
hashes its source, build, files, environment and repeat.

Testbed runs are real time, so nothing checks that they reproduce: no
canary, no --verify, and a reissued job just runs again (a job's second
result is dropped). Instances are the --families types with host_cores
cores, one job at a time each, and of one type where it has capacity:
the type of the run's first launch comes first in every zone of every
region. Docker and the qdisc modules are set up at boot, and shaping is
tried on a dummy device; a host that fails is dropped with its log, as
one that fails to boot. A host runs a testbed job once no container named
tb-* (run.sh's) is running.

With `paired`, the jobs of one environment and repeat, one per arm, go to
one instance together and run there one after another, the arms' order
rotated a place per group (A B, B A, A B, ...), so what differs between
hosts hits every arm alike and runs compare within a host. Each arm goes
first equally often only when the groups are a multiple of the arms (3
repeats of 2 arms put one first twice). A comparison takes as many runs'
time as it has arms; paired = false sends every job its own way.

A finished run's out dir comes back to target/sim-farm/<matrix>/runs/<id>/
(packed on the host, as root if the worker can't read it), and its row
records its arm, commit, environment, repeat, group (the jobs that went
to one host together) and order there, the instance (id, type, zone, CPU
model, kernel, Docker version), when it started, its wall time and exit,
and in `row` what reports aggregate: wall_s, the host's steal and busy
shares of CPU time over the run (steal_pct, busy_pct), steal's worst 10 s
(steal_peak_pct), the 1-minute load's mean and max (load_mean, load_max),
and node-<i>.jsonl's line counts (node_lines, node_lines_min). A run that
leaves no node-<i>.jsonl fails. report adds each instance's runs in order,
and each group's run dirs as <group>=<dir>,..., the testbed scoring
scripts' form.

run.sh's images pull their bases from Docker Hub, whose anonymous limit
counts pulls per address. Each instance has an address of its own, so
instances don't add up against it; one that comes with an address over
the limit fails its builds, and its jobs go to other instances, failing
for good the second time.

AWS: the aws CLI's credentials (its default profile, or the config
file's). A run launches in the regions --regions names, else the config
file's aws_regions, else the aws CLI's one region ($AWS_REGION, or the
profile's), and every call names its region. In each, the farm needs
subnets that map public IPs on launch; on first use there it imports the
key pair and creates the security group, and each run finds that
region's Ubuntu image and spot quota. target/sim-farm/regions lists
every region the farm has launched in from this checkout: status, down,
and a run's adoption and cleanup look in those too, so an instance in a
region since dropped from the config isn't missed. report reads only
local files. The user needs, in each region,
ec2:RunInstances (spot, with tags on launch), ec2:TerminateInstances on
tagged instances, ec2:ImportKeyPair, ec2:CreateSecurityGroup,
ec2:AuthorizeSecurityGroupIngress, the Describe calls for instances,
images, key pairs, security groups, subnets, instance types, type
offerings and spot prices, and servicequotas:GetServiceQuota. SSH uses
--ssh-key ($STRIMS_FARM_SSH_KEY, the config file's ssh_key, or
~/.ssh/id_rsa), whose .pub is imported as the key pair
strims-sim-farm-<file name>; it must work without a prompt (no
passphrase, or loaded into ssh-agent). The security group strims-sim-farm,
in the VPC of each region's public subnets, admits SSH from this host's
public address.

What's particular to the person running the farm goes in
hack/sim-farm/config.toml, which git ignores; flags and the environment
override it:

  ssh_key = "~/.ssh/id_rsa-me"
  aws_profile = "farm"          # else $AWS_PROFILE, or the CLI's default
  aws_region = "us-west-2"      # else $AWS_REGION, or the profile's
  aws_regions = ["us-west-2", "us-east-2"]  # several, instead
  [run]                         # defaults for run's options
  max_vcpus = 128
  hours = 4
"""

import argparse
import collections
import concurrent.futures as cf
import contextlib
import dataclasses
import datetime
import fcntl
import hashlib
import itertools
import json
import math
import os
import re
import shutil
import signal
import statistics
import subprocess
import sys
import tarfile
import threading
import time
import tomllib
import urllib.request
import uuid
from pathlib import Path

HERE = Path(__file__).resolve().parent
CONFIG = HERE / "config.toml"
ROOT = HERE.parent.parent
FARM = ROOT / "target" / "sim-farm"
BIN = FARM / "bin"
# Every region this checkout's farm has launched in, a line each: status,
# down and a run's adoption and cleanup look there too.
REGIONS = FARM / "regions"
TAG = "strims-sim-farm"
UBUNTU_OWNER = "099720109477"
UBUNTU_IMAGE = "ubuntu/images/hvm-ssd-gp3/ubuntu-resolute-26.04-amd64-server-*"
SPOT_QUOTA = "L-34B43A08"
FAMILIES = "c6a,c7a,c8a,m6a,m7a,m8a,r6a,r7a,r8a,c6i,c7i,c8i,m6i,m7i,m8i,r6i,r7i,r8i"
TICK = 15
SSH_GRACE = 180
POOL_BACKOFF = 600
PRICE_REFRESH = 600
BUDGET_REFRESH = 120
SYSTEM_GIB = 4
# What a job is taken to need, in seconds of its slot, when no row says,
# and an instance's boot (launch to its canary passed) before any is
# measured.
DEFAULT_JOB_S = 60
DEFAULT_BOOT_S = 75
# The work, in seconds per slot, an instance's queue holds: short jobs
# enough to last three syncs.
PREFETCH_S = 3 * TICK
# The rows a job's expected wall averages, the latest: code and instance
# types change what a job takes.
RECENT_ROWS = 64
# EC2 bills a minute at least, and an instance's public IPv4 address
# ($0.005/h) and 16 GiB gp3 root volume ($0.08/GiB-month) on top of its
# spot price.
MIN_BILLED_S = 60
INSTANCE_EXTRA_HOURLY = 0.005 + 16 * 0.08 / 730
# Every test row's test, environment, wall and time (index_walls), how far
# each matrix's rows.jsonl has been read into it, and a line per finished
# run.
WALLS = FARM / "walls.jsonl"
WALLS_READ = FARM / "walls-read.json"
COSTS = FARM / "costs.jsonl"
# The seed knobs, which reports don't group by.
SEED_KEYS = {"SIM_SEED", "STRIMS_SIM_SEED"}
# What a test binary is built from: commits that change none of it share
# a binary, and jobs.
SOURCES = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates"]
# What a testbed arm's source always covers: run.sh and its images.
TESTBED = "hack/testbed"
TESTBED_COLUMNS = ["wall_s:mean,max", "steal_pct:mean,max", "load_max:max", "node_lines_min:min"]
# The keys only test matrices take.
TEST_KEYS = {"package", "target", "test", "slot_mem_gib", "canary", "derive"}
CAPACITY_ERRORS = {
    "InsufficientInstanceCapacity",
    "InsufficientCapacity",
    "SpotMaxPriceTooLow",
    "Unsupported",
}
QUOTA_ERRORS = {"MaxSpotInstanceCountExceeded", "VcpuLimitExceeded"}


class Log:
    def __init__(self) -> None:
        self.files = []
        self.lock = threading.Lock()

    def open(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        self.files.append(path.open("a"))

    def __call__(self, msg: str) -> None:
        line = f"[{time.strftime('%H:%M:%S')}] {msg}"
        with self.lock:
            try:
                print(line, flush=True)
            except OSError:
                pass  # the terminal is gone (SIGHUP); the files have it
            for f in self.files:
                f.write(line + "\n")
                f.flush()


log = Log()


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout.strip()


# --- matrices -----------------------------------------------------------


@dataclasses.dataclass(frozen=True)
class Job:
    id: str
    arm: str
    commit: str
    source: str
    bin: str
    test: str
    env: tuple[tuple[str, str], ...]
    # The id of the job whose row this one waits for, and the variables it
    # takes from that row (its [[derive]] table's `set`).
    after: str = ""
    derive: tuple[tuple[str, str], ...] = ()
    # A testbed job's (its bin is its arm's bundle): its repeat, the group
    # it goes to a host with, and the matrix's files as (variable, name in
    # bin/).
    kind: str = "test"
    repeat: int = 0
    group: str = ""
    files: tuple[tuple[str, str], ...] = ()


@dataclasses.dataclass
class Derive:
    """A [[derive]] table."""

    when: dict[str, set[str]]
    src: dict[str, str]
    set: tuple[tuple[str, str], ...]

    def matches(self, env: dict[str, str]) -> bool:
        return all(env.get(k) in vs for k, vs in self.when.items())


def round_half_away(x: float) -> int:
    """x to the nearest integer, halves away from zero, as Rust's
    f64::round does."""
    a = abs(x)
    r = math.floor(a)
    if a - r >= 0.5:
        r += 1
    return int(math.copysign(r, x))


def derived_env(job: "Job", row: dict) -> tuple[dict[str, str], str | None]:
    """The variables `job` takes from the row of the job it waits on, or
    why it can't."""
    out = {}
    for var, spec in job.derive:
        field, _, how = spec.partition(":")
        v = row.get(field)
        if how == "round":
            if isinstance(v, bool) or not isinstance(v, (int, float)):
                return {}, f"{var}: the row's {field} is {v!r}, not a number"
            v = round_half_away(v)
        elif isinstance(v, bool) or not isinstance(v, (int, float, str)):
            return {}, f"{var}: the row's {field} is {v!r}"
        out[var] = str(v)
    return out, None


@dataclasses.dataclass(frozen=True)
class Item:
    """A job as queued: the original or its verify copy, which the worker
    knows by another id."""

    job: Job
    copy: str = ""

    @property
    def wid(self) -> str:
        return self.job.id + self.copy


class Matrix:
    def __init__(self, path: str) -> None:
        self.path = Path(path)
        self.name = self.path.stem
        m = tomllib.loads(self.path.read_text())
        self.kind = m.get("kind", "test")
        self.arms = {k: str(v) for k, v in m["arms"].items()}
        self.env = {k: str(v) for k, v in m.get("env", {}).items()}
        self.axes = {k: [str(x) for x in v] for k, v in m.get("axes", {}).items()}
        if self.kind == "testbed":
            self.parse_testbed(m)
            columns = TESTBED_COLUMNS
        elif self.kind == "test":
            self.package = m["package"]
            self.target = m["target"]
            test = m["test"]
            self.tests = [str(t) for t in test] if isinstance(test, list) else [str(test)]
            self.slot_mem_gib = float(m.get("slot_mem_gib", 2))
            self.canary = {k: str(v) for k, v in m.get("canary", {}).items()}
            self.derives = [self.parse_derive(i, d) for i, d in enumerate(m.get("derive", []))]
            columns = ["delivery:mean,min", "lag_p50_ms:median", "lag_p99_ms:median,max"]
        else:
            sys.exit(f'{self.path}: kind = {self.kind!r}: "test" (the default) or "testbed"')
        rep = m.get("report", {})
        self.columns = rep.get("columns", columns)
        self.group = rep.get(
            "group",
            ["arm"]
            + (["test"] if len(self.tests) > 1 else [])
            + [k for k in self.axes if k not in SEED_KEYS],
        )
        self.state = FARM / self.name
        self.commits: dict[str, str] = {}
        self.sources: dict[str, str] = {}

    def parse_testbed(self, m: dict) -> None:
        def bad(msg: str):
            sys.exit(f"{self.path}: {msg}")

        if TEST_KEYS & set(m):
            bad(f"a testbed matrix takes no {', '.join(sorted(TEST_KEYS & set(m)))}")
        self.tests, self.slot_mem_gib, self.canary, self.derives = [""], 0.0, {}, []
        self.repeats = int(m.get("repeats", 1))
        if self.repeats < 1:
            bad("repeats is at least 1")
        self.host_cores = int(m.get("host_cores", 4))
        self.host_mem_gib = float(m.get("host_mem_gib", 0))
        self.paired = bool(m.get("paired", True))
        self.source_paths = sorted({*map(str, m.get("sources", SOURCES)), TESTBED})
        self.commands = [str(c) for c in m.get("build", {}).get("commands", [])]
        self.artifacts = {k: str(v) for k, v in m.get("artifacts", {}).items()}
        # variable -> its path, which may name the job's variables
        self.files = {k: str(v) for k, v in m.get("files", {}).items()}
        # name in bin/ -> the file here, for the jobs made so far
        self.shipped: dict[str, Path] = {}
        given = [set(self.env) | set(self.axes), set(self.artifacts), set(self.files)]
        twice = set().union(*(a & b for a, b in itertools.combinations(given, 2)))
        if twice:
            bad(f"{sorted(twice)} set twice ([env] or [axes], [artifacts], [files])")

    def parse_derive(self, i: int, d: dict) -> Derive:
        def bad(msg: str):
            sys.exit(f"{self.path}: [[derive]] {i + 1}: {msg}")

        known = set(self.env) | set(self.axes)
        when = {
            k: {str(x) for x in (v if isinstance(v, list) else [v])}
            for k, v in d.get("when", {}).items()
        }
        src = {k: str(v) for k, v in d.get("from", {}).items()}
        sets = tuple(sorted((k, str(v)) for k, v in d.get("set", {}).items()))
        if not when or not src or not sets:
            bad("it needs when, from and set")
        if set(when) - known or set(src) - known:
            bad(f"when and from name {sorted((set(when) | set(src)) - known)}, not in [env] or [axes]")
        if {k for k, _ in sets} & known:
            bad(f"set names {sorted({k for k, _ in sets} & known)}, which [env] or [axes] set")
        for k, spec in sets:
            field, _, how = spec.partition(":")
            if not field or how not in ("", "round"):
                bad(f"set {k} = {spec!r}: a row field, or field:round")
        return Derive(when, src, sets)

    def derive_of(self, env: dict[str, str]) -> Derive | None:
        hits = [d for d in self.derives if d.matches(env)]
        if len(hits) > 1:
            sys.exit(f"{self.path}: a job matches two [[derive]] tables: {env}")
        return hits[0] if hits else None

    def resolve(self) -> None:
        paths = self.source_paths if self.kind == "testbed" else SOURCES
        for arm, ref in self.arms.items():
            commit = git("rev-parse", "--verify", f"{ref}^{{commit}}")
            tree = git("ls-tree", commit, *paths)
            self.commits[arm] = commit
            self.sources[arm] = hashlib.sha256(tree.encode()).hexdigest()[:12]

    def bin_name(self, source: str) -> str:
        if self.kind == "testbed":
            key = json.dumps([source, self.commands, sorted(self.artifacts.items())])
            return f"testbed-{hashlib.sha256(key.encode()).hexdigest()[:12]}.tar.gz"
        return f"{self.package}-{self.target}-{source}"

    def job(self, arm: str, test: str, env: dict[str, str], derive: Derive | None = None,
            after: str = "") -> Job:
        source = self.sources[arm]
        env_t = tuple(sorted(env.items()))
        key = [source, self.package, self.target, test, env_t]
        if derive is not None:
            key.append([sorted(derive.src.items()), derive.set])
        jid = hashlib.sha256(json.dumps(key).encode()).hexdigest()[:16]
        return Job(
            jid, arm, self.commits[arm], source, self.bin_name(source), test, env_t,
            after, derive.set if derive is not None else (),
        )

    def file_of(self, var: str, env: dict[str, str]) -> str:
        """[files]' `var` for a job of `env`: its name in bin/, which hashes
        the file."""
        p = (self.path.parent / expand(self.files[var], env)).resolve()
        if not p.is_file():
            sys.exit(f"{self.path}: [files] {var}: no file {p}")
        name = f"file-{hashlib.sha256(p.read_bytes()).hexdigest()[:12]}-{p.name}"
        self.shipped[name] = p
        return name

    def testbed_job(self, arm: str, env: dict[str, str], repeat: int, group: str) -> Job:
        source = self.sources[arm]
        env_t = tuple(sorted(env.items()))
        files = tuple(sorted((k, self.file_of(k, env)) for k in self.files))
        key = ["testbed", source, self.commands, sorted(self.artifacts.items()), files, env_t, repeat]
        jid = hashlib.sha256(json.dumps(key).encode()).hexdigest()[:16]
        return Job(
            jid, arm, self.commits[arm], source, self.bin_name(source), "", env_t,
            kind="testbed", repeat=repeat, group=group or jid, files=files,
        )

    def testbed_jobs(self) -> list[Job]:
        """Group by group, repeat by repeat: each group's arms in turn, their
        order rotated a place per group."""
        keys = list(self.axes)
        arms = list(self.arms)
        out = []
        for repeat in range(1, self.repeats + 1):
            for combo in itertools.product(*(self.axes[k] for k in keys)):
                env = {**self.env, **dict(zip(keys, combo))}
                group = ""
                if self.paired:
                    key = json.dumps([self.name, sorted(env.items()), repeat])
                    group = hashlib.sha256(key.encode()).hexdigest()[:12]
                k = len(out) // len(arms) % len(arms)
                out += [self.testbed_job(a, env, repeat, group) for a in arms[k:] + arms[:k]]
        return out

    def jobs(self) -> list[Job]:
        """The axes' jobs, and the jobs derived ones wait on, then the
        derived jobs."""
        if self.kind == "testbed":
            return self.testbed_jobs()
        keys = list(self.axes)
        plain, derived = [], []
        ids = set()
        for arm in self.arms:
            for test in self.tests:
                for combo in itertools.product(*(self.axes[k] for k in keys)):
                    env = {**self.env, **dict(zip(keys, combo))}
                    d = self.derive_of(env)
                    if d is None:
                        plain.append(self.job(arm, test, env))
                        ids.add(plain[-1].id)
                        continue
                    src_env = {**env, **d.src}
                    if src_env == env:
                        sys.exit(f"{self.path}: a [[derive]] job would wait on itself: {env}")
                    if self.derive_of(src_env) is not None:
                        sys.exit(f"{self.path}: a job waited on waits on another: {src_env}")
                    src = self.job(arm, test, src_env)
                    derived.append((src, self.job(arm, test, env, d, src.id)))
        out = list(plain)
        for src, _ in derived:
            if src.id not in ids:
                out.append(src)
                ids.add(src.id)
        return out + [j for _, j in derived]

    def canary_job(self) -> Job:
        first = next(j for j in self.jobs() if not j.after)
        return self.job(first.arm, first.test, {**dict(first.env), **self.canary})


def run_name(matrices: list[Matrix]) -> str:
    """The matrices' names, sorted and joined with "+": the run's tag, so a
    rerun of the same matrices adopts its instances. EC2 tag values take
    256 characters, and the Name tag is sim-farm-<run>: a longer name gives
    way to the first matrix's, the count of the others and a hash of all of
    them, and the run's state lists them (run_matrices)."""
    names = sorted(m.name for m in matrices)
    if len(set(names)) < len(names):
        sys.exit(f"two matrices share a name: {names}")
    if len(names) > 1 and any("+" in n for n in names):
        sys.exit(f"a matrix run with others can't have a + in its name: {names}")
    name = "+".join(names)
    if len(f"sim-farm-{name}") > 256:
        h = hashlib.sha1(name.encode()).hexdigest()[:8]
        name = f"{names[0]}+{len(names) - 1}more-{h}"
        if len(f"sim-farm-{name}") > 256:
            sys.exit(f"{names[0]}: the name is too long for EC2's 256-character tag limit")
    return name


def past_boot_s(kind: str) -> float:
    """The mean boot of the last few runs of this kind (costs.jsonl), else
    DEFAULT_BOOT_S."""
    got = [r["boot_s"] for r in read_jsonl(COSTS) if r.get("kind") == kind and r.get("boot_s")][-5:]
    return statistics.fmean(got) if got else DEFAULT_BOOT_S


def past_ratios() -> tuple[float, float, int]:
    """What the last few finished test runs took over their model's wall
    time and cost (costs.jsonl), the medians, and how many runs: what the
    model leaves out (the syncs' latency, the waves of jobs, the end)."""
    past = [
        r for r in read_jsonl(COSTS)
        if r.get("kind") == "test" and not r.get("interrupted") and r.get("wall_s") and r.get("cost")
        and (r.get("estimate") or {}).get("model_wall_s", (r.get("estimate") or {}).get("wall_s"))
    ][-5:]
    if not past:
        return 1.0, 1.0, 0
    model = [(r["estimate"].get("model_wall_s", r["estimate"]["wall_s"]),
              r["estimate"].get("model_cost", r["estimate"]["cost"])) for r in past]
    return (
        statistics.median(r["wall_s"] / w for r, (w, _) in zip(past, model)),
        statistics.median(r["cost"] / c for r, (_, c) in zip(past, model)),
        len(past),
    )


def run_matrices(run: str) -> list[str]:
    """The matrices of a run, as its state lists them."""
    if not run or "/" in run or run.startswith("."):
        return []
    try:
        return (FARM / run / "matrices").read_text().split()
    except OSError:
        return []


def read_jsonl(path: Path) -> list[dict]:
    """A JSON-lines file's records; a last line without its newline is one
    another process is still appending, and is left for the next read."""
    if not path.exists():
        return []
    text = path.read_text()
    out = []
    for line in text[: text.rfind("\n") + 1].splitlines():
        if line.strip():
            out.append(json.loads(line))
    return out


def append_jsonl(path: Path, recs: list[dict]) -> None:
    """Appends the records in one write, holding an exclusive flock on the
    file: runs at once share walls.jsonl and costs.jsonl."""
    if not recs:
        return
    with path.open("a") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        f.write("".join(json.dumps(r) + "\n" for r in recs))
        f.flush()
        os.fsync(f.fileno())


# --- job times ----------------------------------------------------------


def row_head(line: bytes) -> dict | None:
    """A rows.jsonl line's fields but its `row`, which comes last and can
    be large."""
    i = line.find(b', "row": ')
    for text in ([line[:i] + b"}"] if i > 0 else []) + [line]:
        try:
            return json.loads(text)
        except ValueError:
            pass
    return None


def index_walls() -> list[dict]:
    """Every test row's test, environment, wall and time here: walls.jsonl,
    after it takes in what each matrix's rows.jsonl has added since it was
    last read (all of it the first time)."""
    FARM.mkdir(parents=True, exist_ok=True)
    with (FARM / "walls.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            read = json.loads(WALLS_READ.read_text())
        except (OSError, ValueError):
            read = {}
        new = []
        for path in sorted(FARM.glob("*/rows.jsonl")):
            key = path.parent.name
            at = read.get(key, 0)
            if at > path.stat().st_size:
                at = 0  # rewritten
            with path.open("rb") as f:
                f.seek(at)
                for line in f:
                    if not line.endswith(b"\n"):
                        break  # still being written
                    at += len(line)
                    rec = row_head(line)
                    if rec and rec.get("kind", "test") == "test" and rec.get("wall_s"):
                        new.append({
                            "test": rec.get("test"), "env": rec.get("env", {}),
                            "wall_s": rec["wall_s"], "at": rec.get("at", ""),
                        })
            read[key] = at
        append_jsonl(WALLS, new)
        tmp = WALLS_READ.with_suffix(".tmp")
        tmp.write_text(json.dumps(read))
        os.replace(tmp, WALLS_READ)
        return read_jsonl(WALLS)


def env_key(env: dict[str, str], drop: set[str]) -> tuple:
    return tuple(sorted((k, v) for k, v in env.items() if k not in drop))


def recent_mean(walls: list[tuple[str, float]]) -> float:
    """The mean wall of the latest RECENT_ROWS of (time, wall)."""
    return statistics.fmean(w for _, w in sorted(walls)[-RECENT_ROWS:])


def priors(owners: dict[str, list[tuple["Matrix", "Job"]]]) -> dict[str, float]:
    """Each job's expected wall, from past rows (index_walls), if any: the
    mean of the latest RECENT_ROWS rows of its test and environment, seeds
    aside, else of its matrix's cell, the rows of its test whose environment
    is the matrix's [env] but for the axes and what [[derive]] sets (another
    value of an axis), else whose environment has the matrix's [env] among
    other variables (a knob the matrix doesn't name)."""
    recs = index_walls()
    exact = collections.defaultdict(list)
    for r in recs:
        exact[(r["test"], env_key(r["env"], SEED_KEYS))].append((r.get("at", ""), r["wall_s"]))
    out = {}
    missed = collections.defaultdict(list)
    for jid, own in owners.items():
        m, j = own[0]
        walls = exact.get((j.test, env_key(dict(j.env), SEED_KEYS)))
        if walls:
            out[jid] = recent_mean(walls)
        else:
            missed[m].append(j)
    for m, jobs in missed.items():
        drop = SEED_KEYS | set(m.axes) | {k for d in m.derives for k, _ in d.set}
        fixed = {k: v for k, v in m.env.items() if k not in drop}
        cell, loose = collections.defaultdict(list), collections.defaultdict(list)
        for r in recs:
            e = r["env"]
            if r["test"] in m.tests and all(e.get(k) == v for k, v in fixed.items()):
                loose[r["test"]].append((r.get("at", ""), r["wall_s"]))
                if all(k in fixed or k in drop for k in e):
                    cell[r["test"]].append(loose[r["test"]][-1])
        means = {t: recent_mean(cell[t] or w) for t, w in loose.items()}
        for j in jobs:
            if j.test in means:
                out[j.id] = means[j.test]
    return out


# --- build --------------------------------------------------------------


@contextlib.contextmanager
def checkout(commit: str):
    """A worktree of the commit under target/sim-farm/src, for as long as
    the block runs."""
    src = FARM / "src" / commit[:12]
    if src.exists():
        subprocess.run(["git", "worktree", "remove", "--force", str(src)], cwd=ROOT)
        shutil.rmtree(src, ignore_errors=True)
        git("worktree", "prune")
    git("worktree", "add", "--detach", str(src), commit)
    try:
        yield src
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", str(src)], cwd=ROOT)


def build(matrix: Matrix) -> None:
    matrix.resolve()
    BIN.mkdir(parents=True, exist_ok=True)
    if matrix.kind == "testbed":
        build_testbed(matrix)
        return
    for arm, commit in matrix.commits.items():
        out = BIN / matrix.bin_name(matrix.sources[arm])
        if out.exists():
            continue
        log(f"building {arm} at {commit[:12]}")
        with checkout(commit) as src:
            env = {**os.environ, "CARGO_TARGET_DIR": str(ROOT / "target"), "CARGO_INCREMENTAL": "0"}
            cmd = [
                "cargo", "test", "--release", "--locked", "-p", matrix.package,
                "--test", matrix.target, "--no-run", "--message-format=json",
            ]
            p = subprocess.run(cmd, cwd=src, env=env, stdout=subprocess.PIPE, text=True)
            exe = None
            for line in p.stdout.splitlines():
                try:
                    m = json.loads(line)
                except json.JSONDecodeError:
                    continue
                t = m.get("target", {})
                if (
                    m.get("reason") == "compiler-artifact"
                    and m.get("executable")
                    and t.get("name") == matrix.target
                    and t.get("kind") == ["test"]
                ):
                    exe = m["executable"]
            if p.returncode or not exe:
                sys.exit(f"building {arm} ({commit[:12]}) failed")
            tmp = out.with_name(out.name + ".tmp")
            shutil.copy2(exe, tmp)
            os.replace(tmp, out)


def expand(spec: str, env: dict[str, str]) -> str:
    """$VAR and ${VAR} in a path, from `env`."""
    def var(m: re.Match) -> str:
        name = m.group(1) or m.group(2)
        if name not in env:
            sys.exit(f"{spec!r}: ${name} isn't set")
        return env[name]

    return re.sub(r"\$(?:\{(\w+)\}|(\w+))", var, spec)


def build_testbed(matrix: Matrix) -> None:
    matrix.jobs()  # each job's files
    for name, path in matrix.shipped.items():
        out = BIN / name
        if not out.exists():
            tmp = out.with_name(out.name + ".tmp")
            shutil.copy2(path, tmp)
            os.replace(tmp, out)
    for arm, commit in matrix.commits.items():
        out = BIN / matrix.bin_name(matrix.sources[arm])
        if out.exists():
            continue
        log(f"building {arm} at {commit[:12]}")
        with checkout(commit) as src:
            env = {
                **os.environ, "CARGO_TARGET_DIR": str(FARM / "cargo"), "CARGO_INCREMENTAL": "0",
                "STRIMS_ROOT": str(ROOT),
            }
            for cmd in matrix.commands:
                if subprocess.run(["bash", "-c", cmd], cwd=src, env=env).returncode:
                    sys.exit(f"building {arm} ({commit[:12]}): `{cmd}` failed")
            if not (src / TESTBED).is_dir():
                sys.exit(f"{arm} ({commit[:12]}) has no {TESTBED}")
            tmp = out.with_name(out.name + ".tmp")
            with tarfile.open(tmp, "w:gz", dereference=True) as tar:
                tar.add(src / TESTBED, arcname=TESTBED)
                for var, spec in sorted(matrix.artifacts.items()):
                    path = src / expand(spec, env)
                    if not path.exists():
                        sys.exit(f"building {arm} ({commit[:12]}): no artifact {var} at {path}")
                    tar.add(path, arcname=f"artifacts/{var}")
            os.replace(tmp, out)
        log(f"packed {arm} as {out.name} ({out.stat().st_size / 1e6:.1f} MB)")


# --- AWS ----------------------------------------------------------------


class AwsError(Exception):
    def __init__(self, code: str, msg: str) -> None:
        super().__init__(msg)
        self.code = code


def aws(region: str, *args: str) -> dict:
    """An aws CLI call in `region`. It runs in a session of its own, so a ^C
    at the terminal reaches the controller alone, which lets a launch
    finish (Run.held) and terminates the run's instances before it exits."""
    p = subprocess.run(
        ["aws", *args, "--region", region, "--output", "json"],
        capture_output=True, text=True, start_new_session=True,
    )
    if p.returncode:
        m = re.search(r"An error occurred \(([^)]+)\)", p.stderr)
        raise AwsError(m.group(1) if m else "", p.stderr.strip())
    return json.loads(p.stdout) if p.stdout.strip() else {}


def cli_region() -> str:
    """The aws CLI's own region: $AWS_REGION, $AWS_DEFAULT_REGION, or the
    profile's."""
    region = os.environ.get("AWS_REGION") or os.environ.get("AWS_DEFAULT_REGION")
    if not region:
        p = subprocess.run(["aws", "configure", "get", "region"], capture_output=True, text=True)
        region = p.stdout.strip()
    if not region:
        sys.exit(f"no AWS region: aws_regions or aws_region in {CONFIG}, $AWS_REGION, or the profile's")
    return region


def recorded_regions() -> set[str]:
    """Every region this checkout's farm has launched in (REGIONS)."""
    try:
        return set(REGIONS.read_text().split())
    except FileNotFoundError:
        return set()


def record_region(region: str) -> None:
    if region not in recorded_regions():
        REGIONS.parent.mkdir(parents=True, exist_ok=True)
        with REGIONS.open("a") as f:
            f.write(region + "\n")
            f.flush()
            os.fsync(f.fileno())


def in_regions(fn, regions: list[str]) -> dict[str, object]:
    """fn(region) in each region at once: what it returned, or the
    exception it raised."""
    with cf.ThreadPoolExecutor(max_workers=max(1, len(regions))) as ex:
        futs = {r: ex.submit(fn, r) for r in regions}
    return {r: f.exception() or f.result() for r, f in futs.items()}


def in_regions_or_exit(fn, regions: list[str]) -> dict[str, object]:
    """in_regions, raising the first region's error."""
    out = in_regions(fn, regions)
    for r, v in out.items():
        if isinstance(v, BaseException):
            if isinstance(v, (SystemExit, KeyboardInterrupt)):
                raise v
            sys.exit(f"{r}: {v}")
    return out


def tag_of(inst: dict, key: str) -> str | None:
    return next((t["Value"] for t in inst.get("Tags", []) if t["Key"] == key), None)


def in_run(inst: dict, matrix: str) -> bool:
    """Whether the instance is a run's that includes the matrix."""
    run = tag_of(inst, TAG) or ""
    return matrix == run or matrix in run.split("+") or matrix in run_matrices(run)


def farm_instances(region: str, run: str | None) -> list[dict]:
    flt = f"Name=tag:{TAG},Values={run}" if run else f"Name=tag-key,Values={TAG}"
    r = aws(
        region, "ec2", "describe-instances", "--filters", flt,
        "Name=instance-state-name,Values=pending,running,stopping,stopped,shutting-down",
    )
    return [i for res in r["Reservations"] for i in res["Instances"]]


def terminate(region: str, ids: list[str]) -> None:
    if ids:
        aws(region, "ec2", "terminate-instances", "--instance-ids", *ids)


def ensure_key(region: str, key: Path) -> str:
    name = f"{TAG}-{key.name}"
    try:
        aws(region, "ec2", "describe-key-pairs", "--key-names", name)
    except AwsError as e:
        if e.code != "InvalidKeyPair.NotFound":
            raise
        aws(
            region, "ec2", "import-key-pair", "--key-name", name,
            "--public-key-material", f"fileb://{key}.pub",
        )
        log(f"{region}: imported {key}.pub as the key pair {name}")
    return name


def network(region: str) -> tuple[str, dict[str, str]]:
    """The VPC of the region's public subnets, and a subnet per zone."""
    subnets = aws(
        region, "ec2", "describe-subnets", "--filters", "Name=map-public-ip-on-launch,Values=true"
    )["Subnets"]
    if not subnets:
        sys.exit(f"{region}: no subnet maps public IPs on launch")
    vpc = sorted(s["VpcId"] for s in subnets)[0]
    zones: dict[str, str] = {}
    for s in sorted(subnets, key=lambda s: s["SubnetId"]):
        if s["VpcId"] == vpc:
            zones.setdefault(s["AvailabilityZone"], s["SubnetId"])
    return vpc, zones


def public_cidr() -> str:
    ip = urllib.request.urlopen("https://checkip.amazonaws.com", timeout=10).read().decode().strip()
    return f"{ip}/32"


def ensure_sg(region: str, vpc: str, cidr: str) -> str:
    groups = aws(
        region, "ec2", "describe-security-groups", "--filters",
        f"Name=group-name,Values={TAG}", f"Name=vpc-id,Values={vpc}",
    )["SecurityGroups"]
    if groups:
        sg, perms = groups[0]["GroupId"], groups[0]["IpPermissions"]
    else:
        sg = aws(
            region, "ec2", "create-security-group", "--group-name", TAG,
            "--description", "sim farm: SSH from the controller", "--vpc-id", vpc,
        )["GroupId"]
        perms = []
        log(f"{region}: created the security group {TAG} ({sg})")
    if not any(
        p.get("FromPort") == 22 and any(r["CidrIp"] == cidr for r in p.get("IpRanges", []))
        for p in perms
    ):
        try:
            aws(
                region, "ec2", "authorize-security-group-ingress", "--group-id", sg,
                "--protocol", "tcp", "--port", "22", "--cidr", cidr,
            )
            log(f"{region}: {TAG} admits SSH from {cidr}")
        except AwsError as e:
            if e.code != "InvalidPermission.Duplicate":
                raise
    return sg


def image(region: str) -> tuple[str, str]:
    imgs = aws(
        region, "ec2", "describe-images", "--owners", UBUNTU_OWNER, "--filters",
        f"Name=name,Values={UBUNTU_IMAGE}", "Name=state,Values=available",
        "Name=architecture,Values=x86_64",
    )["Images"]
    if not imgs:
        sys.exit(f"{region}: no image matches {UBUNTU_IMAGE}")
    img = max(imgs, key=lambda i: i["CreationDate"])
    return img["ImageId"], img["RootDeviceName"]


def spot_quota(region: str) -> int | None:
    try:
        q = aws(
            region, "service-quotas", "get-service-quota", "--service-code", "ec2",
            "--quota-code", SPOT_QUOTA,
        )
        return int(q["Quota"]["Value"])
    except AwsError:
        return None


def spot_vcpus_outside(region: str, mine: set[str]) -> int:
    """The spot vCPUs the region's quota counts, but for the instances
    `mine`. Instances shutting down still count."""
    r = aws(
        region, "ec2", "describe-instances", "--filters", "Name=instance-lifecycle,Values=spot",
        "Name=instance-state-name,Values=pending,running,shutting-down",
    )
    total = 0
    for res in r["Reservations"]:
        for i in res["Instances"]:
            if i["InstanceId"] not in mine:
                c = i["CpuOptions"]
                total += c["CoreCount"] * c["ThreadsPerCore"]
    return total


@dataclasses.dataclass(frozen=True)
class Spec:
    vcpus: int
    cores: int
    mem_gib: float
    slots: int


@dataclasses.dataclass(frozen=True)
class Pool:
    type: str
    zone: str
    subnet: str
    spec: Spec
    price: float
    region: str

    @property
    def per_slot(self) -> float:
        return self.price / self.spec.slots

    @property
    def key(self) -> tuple[str, str]:
        return (self.type, self.zone)


@dataclasses.dataclass(eq=False)
class Region:
    """A region the run launches in: what its instances launch with, its
    pools, and its share of its spot quota (None until read)."""

    name: str
    key_name: str
    zones: dict[str, str]  # zone -> subnet
    sg: str
    ami: tuple[str, str]
    pools: list[Pool] = dataclasses.field(default_factory=list)
    budget: int | None = None
    next_budget: float = 0.0


@dataclasses.dataclass(frozen=True)
class Testbed:
    """A testbed run's hosts: one slot each, of exactly `cores` cores."""

    cores: int
    mem_gib: float


def specs(region: str, slot_mem_gib: float, types: list[str] | None = None,
          families: list[str] = (), min_vcpus: int = 0, max_cores: int = 1 << 30,
          testbed: Testbed | None = None) -> dict[str, Spec]:
    if types is not None:
        flt = ["Name=instance-type,Values=" + ",".join(types)]
    else:
        flt = [
            "Name=instance-type,Values=" + ",".join(f"{f}.*" for f in families),
            "Name=processor-info.supported-architecture,Values=x86_64",
            "Name=bare-metal,Values=false",
        ]
    out = {}
    for t in aws(region, "ec2", "describe-instance-types", "--filters", *flt)["InstanceTypes"]:
        v = t["VCpuInfo"]["DefaultVCpus"]
        c = t["VCpuInfo"].get("DefaultCores", v)
        mem = t["MemoryInfo"]["SizeInMiB"] / 1024
        if testbed is not None:
            if types is not None or (c == testbed.cores and mem >= testbed.mem_gib):
                out[t["InstanceType"]] = Spec(v, c, mem, 1)
            continue
        if v < min_vcpus or c > max_cores:
            continue
        slots = min(c, int((mem - SYSTEM_GIB) // slot_mem_gib))
        if slots >= 1:
            out[t["InstanceType"]] = Spec(v, c, mem, slots)
    return out


def pool_order(p: Pool) -> tuple:
    return (p.per_slot, p.spec.vcpus / p.spec.slots, p.type, p.zone)


def cheapest(pools: list[Pool], rooms: dict[str, float], caps: dict[str, float],
             per_pool: collections.Counter, deficit: float, pinned: str | None = None) -> Pool | None:
    """The pool to launch in for `deficit` slots: of those with space in
    their region's room and under their cap (vCPUs; one instance always
    fits the cap), the one whose price per slot it would fill is least, in
    any region, then whose quota's vCPUs per slot: as the queue drains,
    smaller types. A testbed run's first type comes first, so its hosts are
    alike."""
    return min(
        (
            p for p in pools
            if p.spec.vcpus <= rooms[p.region]
            and per_pool[p.key] + p.spec.vcpus <= max(caps[p.region], p.spec.vcpus)
        ),
        key=lambda p: (
            pinned is not None and p.type != pinned,
            p.price / min(p.spec.slots, deficit), p.spec.vcpus / p.spec.slots,
        ),
        default=None,
    )


def plan_fleet(pools: list[Pool], budgets: dict[str, int], max_vcpus: int | None,
               want: float) -> list[Pool]:
    """The instances a run launches for `want` slots, as Run.scale picks
    them, if every pool had capacity."""
    used: collections.Counter = collections.Counter()
    per_pool: collections.Counter = collections.Counter()
    out = []
    while want > 0:
        total = sum(used.values())
        rooms = {
            r: b - used[r] if max_vcpus is None else min(b - used[r], max_vcpus - total)
            for r, b in budgets.items()
        }
        caps = {r: (b if max_vcpus is None else min(b, max_vcpus)) / 2 for r, b in budgets.items()}
        p = cheapest(pools, rooms, caps, per_pool, want)
        if p is None or (out and want < p.spec.slots / 2):
            break
        out.append(p)
        used[p.region] += p.spec.vcpus
        per_pool[p.key] += p.spec.vcpus
        want -= p.spec.slots
    return out


def pools(region: str, slot_mem_gib: float, families: list[str], max_cores: int,
          zones: dict[str, str], testbed: Testbed | None = None) -> list[Pool]:
    if testbed is not None:
        sp = specs(region, 0, families=families, testbed=testbed)
    else:
        sp = specs(region, slot_mem_gib, families=families, min_vcpus=32, max_cores=max_cores)
    if not sp:
        return []
    names = sorted(sp)
    offered = {
        (o["InstanceType"], o["Location"])
        for o in aws(
            region, "ec2", "describe-instance-type-offerings", "--location-type", "availability-zone",
            "--filters", "Name=instance-type,Values=" + ",".join(names),
            "Name=location,Values=" + ",".join(zones),
        )["InstanceTypeOfferings"]
    }
    now = datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S")
    prices: dict[tuple[str, str], float] = {}
    for r in aws(
        region, "ec2", "describe-spot-price-history", "--product-descriptions", "Linux/UNIX",
        "--start-time", now, "--instance-types", *names,
        "--filters", "Name=availability-zone,Values=" + ",".join(zones),
    )["SpotPriceHistory"]:
        prices.setdefault((r["InstanceType"], r["AvailabilityZone"]), float(r["SpotPrice"]))
    out = [
        Pool(t, z, zones[z], sp[t], prices[(t, z)], region)
        for (t, z) in offered if (t, z) in prices
    ]
    out.sort(key=pool_order)
    return out


def launch(run: str, pool: Pool, region: Region, user_data: Path) -> str:
    tags = f"{{Key={TAG},Value={run}}},{{Key=Name,Value=sim-farm-{run}}}"
    r = aws(
        region.name, "ec2", "run-instances", "--count", "1",
        "--image-id", region.ami[0], "--instance-type", pool.type, "--subnet-id", pool.subnet,
        "--security-group-ids", region.sg, "--key-name", region.key_name,
        "--instance-market-options", json.dumps({
            "MarketType": "spot",
            "SpotOptions": {"SpotInstanceType": "one-time", "InstanceInterruptionBehavior": "terminate"},
        }),
        "--instance-initiated-shutdown-behavior", "terminate",
        "--block-device-mappings", json.dumps([{
            "DeviceName": region.ami[1],
            "Ebs": {"VolumeSize": 16, "VolumeType": "gp3", "DeleteOnTermination": True},
        }]),
        "--metadata-options", "HttpTokens=required,HttpEndpoint=enabled",
        "--tag-specifications", f"ResourceType=instance,Tags=[{tags}]", f"ResourceType=volume,Tags=[{tags}]",
        "--user-data", f"file://{user_data}",
        "--client-token", uuid.uuid4().hex,
    )
    return r["Instances"][0]["InstanceId"]


# --- hosts --------------------------------------------------------------


@dataclasses.dataclass(eq=False)
class Host:
    id: str
    type: str
    zone: str
    spec: Spec
    price: float
    launched: float
    # booting -> ready, or adopting -> ready; lost at the end.
    state: str = "booting"
    ip: str | None = None
    boot: cf.Future | None = None
    assigned: dict[str, Item] = dataclasses.field(default_factory=dict)
    acks: list[str] = dataclasses.field(default_factory=list)
    failing_since: float | None = None
    ended: float | None = None
    note: str | None = None  # why its worker waits, as it last said
    region: str = ""
    # Its boot's steps: running with an address, SSH answering, the worker
    # started, and ready (its canary passed, for a new one of test jobs).
    running_at: float | None = None
    ssh_at: float | None = None
    up_at: float | None = None
    ready_at: float | None = None
    busy_s: float = 0.0  # its jobs' walls, summed
    adopted: bool = False

    @property
    def slots(self) -> int:
        return self.spec.slots

    @property
    def hourly(self) -> float:
        return self.price + INSTANCE_EXTRA_HOURLY

    def cost(self, now: float) -> float:
        return self.hourly * max(MIN_BILLED_S, (self.ended or now) - self.launched) / 3600

    def boot_s(self) -> float | None:
        """Launch to ready, for an instance this run launched."""
        return None if self.ready_at is None or self.adopted else self.ready_at - self.launched

    def steps(self) -> str:
        """Its boot's steps, in seconds after its launch."""
        marks = [
            ("running", self.running_at), ("SSH", self.ssh_at),
            ("started", self.up_at), ("ready", self.ready_at),
        ]
        return ", ".join(f"{name} +{t - self.launched:.0f} s" for name, t in marks if t is not None)


class Ssh:
    def __init__(self, key: Path, state: Path) -> None:
        self.key = key
        self.known = state / "known_hosts"
        self.known.mkdir(parents=True, exist_ok=True)

    def opts(self, host: Host) -> list[str]:
        return [
            "-i", str(self.key), "-o", "IdentitiesOnly=yes", "-o", "BatchMode=yes",
            "-o", "StrictHostKeyChecking=accept-new",
            "-o", f"UserKnownHostsFile={self.known / host.id}",
            "-o", "ConnectTimeout=10", "-o", "ServerAliveInterval=15",
            "-o", "ServerAliveCountMax=3", "-o", "LogLevel=ERROR",
        ]

    def run(self, host: Host, cmd: str, input: str | None = None, timeout: float = 120) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["ssh", *self.opts(host), f"ubuntu@{host.ip}", cmd],
            input=input, capture_output=True, text=True, timeout=timeout,
        )

    def put(self, host: Host, files: list[Path], dest: str) -> None:
        p = subprocess.run(
            ["scp", "-q", *self.opts(host), *map(str, files), f"ubuntu@{host.ip}:{dest}"],
            capture_output=True, text=True, timeout=600,
        )
        if p.returncode:
            raise RuntimeError(f"scp: {p.stderr.strip()}")

    def get(self, host: Host, src: str, dest: Path) -> None:
        p = subprocess.run(
            ["scp", "-q", *self.opts(host), f"ubuntu@{host.ip}:{src}", str(dest)],
            capture_output=True, text=True, timeout=600,
        )
        if p.returncode:
            raise RuntimeError(f"scp: {p.stderr.strip()}")


# --- run ----------------------------------------------------------------


class Run:
    def __init__(self, matrices: list[Matrix], args: argparse.Namespace) -> None:
        self.ms = matrices
        self.name = run_name(matrices)
        self.args = args
        self.local = args.local
        # The run's own files; each matrix's rows and log are in its state.
        self.state = FARM / self.name
        self.slot_mem_gib = max(m.slot_mem_gib for m in matrices)
        self.items: dict[str, Item] = {}
        self.rows: dict[str, dict] = {}  # job id -> its first row, in any matrix
        self.have: dict[str, set[str]] = {}  # matrix -> job ids with rows
        counts: collections.Counter = collections.Counter()
        for m in matrices:
            c = collections.Counter()
            for r in read_jsonl(m.state / "rows.jsonl"):
                self.rows.setdefault(r["id"], r)
                c[r["id"]] += 1
            self.have[m.name] = set(c)
            counts |= c
        # job id -> the matrices that have it, each with its own arm and commit
        self.owners: dict[str, list[tuple[Matrix, Job]]] = {}
        self.njobs: dict[str, int] = {}
        self.done: set[str] = set()  # wids with a row, or failed for good
        self.attempts: collections.Counter = collections.Counter()
        self.took: dict[str, set[str]] = collections.defaultdict(set)  # job id -> hosts
        self.queue: collections.deque[Item] = collections.deque()
        # job id -> the items that wait for its row
        self.waiting: dict[str, list[Item]] = {}
        self.derived: dict[str, dict[str, str]] = {}  # job id -> what it took from the row
        self.hosts: list[Host] = []
        self.pool = cf.ThreadPoolExecutor(max_workers=16)
        # The instances' boots, a thread each from launch to ready, which
        # wake the loop when they end.
        self.booter = cf.ThreadPoolExecutor(max_workers=64)
        self.wake = threading.Event()
        self.regions: dict[str, Region] = {}
        self.pools: list[Pool] = []  # every region's, in one ranking
        self.priced = 0.0
        # The regions to look for the run's instances in: its own, and
        # every region the farm has launched in.
        self.sweep: list[str] = []
        # A ^C that came during a launch (held).
        self.holding = False
        self.deferred = False
        self.verified = 0
        self.mismatches = 0
        self.failed = 0
        self.seen: dict[str, list[float]] = collections.defaultdict(list)  # matrix -> walls this run
        self.ran = 0  # rows this run stored, verify copies too
        self.peak_slots = 0
        self.estimated: dict | None = None
        self.boot_prior = past_boot_s("testbed" if matrices[0].kind == "testbed" else "test")
        self.boot_failures = 0
        self.stopping = threading.Event()
        kinds = {m.kind for m in matrices}
        if len(kinds) > 1:
            sys.exit("testbed matrices run apart from test matrices: they want other instances")
        self.testbed: Testbed | None = None
        if kinds == {"testbed"}:
            cores = {m.host_cores for m in matrices}
            if len(cores) > 1:
                sys.exit(f"testbed matrices run together want one host_cores, not {sorted(cores)}")
            self.testbed = Testbed(cores.pop(), max(m.host_mem_gib for m in matrices))
        # A testbed job's place in the queue it went to a host with, and its
        # order there; the type the run's instances take where they can.
        self.seqs: dict[str, int] = {}
        self.order: dict[str, int] = {}
        self.pinned: str | None = None

        def add(item: Item, done: bool) -> None:
            self.items[item.wid] = item
            if done:
                self.done.add(item.wid)
            elif item.job.after:
                # Queued by run(), once the logs are open, if it has its row.
                self.waiting.setdefault(item.job.after, []).append(item)
            else:
                self.queue.append(item)

        # One queue, matrix by matrix; a job two matrices share runs once.
        for m in matrices:
            jobs = m.jobs()
            self.njobs[m.name] = len(jobs)
            for j in jobs:
                own = self.owners.setdefault(j.id, [])
                if any(o is m for o, _ in own):
                    continue
                own.append((m, j))
                if len(own) == 1:
                    add(Item(j), j.id in self.rows)
            # The verify copies: the first jobs by id that haven't two rows yet.
            for j in sorted(jobs, key=lambda j: j.id)[: args.verify if m.kind == "test" else 0]:
                item = Item(self.owners[j.id][0][1], "v")
                if item.wid not in self.items:
                    add(item, counts[j.id] >= 2)
        firsts = [own[0][1] for own in self.owners.values()]
        self.bins = sorted({j.bin for j in firsts} | {name for j in firsts for _, name in j.files})
        # job id -> its expected wall, from past rows. Test jobs run longest
        # first, and before them those no row says, which may be long, so
        # the last to finish are short (a stable sort: otherwise matrix by
        # matrix).
        self.prior: dict[str, float] = {}
        if self.testbed is None:
            self.prior = priors(self.owners)
            self.queue = collections.deque(sorted(
                self.queue, key=lambda i: (i.job.id in self.prior, -self.prior.get(i.job.id, 0))
            ))

    # -- results

    def share_rows(self) -> None:
        """Copies into each matrix the rows another has of its jobs: runs
        are deterministic, so a job runs once for every matrix."""
        n = 0
        for m in self.ms:
            recs = [
                {**self.rows[j.id], "arm": j.arm, "commit": j.commit}
                for jid, own in self.owners.items()
                for o, j in own
                if o is m and jid in self.rows and jid not in self.have[m.name]
            ]
            for r in recs:
                if r.get("kind") == "testbed":
                    self.share_run(r["id"], m)
            append_jsonl(m.state / "rows.jsonl", recs)
            n += len(recs)
        if n:
            log(f"copied {n} rows from one matrix to another")

    def share_run(self, jid: str, m: Matrix) -> None:
        """Copies a testbed job's run dir into m's from a matrix that has it."""
        dest = m.state / "runs" / jid
        src = next((o.state / "runs" / jid for o in self.ms if (o.state / "runs" / jid).is_dir()), None)
        if src is not None and not dest.exists():
            shutil.copytree(src, dest)

    def env_of(self, job: Job) -> dict[str, str]:
        return {**dict(job.env), **self.derived.get(job.id, {})}

    def wire(self, item: Item) -> dict:
        j = item.job
        w = {"id": item.wid, "kind": j.kind, "bin": j.bin, "test": j.test, "env": self.env_of(j)}
        if j.kind == "testbed":
            w["files"] = dict(j.files)
            w["seq"] = self.seqs.get(item.wid, 0)
        return w

    def release(self, jid: str) -> None:
        """Queues the items that wait for job `jid`'s row, at the head."""
        ready = []
        for item in self.waiting.pop(jid, []):
            if item.job.id not in self.derived:
                took, why = derived_env(item.job, self.rows[jid]["row"])
                if why is not None:
                    self.give_up(None, item, {"error": f"waited on {jid}: {why}"})
                    continue
                self.derived[item.job.id] = took
            ready.append(item)
        self.queue.extendleft(reversed(ready))

    def store_testbed(self, host: Host, item: Item, res: dict) -> None:
        """A testbed run's row, its run dir fetched already (fetch)."""
        self.done.add(item.wid)
        jid = item.job.id
        if jid in self.rows:
            log(f"job {jid} ran again on {host.id}; keeping its first run")
            return
        self.ran += 1
        at = datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds")
        for m, j in self.owners[jid]:
            self.share_run(jid, m)
            rec = {
                "id": j.id,
                "kind": "testbed",
                "arm": j.arm,
                "commit": j.commit,
                "env": self.env_of(j),
                "repeat": j.repeat,
                "group": j.group,
                "order": self.order.get(item.wid),
                "host": host.id,
                "type": host.type,
                "zone": host.zone,
                **{k: res.get(k) for k in ("cpu_model", "kernel", "docker", "started", "wall_s", "exit")},
                "at": at,
                "run_dir": f"runs/{jid}",
                "row": res["row"],
            }
            append_jsonl(m.state / "rows.jsonl", [rec])
            self.rows.setdefault(jid, rec)
        if res.get("wall_s"):
            self.seen[self.owners[jid][0][0].name].append(res["wall_s"])
        self.release(jid)

    def store(self, host: Host, item: Item, res: dict) -> None:
        if item.job.kind == "testbed":
            self.store_testbed(host, item, res)
            return
        at = datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds")
        recs = []
        for m, j in self.owners[item.job.id]:
            rec = {
                "id": j.id,
                "arm": j.arm,
                "commit": j.commit,
                "test": j.test,
                "env": self.env_of(j),
                "copy": item.copy,
                "host": host.id,
                "type": host.type,
                "zone": host.zone,
                "wall_s": res.get("wall_s"),
                "cpu_s": res.get("cpu_s"),
                "max_rss_mb": res.get("max_rss_mb"),
                "at": at,
                "row": res["row"],
            }
            append_jsonl(m.state / "rows.jsonl", [rec])
            recs.append((m, rec))
        self.done.add(item.wid)
        self.ran += 1
        if res.get("wall_s"):
            self.seen[self.owners[item.job.id][0][0].name].append(res["wall_s"])
        rec = recs[0][1]
        first = self.rows.setdefault(item.job.id, rec)
        self.release(item.job.id)
        if first is not rec:
            key = lambda r: tuple(r["row"].get(k) for k in ("trace", "frames", "bytes"))
            if key(first) == key(rec):
                self.verified += 1
            else:
                self.mismatches += 1
                for m, r in recs:
                    append_jsonl(m.state / "mismatches.jsonl", [{"first": first, "again": r}])
                log(f"MISMATCH: job {item.job.id} ({item.job.arm} {dict(item.job.env)}): "
                    f"{key(first)} on {first['host']}, {key(rec)} on {host.id}")

    def fail(self, host: Host, item: Item, res: dict) -> None:
        self.attempts[item.wid] += 1
        tail = (res.get("log_tail") or "").splitlines()[-3:]
        log(f"job {item.wid} failed on {host.id}: {res.get('error')}" + "".join(f"\n    {t}" for t in tail))
        if self.attempts[item.wid] < 2:
            self.queue.appendleft(item)
        else:
            self.give_up(host.id, item, res)

    def give_up(self, host: str | None, item: Item, res: dict) -> None:
        """Records the item as failed for good, and so the items that wait
        on its job if that job is done without a row."""
        self.done.add(item.wid)
        self.failed += 1
        for m, j in self.owners[item.job.id]:
            append_jsonl(m.state / "failures.jsonl", [{
                "id": j.id, "arm": j.arm, "env": self.env_of(j), "host": host, **res,
            }])
        jid = item.job.id
        if jid in self.waiting and jid not in self.rows and all(
            i.wid in self.done for i in self.items.values() if i.job.id == jid and i.copy != "canary"
        ):
            for w in self.waiting.pop(jid):
                log(f"job {w.wid} waited on {jid}, which failed")
                self.give_up(None, w, {"error": f"waited on {jid}, which failed"})

    def requeue(self, host: Host) -> None:
        back = [i for i in host.assigned.values() if i.wid not in self.done and i.copy != "canary"]
        self.queue.extendleft(reversed(back))
        host.assigned.clear()

    def lose(self, host: Host, why: str) -> None:
        log(f"lost {host.id} ({host.type} in {host.zone}): {why}; {len(host.assigned)} jobs requeued")
        self.requeue(host)
        host.state = "lost"
        host.ended = time.time()
        if not self.local:
            try:
                terminate(host.region, [host.id])
            except AwsError as e:
                log(f"terminating {host.id}: {e}")

    # -- dispatch

    def take(self, host: Host) -> list[Item]:
        if host.state != "ready":
            return []
        others = sum(1 for h in self.hosts if h is not host and h.state != "lost")
        if self.testbed is not None:
            return self.take_group(host, others)
        # Its free slots, then jobs queued behind the running ones while
        # they're short (no longer than PREFETCH_S), which a sync's worth
        # couldn't keep busy: up to PREFETCH_S of work per slot, and no more
        # work than its slots' share of what's left. A long job queued
        # behind others would start a job's length late and could end the
        # run late, so one at the head stops it.
        ready = sum(h.slots for h in self.hosts if h.state == "ready")
        goal = min(host.slots * PREFETCH_S, self.work_left() * host.slots / max(ready, 1))
        have = sum(self.expected(i) for i in host.assigned.values() if i.copy != "canary")
        out, skipped = [], []
        while self.queue and (len(host.assigned) < host.slots or have < goal):
            item = self.queue.popleft()
            if item.wid in self.done or item.wid in host.assigned:
                continue
            if host.id in self.took[item.job.id] and others:
                skipped.append(item)
                continue
            if len(host.assigned) >= host.slots and self.expected(item) > PREFETCH_S:
                self.queue.appendleft(item)
                break
            out.append(item)
            host.assigned[item.wid] = item
            self.took[item.job.id].add(host.id)
            have += self.expected(item)
        self.queue.extendleft(reversed(skipped))
        return out

    def take_group(self, host: Host, others: int) -> list[Item]:
        """A testbed host's next group, whole, once it has nothing left: the
        first queued group none of whose jobs it took, while there are other
        hosts to take those."""
        if host.assigned:
            return []
        self.queue = collections.deque(i for i in self.queue if i.wid not in self.done)
        for g in dict.fromkeys(i.job.group for i in self.queue):
            items = [i for i in self.queue if i.job.group == g]
            if others and any(host.id in self.took[i.job.id] for i in items):
                continue
            self.queue = collections.deque(i for i in self.queue if i.job.group != g)
            for k, item in enumerate(items):
                host.assigned[item.wid] = item
                self.took[item.job.id].add(host.id)
                self.order[item.wid] = k
                self.seqs[item.wid] = len(self.seqs) + 1
            return items
        return []

    def exchange(self, host: Host, jobs: list[Item], acks: list[str]) -> dict:
        """A sync, then the fetch of the testbed runs it reports done: one
        that didn't come is held back from absorb until a later sync."""
        reply = self.sync(host, jobs, acks)
        if self.testbed is not None:
            done = []
            for res in reply["done"]:
                if self.fetch(host, res):
                    done.append(res)
                else:
                    reply.setdefault("held", []).append(res["id"])
            reply["done"] = done
        return reply

    def fetch(self, host: Host, res: dict) -> bool:
        """Brings a finished testbed run's out dir here, as the first owner's
        runs/<id>, extracted as this user."""
        item = self.items.get(res["id"])
        if res.get("row") is None or item is None:
            return True
        jid = item.job.id
        dest = self.owners[jid][0][0].state / "runs" / jid
        if dest.exists():
            return True
        dest.parent.mkdir(parents=True, exist_ok=True)
        tmp = dest.with_name(f".{jid}.tmp")
        tar = dest.with_name(f".{jid}.tar.gz")
        try:
            if self.local:
                shutil.copyfile(self.local_dir / "done" / f"{res['id']}.tar.gz", tar)
            else:
                self.ssh.get(host, f"farm/done/{res['id']}.tar.gz", tar)
            shutil.rmtree(tmp, ignore_errors=True)
            with tarfile.open(tar) as t:
                t.extractall(tmp, filter="data")
            os.replace(tmp / "out", dest)
            return True
        except Exception as e:
            log(f"fetching job {jid}'s run from {host.id}: {e}")
            return False
        finally:
            tar.unlink(missing_ok=True)
            shutil.rmtree(tmp, ignore_errors=True)

    def sync(self, host: Host, jobs: list[Item], acks: list[str]) -> dict:
        req = json.dumps({"ack": acks, "jobs": [self.wire(i) for i in jobs]})
        if self.local:
            p = subprocess.run(
                [sys.executable, str(HERE / "worker.py"), "sync", "--dir", str(self.local_dir)],
                input=req, capture_output=True, text=True, timeout=60,
            )
        else:
            p = self.ssh.run(host, "python3 farm/worker.py sync", input=req, timeout=60)
        if p.returncode:
            raise RuntimeError(p.stderr.strip() or f"exit {p.returncode}")
        return json.loads(p.stdout)

    def absorb(self, host: Host, reply: dict) -> None:
        for res in reply["done"]:
            wid = res["id"]
            host.acks.append(wid)
            item = host.assigned.pop(wid, None)
            if item is None:
                item = self.items.get(wid)
                if item is None or wid in self.done:
                    continue
            if item.copy == "canary":
                continue  # the boot's, whose ack didn't arrive
            host.busy_s += res.get("wall_s") or 0
            if res.get("row") is not None:
                self.store(host, item, res)
            else:
                self.fail(host, item, res)
        there = set(reply["queued"]) | set(reply["running"]) | set(reply.get("held", []))
        note = reply.get("note")
        if note != host.note:
            if note:
                log(f"{host.id}: {note}")
            host.note = note
        for wid in there - set(host.assigned):
            item = self.items.get(wid)
            if item is not None and wid not in self.done:
                host.assigned[wid] = item
                self.took[item.job.id].add(host.id)
                try:
                    self.queue.remove(item)
                except ValueError:
                    pass
                waits = self.waiting.get(item.job.after, [])
                if item in waits:
                    waits.remove(item)
                    if not waits:
                        del self.waiting[item.job.after]
        gone = [host.assigned.pop(w) for w in list(host.assigned) if w not in there]
        back = [i for i in gone if i.copy != "canary" and i.wid not in self.done]
        if back:
            log(f"{host.id} lost track of {len(back)} jobs; requeued")
            self.queue.extendleft(reversed(back))
        if reply.get("interrupting"):
            # A pool AWS takes instances back from is one to stay out of.
            if not self.local:
                self.backoff[(host.type, host.zone)] = time.time() + POOL_BACKOFF
            self.lose(host, f"spot interruption notice; skipping {host.type} in {host.zone} "
                      f"for {POOL_BACKOFF // 60} min")
        elif not reply.get("serving") and host.assigned:
            log(f"{host.id}: the worker isn't running; restarting it")
            if self.local:
                self.start_local()
            else:
                self.pool.submit(self.start_worker, host)

    # -- the canary

    def local_canary(self) -> str:
        job = self.ms[0].canary_job()
        path = self.state / "canary.json"
        if path.exists():
            saved = json.loads(path.read_text())
            if saved.get("id") == job.id:
                return saved["trace"]
        log(f"running the canary here ({job.arm}, {dict(job.env)})")
        work = self.state / "canary"
        shutil.rmtree(work, ignore_errors=True)
        work.mkdir(parents=True)
        p = subprocess.run(
            [str(BIN / job.bin), "--exact", job.test, "--include-ignored", "--nocapture", "--test-threads", "1"],
            cwd=work, env={**os.environ, **dict(job.env)}, capture_output=True, text=True,
        )
        rows = [l[l.index("ROW {") + 4 :] for l in p.stdout.splitlines() if "ROW {" in l]
        if p.returncode or len(rows) != 1:
            sys.exit(f"the canary failed here:\n{p.stdout[-2000:]}\n{p.stderr[-2000:]}")
        trace = json.loads(rows[0])["trace"]
        path.write_text(json.dumps({"id": job.id, "trace": trace}))
        shutil.rmtree(work, ignore_errors=True)
        return trace

    def check_canary(self, host: Host, res: dict) -> None:
        trace = (res.get("row") or {}).get("trace")
        if trace == self.canary_trace:
            host.state = "ready"
            log(f"{host.id} ({host.type} in {host.zone}) passed the canary; {host.slots} slots; "
                f"{host.steps()}")
            return
        why = res.get("error") or f"trace {trace}, here {self.canary_trace}"
        log(f"CANARY FAILED on {host.id} ({host.type}): {why}\n{res.get('log_tail', '')}")
        raise SystemExit("the binaries don't reproduce on the instances; stopping")

    # -- instances

    def start_worker(self, host: Host) -> None:
        cmd = (
            "sudo systemctl reset-failed sim-farm-worker 2>/dev/null; "
            "systemctl is-active --quiet sim-farm-worker || "
            "sudo systemd-run --unit sim-farm-worker --uid ubuntu --gid ubuntu "
            + ("-p SupplementaryGroups=docker " if self.testbed is not None else "")
            + "--working-directory /home/ubuntu/farm "
            f"python3 /home/ubuntu/farm/worker.py serve --slots {host.slots} "
            f"--job-timeout {self.args.job_timeout} --idle-minutes {self.args.idle_minutes} "
            "--imds --poweroff"
        )
        p = self.ssh.run(host, cmd)
        if p.returncode:
            raise RuntimeError(f"starting the worker: {p.stderr.strip()}")

    def start_boot(self, host: Host) -> None:
        host.boot = self.booter.submit(self.bootstrap, host)
        host.boot.add_done_callback(lambda _: self.wake.set())

    def booting(self, host: Host) -> None:
        if self.stopping.is_set():
            raise RuntimeError("stopping")
        if host.state == "lost":
            raise RuntimeError("lost")

    def bootstrap(self, host: Host) -> dict | None:
        """An instance's boot, on a thread of its own from its launch: its
        address once it's running, SSH, the worker and the binaries, the
        worker started, and for a new instance of test jobs the canary, whose
        result it returns (check_canary). None of it waits for a tick."""
        start = time.time()
        while host.ip is None:
            self.booting(host)
            try:
                got = aws(host.region, "ec2", "describe-instances", "--instance-ids", host.id)
                inst = got["Reservations"][0]["Instances"][0]
                st = inst["State"]["Name"]
                if st not in ("pending", "running"):
                    raise RuntimeError(f"it's {st}")
                if st == "running" and inst.get("PublicIpAddress"):
                    host.ip, host.running_at = inst["PublicIpAddress"], time.time()
                    break
            except AwsError as e:
                if e.code != "InvalidInstanceID.NotFound":  # not listed yet
                    raise
            if time.time() - start > 600:
                raise RuntimeError("not running after 10 minutes")
            time.sleep(2)
        denied = None
        while True:
            self.booting(host)
            try:
                # The boot leaves farm/bin last, or a failed setup's log.
                p = self.ssh.run(
                    host, "if [ -e farm/host-failed ]; then tail -n 20 farm/host-failed; exit 3; fi; "
                    "test -d farm/bin", timeout=30,
                )
                if p.returncode == 0:
                    break
                if p.returncode == 3:
                    raise RuntimeError("its setup failed:\n" + p.stdout.rstrip())
                why = p.stderr.strip()
            except subprocess.TimeoutExpired:
                why = "timeout"
            # Refused keys outlast the boot: a passphrase without an agent.
            if "Permission denied" in why:
                denied = denied or time.time()
                if time.time() - denied > 120:
                    raise SystemExit(
                        f"SSH refuses {self.ssh.key}: {why}\n"
                        "a key with a passphrase needs ssh-agent (eval $(ssh-agent); ssh-add KEY)"
                    )
            else:
                denied = None
            if time.time() - start > 600:
                raise RuntimeError(f"no SSH after 10 minutes: {why}")
            time.sleep(2)
        host.ssh_at = time.time()
        self.ssh.put(host, [HERE / "worker.py"], "farm/")
        self.ssh.put(host, [BIN / b for b in self.bins], "farm/bin/")
        self.start_worker(host)
        host.up_at = time.time()
        res = None
        if not host.adopted and self.testbed is None:
            res = self.run_canary(host)
        host.ready_at = time.time()
        return res

    def run_canary(self, host: Host) -> dict:
        """The canary's result from the instance, polled a second apart (a
        sync that fails is tried again), and acked."""
        wid = self.canary_item.wid
        jobs = [self.canary_item]
        deadline = time.time() + self.args.job_timeout * 60 + 120
        while time.time() < deadline:
            self.booting(host)
            try:
                reply = self.sync(host, jobs, [])
            except Exception:
                time.sleep(1)
                continue
            jobs = []
            res = next((r for r in reply["done"] if r["id"] == wid), None)
            if res is not None:
                with contextlib.suppress(Exception):
                    self.sync(host, [], [wid])
                return res
            time.sleep(1)
        raise RuntimeError("the canary didn't finish")

    def refresh(self, now: float) -> None:
        """Each live host's state, from its region, for the ones that are
        gone (the boot finds the address); a region that can't be listed
        this time leaves its hosts as they were."""
        regions = sorted({h.region for h in self.hosts if h.state != "lost"})
        by_id, unlisted = {}, set()
        for r, got in in_regions(lambda r: farm_instances(r, self.name), regions).items():
            if isinstance(got, BaseException):
                log(f"{r}: listing the run's instances: {got}")
                unlisted.add(r)
            else:
                by_id.update((i["InstanceId"], i) for i in got)
        for h in self.hosts:
            if h.state == "lost" or h.region in unlisted:
                continue
            inst = by_id.get(h.id)
            if inst is None and now - h.launched < 120:
                continue  # not listed yet
            st = inst["State"]["Name"] if inst else "gone"
            if st not in ("pending", "running"):
                reason = (inst or {}).get("StateReason", {}).get("Message", st)
                self.lose(h, reason)

    def boots(self) -> None:
        """The instances whose boot has ended: ready (a new one of test jobs
        once its canary's trace is the one here), or lost."""
        for h in self.hosts:
            if h.state not in ("booting", "adopting") or h.boot is None or not h.boot.done():
                continue
            try:
                res = h.boot.result()
            except Exception as e:
                self.lose(h, f"bootstrap failed: {e}")
                self.boot_failures += 1
                if self.boot_failures >= 3:
                    raise SystemExit("instances keep failing to bootstrap; stopping")
                continue
            if res is not None:
                self.check_canary(h, res)
                continue
            h.state = "ready"
            log(f"{h.id} ({h.type} in {h.zone}) is ready; {h.steps()}")

    def read_budgets(self, now: float) -> None:
        """The budgets of the regions whose reading is due: each region's
        spot quota less what else counts against it there (or, if the quota
        can't be read, --max-region-vcpus or --max-vcpus), at most
        --max-region-vcpus."""
        due = [r for r in self.regions.values() if now >= r.next_budget]
        if not due:
            return
        mine = {h.id for h in self.hosts if h.state != "lost"}
        got = in_regions(lambda r: (spot_quota(r), spot_vcpus_outside(r, mine)), [r.name for r in due])
        cap = self.args.max_region_vcpus
        for r in due:
            r.next_budget = now + BUDGET_REFRESH
            res = got[r.name]
            if isinstance(res, BaseException):
                if r.budget is None:
                    sys.exit(f"{r.name}: reading the spot budget: {res}")
                log(f"{r.name}: reading the spot budget: {res}; it stays {r.budget} vCPUs")
                continue
            quota, outside = res
            if quota is not None:
                budget = quota - outside
            elif cap is not None or self.args.max_vcpus is not None:
                budget = cap if cap is not None else self.args.max_vcpus
            elif r.budget is None:
                sys.exit(f"{r.name}: can't read the spot vCPU quota; pass --max-vcpus or --max-region-vcpus")
            else:
                continue
            if cap is not None:
                budget = min(budget, cap)
            if budget != r.budget:
                log(f"{r.name}: budget {budget} vCPUs (quota {quota}, {outside} in other spot instances)")
            r.budget = budget

    def live_vcpus(self, region: str | None = None) -> int:
        return sum(
            h.spec.vcpus for h in self.hosts
            if h.state != "lost" and (region is None or h.region == region)
        )

    def room(self, r: Region) -> int:
        """The vCPUs the run can still launch in the region: what its budget
        leaves there, and --max-vcpus over every region."""
        room = r.budget - self.live_vcpus(r.name)
        if self.args.max_vcpus is not None:
            room = min(room, self.args.max_vcpus - self.live_vcpus())
        return room

    def pool_cap(self, r: Region) -> float:
        """The vCPUs a pool of the region may take: half of what the run may
        have there."""
        if self.args.max_vcpus is None:
            return r.budget / 2
        return min(r.budget, self.args.max_vcpus) / 2

    def pick(self, deficit: float, now: float) -> Pool | None:
        per_pool = collections.Counter()
        for h in self.hosts:
            if h.state != "lost":
                per_pool[(h.type, h.zone)] += h.spec.vcpus
        return cheapest(
            [p for p in self.pools if self.backoff.get(p.key, 0) <= now],
            {name: self.room(r) for name, r in self.regions.items()},
            {name: self.pool_cap(r) for name, r in self.regions.items()},
            per_pool, deficit, self.pinned,
        )

    # -- sizing

    def expected(self, item: Item) -> float:
        """What an item is expected to take, in seconds of its slot: its
        prior, else the mean of what its matrix's jobs took this run, else
        DEFAULT_JOB_S."""
        e = self.prior.get(item.job.id)
        if e is not None:
            return e
        seen = self.seen.get(self.owners[item.job.id][0][0].name)
        return statistics.fmean(seen) if seen else DEFAULT_JOB_S

    def work_left(self) -> float:
        """The work left, in slot-seconds: the queued and waiting jobs'
        expected walls, and half theirs for the jobs out."""
        w = sum(self.expected(i) for i in self.queue if i.wid not in self.done)
        w += sum(self.expected(i) for items in self.waiting.values() for i in items)
        w += sum(
            self.expected(i) / 2
            for h in self.hosts if h.state != "lost"
            for i in h.assigned.values() if i.copy != "canary"
        )
        return w

    def boot_s(self) -> float:
        """An instance's boot, launch to ready: the mean of this run's, else
        of the last runs'."""
        got = [b for h in self.hosts if (b := h.boot_s()) is not None]
        return statistics.fmean(got) if got else self.boot_prior

    def wanted(self, now: float) -> float:
        """The slots worth launching: with B the boot and k --work-per-boot,
        the work the live instances won't have done (1 + k)B from now, over
        the kB each new slot would then have done. A booting instance's
        slots count from when it should be ready."""
        k, b = self.args.work_per_boot, self.boot_s()
        d = (1 + k) * b
        can = 0.0
        for h in self.hosts:
            if h.state == "ready":
                can += h.slots * d
            elif h.state != "lost":
                can += h.slots * max(0.0, d - max(0.0, h.launched + b - now))
        return (self.work_left() - can) / (k * b)

    def estimate(self) -> dict:
        """What the run should take, before it launches: its work, the
        instances Run.scale would launch for it if every pool had capacity,
        and the model's wall time, a boot then the work over their slots (or
        the longest job, if longer) and two syncs, and their cost for that
        long; then both times what the last runs took over their model's
        (past_ratios)."""
        work = self.work_left()
        items = [i for i in self.queue if i.wid not in self.done]
        items += [i for w in self.waiting.values() for i in w]
        k, b = self.args.work_per_boot, self.boot_s()
        budgets = {n: r.budget - self.live_vcpus(n) for n, r in self.regions.items()}
        fleet = plan_fleet(self.pools, budgets, self.args.max_vcpus, work / (k * b))
        slots = sum(p.spec.slots for p in fleet)
        longest = max((self.expected(i) for i in items), default=0.0)
        wall = b + max(work / slots, longest) + 2 * TICK if slots else None
        cost = sum(p.price + INSTANCE_EXTRA_HOURLY for p in fleet) * wall / 3600 if slots else None
        wall_x, cost_x, runs = past_ratios()
        by_region = collections.Counter()
        for p in fleet:
            by_region[p.region] += p.spec.slots
        return {
            "jobs": len(items),
            "timed": sum(1 for i in items if i.job.id in self.prior),
            "work_slot_h": round(work / 3600, 2),
            "boot_s": round(b),
            "instances": len(fleet),
            "slots": slots,
            "slots_by_region": dict(by_region),
            "model_wall_s": round(wall) if wall else None,
            "model_cost": round(cost, 3) if wall else None,
            "past_runs": runs,
            "wall_s": round(wall * wall_x) if wall else None,
            "cost": round(cost * cost_x, 2) if wall else None,
        }

    def say_estimate(self, est: dict) -> str:
        timed = f"{est['timed'] / est['jobs']:.0%}" if est["jobs"] else "-"
        head = (
            f"{est['jobs']} jobs, {est['work_slot_h']:.1f} slot-h of work ({timed} of the jobs timed by "
            f"earlier rows), a boot of {est['boot_s']} s, --work-per-boot {self.args.work_per_boot:g}: "
        )
        if not est["slots"]:
            return head + "no pool fits the budget"
        where = ", ".join(f"{n} {r}" for r, n in sorted(est["slots_by_region"].items()))
        out = (
            f"{est['instances']} instances with {est['slots']} slots ({where}), about "
            f"{est['wall_s'] / 60:.1f} min and ${est['cost']:.2f}"
        )
        if est["past_runs"]:
            out += (
                f" (the model's {est['model_wall_s'] / 60:.1f} min and ${est['model_cost']:.2f}, scaled as "
                f"the last {est['past_runs']} runs took over theirs)"
            )
        return head + out

    def account(self, now: float, interrupted: bool) -> None:
        """The run's cost, split into boots (launch to ready), jobs (their
        walls) and idle slots, in the log and as a line of costs.jsonl."""
        if self.local or not self.hosts:
            return
        mine = [h for h in self.hosts if not h.adopted]
        start = min((h.launched for h in mine), default=min(h.launched for h in self.hosts))
        cost = sum(h.cost(now) for h in self.hosts)
        boot = sum(h.hourly * (min(h.ready_at or now, h.ended or now) - h.launched) / 3600 for h in mine)
        busy = sum(h.hourly / h.slots * h.busy_s / 3600 for h in self.hosts)
        idle = max(0.0, cost - boot - busy)
        boots = collections.defaultdict(list)
        for h in mine:
            if (b := h.boot_s()) is not None:
                boots[h.region].append(b)
        allboots = [b for v in boots.values() for b in v]
        rec = {
            "run": self.name,
            "kind": "testbed" if self.testbed is not None else "test",
            "at": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
            "regions": sorted(self.regions),
            "interrupted": interrupted,
            "jobs": self.ran,
            "wall_s": round(now - start),
            "instances": len(self.hosts),
            "peak_slots": self.peak_slots,
            "cost": round(cost, 4),
            "boot_cost": round(boot, 4),
            "busy_cost": round(busy, 4),
            "idle_cost": round(idle, 4),
            "busy_slot_h": round(sum(h.busy_s for h in self.hosts) / 3600, 2),
            "boot_s": round(statistics.fmean(allboots)) if allboots else None,
            "boot_s_by_region": {r: round(statistics.fmean(v)) for r, v in sorted(boots.items())},
            "work_per_boot": self.args.work_per_boot,
            "estimate": self.estimated,
        }
        append_jsonl(COSTS, [rec])
        msg = (
            f"{self.ran} jobs in {rec['wall_s'] / 60:.1f} min on {len(self.hosts)} instances "
            f"({self.peak_slots} slots at most): ${cost:.2f}"
        )
        if self.ran:
            msg += f", ${cost / self.ran * 1000:.2f} per 1,000 jobs"
        msg += f"; boots ${boot:.2f}, jobs ${busy:.2f} ({rec['busy_slot_h']:.1f} slot-h), idle ${idle:.2f}"
        if boots:
            msg += "; boot " + ", ".join(f"{r} {s} s" for r, s in rec["boot_s_by_region"].items())
        est = self.estimated
        if est and est.get("wall_s"):
            msg += f"; estimated {est['wall_s'] / 60:.1f} min, ${est['cost']:.2f}"
        log(msg)

    @contextlib.contextmanager
    def held(self):
        """Holds a ^C (or SIGTERM, SIGHUP) back until the block is done: an
        instance being launched is tracked before the run stops, and so
        terminated."""
        self.holding = True
        try:
            yield
        finally:
            self.holding = False
            if self.deferred:
                self.deferred = False
                log("a signal came during a launch, which has answered; stopping")
                raise KeyboardInterrupt

    def scale(self, now: float) -> None:
        live = [h for h in self.hosts if h.state != "lost"]
        if self.testbed is not None:
            # A host per group, and a host with nothing assigned takes one.
            groups = {i.job.group for i in self.queue if i.wid not in self.done}
            deficit = len(groups) - sum(1 for h in live if not h.assigned)
        else:
            # A slot per job at most, and only the slots whose work pays for
            # their boot (wanted).
            waiting = sum(len(w) for w in self.waiting.values())
            demand = len(self.queue) + waiting + sum(len(h.assigned) for h in live)
            deficit = min(demand - sum(h.slots for h in live), self.wanted(now))
        if deficit > 0:
            self.read_budgets(now)
        launched = 0
        while deficit > 0 and launched < 3:
            pool = self.pick(deficit, now)
            if pool is None:
                if not live and not launched and now - self.last_stuck > 60:
                    self.last_stuck = now
                    log("no pool fits the budget or has capacity; waiting")
                return
            # An instance of twice the slots wanted or more waits, unless the
            # run has none.
            if self.testbed is None and deficit < pool.spec.slots / 2 and (live or launched):
                return
            r = self.regions[pool.region]
            record_region(r.name)
            try:
                with self.held():
                    hid = launch(self.name, pool, r, self.user_data)
                    self.hosts.append(
                        Host(hid, pool.type, pool.zone, pool.spec, pool.price, now, region=r.name)
                    )
                    self.start_boot(self.hosts[-1])
                    log(f"launched {hid}: {pool.type} in {pool.zone}, {pool.spec.slots} slots, "
                        f"${pool.price:.3f}/h")
            except AwsError as e:
                if e.code in CAPACITY_ERRORS:
                    log(f"{pool.type} in {pool.zone}: {e.code}; skipping it for {POOL_BACKOFF // 60} min")
                    self.backoff[pool.key] = now + POOL_BACKOFF
                    continue
                if e.code in QUOTA_ERRORS:
                    # The region takes no more until its quota is read again:
                    # what counts against it can lag what's running.
                    r.budget = self.live_vcpus(r.name)
                    r.next_budget = now + BUDGET_REFRESH
                    log(f"{r.name}: {e.code}: its budget is {r.budget} vCPUs for {BUDGET_REFRESH // 60} min")
                    continue
                if e.code == "AuthFailure.ServiceLinkedRoleCreationNotPermitted":
                    raise SystemExit(
                        "the account's first spot instance needs the Spot service-linked role; an "
                        "admin creates it once: aws iam create-service-linked-role "
                        "--aws-service-name spot.amazonaws.com"
                    )
                raise
            if self.testbed is not None and self.pinned is None:
                self.pinned = pool.type
            deficit -= pool.spec.slots
            launched += 1

    def retire_idle(self) -> None:
        if self.queue or self.waiting:
            return
        for h in self.hosts:
            if h.state == "ready" and not h.assigned:
                log(f"terminating {h.id}: nothing left for it")
                h.state = "lost"
                h.ended = time.time()
                try:
                    terminate(h.region, [h.id])
                except AwsError as e:
                    log(f"terminating {h.id}: {e}")

    def terminate_all(self) -> None:
        """Terminates the run's instances: those it tracks, each in its
        region, then any its tag finds in a region it may have used (one
        whose launch never answered)."""
        live = [h for h in self.hosts if h.state != "lost"]
        if live:
            log(f"terminating {len(live)} instances")
        by_region = collections.defaultdict(list)
        for h in live:
            by_region[h.region].append(h.id)
        gone = set()
        for r, ids in sorted(by_region.items()):
            try:
                terminate(r, ids)
                gone.update(ids)
            except AwsError as e:
                log(f"{r}: terminating {', '.join(ids)}: {e}")
        for h in live:
            h.state, h.ended = "lost", time.time()
        # What the tag finds still up is terminated (again): a terminate
        # that failed, one the listing doesn't show yet, or an instance the
        # run never tracked, which is the one worth a word.
        known = {h.id for h in self.hosts}
        for r, got in in_regions(lambda r: farm_instances(r, self.name), self.sweep).items():
            if isinstance(got, BaseException):
                log(f"{r}: CAN'T LIST the run's instances ({got}); "
                    f"`farm.py down {self.name}` terminates any left")
                continue
            left = [
                i["InstanceId"] for i in got
                if i["State"]["Name"] != "shutting-down" and i["InstanceId"] not in gone
            ]
            untracked = [i for i in left if i not in known]
            if untracked:
                log(f"{r}: terminating {len(untracked)} instances of the run it didn't track: "
                    f"{', '.join(untracked)}")
            try:
                terminate(r, left)
            except AwsError as e:
                log(f"{r}: terminating {', '.join(left)}: {e}")

    # -- the loop

    def setup_aws(self) -> None:
        key = Path(self.args.ssh_key).expanduser()
        if not key.exists() or not Path(f"{key}.pub").exists():
            sys.exit(f"{key} or its .pub is missing (--ssh-key, $STRIMS_FARM_SSH_KEY, ssh_key in {CONFIG})")
        self.ssh = Ssh(key, self.state)
        names = self.args.regions
        self.sweep = sorted(set(names) | recorded_regions())
        cidr = public_cidr()

        def setup(name: str) -> Region:
            vpc, zones = network(name)
            return Region(name, ensure_key(name, key), zones, ensure_sg(name, vpc, cidr), image(name))

        self.regions = dict(in_regions_or_exit(setup, names))
        self.families = self.args.families.split(",")
        self.find_pools()
        if not self.pools:
            sys.exit("no instance type fits (families, sizes, slot_mem_gib or host_cores, regions)")
        self.backoff: dict[tuple[str, str], float] = {}
        self.last_stuck = 0.0
        minutes = int(self.args.hours * 60)
        self.user_data = self.state / "user-data.sh"
        self.user_data.write_text(
            (HERE / "user-data.sh").read_text().replace("@MINUTES@", str(minutes))
            .replace("@DOCKER@", "1" if self.testbed is not None else "0")
        )
        if self.testbed is None:
            canary = self.ms[0].canary_job()
            self.canary_item = Item(canary, "canary")
            self.items[self.canary_item.wid] = self.canary_item
            self.canary_trace = self.local_canary()
        # Instances a previous controller left running, in any region.
        def adoptable(r: str) -> list[tuple[dict, Spec | None]]:
            found = [i for i in farm_instances(r, self.name) if i["State"]["Name"] in ("pending", "running")]
            if not found:
                return []
            types = sorted({i["InstanceType"] for i in found})
            sp = specs(r, self.slot_mem_gib, types=types, testbed=self.testbed)
            return [(i, sp.get(i["InstanceType"])) for i in found]

        for r, found in in_regions_or_exit(adoptable, self.sweep).items():
            for i, spec in found:
                if spec is None:
                    continue
                zone = i["Placement"]["AvailabilityZone"]
                price = next((p.price for p in self.pools if p.key == (i["InstanceType"], zone)), 0.0)
                launched = datetime.datetime.fromisoformat(i["LaunchTime"]).timestamp()
                h = Host(
                    i["InstanceId"], i["InstanceType"], zone, spec, price, launched,
                    state="adopting", region=r, adopted=True,
                )
                self.hosts.append(h)
                self.start_boot(h)
                if self.testbed is not None and self.pinned is None:
                    self.pinned = h.type
                log(f"adopting {h.id} ({h.type} in {h.zone})")
        self.read_budgets(time.time())
        log("best pools: " + ", ".join(
            f"{p.type}@{self.where(p.zone)} {p.spec.slots} slots ${p.per_slot * 1000:.1f}m/slot-h"
            for p in self.pools[:5]
        ))
        if self.testbed is None:
            self.estimated = self.estimate()
            log("estimate: " + self.say_estimate(self.estimated))

    def estimate_only(self) -> int:
        """`estimate`: the run's estimate, with nothing changed in AWS (no
        key pair, security group or launch)."""
        if self.testbed is not None:
            sys.exit("estimate takes test matrices")

        def setup(name: str) -> Region:
            _, zones = network(name)
            return Region(name, "", zones, "", ("", ""))

        self.regions = dict(in_regions_or_exit(setup, self.args.regions))
        self.families = self.args.families.split(",")
        self.find_pools()
        if not self.pools:
            sys.exit("no instance type fits (families, sizes, slot_mem_gib, regions)")
        self.read_budgets(time.time())
        print(self.say_estimate(self.estimate()))
        return 0

    def where(self, zone: str) -> str:
        """A zone as the log shows it: its letter, or all of it when the
        run spans regions."""
        return zone if len(self.regions) > 1 else zone[-1]

    def find_pools(self) -> None:
        """Every region's pools, in one ranking; a region whose prices
        can't be read again keeps the pools it had."""
        def find(r: str) -> list[Pool]:
            return pools(
                r, self.slot_mem_gib, self.families, self.args.max_instance_cores,
                self.regions[r].zones, self.testbed,
            )

        for r, got in in_regions(find, list(self.regions)).items():
            if isinstance(got, BaseException):
                if not self.priced:
                    sys.exit(f"{r}: finding its pools: {got}")
                log(f"{r}: finding its pools: {got}; keeping the last")
            else:
                self.regions[r].pools = got
        self.pools = sorted((p for r in self.regions.values() for p in r.pools), key=pool_order)
        self.priced = time.time()

    def setup_local(self) -> None:
        self.local_dir = self.state / "local"
        (self.local_dir / "bin").mkdir(parents=True, exist_ok=True)
        for b in self.bins:
            dst = self.local_dir / "bin" / b
            if not dst.exists():
                shutil.copy2(BIN / b, dst)
        if self.testbed is not None:
            slots = 1
        else:
            slots = self.args.slots or max(1, (os.cpu_count() or 2) // 2)
        self.hosts.append(Host("local", "local", "-", Spec(slots, slots, 0, slots), 0.0, time.time(), state="ready"))
        self.local_slots = slots
        self.start_local()

    def start_local(self) -> None:
        self.serve = subprocess.Popen(
            [sys.executable, str(HERE / "worker.py"), "serve", "--dir", str(self.local_dir),
             "--slots", str(self.local_slots), "--job-timeout", str(self.args.job_timeout),
             "--idle-minutes", "5"],
            stdin=subprocess.DEVNULL,
        )

    def progress(self, now: float) -> None:
        total = sum(1 for i in self.items.values() if i.copy != "canary")
        ndone = len(self.done)
        live = [h for h in self.hosts if h.state != "lost"]
        out = sum(len(h.assigned) for h in live)
        spend = sum(h.cost(now) for h in self.hosts)
        kinds = collections.Counter(f"{h.type}@{self.where(h.zone)}:{h.state}" for h in live)
        eta = ""
        slots = sum(h.slots for h in live if h.state == "ready")
        if slots and (self.prior or any(self.seen.values())):
            eta = f", about {self.work_left() / slots / 60:.0f} min left"
        waiting = sum(len(w) for w in self.waiting.values())
        log(
            f"{ndone}/{total} done ({self.failed} failed, {self.verified} verified, "
            f"{self.mismatches} mismatched), {out} out, {len(self.queue)} queued"
            + (f", {waiting} waiting" if waiting else "") + "; "
            + (", ".join(f"{k} x{n}" for k, n in sorted(kinds.items())) or "no instances")
            + (f"; ${spend:.2f}" if not self.local else "") + eta
        )

    def tick(self, now: float) -> None:
        if not self.local:
            if now - self.priced > PRICE_REFRESH:
                self.find_pools()
            self.refresh(now)
            self.boots()
        ready = [h for h in self.hosts if h.state == "ready"]
        batches = {h.id: (self.take(h), list(h.acks)) for h in ready}
        futs = {h.id: self.pool.submit(self.exchange, h, *batches[h.id]) for h in ready}
        for h in ready:
            try:
                reply = futs[h.id].result()
            except Exception as e:
                h.failing_since = h.failing_since or now
                if now - h.failing_since > SSH_GRACE:
                    self.lose(h, f"no sync for {SSH_GRACE // 60} min: {e}")
                continue
            h.failing_since = None
            sent = batches[h.id][1]
            h.acks = h.acks[len(sent):]
            self.absorb(h, reply)
        if self.testbed is None:
            self.refill([h for h in ready if h.state == "ready"])
        if not self.local:
            self.retire_idle()
            self.scale(now)
            self.peak_slots = max(self.peak_slots, sum(h.slots for h in self.hosts if h.state != "lost"))

    def refill(self, hosts: list[Host]) -> None:
        """A second sync for each instance with room once the first has told
        what finished, which the first's jobs were picked without: short
        jobs would otherwise wait a tick for slots that came free. One that
        fails gives its jobs back to the queue."""
        more = {h.id: (h, self.take(h)) for h in hosts if self.queue}
        futs = {hid: self.pool.submit(self.exchange, h, jobs, []) for hid, (h, jobs) in more.items() if jobs}
        for hid, fut in futs.items():
            h, jobs = more[hid]
            try:
                reply = fut.result()
            except Exception:
                for item in jobs:
                    h.assigned.pop(item.wid, None)
                    self.took[item.job.id].discard(h.id)
                self.queue.extendleft(reversed(jobs))
                continue
            self.absorb(h, reply)

    def finished(self) -> bool:
        return not self.queue and not self.waiting and not any(
            i.copy != "canary" for h in self.hosts if h.state != "lost" for i in h.assigned.values()
        )

    def run(self) -> int:
        self.state.mkdir(parents=True, exist_ok=True)
        (self.state / "matrices").write_text("".join(f"{m.name}\n" for m in self.ms))
        for m in self.ms:
            log.open(m.state / "farm.log")
        to_run = sum(1 for i in self.items.values() if i.wid not in self.done)
        if len(self.ms) == 1:
            m = self.ms[0]
            log(f"{m.name}: {self.njobs[m.name]} jobs over {len(m.arms)} arms, {to_run} to run")
        else:
            for m in self.ms:
                log(f"{m.name}: {self.njobs[m.name]} jobs over {len(m.arms)} arms")
            shared = sum(1 for own in self.owners.values() if len(own) > 1)
            log(f"{self.name}: {len(self.owners)} jobs, {shared} of them in more than one "
                f"matrix; {to_run} to run")
            self.share_rows()
        for jid in [j for j in self.waiting if j in self.rows]:
            self.release(jid)
        if self.local:
            self.setup_local()
        else:
            self.setup_aws()
        interrupted = False
        last_progress = 0.0

        # No log() here: the handler runs on the main thread, which may hold
        # its lock.
        def interrupt(*_):
            if self.stopping.is_set():
                with contextlib.suppress(OSError):
                    os.write(2, b"stopping already; cleaning up first\n")
                return
            if self.holding:
                self.deferred = True
                return
            raise KeyboardInterrupt

        for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            signal.signal(sig, interrupt)
        try:
            while not self.finished():
                self.wake.clear()
                now = time.time()
                self.tick(now)
                if now - last_progress >= 60 or self.finished():
                    self.progress(now)
                    last_progress = now
                if not self.finished():
                    self.wake.wait(TICK if not self.local else 2)
            # Ack what's left, so a kept instance's worker can go idle.
            for h in self.hosts:
                if h.state != "lost" and h.acks:
                    try:
                        self.sync(h, [], h.acks)
                    except Exception:
                        pass
        except KeyboardInterrupt:
            interrupted = True
            log("interrupted")
        finally:
            self.stopping.set()
            if self.local:
                # The worker may be one a previous controller started, or
                # this one's, before it has written its pid.
                try:
                    os.kill(int((self.local_dir / "serve.pid").read_text()), signal.SIGTERM)
                except (OSError, ValueError):
                    pass
                self.serve.terminate()
                self.serve.wait()
            elif not (interrupted and self.args.keep):
                self.terminate_all()
            else:
                kept = collections.Counter(h.region for h in self.hosts if h.state != "lost")
                if kept:
                    log("keeping " + ", ".join(f"{n} in {r}" for r, n in sorted(kept.items()))
                        + f"; `farm.py down {self.name}` terminates them")
            self.pool.shutdown(wait=False, cancel_futures=True)
            self.booter.shutdown(wait=False, cancel_futures=True)
            self.progress(time.time())
            if not self.local and self.hosts:
                used = collections.Counter(f"{h.type} in {h.zone}" for h in self.hosts)
                log("instances: " + ", ".join(f"{k} x{n}" for k, n in sorted(used.items())))
            self.account(time.time(), interrupted)
        if interrupted:
            return 130
        for m in self.ms:
            if len(self.ms) > 1:
                print(f"\n{m.name}:\n")
            report(m, [], None)
        return 1 if self.mismatches or self.failed else 0


# --- report -------------------------------------------------------------


def median(v: list[float]) -> float:
    return sorted(v)[len(v) // 2]


AGGS = {"mean": statistics.fmean, "median": median, "min": min, "max": max, "sum": sum}


def fmt(x: float | None) -> str:
    if x is None:
        return "–"
    if float(x).is_integer() and abs(x) < 1e15:
        return str(int(x))
    a = abs(x)
    return f"{x:.0f}" if a >= 100 else f"{x:.2f}" if a >= 1 else f"{x:.4f}"


def short(key: str) -> str:
    for prefix in ("STRIMS_SIM_", "SIM_"):
        if key.startswith(prefix):
            return key.removeprefix(prefix).lower()
    return key


def report(matrix: Matrix, where: list[str], group: list[str] | None) -> None:
    matrix.resolve()
    jobs = matrix.jobs()
    ids = {j.id for j in jobs}
    recs = read_jsonl(matrix.state / "rows.jsonl")
    first: dict[str, dict] = {}
    for r in recs:
        if r["id"] in ids:
            first.setdefault(r["id"], r)
    stale = len({r["id"] for r in recs} - ids)
    group = group or matrix.group
    conds = [w.split("=", 1) for w in where]

    def value(rec: dict, key: str) -> str:
        if key in ("arm", "test"):
            return rec.get(key, "")
        env = rec["env"]
        for k in (key, f"SIM_{key.upper()}", f"STRIMS_SIM_{key.upper()}"):
            if k in env:
                return env[k]
        # The row's own fields: host, type, zone, repeat, ...
        v = rec.get(key)
        return "" if v is None or isinstance(v, dict) else str(v)

    groups: dict[tuple, list[dict]] = {}
    for j in jobs:
        rec = first.get(j.id)
        if rec is None or any(value(rec, k) != v for k, v in conds):
            continue
        groups.setdefault(tuple(value(rec, g) for g in group), []).append(rec)

    heads, cols = [], []
    for spec in matrix.columns:
        field, _, aggs = spec.partition(":")
        if aggs == "ratio":
            num, den = field.split("/")
            heads.append(f"{field} %")
            cols.append(("ratio", num, den))
        else:
            for a in (aggs or "mean").split(","):
                heads.append(f"{field} {a}")
                cols.append(("agg", field, a))
    print("| " + " | ".join([*map(short, group), "n", *heads]) + " |")
    print("|" + "---|" * (len(group) + 1 + len(heads)))
    for key, recs in groups.items():
        rows = [r["row"] for r in recs]
        cells = [*key, str(len(rows))]
        for kind, a, b in cols:
            if kind == "ratio":
                vs = [(r.get(a), r.get(b)) for r in rows if r.get(a) is not None and r.get(b) is not None]
                den = sum(d for _, d in vs)
                cells.append(f"{100 * sum(n for n, _ in vs) / den:.1f}" if den else "–")
            else:
                vs = [r[a] for r in rows if r.get(a) is not None]
                cells.append(fmt(AGGS[b](vs)) if vs else "–")
        print("| " + " | ".join(cells) + " |")
    if matrix.kind == "testbed":
        testbed_report(matrix, groups)
    missing = len(ids) - len(first)
    notes = [f"{len(first)}/{len(ids)} jobs have rows"]
    if stale:
        notes.append(f"{stale} rows of other commits or configs ignored")
    mism = [m for m in read_jsonl(matrix.state / "mismatches.jsonl") if m["first"]["id"] in ids]
    if mism:
        notes.append(f"{len(mism)} TRACE MISMATCHES (mismatches.jsonl)")
    fails = {f["id"] for f in read_jsonl(matrix.state / "failures.jsonl")} & ids
    if fails:
        notes.append(f"{len(fails)} jobs failed (failures.jsonl)")
    if missing:
        notes.append(f"{missing} missing")
    print("\n" + "; ".join(notes))


def testbed_report(matrix: Matrix, groups: dict[tuple, list[dict]]) -> None:
    """Each instance's runs in the order they ran, and each group's run
    dirs in the form the testbed scoring scripts take."""
    recs = sorted((r for rs in groups.values() for r in rs), key=lambda r: r.get("started") or "")
    hosts: dict[str, list[dict]] = {}
    for r in recs:
        hosts.setdefault(r["host"], []).append(r)
    print("\n| host | type | zone | cpu | steal % max | load max | runs |")
    print("|" + "---|" * 7)
    for h, rs in hosts.items():
        steal = max(r["row"].get("steal_pct") or 0 for r in rs)
        load = max(r["row"].get("load_max") or 0 for r in rs)
        runs = ", ".join(
            " ".join([r["arm"], *(r["env"].get(k, "") for k in matrix.axes), f"r{r['repeat']}"])
            for r in rs
        )
        print(f"| {h} | {rs[0]['type']} | {rs[0].get('zone') or '-'} | "
              f"{rs[0].get('cpu_model') or '-'} | {fmt(steal)} | {fmt(load)} | {runs} |")
    print()
    for key, rs in groups.items():
        label = "-".join(key).replace(" ", "_") or "all"
        dirs = [str(matrix.state / r["run_dir"]) for r in sorted(rs, key=lambda r: r["repeat"])]
        print(f"{label}={','.join(dirs)}")


# --- status and down ----------------------------------------------------


def matrix_at(arg: str) -> Matrix:
    path = Path(arg)
    return Matrix(arg if path.exists() else str(HERE / "matrices" / f"{arg}.toml"))


def sweep_regions(regions: list[str]) -> list[str]:
    """The regions status and down look in: the configured ones, and every
    one the farm has launched in."""
    return sorted(set(regions) | recorded_regions())


def status(arg: str | None, regions: list[str]) -> int:
    m = matrix_at(arg) if arg else None
    now = time.time()
    failed = 0
    for r, got in in_regions(lambda r: farm_instances(r, None), sweep_regions(regions)).items():
        if isinstance(got, BaseException):
            print(f"{r}: CAN'T LIST instances: {got}")
            failed += 1
            continue
        for i in got:
            if m and not in_run(i, m.name):
                continue
            launched = datetime.datetime.fromisoformat(i["LaunchTime"]).timestamp()
            print(
                f"{i['InstanceId']}  {tag_of(i, TAG):20} {i['InstanceType']:14} "
                f"{i['Placement']['AvailabilityZone']:14} {i['State']['Name']:13} "
                f"up {(now - launched) / 60:5.0f} min  {i.get('PublicIpAddress', '-')}"
            )
    if m:
        m.resolve()
        ids = {j.id for j in m.jobs()}
        have = {r["id"] for r in read_jsonl(m.state / "rows.jsonl")} & ids
        print(f"{m.name}: {len(have)}/{len(ids)} jobs have rows")
    return 1 if failed else 0


def down(name: str | None, regions: list[str]) -> int:
    def one(r: str) -> int:
        ids = [
            i["InstanceId"] for i in farm_instances(r, None)
            if i["State"]["Name"] != "shutting-down" and (name is None or in_run(i, name))
        ]
        terminate(r, ids)
        return len(ids)

    failed = 0
    for r, got in in_regions(one, sweep_regions(regions)).items():
        if isinstance(got, BaseException):
            print(f"{r}: FAILED: {got}")
            failed += 1
        else:
            print(f"{r}: " + (f"terminating {got} instances" if got else "no instances"))
    return 1 if failed else 0


def config() -> dict:
    """The config file's settings, with AWS's applied to this process's
    environment, where the aws CLI reads them."""
    if not CONFIG.exists():
        return {}
    cfg = tomllib.loads(CONFIG.read_text())
    unknown = set(cfg) - {"ssh_key", "aws_profile", "aws_region", "aws_regions", "run"}
    if unknown:
        sys.exit(f"{CONFIG}: unknown settings {sorted(unknown)}")
    if "aws_region" in cfg and "aws_regions" in cfg:
        sys.exit(f"{CONFIG}: aws_region or aws_regions, not both")
    if "aws_regions" in cfg:
        regions = cfg["aws_regions"]
        if not isinstance(regions, list) or not regions or not all(isinstance(r, str) and r for r in regions):
            sys.exit(f"{CONFIG}: aws_regions is a list of region names")
    if "aws_profile" in cfg:
        os.environ.setdefault("AWS_PROFILE", cfg["aws_profile"])
    if "aws_region" in cfg:
        os.environ.setdefault("AWS_REGION", cfg["aws_region"])
    return cfg


def regions_of(flag: str | None, cfg: dict) -> list[str]:
    """The regions a run launches in: --regions, the config's aws_regions,
    or the aws CLI's one region."""
    if flag:
        names = [r.strip() for r in flag.split(",") if r.strip()]
    elif "aws_regions" in cfg:
        names = list(cfg["aws_regions"])
    else:
        names = [cli_region()]
    return list(dict.fromkeys(names))


def main() -> int:
    cfg = config()
    p = argparse.ArgumentParser(
        description=__doc__.splitlines()[0], formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = p.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("matrices", nargs="+")
    # estimate takes run's options, and so its [run] defaults.
    defaults = cfg.get("run", {})
    for cmd in ("run", "estimate"):
        r = sub.add_parser(cmd)
        r.add_argument("matrices", nargs="+")
        r.add_argument("--max-vcpus", type=int)
        r.add_argument("--max-region-vcpus", type=int)
        r.add_argument("--regions", help="R1,R2,...: the regions to launch in")
        r.add_argument("--families", default=FAMILIES)
        r.add_argument("--max-instance-cores", type=int, default=64)
        r.add_argument("--work-per-boot", type=float, default=2)
        r.add_argument("--hours", type=float, default=6)
        r.add_argument("--verify", type=int, default=2)
        r.add_argument("--job-timeout", type=float, default=60, help="minutes")
        r.add_argument("--idle-minutes", type=float, default=15)
        r.add_argument("--keep", action="store_true")
        r.add_argument("--local", action="store_true")
        r.add_argument("--slots", type=int)
        r.add_argument(
            "--ssh-key",
            default=os.environ.get("STRIMS_FARM_SSH_KEY") or cfg.get("ssh_key") or "~/.ssh/id_rsa",
        )
        known = {a.dest for a in r._actions} - {"help", "matrices", "ssh_key", "regions"}
        if set(defaults) - known:
            sys.exit(f"{CONFIG}: unknown [run] settings {sorted(set(defaults) - known)}")
        r.set_defaults(**defaults)
    s = sub.add_parser("status")
    s.add_argument("matrix", nargs="?")
    s.add_argument("--regions")
    d = sub.add_parser("down")
    d.add_argument("matrix", nargs="?")
    d.add_argument("--regions")
    rp = sub.add_parser("report")
    rp.add_argument("matrix")
    rp.add_argument("--where", action="append", default=[])
    rp.add_argument("--group", type=lambda s: s.split(","))
    args = p.parse_args()

    def name(m: str | None) -> str | None:
        return Path(m).stem if m else None

    if args.cmd == "build":
        for m in args.matrices:
            build(Matrix(m))
    elif args.cmd in ("run", "estimate"):
        if args.work_per_boot <= 0:
            sys.exit("--work-per-boot is more than 0")
        ms = [Matrix(m) for m in args.matrices]
        run_name(ms)  # its checks, before the builds
        if args.cmd == "estimate":
            args.regions = regions_of(args.regions, cfg)
            for m in ms:
                m.resolve()
            return Run(ms, args).estimate_only()
        if not args.local:
            args.regions = regions_of(args.regions, cfg)
        for m in ms:
            build(m)
        return Run(ms, args).run()
    elif args.cmd == "status":
        return status(args.matrix, regions_of(args.regions, cfg))
    elif args.cmd == "down":
        return down(name(args.matrix), regions_of(args.regions, cfg))
    elif args.cmd == "report":
        report(Matrix(args.matrix), args.where, args.group)
    return 0


if __name__ == "__main__":
    sys.exit(main())
