#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Runs the sims' trial matrices on EC2 spot instances, one job per run.

  hack/sim-farm/farm.py build  MATRIX
  hack/sim-farm/farm.py run    MATRIX [--max-vcpus N] [--families F,...]
                               [--max-instance-cores N] [--hours H]
                               [--verify N] [--job-timeout MIN] [--keep]
                               [--local [--slots N]]
  hack/sim-farm/farm.py status [MATRIX]
  hack/sim-farm/farm.py down   [MATRIX]
  hack/sim-farm/farm.py report MATRIX [--where KEY=VALUE ...] [--group KEY,...]

A matrix (matrices/*.toml) names a test that runs one config from its
environment and prints one `ROW {json}` line (the ppspp sim's `sim_job`),
the arms to run it at (git refs), the environment every job shares, and
the axes whose product gives the jobs:

  package = "strims-ppspp"      # the test's crate,
  target = "sim"                # integration test target
  test = "sim_job"              # and test
  slot_mem_gib = 3              # the memory one job needs
  [arms]    name = "git ref"
  [env]     SIM_X = "fixed"
  [axes]    SIM_Y = ["a", "b"]
  [canary]  SIM_SECS = "2"      # overrides for the canary job (below)
  [report]  columns = ["field:mean,max", "num/den:ratio"]
            group = ["arm", "SIM_Y"]   # default: arm and every axis but SIM_SEED

An arm's source is the tree of what its binary is built from (Cargo.toml,
Cargo.lock, rust-toolchain.toml and crates/ at its commit), so commits
elsewhere change nothing. build checks an arm's commit out in a worktree
under target/sim-farm/src, builds the test's release binary into the
shared target directory, and keeps it as
target/sim-farm/bin/<crate>-<target>-<source>. Arms build from commits,
never from the working tree.

run builds what's missing, then keeps spot instances busy until every job
has a row. Its state is target/sim-farm/<matrix>/: rows.jsonl (a row per
finished job, with its arm, commit, environment, host, CPU time and peak
RSS), failures.jsonl, mismatches.jsonl and farm.log. A job's id hashes its
arm's source and its environment, so a rerun skips the jobs that have
rows, and an arm whose source changed runs again.

- Instances run one job per physical core, fewer if memory binds
  (slot_mem_gib each, and 4 GiB for the system). The candidates are the
  x86 types of --families from 8xlarge up to --max-instance-cores (64)
  cores, in the zones the VPC's public subnets cover, ranked by spot price
  per slot it would fill, then by the quota's vCPUs per slot (an SMT core
  costs two).
  No pool (a type in a zone) takes more than half of the vCPU budget: the
  spot quota, less spot vCPUs running outside this matrix, or --max-vcpus.
  A pool out of capacity is skipped for 10 minutes.
- Instances are one-time spot instances, tagged strims-sim-farm=<matrix>,
  that terminate when they shut down. Each shuts itself down after
  --hours (6), and its worker powers it off after --idle-minutes (15) with
  nothing to do and no word from the controller, so a controller that
  dies costs little. `down` terminates them.
- Each new instance first runs the canary: the first job with the
  [canary] overrides, also run here. A different trace means the binaries
  don't reproduce across hosts, and the run stops.
- Every 15 s the controller syncs each instance over SSH (worker.py sync):
  it tops the instance's queue up to its slots plus a quarter, stores the
  rows that came back (fsync), and acks them on the next sync.
- An instance that is gone, has a spot interruption notice, or fails SSH
  for 3 minutes is terminated, and its jobs go back to the head of the
  queue. Runs are deterministic, so a reissued job must print the same
  row: a job whose row arrives twice is compared on (trace, frames,
  bytes), and --verify N (2) runs N jobs twice on purpose, on different
  instances. A job that fails twice is recorded and dropped.
- Instances with nothing left to run are terminated. On ^C (or SIGTERM or
  SIGHUP) the run's instances are terminated unless --keep; a rerun finds
  instances by tag and adopts their jobs.

--local runs the jobs here instead, --slots at a time (default half the
cores), with no AWS: for small matrices and for measuring one job.

AWS: the aws CLI's default credentials and region. The user needs
ec2:RunInstances (spot, with tags on launch), ec2:TerminateInstances on
tagged instances, ec2:ImportKeyPair, ec2:CreateSecurityGroup,
ec2:AuthorizeSecurityGroupIngress, the Describe calls for instances,
images, key pairs, security groups, subnets, instance types, type
offerings and spot prices, and servicequotas:GetServiceQuota. SSH uses
--ssh-key ($STRIMS_FARM_SSH_KEY, default ~/.ssh/id_rsa), whose .pub is
imported as the key pair strims-sim-farm-<file name>; it must work without
a prompt (no passphrase, or loaded into ssh-agent). The security group
strims-sim-farm admits SSH from this host's public address.
"""

import argparse
import collections
import concurrent.futures as cf
import dataclasses
import datetime
import hashlib
import itertools
import json
import os
import re
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
import tomllib
import urllib.request
import uuid
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
FARM = ROOT / "target" / "sim-farm"
BIN = FARM / "bin"
TAG = "strims-sim-farm"
UBUNTU_OWNER = "099720109477"
UBUNTU_IMAGE = "ubuntu/images/hvm-ssd-gp3/ubuntu-resolute-26.04-amd64-server-*"
SPOT_QUOTA = "L-34B43A08"
FAMILIES = "c6a,c7a,c8a,m6a,m7a,m8a,r6a,r7a,r8a,c6i,c7i,c8i,m6i,m7i,m8i,r6i,r7i,r8i"
TICK = 15
SSH_GRACE = 180
POOL_BACKOFF = 600
PRICE_REFRESH = 600
SYSTEM_GIB = 4
# What a test binary is built from: commits that change none of it share
# a binary, and jobs.
SOURCES = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates"]
CAPACITY_ERRORS = {
    "InsufficientInstanceCapacity",
    "InsufficientCapacity",
    "SpotMaxPriceTooLow",
    "Unsupported",
}
QUOTA_ERRORS = {"MaxSpotInstanceCountExceeded", "VcpuLimitExceeded"}


class Log:
    def __init__(self) -> None:
        self.file = None
        self.lock = threading.Lock()

    def open(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        self.file = path.open("a")

    def __call__(self, msg: str) -> None:
        line = f"[{time.strftime('%H:%M:%S')}] {msg}"
        with self.lock:
            print(line, flush=True)
            if self.file:
                self.file.write(line + "\n")
                self.file.flush()


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


@dataclasses.dataclass(frozen=True)
class Item:
    """A job as queued: the original or its verify copy, which the worker
    knows by another id."""

    job: Job
    copy: str = ""

    @property
    def wid(self) -> str:
        return self.job.id + self.copy

    def wire(self) -> dict:
        return {"id": self.wid, "bin": self.job.bin, "test": self.job.test, "env": dict(self.job.env)}


class Matrix:
    def __init__(self, path: str) -> None:
        self.path = Path(path)
        self.name = self.path.stem
        m = tomllib.loads(self.path.read_text())
        self.package = m["package"]
        self.target = m["target"]
        self.test = m["test"]
        self.slot_mem_gib = float(m.get("slot_mem_gib", 2))
        self.arms = {k: str(v) for k, v in m["arms"].items()}
        self.env = {k: str(v) for k, v in m.get("env", {}).items()}
        self.axes = {k: [str(x) for x in v] for k, v in m.get("axes", {}).items()}
        self.canary = {k: str(v) for k, v in m.get("canary", {}).items()}
        rep = m.get("report", {})
        self.columns = rep.get(
            "columns", ["delivery:mean,min", "lag_p50_ms:median", "lag_p99_ms:median,max"]
        )
        self.group = rep.get("group", ["arm"] + [k for k in self.axes if k != "SIM_SEED"])
        self.state = FARM / self.name
        self.commits: dict[str, str] = {}
        self.sources: dict[str, str] = {}

    def resolve(self) -> None:
        for arm, ref in self.arms.items():
            commit = git("rev-parse", "--verify", f"{ref}^{{commit}}")
            tree = git("ls-tree", commit, *SOURCES)
            self.commits[arm] = commit
            self.sources[arm] = hashlib.sha256(tree.encode()).hexdigest()[:12]

    def bin_name(self, source: str) -> str:
        return f"{self.package}-{self.target}-{source}"

    def job(self, arm: str, env: dict[str, str]) -> Job:
        source = self.sources[arm]
        env_t = tuple(sorted(env.items()))
        key = json.dumps([source, self.package, self.target, self.test, env_t])
        jid = hashlib.sha256(key.encode()).hexdigest()[:16]
        return Job(jid, arm, self.commits[arm], source, self.bin_name(source), self.test, env_t)

    def jobs(self) -> list[Job]:
        keys = list(self.axes)
        return [
            self.job(arm, {**self.env, **dict(zip(keys, combo))})
            for arm in self.arms
            for combo in itertools.product(*(self.axes[k] for k in keys))
        ]

    def canary_job(self) -> Job:
        first = self.jobs()[0]
        return self.job(first.arm, {**dict(first.env), **self.canary})


def read_jsonl(path: Path) -> list[dict]:
    if not path.exists():
        return []
    out = []
    for line in path.read_text().splitlines():
        if line.strip():
            out.append(json.loads(line))
    return out


def append_jsonl(path: Path, recs: list[dict]) -> None:
    if not recs:
        return
    with path.open("a") as f:
        for r in recs:
            f.write(json.dumps(r) + "\n")
        f.flush()
        os.fsync(f.fileno())


# --- build --------------------------------------------------------------


def build(matrix: Matrix) -> None:
    matrix.resolve()
    BIN.mkdir(parents=True, exist_ok=True)
    for arm, commit in matrix.commits.items():
        out = BIN / matrix.bin_name(matrix.sources[arm])
        if out.exists():
            continue
        src = FARM / "src" / commit[:12]
        if src.exists():
            subprocess.run(["git", "worktree", "remove", "--force", str(src)], cwd=ROOT)
            shutil.rmtree(src, ignore_errors=True)
            git("worktree", "prune")
        log(f"building {arm} at {commit[:12]}")
        git("worktree", "add", "--detach", str(src), commit)
        try:
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
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", str(src)], cwd=ROOT)


# --- AWS ----------------------------------------------------------------


class AwsError(Exception):
    def __init__(self, code: str, msg: str) -> None:
        super().__init__(msg)
        self.code = code


def aws(*args: str) -> dict:
    p = subprocess.run(["aws", *args, "--output", "json"], capture_output=True, text=True)
    if p.returncode:
        m = re.search(r"An error occurred \(([^)]+)\)", p.stderr)
        raise AwsError(m.group(1) if m else "", p.stderr.strip())
    return json.loads(p.stdout) if p.stdout.strip() else {}


def tag_of(inst: dict, key: str) -> str | None:
    return next((t["Value"] for t in inst.get("Tags", []) if t["Key"] == key), None)


def farm_instances(name: str | None) -> list[dict]:
    flt = f"Name=tag:{TAG},Values={name}" if name else f"Name=tag-key,Values={TAG}"
    r = aws(
        "ec2", "describe-instances", "--filters", flt,
        "Name=instance-state-name,Values=pending,running,stopping,stopped,shutting-down",
    )
    return [i for res in r["Reservations"] for i in res["Instances"]]


def terminate(ids: list[str]) -> None:
    if ids:
        aws("ec2", "terminate-instances", "--instance-ids", *ids)


def ensure_key(key: Path) -> str:
    name = f"{TAG}-{key.name}"
    try:
        aws("ec2", "describe-key-pairs", "--key-names", name)
    except AwsError as e:
        if e.code != "InvalidKeyPair.NotFound":
            raise
        aws("ec2", "import-key-pair", "--key-name", name, "--public-key-material", f"fileb://{key}.pub")
        log(f"imported {key}.pub as the key pair {name}")
    return name


def network() -> tuple[str, dict[str, str]]:
    """The VPC of the public subnets, and a subnet per zone."""
    subnets = aws(
        "ec2", "describe-subnets", "--filters", "Name=map-public-ip-on-launch,Values=true"
    )["Subnets"]
    if not subnets:
        sys.exit("no subnet maps public IPs on launch")
    vpc = sorted(s["VpcId"] for s in subnets)[0]
    zones: dict[str, str] = {}
    for s in sorted(subnets, key=lambda s: s["SubnetId"]):
        if s["VpcId"] == vpc:
            zones.setdefault(s["AvailabilityZone"], s["SubnetId"])
    return vpc, zones


def ensure_sg(vpc: str) -> str:
    groups = aws(
        "ec2", "describe-security-groups", "--filters",
        f"Name=group-name,Values={TAG}", f"Name=vpc-id,Values={vpc}",
    )["SecurityGroups"]
    if groups:
        sg, perms = groups[0]["GroupId"], groups[0]["IpPermissions"]
    else:
        sg = aws(
            "ec2", "create-security-group", "--group-name", TAG,
            "--description", "sim farm: SSH from the controller", "--vpc-id", vpc,
        )["GroupId"]
        perms = []
        log(f"created the security group {TAG} ({sg})")
    ip = urllib.request.urlopen("https://checkip.amazonaws.com", timeout=10).read().decode().strip()
    cidr = f"{ip}/32"
    if not any(
        p.get("FromPort") == 22 and any(r["CidrIp"] == cidr for r in p.get("IpRanges", []))
        for p in perms
    ):
        try:
            aws(
                "ec2", "authorize-security-group-ingress", "--group-id", sg,
                "--protocol", "tcp", "--port", "22", "--cidr", cidr,
            )
            log(f"{TAG} admits SSH from {cidr}")
        except AwsError as e:
            if e.code != "InvalidPermission.Duplicate":
                raise
    return sg


def image() -> tuple[str, str]:
    imgs = aws(
        "ec2", "describe-images", "--owners", UBUNTU_OWNER, "--filters",
        f"Name=name,Values={UBUNTU_IMAGE}", "Name=state,Values=available",
        "Name=architecture,Values=x86_64",
    )["Images"]
    if not imgs:
        sys.exit(f"no image matches {UBUNTU_IMAGE}")
    img = max(imgs, key=lambda i: i["CreationDate"])
    return img["ImageId"], img["RootDeviceName"]


def spot_quota() -> int | None:
    try:
        q = aws("service-quotas", "get-service-quota", "--service-code", "ec2", "--quota-code", SPOT_QUOTA)
        return int(q["Quota"]["Value"])
    except AwsError:
        return None


def spot_vcpus_outside(name: str) -> int:
    r = aws(
        "ec2", "describe-instances", "--filters", "Name=instance-lifecycle,Values=spot",
        "Name=instance-state-name,Values=pending,running",
    )
    total = 0
    for res in r["Reservations"]:
        for i in res["Instances"]:
            if tag_of(i, TAG) != name:
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

    @property
    def per_slot(self) -> float:
        return self.price / self.spec.slots

    @property
    def key(self) -> tuple[str, str]:
        return (self.type, self.zone)


def specs(matrix: Matrix, types: list[str] | None = None, families: list[str] = (),
          min_vcpus: int = 0, max_cores: int = 1 << 30) -> dict[str, Spec]:
    if types is not None:
        flt = ["Name=instance-type,Values=" + ",".join(types)]
    else:
        flt = [
            "Name=instance-type,Values=" + ",".join(f"{f}.*" for f in families),
            "Name=processor-info.supported-architecture,Values=x86_64",
            "Name=bare-metal,Values=false",
        ]
    out = {}
    for t in aws("ec2", "describe-instance-types", "--filters", *flt)["InstanceTypes"]:
        v = t["VCpuInfo"]["DefaultVCpus"]
        c = t["VCpuInfo"].get("DefaultCores", v)
        mem = t["MemoryInfo"]["SizeInMiB"] / 1024
        if v < min_vcpus or c > max_cores:
            continue
        slots = min(c, int((mem - SYSTEM_GIB) // matrix.slot_mem_gib))
        if slots >= 1:
            out[t["InstanceType"]] = Spec(v, c, mem, slots)
    return out


def pools(matrix: Matrix, families: list[str], max_cores: int, zones: dict[str, str]) -> list[Pool]:
    sp = specs(matrix, families=families, min_vcpus=32, max_cores=max_cores)
    if not sp:
        return []
    names = sorted(sp)
    offered = {
        (o["InstanceType"], o["Location"])
        for o in aws(
            "ec2", "describe-instance-type-offerings", "--location-type", "availability-zone",
            "--filters", "Name=instance-type,Values=" + ",".join(names),
            "Name=location,Values=" + ",".join(zones),
        )["InstanceTypeOfferings"]
    }
    now = datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S")
    prices: dict[tuple[str, str], float] = {}
    for r in aws(
        "ec2", "describe-spot-price-history", "--product-descriptions", "Linux/UNIX",
        "--start-time", now, "--instance-types", *names,
        "--filters", "Name=availability-zone,Values=" + ",".join(zones),
    )["SpotPriceHistory"]:
        prices.setdefault((r["InstanceType"], r["AvailabilityZone"]), float(r["SpotPrice"]))
    out = [Pool(t, z, zones[z], sp[t], prices[(t, z)]) for (t, z) in offered if (t, z) in prices]
    out.sort(key=lambda p: (p.per_slot, p.spec.vcpus / p.spec.slots, p.type, p.zone))
    return out


def launch(matrix: Matrix, pool: Pool, ami: tuple[str, str], sg: str, key_name: str,
           user_data: Path) -> str:
    tags = f"{{Key={TAG},Value={matrix.name}}},{{Key=Name,Value=sim-farm-{matrix.name}}}"
    r = aws(
        "ec2", "run-instances", "--count", "1",
        "--image-id", ami[0], "--instance-type", pool.type, "--subnet-id", pool.subnet,
        "--security-group-ids", sg, "--key-name", key_name,
        "--instance-market-options", json.dumps({
            "MarketType": "spot",
            "SpotOptions": {"SpotInstanceType": "one-time", "InstanceInterruptionBehavior": "terminate"},
        }),
        "--instance-initiated-shutdown-behavior", "terminate",
        "--block-device-mappings", json.dumps([{
            "DeviceName": ami[1],
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
    # booting -> canary -> ready, or adopting -> ready; lost at the end.
    state: str = "booting"
    ip: str | None = None
    boot: cf.Future | None = None
    assigned: dict[str, Item] = dataclasses.field(default_factory=dict)
    acks: list[str] = dataclasses.field(default_factory=list)
    failing_since: float | None = None
    ended: float | None = None

    @property
    def slots(self) -> int:
        return self.spec.slots

    def cost(self, now: float) -> float:
        return self.price * ((self.ended or now) - self.launched) / 3600


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


# --- run ----------------------------------------------------------------


class Run:
    def __init__(self, matrix: Matrix, args: argparse.Namespace) -> None:
        self.m = matrix
        self.args = args
        self.local = args.local
        self.state = matrix.state
        self.rows_path = self.state / "rows.jsonl"
        self.jobs = matrix.jobs()
        self.items: dict[str, Item] = {}
        self.rows: dict[str, dict] = {}
        for r in read_jsonl(self.rows_path):
            self.rows.setdefault(r["id"], r)
        self.done: set[str] = set()  # wids with a row, or failed for good
        self.attempts: collections.Counter = collections.Counter()
        self.took: dict[str, set[str]] = collections.defaultdict(set)  # job id -> hosts
        self.queue: collections.deque[Item] = collections.deque()
        self.hosts: list[Host] = []
        self.pool = cf.ThreadPoolExecutor(max_workers=16)
        self.verified = 0
        self.mismatches = 0
        self.failed = 0
        self.walls: list[float] = []
        self.boot_failures = 0
        self.stopping = threading.Event()
        counts = collections.Counter(r["id"] for r in read_jsonl(self.rows_path))
        for j in self.jobs:
            item = Item(j)
            self.items[item.wid] = item
            if j.id in self.rows:
                self.done.add(item.wid)
            else:
                self.queue.append(item)
        # The verify copies: the first jobs by id that haven't two rows yet.
        for j in sorted(self.jobs, key=lambda j: j.id)[: args.verify]:
            item = Item(j, "v")
            self.items[item.wid] = item
            if counts[j.id] >= 2:
                self.done.add(item.wid)
            else:
                self.queue.append(item)

    # -- results

    def store(self, host: Host, item: Item, res: dict) -> None:
        rec = {
            "id": item.job.id,
            "arm": item.job.arm,
            "commit": item.job.commit,
            "env": dict(item.job.env),
            "copy": item.copy,
            "host": host.id,
            "type": host.type,
            "wall_s": res.get("wall_s"),
            "cpu_s": res.get("cpu_s"),
            "max_rss_mb": res.get("max_rss_mb"),
            "at": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
            "row": res["row"],
        }
        append_jsonl(self.rows_path, [rec])
        self.done.add(item.wid)
        if res.get("wall_s"):
            self.walls.append(res["wall_s"])
        first = self.rows.setdefault(item.job.id, rec)
        if first is not rec:
            key = lambda r: tuple(r["row"].get(k) for k in ("trace", "frames", "bytes"))
            if key(first) == key(rec):
                self.verified += 1
            else:
                self.mismatches += 1
                append_jsonl(self.state / "mismatches.jsonl", [{"first": first, "again": rec}])
                log(f"MISMATCH: job {item.job.id} ({item.job.arm} {dict(item.job.env)}): "
                    f"{key(first)} on {first['host']}, {key(rec)} on {host.id}")

    def fail(self, host: Host, item: Item, res: dict) -> None:
        self.attempts[item.wid] += 1
        tail = (res.get("log_tail") or "").splitlines()[-3:]
        log(f"job {item.wid} failed on {host.id}: {res.get('error')}" + "".join(f"\n    {t}" for t in tail))
        if self.attempts[item.wid] < 2:
            self.queue.appendleft(item)
        else:
            self.done.add(item.wid)
            self.failed += 1
            append_jsonl(self.state / "failures.jsonl", [{
                "id": item.job.id, "arm": item.job.arm, "env": dict(item.job.env),
                "host": host.id, **res,
            }])

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
                terminate([host.id])
            except AwsError as e:
                log(f"terminating {host.id}: {e}")

    # -- dispatch

    def take(self, host: Host) -> list[Item]:
        if host.state == "canary":
            if any(i.copy == "canary" for i in host.assigned.values()):
                return []
            item = self.canary_item
            host.assigned[item.wid] = item
            return [item]
        if host.state != "ready":
            return []
        want = host.slots + max(1, host.slots // 4) - len(host.assigned)
        others = sum(1 for h in self.hosts if h is not host and h.state != "lost")
        out, skipped = [], []
        while want > 0 and self.queue:
            item = self.queue.popleft()
            if item.wid in self.done or item.wid in host.assigned:
                continue
            if host.id in self.took[item.job.id] and others:
                skipped.append(item)
                continue
            out.append(item)
            host.assigned[item.wid] = item
            self.took[item.job.id].add(host.id)
            want -= 1
        self.queue.extendleft(reversed(skipped))
        return out

    def sync(self, host: Host, jobs: list[Item], acks: list[str]) -> dict:
        req = json.dumps({"ack": acks, "jobs": [i.wire() for i in jobs]})
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
                self.check_canary(host, res)
            elif res.get("row") is not None:
                self.store(host, item, res)
            else:
                self.fail(host, item, res)
        there = set(reply["queued"]) | set(reply["running"])
        for wid in there - set(host.assigned):
            item = self.items.get(wid)
            if item is not None and wid not in self.done:
                host.assigned[wid] = item
                self.took[item.job.id].add(host.id)
                try:
                    self.queue.remove(item)
                except ValueError:
                    pass
        gone = [host.assigned.pop(w) for w in list(host.assigned) if w not in there]
        back = [i for i in gone if i.copy != "canary" and i.wid not in self.done]
        if back:
            log(f"{host.id} lost track of {len(back)} jobs; requeued")
            self.queue.extendleft(reversed(back))
        if reply.get("interrupting"):
            self.lose(host, "spot interruption notice")
        elif not reply.get("serving") and host.assigned:
            log(f"{host.id}: the worker isn't running; restarting it")
            if self.local:
                self.start_local()
            else:
                self.pool.submit(self.start_worker, host)

    # -- the canary

    def local_canary(self) -> str:
        job = self.m.canary_job()
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
            [str(BIN / job.bin), "--exact", job.test, "--ignored", "--nocapture", "--test-threads", "1"],
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
            log(f"{host.id} ({host.type} in {host.zone}) passed the canary; {host.slots} slots")
            host.state = "ready"
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
            "--working-directory /home/ubuntu/farm "
            f"python3 /home/ubuntu/farm/worker.py serve --slots {host.slots} "
            f"--job-timeout {self.args.job_timeout} --idle-minutes {self.args.idle_minutes} "
            "--imds --poweroff"
        )
        p = self.ssh.run(host, cmd)
        if p.returncode:
            raise RuntimeError(f"starting the worker: {p.stderr.strip()}")

    def bootstrap(self, host: Host) -> None:
        start = time.time()
        denied = None
        while True:
            if self.stopping.is_set():
                raise RuntimeError("stopping")
            try:
                p = self.ssh.run(host, "test -d farm/bin", timeout=30)
                if p.returncode == 0:
                    break
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
            time.sleep(5)
        self.ssh.put(host, [HERE / "worker.py"], "farm/")
        self.ssh.put(host, [BIN / b for b in sorted({j.bin for j in self.jobs})], "farm/bin/")
        self.start_worker(host)

    def refresh(self, now: float) -> None:
        by_id = {i["InstanceId"]: i for i in farm_instances(self.m.name)}
        for h in self.hosts:
            if h.state == "lost":
                continue
            inst = by_id.get(h.id)
            if inst is None and now - h.launched < 120:
                continue  # not listed yet
            st = inst["State"]["Name"] if inst else "gone"
            if st not in ("pending", "running"):
                reason = (inst or {}).get("StateReason", {}).get("Message", st)
                self.lose(h, reason)
                continue
            h.ip = inst.get("PublicIpAddress")
            if h.boot is None and st == "running" and h.ip:
                h.boot = self.pool.submit(self.bootstrap, h)
            elif h.boot is not None and h.boot.done() and h.state in ("booting", "adopting"):
                try:
                    h.boot.result()
                    h.state = "canary" if h.state == "booting" else "ready"
                except Exception as e:
                    self.lose(h, f"bootstrap failed: {e}")
                    self.boot_failures += 1
                    if self.boot_failures >= 3:
                        raise SystemExit("instances keep failing to bootstrap; stopping")

    def budget_room(self) -> int:
        live = sum(h.spec.vcpus for h in self.hosts if h.state != "lost")
        return self.budget - live

    def pick(self, deficit: int, now: float) -> Pool | None:
        room = self.budget_room()
        per_pool = collections.Counter()
        for h in self.hosts:
            if h.state != "lost":
                per_pool[(h.type, h.zone)] += h.spec.vcpus
        cands = [
            p for p in self.pools
            if self.backoff.get(p.key, 0) <= now
            and p.spec.vcpus <= room
            and per_pool[p.key] + p.spec.vcpus <= max(self.budget / 2, p.spec.vcpus)
        ]
        # The price per slot it can fill: as the queue drains, smaller types.
        return min(
            cands,
            key=lambda p: (p.price / min(p.spec.slots, deficit), p.spec.vcpus / p.spec.slots),
            default=None,
        )

    def scale(self, now: float) -> None:
        live = [h for h in self.hosts if h.state != "lost"]
        demand = len(self.queue) + sum(len(h.assigned) for h in live)
        deficit = demand - sum(h.slots for h in live)
        launched = 0
        while deficit > 0 and launched < 3:
            pool = self.pick(deficit, now)
            if pool is None:
                if not live and not launched and now - self.last_stuck > 60:
                    self.last_stuck = now
                    log("no pool fits the budget or has capacity; waiting")
                return
            try:
                hid = launch(self.m, pool, self.ami, self.sg, self.key_name, self.user_data)
            except AwsError as e:
                if e.code in CAPACITY_ERRORS:
                    log(f"{pool.type} in {pool.zone}: {e.code}; skipping it for {POOL_BACKOFF // 60} min")
                    self.backoff[pool.key] = now + POOL_BACKOFF
                    continue
                if e.code in QUOTA_ERRORS:
                    self.budget = self.budget - self.budget_room()
                    log(f"{e.code}: the budget is now {self.budget} vCPUs")
                    return
                if e.code == "AuthFailure.ServiceLinkedRoleCreationNotPermitted":
                    raise SystemExit(
                        "the account's first spot instance needs the Spot service-linked role; an "
                        "admin creates it once: aws iam create-service-linked-role "
                        "--aws-service-name spot.amazonaws.com"
                    )
                raise
            h = Host(hid, pool.type, pool.zone, pool.spec, pool.price, now)
            self.hosts.append(h)
            log(f"launched {hid}: {pool.type} in {pool.zone}, {pool.spec.slots} slots, ${pool.price:.3f}/h")
            deficit -= pool.spec.slots
            launched += 1

    def retire_idle(self) -> None:
        if self.queue:
            return
        for h in self.hosts:
            if h.state == "ready" and not h.assigned:
                log(f"terminating {h.id}: nothing left for it")
                h.state = "lost"
                h.ended = time.time()
                try:
                    terminate([h.id])
                except AwsError as e:
                    log(f"terminating {h.id}: {e}")

    # -- the loop

    def setup_aws(self) -> None:
        key = Path(self.args.ssh_key).expanduser()
        if not key.exists() or not Path(f"{key}.pub").exists():
            sys.exit(f"{key} or its .pub is missing (--ssh-key, $STRIMS_FARM_SSH_KEY)")
        self.ssh = Ssh(key, self.state)
        self.key_name = ensure_key(key)
        vpc, self.zones = network()
        self.sg = ensure_sg(vpc)
        self.ami = image()
        quota = spot_quota()
        outside = spot_vcpus_outside(self.m.name)
        if quota is None and self.args.max_vcpus is None:
            sys.exit("can't read the spot vCPU quota; pass --max-vcpus")
        cap = quota - outside if quota is not None else self.args.max_vcpus
        self.budget = min(cap, self.args.max_vcpus or cap)
        self.families = self.args.families.split(",")
        self.pools = pools(self.m, self.families, self.args.max_instance_cores, self.zones)
        self.priced = time.time()
        if not self.pools:
            sys.exit("no instance type fits (families, sizes, slot_mem_gib, zones)")
        log(
            f"budget {self.budget} vCPUs (quota {quota}, {outside} in other spot instances); "
            f"best pools: " + ", ".join(
                f"{p.type}@{p.zone[-1]} {p.spec.slots} slots ${p.per_slot * 1000:.1f}m/slot-h"
                for p in self.pools[:5]
            )
        )
        self.backoff: dict[tuple[str, str], float] = {}
        self.last_stuck = 0.0
        minutes = int(self.args.hours * 60)
        self.user_data = self.state / "user-data.sh"
        self.user_data.write_text((HERE / "user-data.sh").read_text().replace("@MINUTES@", str(minutes)))
        canary = self.m.canary_job()
        self.canary_item = Item(canary, "canary")
        self.items[self.canary_item.wid] = self.canary_item
        self.canary_trace = self.local_canary()
        # Instances a previous controller left running.
        found = [i for i in farm_instances(self.m.name) if i["State"]["Name"] in ("pending", "running")]
        if found:
            sp = specs(self.m, types=sorted({i["InstanceType"] for i in found}))
            for i in found:
                spec = sp.get(i["InstanceType"])
                if spec is None:
                    continue
                price = next((p.price for p in self.pools if p.key == (i["InstanceType"], i["Placement"]["AvailabilityZone"])), 0.0)
                launched = datetime.datetime.fromisoformat(i["LaunchTime"]).timestamp()
                h = Host(i["InstanceId"], i["InstanceType"], i["Placement"]["AvailabilityZone"], spec, price, launched, state="adopting")
                self.hosts.append(h)
                log(f"adopting {h.id} ({h.type} in {h.zone})")

    def setup_local(self) -> None:
        self.local_dir = self.state / "local"
        (self.local_dir / "bin").mkdir(parents=True, exist_ok=True)
        for b in sorted({j.bin for j in self.jobs}):
            dst = self.local_dir / "bin" / b
            if not dst.exists():
                shutil.copy2(BIN / b, dst)
        slots = self.args.slots or max(1, (os.cpu_count() or 2) // 2)
        self.hosts.append(Host("local", "local", "-", Spec(slots, slots, 0, slots), 0.0, time.time(), state="ready"))
        self.budget = slots
        self.start_local()

    def start_local(self) -> None:
        self.serve = subprocess.Popen(
            [sys.executable, str(HERE / "worker.py"), "serve", "--dir", str(self.local_dir),
             "--slots", str(self.budget), "--job-timeout", str(self.args.job_timeout),
             "--idle-minutes", "5"],
            stdin=subprocess.DEVNULL,
        )

    def progress(self, now: float) -> None:
        total = sum(1 for i in self.items.values() if i.copy != "canary")
        ndone = len(self.done)
        live = [h for h in self.hosts if h.state != "lost"]
        out = sum(len(h.assigned) for h in live)
        spend = sum(h.cost(now) for h in self.hosts)
        kinds = collections.Counter(f"{h.type}@{h.zone[-1]}:{h.state}" for h in live)
        eta = ""
        slots = sum(h.slots for h in live if h.state == "ready")
        if self.walls and slots:
            left = len(self.queue) + out
            mins = statistics.fmean(self.walls) * left / slots / 60
            eta = f", about {mins:.0f} min left"
        log(
            f"{ndone}/{total} done ({self.failed} failed, {self.verified} verified, "
            f"{self.mismatches} mismatched), {out} out, {len(self.queue)} queued; "
            + (", ".join(f"{k} x{n}" for k, n in sorted(kinds.items())) or "no instances")
            + (f"; ${spend:.2f}" if not self.local else "") + eta
        )

    def tick(self, now: float) -> None:
        if not self.local:
            if now - self.priced > PRICE_REFRESH:
                self.pools = pools(self.m, self.families, self.args.max_instance_cores, self.zones)
                self.priced = now
            self.refresh(now)
        ready = [h for h in self.hosts if h.state in ("canary", "ready")]
        batches = {h.id: (self.take(h), list(h.acks)) for h in ready}
        futs = {h.id: self.pool.submit(self.sync, h, *batches[h.id]) for h in ready}
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
        if not self.local:
            self.retire_idle()
            self.scale(now)

    def finished(self) -> bool:
        return not self.queue and not any(
            i.copy != "canary" for h in self.hosts if h.state != "lost" for i in h.assigned.values()
        )

    def run(self) -> int:
        self.state.mkdir(parents=True, exist_ok=True)
        log.open(self.state / "farm.log")
        log(f"{self.m.name}: {len(self.jobs)} jobs over {len(self.m.arms)} arms, "
            f"{sum(1 for i in self.items.values() if i.wid not in self.done)} to run")
        if self.local:
            self.setup_local()
        else:
            self.setup_aws()
        interrupted = False
        last_progress = 0.0

        def interrupt(*_):
            raise KeyboardInterrupt

        for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            signal.signal(sig, interrupt)
        try:
            while not self.finished():
                now = time.time()
                self.tick(now)
                if now - last_progress >= 60 or self.finished():
                    self.progress(now)
                    last_progress = now
                if not self.finished():
                    time.sleep(TICK if not self.local else 2)
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
                # The worker may be one a previous controller started.
                try:
                    os.kill(int((self.local_dir / "serve.pid").read_text()), signal.SIGTERM)
                except (OSError, ValueError):
                    pass
                self.serve.wait()
            elif not (interrupted and self.args.keep):
                live = [h for h in self.hosts if h.state != "lost"]
                if live:
                    log(f"terminating {len(live)} instances")
                    try:
                        terminate([h.id for h in live])
                    except AwsError as e:
                        log(f"terminating: {e}")
                    for h in live:
                        h.state, h.ended = "lost", time.time()
            self.pool.shutdown(wait=False, cancel_futures=True)
            self.progress(time.time())
        if interrupted:
            return 130
        report(self.m, [], None)
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
    return key.removeprefix("SIM_").lower() if key.startswith("SIM_") else key


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
        if key == "arm":
            return rec["arm"]
        env = rec["env"]
        return env.get(key, env.get(f"SIM_{key.upper()}", ""))

    groups: dict[tuple, list[dict]] = {}
    for j in jobs:
        rec = first.get(j.id)
        if rec is None or any(value(rec, k) != v for k, v in conds):
            continue
        groups.setdefault(tuple(value(rec, g) for g in group), []).append(rec["row"])

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
    for key, rows in groups.items():
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


# --- status and down ----------------------------------------------------


def matrix_at(arg: str) -> Matrix:
    path = Path(arg)
    return Matrix(arg if path.exists() else str(HERE / "matrices" / f"{arg}.toml"))


def status(arg: str | None) -> None:
    m = matrix_at(arg) if arg else None
    now = time.time()
    for i in farm_instances(m.name if m else None):
        launched = datetime.datetime.fromisoformat(i["LaunchTime"]).timestamp()
        print(
            f"{i['InstanceId']}  {tag_of(i, TAG):20} {i['InstanceType']:14} "
            f"{i['Placement']['AvailabilityZone']:11} {i['State']['Name']:13} "
            f"up {(now - launched) / 60:5.0f} min  {i.get('PublicIpAddress', '-')}"
        )
    if m:
        m.resolve()
        ids = {j.id for j in m.jobs()}
        have = {r["id"] for r in read_jsonl(m.state / "rows.jsonl")} & ids
        print(f"{m.name}: {len(have)}/{len(ids)} jobs have rows")


def down(name: str | None) -> None:
    ids = [i["InstanceId"] for i in farm_instances(name) if i["State"]["Name"] != "shutting-down"]
    terminate(ids)
    print(f"terminating {len(ids)} instances" if ids else "no instances")


def main() -> int:
    p = argparse.ArgumentParser(
        description=__doc__.splitlines()[0], formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = p.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("matrix")
    r = sub.add_parser("run")
    r.add_argument("matrix")
    r.add_argument("--max-vcpus", type=int)
    r.add_argument("--families", default=FAMILIES)
    r.add_argument("--max-instance-cores", type=int, default=64)
    r.add_argument("--hours", type=float, default=6)
    r.add_argument("--verify", type=int, default=2)
    r.add_argument("--job-timeout", type=float, default=60, help="minutes")
    r.add_argument("--idle-minutes", type=float, default=15)
    r.add_argument("--keep", action="store_true")
    r.add_argument("--local", action="store_true")
    r.add_argument("--slots", type=int)
    r.add_argument("--ssh-key", default=os.environ.get("STRIMS_FARM_SSH_KEY", "~/.ssh/id_rsa"))
    s = sub.add_parser("status")
    s.add_argument("matrix", nargs="?")
    d = sub.add_parser("down")
    d.add_argument("matrix", nargs="?")
    rp = sub.add_parser("report")
    rp.add_argument("matrix")
    rp.add_argument("--where", action="append", default=[])
    rp.add_argument("--group", type=lambda s: s.split(","))
    args = p.parse_args()

    def name(m: str | None) -> str | None:
        return Path(m).stem if m else None

    if args.cmd == "build":
        build(Matrix(args.matrix))
    elif args.cmd == "run":
        m = Matrix(args.matrix)
        build(m)
        return Run(m, args).run()
    elif args.cmd == "status":
        status(args.matrix)
    elif args.cmd == "down":
        down(name(args.matrix))
    elif args.cmd == "report":
        report(Matrix(args.matrix), args.where, args.group)
    return 0


if __name__ == "__main__":
    sys.exit(main())
