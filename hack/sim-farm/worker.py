#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sim farm's worker: runs farm.py's jobs on one host.

  worker.py serve [--dir D] --slots N [--job-timeout MIN] [--idle-minutes MIN]
                  [--imds [--imds-url URL]] [--poweroff]
                  [--run RUN --instance ID --policy FILE [--rotate-mb N]
                   [--end-at T] [--duckdb PATH]]
  worker.py sync [--dir D]

`serve` runs up to N jobs at once from D/queue/, by their `seq` if they
have one, then oldest first. A job is a test binary in D/bin/ run as one
test (TEST_ARGS: `--exact <test> --include-ignored --nocapture
--test-threads 1`) with TEST_ENV and the job's environment, in
D/work/<id>/, its output in out.log there. A finished job
leaves D/done/<id>.json: its exit, the single `ROW {json}` it printed,
its wall and CPU time, its peak RSS, and the tail of its log if it
failed. A job past --job-timeout minutes is killed.

A test job also writes its tables (hack/sim-db) into an out dir, its
sim's knob (the job's `out`) naming D/work/<id>/out, and the worker keeps
them in a store of its own (Store): a finished job's out dir waits in
D/fold/ until a thread folds it, a batch a transaction, into the open
generation, D/store/gen-<seq>.duckdb, and only then leaves its result in
done/, its `row` thinned to what the controller reads (trace, frames,
bytes, and the job's `fields`) and `flush` naming the generation. A
generation is closed when it reaches --rotate-mb, when the controller
asks for a flush with nothing left here to run (its sync's `flush`), at
--end-at, and on an interruption notice; another thread then exports
each closed one, one at a time, to zstd Parquet with a manifest
(hack/sim-db's export) and uploads its files by the --policy file's
presigned POST policies (s3sign.py) while jobs run on: the tables to
sim-data/<run>/<instance>/<seq>/<table>.parquet, then manifest.json, then
the bulky tables (reads, forecast) to sim-samples/.../<table>.parquet. A
flush that lands leaves D/flushes/<seq>.json for the controller, and its
generation is deleted.
A job of `"store": false` (the canary) keeps its whole row and no tables.
The policy file is {"data": policy, "samples": policy}, or {"dir": D2}
to copy flushes into D2/<run>/<instance>/<seq>/ instead (--local: the
cache's farm/ dir).

With --end-at T (seconds since the epoch) it starts no job expected
(`expected_s`) to end after T, and at T kills what still runs (requeued
by the controller, `requeue`), flushes what it holds, and tells the
controller (sync's `ending`, the shutdown's time END_GRACE_S later),
which takes it as an interruption.

A testbed job (kind "testbed") runs hack/testbed/run.sh from its bundle,
a tarball in D/bin/ extracted once into D/trees/, as `run.sh
D/work/<id>/out`, with the job's environment, each of the bundle's
artifacts/<VAR> as $VAR, and each of the job's files (in D/bin/) as its
variable. It starts once no container named tb-* is running: run.sh's
names are fixed, so two testbeds on one Docker host take each other's
containers. A timed-out run's containers, which outlive it, are removed.
While it runs, serve samples /proc/stat and the load average each
second, since steal and load from elsewhere bend a real-time run. A
finished run leaves its out dir as D/done/<id>.tar.gz beside its result,
whose row holds the host's steal and busy shares of CPU time over the
run, steal's worst 10 s, the load's mean and max, and node-<i>.jsonl's
line counts; with no node-<i>.jsonl it fails. The containers write some
of the out dir as root: what the worker can't read or remove goes
through sudo where it needs no password (EC2), else a container.

With --imds it polls the instance metadata (--imds-url, for tests) every
5 s for a spot interruption notice, which comes 2 minutes before EC2
takes the host. Once one comes it starts no job, kills at once the
running test jobs not expected to end within NOTICE_FINISH_S, and the
rest at that long after the notice; it folds what finished, closes the
generation and exports and uploads every closed one with all the cores,
the tables first, so it is uploading well within 90 s of the notice;
an upload still going NOTICE_ABORT_S before the notice's time is cut off.
A killed job's result says `requeue`. The controller winding down (an
exception, a signal: its sync's `wind_down`, a time) takes the same path
to that time, but as no interruption, so it's not told back as one. With
nothing running, queued or
left to fold or upload and no sync for --idle-minutes it exits, and with
--poweroff it powers the host off first; farm.py launches instances that
terminate on shutdown, so a host whose controller went away stops costing
money.

`sync` is the controller's one round trip per tick. It reads one JSON
object from stdin, {"ack": [ids], "jobs": [jobs], "drop": [ids],
"ack_flushes": [seqs], "flush": bool, "wind_down": time or null}: it
deletes the acked results and flush records (the controller has stored
them), queues the jobs, deletes the dropped ones from the queue if serve
hasn't started them, with `flush` asks serve to close the open generation
once nothing is left to run, and with `wind_down` to stop and flush by
then. It prints one JSON object: the results not yet acked, the queued,
running and folding ids, when each running job started, the flushes that
landed and the store's state, the store's S3 POSTs by flush (each attempt,
the bytes it sent, and those that landed: `s3`, which farm.py prices),
whether serve is up, when the host will be
interrupted (the notice's time, or null), and why serve waits, if it
does. A result stays until it is acked, which the controller does for a
stored job once its flush has landed, so a sync whose reply is lost, or a
controller that restarts, loses nothing.

D defaults to this file's directory. Every write is a rename, so serve
and sync can run at once.
"""

import argparse
import collections
import datetime
import fcntl
import functools
import importlib.util
import json
import os
import shutil
import signal
import socket
import statistics
import subprocess
import sys
import tarfile
import threading
import time
import traceback
import urllib.request
from pathlib import Path

LOG_TAIL = 40
IMDS_URL = "http://169.254.169.254"
# After an interruption notice: the running jobs expected to end within
# this long run on, the rest are killed at once, and these too this long
# after it; an upload still going this long before the notice's time is
# cut off.
NOTICE_FINISH_S = 20
NOTICE_ABORT_S = 5
# Finished jobs fold into the store this many a transaction.
FOLD_BATCH = 32
# DuckDB's share of the host while jobs run.
STORE_THREADS = 2
STORE_MEMORY = "1GB"
# What a stored job's result keeps of its row: the controller compares
# reruns on these; a job's `fields` add what it reads ([[derive]], report).
THIN = ("trace", "frames", "bytes", "passed", "failure")
# Retries of an upload that fails, a delay before each.
UPLOAD_RETRIES = (2, 5, 15, 30, 60)
# --end-at is this long before the host's shutdown (farm.py's start_worker).
END_GRACE_S = 600
# Samples, a second apart: the stretch steal's peak is over.
STEAL_WINDOW = 10
# What runs as root where sudo wants a password.
ROOT_IMAGE = "ubuntu:26.04"
# How a test job runs: its binary with these arguments ("<test>" its test),
# and this environment and then the job's over the host's PATH and HOME.
# farm.py hashes both into the job's key.
TEST_ARGS = ["--exact", "<test>", "--include-ignored", "--nocapture", "--test-threads", "1"]
TEST_ENV = {"RUST_BACKTRACE": "1"}


def dirs(root: Path) -> dict[str, Path]:
    d = {k: root / k for k in ("bin", "queue", "work", "done", "fold", "store", "flush", "flushes")}
    for p in d.values():
        p.mkdir(parents=True, exist_ok=True)
    return d


def write_atomic(path: Path, data: str) -> None:
    tmp = path.with_name(f".{path.name}.tmp")
    tmp.write_text(data)
    os.replace(tmp, path)


def pid_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def cpu_times() -> list[int]:
    """/proc/stat's CPU times: user nice system idle iowait irq softirq
    steal."""
    with open("/proc/stat") as f:
        return [int(x) for x in f.readline().split()[1:9]]


class Watch:
    """The host's CPU times and load while a testbed job runs."""

    def __init__(self) -> None:
        self.times = [cpu_times()]
        self.loads = [os.getloadavg()[0]]

    def sample(self) -> None:
        self.times.append(cpu_times())
        self.loads.append(os.getloadavg()[0])

    def summary(self) -> dict:
        def delta(a: list[int], b: list[int]) -> list[int]:
            return [y - x for x, y in zip(a, b)]

        def steal(d: list[int]) -> float:
            return 100 * d[7] / max(sum(d), 1)

        t = self.times
        d = delta(t[0], t[-1])
        peaks = [steal(delta(t[k], t[k + STEAL_WINDOW])) for k in range(len(t) - STEAL_WINDOW)]
        return {
            "steal_pct": round(steal(d), 2),
            "steal_peak_pct": round(max(peaks, default=steal(d)), 2),
            "busy_pct": round(100 * (sum(d) - d[3] - d[4]) / max(sum(d), 1), 2),
            "load_mean": round(statistics.fmean(self.loads), 2),
            "load_max": round(max(self.loads), 2),
        }


class Running:
    def __init__(self, job: dict, proc: subprocess.Popen, started: float, watch: Watch | None = None):
        self.job = job
        self.proc = proc
        self.started = started
        self.watch = watch
        self.at = datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds")


def find_row(log: Path) -> tuple[dict | None, str | None]:
    """The job's row, or why there is none."""
    rows = []
    try:
        with log.open(errors="replace") as f:
            for line in f:
                # libtest's "test <name> ... " precedes the test's output.
                i = line.find("ROW {")
                if i >= 0:
                    rows.append(line[i + 4 :])
    except FileNotFoundError:
        return None, "no log"
    if len(rows) != 1:
        return None, f"{len(rows)} ROW lines"
    try:
        return json.loads(rows[0]), None
    except json.JSONDecodeError as e:
        return None, f"bad ROW: {e}"


def queue_order(p: Path) -> tuple:
    """A queued job's place: its seq if it has one, then its age."""
    try:
        return (json.loads(p.read_text()).get("seq", 0), p.stat().st_mtime, p.name)
    except (OSError, ValueError):
        return (0, 0.0, p.name)


def log_tail(log: Path) -> str:
    try:
        lines = log.read_text(errors="replace").splitlines()
    except FileNotFoundError:
        return ""
    return "\n".join(lines[-LOG_TAIL:])


def spot_notice(url: str = IMDS_URL) -> float | None:
    """When EC2 will interrupt this spot instance, if it has scheduled it:
    the notice's time, else two minutes from now."""
    base = f"{url.rstrip('/')}/latest"
    try:
        req = urllib.request.Request(
            f"{base}/api/token",
            method="PUT",
            headers={"X-aws-ec2-metadata-token-ttl-seconds": "60"},
        )
        token = urllib.request.urlopen(req, timeout=1).read().decode()
        req = urllib.request.Request(
            f"{base}/meta-data/spot/instance-action",
            headers={"X-aws-ec2-metadata-token": token},
        )
        action = urllib.request.urlopen(req, timeout=1).read()
    except Exception:
        return None
    return notice_time(action)


def notice_time(action: bytes) -> float:
    """An instance-action notice's time ({"action": ..., "time":
    "2026-10-29T14:49:40Z"}), else two minutes from now."""
    try:
        t = json.loads(action)["time"]
        return datetime.datetime.fromisoformat(t.replace("Z", "+00:00")).timestamp()
    except (ValueError, KeyError, TypeError, AttributeError):
        return time.time() + 120


# --- the store


@functools.cache
def simdb():
    """hack/sim-db/simdb.py: beside this file on a host (farm.py puts it
    there), in the checkout here."""
    here = Path(__file__).resolve().parent
    for p in (here / "simdb.py", here.parent / "sim-db" / "simdb.py"):
        if p.exists():
            spec = importlib.util.spec_from_file_location("simdb", p)
            m = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(m)
            return m
    raise RuntimeError(f"no simdb.py beside {here} or in hack/sim-db")


@functools.cache
def s3sign():
    spec = importlib.util.spec_from_file_location("s3sign", Path(__file__).resolve().parent / "s3sign.py")
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


class Sink:
    """Where a flush's files go: S3, by the policy file's presigned POST
    policies ({"data": ..., "samples": ...}), or a directory ({"dir": D},
    each flush made whole there by a rename)."""

    def __init__(self, policy: dict, run: str, instance: str, count=None) -> None:
        self.p = policy
        self.run, self.instance = run, instance
        # count(seq, posts, sent, stored): each POST it makes (S3 bills the
        # request, failed or not), the bytes it sent, and those it stored.
        self.count = count or (lambda *_: None)

    def put(self, seq: int, files: list[tuple[str, Path]], abort_at: float | None) -> bool:
        """Puts `files` ((name, path), in order: the tables, manifest.json,
        then the bulky ones, simdb's SAMPLE_TABLES) as flush `seq`'s;
        returns whether the bulky ones landed too. Raises if a table or the
        manifest didn't."""
        if "dir" in self.p:
            dest = Path(self.p["dir"]) / self.run / self.instance / str(seq)
            tmp = dest.with_name(f".{dest.name}.tmp{os.getpid()}")
            shutil.rmtree(tmp, ignore_errors=True)
            tmp.mkdir(parents=True)
            for name, path in files:
                shutil.copyfile(path, tmp / name)
            shutil.rmtree(dest, ignore_errors=True)
            os.replace(tmp, dest)
            return True
        reads = True
        samples = {f"{t}.parquet" for t in simdb().SAMPLE_TABLES}
        for name, path in files:
            pol = self.p["samples" if name in samples else "data"]
            try:
                self.post(seq, pol, f"{pol['prefix']}{seq}/{name}", path, abort_at)
            except Exception:
                if name not in samples:
                    raise
                reads = False
        return reads

    def post(self, seq: int, pol: dict, key: str, path: Path, abort_at: float | None) -> None:
        sent = s3sign().body_size(pol, key, path)
        for delay in (*UPLOAD_RETRIES, None):
            left = None if abort_at is None else abort_at - time.time()
            if left is not None and left <= 0:
                raise TimeoutError(f"{key}: cut off at the interruption")
            try:
                self.count(seq, 1, sent, 0)
                s3sign().post(pol, key, path, timeout=min(120, left) if left else 120)
                self.count(seq, 0, 0, path.stat().st_size)
                return
            except Exception:
                if delay is None or (abort_at is not None and time.time() + delay >= abort_at):
                    raise
                time.sleep(delay)


class Store:
    """A host's store of its test jobs' tables (the module docs): the
    folder thread is the only writer of the open generation, and rotates
    it when asked or full; the flusher exports and uploads closed ones, a
    generation at a time. Generations an earlier serve left are closed, and
    flushed first."""

    def __init__(self, root: Path, args: argparse.Namespace) -> None:
        self.root = root
        self.run, self.instance = args.run, args.instance
        self.sink = Sink(json.loads(Path(args.policy).read_text()), args.run, args.instance, self.count)
        self.rotate_bytes = args.rotate_mb * 1024 * 1024
        self.dir, self.fold_dir = root / "store", root / "fold"
        # Its S3 requests and bytes by flush (sync's `s3`), kept across
        # serves.
        self.ops_lock = threading.Lock()
        try:
            self.ops = json.loads((self.dir / "ops.json").read_text())
        except (OSError, ValueError):
            self.ops = {"flushes": {}}
        self.flush_dir, self.landed_dir = root / "flush", root / "flushes"
        # simdb's spill and the DuckDB it runs.
        os.environ["SIM_DB_ROOT"] = str(self.dir)
        if args.duckdb:
            os.environ["SIM_DUCKDB"] = str(args.duckdb)
        self.cv = threading.Condition()
        gens = sorted(int(p.stem.split("-")[1]) for p in self.dir.glob("gen-*.duckdb"))
        landed = [int(p.stem) for p in self.landed_dir.glob("*.json")]
        try:
            saved = int((self.dir / "next").read_text())
        except (OSError, ValueError):
            saved = 0
        self.closed = collections.deque(gens)
        self.seq = max([saved - 1, *gens, *landed], default=-1) + 1
        self.open_jobs = 0
        self.want_rotate = False
        self.flushing: int | None = None
        self.error: str | None = None
        self.threads = STORE_THREADS
        self.abort_at: float | None = None
        self.stopping = False
        self.save()
        self.workers = [threading.Thread(target=self.guard, args=(f,), daemon=True)
                        for f in (self.folder, self.flusher)]
        for t in self.workers:
            t.start()

    def src(self, seq: int) -> str:
        return f"farm/{self.run}/{self.instance}/{seq}"

    def db(self, seq: int) -> Path:
        return self.dir / f"gen-{seq}.duckdb"

    def save(self) -> None:
        """The store's state for sync (status.json), and the next seq."""
        write_atomic(self.dir / "next", str(self.seq + 1))
        write_atomic(self.dir / "status.json", json.dumps({
            "open": self.seq, "open_jobs": self.open_jobs, "folding": len(self.folds()),
            "closed": list(self.closed), "flushing": self.flushing, "error": self.error,
        }))

    def folds(self) -> list[str]:
        return sorted(p.stem for p in self.fold_dir.glob("*.json"))

    def count(self, seq: int, posts: int, sent: int, stored: int) -> None:
        """Counts flush `seq`'s S3 POSTs, the bytes they sent, and those
        that landed (ops.json)."""
        with self.ops_lock:
            f = self.ops["flushes"].setdefault(str(seq), {"posts": 0, "sent": 0, "stored": 0})
            f["posts"] += posts
            f["sent"] += sent
            f["stored"] += stored
            write_atomic(self.dir / "ops.json", json.dumps(self.ops))

    def add(self, jid: str, job: dict, result: dict, row: dict, out: Path) -> None:
        """A finished job: its out dir and result wait in fold/."""
        dest = self.fold_dir / jid
        shutil.rmtree(dest, ignore_errors=True)
        if out.is_dir():
            os.replace(out, dest)
        else:
            dest.mkdir()
        write_atomic(self.fold_dir / f"{jid}.json", json.dumps({"job": job, "result": result, "row": row}))
        with self.cv:
            self.cv.notify_all()

    def rotate(self) -> None:
        """Asks the folder to close the open generation, once what waits to
        fold is in it."""
        with self.cv:
            self.want_rotate = True
            self.cv.notify_all()

    def hurry(self, abort_at: float) -> None:
        """An interruption: every core for the store, uploads cut off at
        `abort_at`, and the open generation closed."""
        self.threads = os.cpu_count() or STORE_THREADS
        self.abort_at = abort_at
        self.rotate()

    def idle(self) -> bool:
        """Nothing left to fold, close or upload."""
        with self.cv:
            return not self.folds() and not self.closed and self.flushing is None and not self.open_jobs

    def close(self, timeout: float) -> None:
        """Flushes what it holds, waiting up to `timeout`, then stops."""
        self.rotate()
        end = time.time() + timeout
        while time.time() < end and not self.idle():
            time.sleep(0.5)
        with self.cv:
            self.stopping = True
            self.cv.notify_all()

    def guard(self, f) -> None:
        try:
            f()
        except Exception:
            self.error = traceback.format_exc()[-1000:]
            with self.cv:
                self.save()

    def folder(self) -> None:
        while True:
            with self.cv:
                while not self.folds() and not self.want_rotate and not self.stopping:
                    self.cv.wait(1)
                if self.stopping and not self.folds():
                    return
            batch = self.folds()[:FOLD_BATCH]
            if batch:
                try:
                    self.fold(batch)
                except Exception as e:
                    self.error = f"fold: {e}"[-1000:]
                    with self.cv:
                        self.save()
                    time.sleep(5)
                    continue
            with self.cv:
                full = self.db(self.seq).exists() and self.size(self.seq) >= self.rotate_bytes
                if (self.want_rotate and not self.folds()) or full:
                    self.want_rotate = self.want_rotate and bool(self.folds())
                    if self.open_jobs:
                        self.closed.append(self.seq)
                        self.seq += 1
                        self.open_jobs = 0
                        self.cv.notify_all()
                self.save()

    def size(self, seq: int) -> int:
        db = self.db(seq)
        return sum(p.stat().st_size for p in (db, db.with_suffix(".duckdb.wal")) if p.exists())

    def fold(self, wids: list[str]) -> None:
        """Folds finished jobs into the open generation, then leaves their
        results in done/ naming it; one without tables fails."""
        recs = {}
        for w in wids:
            try:
                recs[w] = json.loads((self.fold_dir / f"{w}.json").read_text())
            except (OSError, ValueError):
                (self.fold_dir / f"{w}.json").unlink(missing_ok=True)
        seq = self.seq
        jobs = {w: self.fold_job(w, r) for w, r in recs.items()}
        failed = {}
        try:
            simdb().fold(self.db(seq), jobs.values(), self.src(seq), threads=self.threads, memory=STORE_MEMORY)
        except simdb().FoldError:
            for w, j in jobs.items():
                try:
                    simdb().fold(self.db(seq), [j], self.src(seq), threads=self.threads, memory=STORE_MEMORY)
                except simdb().FoldError as e:
                    failed[w] = str(e)
        for w, r in recs.items():
            res = dict(r["result"])
            if w in failed:
                res.update(row=None, error=failed[w])
            else:
                res["flush"] = seq
            write_atomic(self.root / "done" / f"{w}.json", json.dumps(res))
            shutil.rmtree(self.fold_dir / w, ignore_errors=True)
            (self.fold_dir / f"{w}.json").unlink(missing_ok=True)
        with self.cv:
            self.open_jobs += len(recs) - len(failed)

    def fold_job(self, wid: str, rec: dict) -> dict:
        job, res = rec["job"], rec["result"]
        env = job.get("env", {})
        seed = env.get("SIM_SEED", env.get("STRIMS_SIM_SEED"))
        return {
            "k": job.get("key") or wid, "dir": self.fold_dir / wid, "sim": job.get("sim"), "test": job.get("test"),
            "cell": job.get("matrix"), "arm": job.get("arm"), "seed": int(seed) if str(seed or "").isdigit() else None,
            "env": env, "row": rec["row"], "wall_s": res.get("wall_s"), "rss_mb": res.get("max_rss_mb"),
        }

    def flusher(self) -> None:
        while True:
            with self.cv:
                while not self.closed and not self.stopping:
                    self.cv.wait(1)
                if not self.closed:
                    return
                seq = self.flushing = self.closed[0]
                self.save()
            try:
                self.flush(seq)
                with self.cv:
                    self.closed.popleft()
                    self.error = None
            except Exception as e:
                self.error = f"flush {seq}: {e}"[-1000:]
                if self.abort_at is not None and time.time() >= self.abort_at:
                    return
                # The uploads retried already (UPLOAD_RETRIES): try the
                # generation again later, soon once interrupted.
                time.sleep(5 if self.abort_at is not None else 60)
            finally:
                with self.cv:
                    self.flushing = None
                    self.save()

    def flush(self, seq: int) -> None:
        """Exports generation `seq` (once) and uploads it; once it has
        landed, leaves its record in flushes/ and deletes it."""
        db, dest = self.db(seq), self.flush_dir / str(seq)
        if (dest / "manifest.json").exists():
            manifest = json.loads((dest / "manifest.json").read_text())
        elif db.exists():
            manifest = simdb().export(db, dest, threads=self.threads)
        else:
            manifest = None
        if manifest is not None:
            tables = manifest["tables"]
            bulky = simdb().SAMPLE_TABLES
            files = [(f"{t}.parquet", dest / f"{t}.parquet") for t in tables if t not in bulky]
            files.append(("manifest.json", dest / "manifest.json"))
            files += [(f"{t}.parquet", dest / f"{t}.parquet") for t in bulky if t in tables]
            reads = self.sink.put(seq, files, self.abort_at)
            write_atomic(self.landed_dir / f"{seq}.json", json.dumps({
                "seq": seq, "src": self.src(seq), "jobs": len(manifest["keys"]), "reads": reads and "reads" in tables,
                "bytes": sum(t["bytes"] for t in tables.values()),
                "at": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
            }))
        for p in (db, db.with_suffix(".duckdb.wal")):
            p.unlink(missing_ok=True)
        shutil.rmtree(dest, ignore_errors=True)


# --- testbed jobs


@functools.cache
def mem_available_mb() -> int | None:
    try:
        return next(
            int(line.split()[1]) // 1024
            for line in Path("/proc/meminfo").read_text().splitlines()
            if line.startswith("MemAvailable:")
        )
    except (OSError, StopIteration):
        return None


def host_info() -> dict:
    """The CPU model, kernel and Docker version, for testbed rows."""
    model = None
    try:
        model = next(
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        )
    except (OSError, StopIteration):
        pass
    docker = None
    try:
        p = subprocess.run(["docker", "version", "--format", "{{.Server.Version}}"],
                           capture_output=True, text=True, timeout=30)
        docker = p.stdout.strip() or None
    except (OSError, subprocess.TimeoutExpired):
        pass
    return {"cpu_model": model, "kernel": os.uname().release, "docker": docker}


@functools.cache
def sudo_ok() -> bool:
    try:
        return subprocess.run(["sudo", "-n", "true"], capture_output=True).returncode == 0
    except OSError:
        return False


def as_root(cmd: list[str], under: Path) -> None:
    """Runs cmd as root: through sudo where it needs no password, else in
    a container that mounts `under` at its own path."""
    if sudo_ok():
        full = ["sudo", "-n", *cmd]
    else:
        full = [
            "docker", "run", "--rm", "--network", "none", "-v", f"{under}:{under}",
            "--entrypoint", cmd[0], ROOT_IMAGE, *cmd[1:],
        ]
    p = subprocess.run(full, stdin=subprocess.DEVNULL, capture_output=True, text=True)
    if p.returncode:
        raise RuntimeError(f"{' '.join(cmd[:2])} as root: {p.stderr.strip()[-500:]}")


def remove(path: Path, root: Path) -> None:
    """Removes a work dir, and as root what a testbed's containers left
    in it."""
    shutil.rmtree(path, ignore_errors=True)
    if path.exists():
        try:
            as_root(["rm", "-rf", str(path)], root)
        except Exception:
            pass


def pack(out: Path, dest: Path, root: Path) -> None:
    """The out dir as the tarball dest, holding it as out/; as root if the
    worker can't read some of it."""
    tmp = dest.with_name(f".{dest.name}.tmp")
    try:
        with tarfile.open(tmp, "w:gz") as tar:
            tar.add(out, arcname="out")
    except PermissionError:
        tmp.unlink(missing_ok=True)
        as_root(["tar", "-czf", str(tmp), "-C", str(out.parent), out.name], root)
    os.replace(tmp, dest)


def node_lines(out: Path) -> dict[str, int]:
    """Each node-<i>.jsonl's line count, by i."""
    lines = {}
    for p in sorted(out.glob("node-*.jsonl")):
        with p.open("rb") as f:
            lines[p.stem.removeprefix("node-")] = sum(1 for _ in f)
    return lines


def testbed_names(stopped: bool = False) -> list[str]:
    """The running containers named as run.sh names its own, and with
    `stopped` the stopped ones."""
    try:
        p = subprocess.run(["docker", "ps", *(["-a"] if stopped else []), "--format", "{{.Names}}"],
                           capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return []
    return [n for n in p.stdout.split() if n.startswith("tb-")]


def clear_testbed() -> None:
    """Removes a killed run's containers, which outlive run.sh."""
    names = testbed_names(stopped=True)
    if names:
        subprocess.run(["docker", "rm", "-f", *names], capture_output=True)


def tree_of(root: Path, bundle: str) -> Path:
    """The bundle's tree, extracted on first use."""
    tree = root / "trees" / bundle.removesuffix(".tar.gz")
    if not tree.is_dir():
        tmp = tree.with_name(f".{tree.name}.tmp")
        shutil.rmtree(tmp, ignore_errors=True)
        tmp.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(root / "bin" / bundle) as t:
            t.extractall(tmp, filter="data")
        os.replace(tmp, tree)
    return tree


def test_run(binary: Path, test: str, env: dict[str, str], home: str) -> tuple[list[str], dict[str, str]]:
    """A test job's command and environment (TEST_ARGS, TEST_ENV); HOME
    is `home` where the host has none."""
    cmd = [str(binary), *(test if a == "<test>" else a for a in TEST_ARGS)]
    return cmd, {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": os.environ.get("HOME", home),
        **TEST_ENV,
        **env,
    }


def testbed_run(root: Path, job: dict, work: Path) -> tuple[list[str], dict[str, str]]:
    """run.sh from the job's bundle, and its environment."""
    tree = tree_of(root, job["bin"])
    env = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": os.environ.get("HOME", str(root)),
        **job["env"],
    }
    arts = tree / "artifacts"
    if arts.is_dir():
        env.update({a.name: str(a) for a in arts.iterdir()})
    env.update({var: str(root / "bin" / name) for var, name in job.get("files", {}).items()})
    return ["bash", str(tree / "hack" / "testbed" / "run.sh"), str(work / "out")], env


def finish_testbed(root: Path, r: Running, work: Path, ok: bool) -> dict:
    """A finished testbed run's fields of its result: its out dir packed
    in done/, and the host's share of the run in its row."""
    res = {"started": r.at, **host_info()}
    if not ok:
        return res
    out = work / "out"
    lines = node_lines(out) if out.is_dir() else {}
    if not lines:
        return {**res, "error": "no node-<i>.jsonl in the out dir"}
    try:
        pack(out, root / "done" / f"{r.job['id']}.tar.gz", root)
    except Exception as e:
        return {**res, "error": f"packing the out dir: {e}"}
    row = {**r.watch.summary(), "node_lines": lines, "node_lines_min": min(lines.values())}
    return {**res, "row": row}


def kill(r: Running) -> int:
    """Kills a running job's process group; its wait status."""
    try:
        os.killpg(r.proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    _, status, _ = os.wait4(r.proc.pid, 0)
    return status


def serve(args: argparse.Namespace) -> None:
    root = args.dir
    d = dirs(root)
    pidfile = root / "serve.pid"
    # One serve a dir, for as long as it lives: a second, started before the
    # first wrote its pid (a sync that came first), would share its store.
    lock = (root / "serve.lock").open("w")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        sys.exit("serve is already running")
    write_atomic(pidfile, str(os.getpid()))
    for name in ("interrupting", "ending", "note", "flush-wanted", "wind-down"):
        (root / name).unlink(missing_ok=True)
    (root / "last_sync").touch()
    # A job a previous serve left running goes back to the queue.
    for w in d["work"].iterdir():
        job = w / "job.json"
        if job.exists():
            os.replace(job, d["queue"] / f"{w.name}.json")
        remove(w, root)
    store = Store(root, args) if args.run else None

    host = socket.gethostname()
    running: dict[str, Running] = {}
    # The interruption notice: when it came (monotonic), and the notice's
    # time; `ending` once --end-at has passed.
    noticed: float | None = None
    deadline: float | None = None
    ending = False
    winding = False
    next_notice = 0.0
    stop = False
    note = None

    def on_term(*_):
        nonlocal stop
        stop = True

    signal.signal(signal.SIGTERM, on_term)
    signal.signal(signal.SIGINT, on_term)

    def stored(job: dict) -> bool:
        return store is not None and job.get("kind") != "testbed" and job.get("store", True)

    def end_job(jid: str, r: Running, status: int, usage, now: float, timed_out: bool = False,
                killed: str | None = None) -> None:
        r.proc.returncode = os.waitstatus_to_exitcode(status)
        work = d["work"] / jid
        log = work / "out.log"
        exit_code = r.proc.returncode
        wall = round(now - r.started, 3)
        extra = {}
        if r.watch is None:
            row, why = find_row(log)
        else:
            if timed_out or killed:
                clear_testbed()
            extra = finish_testbed(root, r, work, exit_code == 0 and not timed_out and not killed)
            row, why = extra.pop("row", None), extra.pop("error", None)
            if row is not None:
                row["wall_s"] = wall
        ok = exit_code == 0 and row is not None and not timed_out and not killed
        result = {
            "id": jid,
            "exit": "timeout" if timed_out else "interrupted" if killed else exit_code,
            "row": row if ok else None,
            "error": None if ok else ("timeout" if timed_out else killed or why or f"exit {exit_code}"),
            "wall_s": wall,
            "cpu_s": round(usage.ru_utime + usage.ru_stime, 3) if usage else None,
            "max_rss_mb": round(usage.ru_maxrss / 1024, 1) if usage else None,
            "host": host,
            **extra,
        }
        if killed:
            result["requeue"] = True
        elif not ok:
            result["log_tail"] = log_tail(log)
        if ok and stored(r.job):
            keep = (*THIN, *r.job.get("fields", ()))
            store.add(jid, r.job, {**result, "row": {k: row[k] for k in keep if k in row}}, row, work / "out")
        else:
            write_atomic(d["done"] / f"{jid}.json", json.dumps(result))
        remove(work, root)
        del running[jid]

    while not stop:
        now = time.monotonic()
        for r in running.values():
            if r.watch is not None:
                r.watch.sample()
        for jid, r in list(running.items()):
            pid, status, usage = os.wait4(r.proc.pid, os.WNOHANG)
            if pid == 0:
                if now - r.started < args.job_timeout * 60:
                    continue
                end_job(jid, r, kill(r), None, now, timed_out=True)
                continue
            end_job(jid, r, status, usage, now)

        if args.imds and noticed is None and now >= next_notice:
            next_notice = now + 5
            at = spot_notice(args.imds_url)
            if at is not None:
                noticed, deadline = now, at
                write_atomic(root / "interrupting", str(at))
                if store is not None:
                    store.hurry(at - NOTICE_ABORT_S)
        if noticed is None and not ending and (root / "wind-down").exists():
            try:
                at = float((root / "wind-down").read_text())
            except (OSError, ValueError):
                at = time.time() + NOTICE_FINISH_S
            noticed, deadline, winding = now, at, True
            if store is not None:
                store.hurry(at - NOTICE_ABORT_S)
        if args.end_at and noticed is None and not ending and time.time() >= args.end_at:
            # The controller takes it as an interruption due at the
            # shutdown, END_GRACE_S on, and gives back what's queued here.
            ending = True
            deadline = args.end_at + END_GRACE_S
            write_atomic(root / "ending", str(deadline))
            if store is not None:
                store.rotate()
        # Interrupted: the jobs that can't end in time are killed at once,
        # the rest NOTICE_FINISH_S after the notice; past --end-at, all at
        # once.
        if noticed is not None or ending:
            for jid, r in list(running.items()):
                late = r.started + float(r.job.get("expected_s") or 0) > now + NOTICE_FINISH_S
                if ending or late or now - noticed >= NOTICE_FINISH_S:
                    why = ("interrupted: past --end-at" if ending else "interrupted: the controller winds down"
                           if winding else "interrupted: a spot interruption notice")
                    end_job(jid, r, kill(r), None, now, killed=why)
            if store is not None and not running and not store.want_rotate and store.open_jobs:
                store.rotate()

        queued = sorted(d["queue"].glob("*.json"), key=queue_order)
        waits = None
        while len(running) < args.slots and queued and noticed is None and not ending:
            q = queued[0]
            try:
                job = json.loads(q.read_text())
            except FileNotFoundError:
                queued.pop(0)
                continue
            testbed = job.get("kind") == "testbed"
            if args.end_at and not testbed and time.time() + float(job.get("expected_s") or 0) > args.end_at:
                waits = "starting no job that would end past --end-at"
                break
            if testbed:
                others = testbed_names()
                if others:
                    waits = f"waiting for {others[0]}, another testbed's container, to end"
                    break
            # A job's slot holds its memory: with less free, the jobs
            # running took more, and another could run the host out.
            elif args.min_free_mb and running and (free := mem_available_mb()) is not None and free < args.min_free_mb:
                waits = f"waiting for memory: {free} MiB free, a job's slot is {args.min_free_mb}"
                break
            queued.pop(0)
            jid = q.stem
            work = d["work"] / jid
            work.mkdir(parents=True, exist_ok=True)
            try:
                os.replace(q, work / "job.json")
            except FileNotFoundError:
                continue
            try:
                if testbed:
                    cmd, env = testbed_run(root, job, work)
                else:
                    cmd, env = test_run(d["bin"] / job["bin"], job["test"], job["env"], str(root))
                    if stored(job) and job.get("out"):
                        (work / "out").mkdir(exist_ok=True)
                        env[job["out"]] = str(work / "out")
                with (work / "out.log").open("wb") as out:
                    proc = subprocess.Popen(
                        cmd,
                        cwd=work,
                        env=env,
                        stdin=subprocess.DEVNULL,
                        stdout=out,
                        stderr=subprocess.STDOUT,
                        start_new_session=True,
                    )
            except (OSError, tarfile.TarError) as e:
                result = {"id": jid, "exit": None, "row": None, "error": str(e), "host": host}
                write_atomic(d["done"] / f"{jid}.json", json.dumps(result))
                shutil.rmtree(work, ignore_errors=True)
                continue
            running[jid] = Running(job, proc, now, Watch() if testbed else None)
        if waits != note:
            note = waits
            if note:
                write_atomic(root / "note", note)
            else:
                (root / "note").unlink(missing_ok=True)

        # The controller's flush, once nothing is left here to run.
        if store is not None and (root / "flush-wanted").exists() and not running and not queued:
            (root / "flush-wanted").unlink(missing_ok=True)
            store.rotate()

        if not running and not queued:
            idle = time.time() - (root / "last_sync").stat().st_mtime
            if idle > args.idle_minutes * 60:
                # What it holds goes up before it does.
                if store is not None and not store.idle():
                    if not store.want_rotate:
                        store.rotate()
                else:
                    if args.poweroff:
                        subprocess.run(["sudo", "poweroff"], check=False)
                    break
        # A short job's slot shouldn't wait long for the next.
        time.sleep(0.2 if running or queued else 1)

    for r in running.values():
        try:
            os.killpg(r.proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    if any(r.watch is not None for r in running.values()):
        clear_testbed()
    if store is not None:
        # What finished is worth its upload: an interruption's goes on
        # until it's cut off, a stopped serve's for a minute.
        store.close(max(0.0, deadline - time.time()) if deadline else 60)
    pidfile.unlink(missing_ok=True)


def sync(args: argparse.Namespace) -> None:
    root = args.dir
    d = dirs(root)
    req = json.loads(sys.stdin.read() or "{}")
    for jid in req.get("ack", []):
        (d["done"] / f"{jid}.tar.gz").unlink(missing_ok=True)
        (d["done"] / f"{jid}.json").unlink(missing_ok=True)
    for seq in req.get("ack_flushes", []):
        (d["flushes"] / f"{int(seq)}.json").unlink(missing_ok=True)
    # A dropped job that serve has started already runs on.
    for jid in req.get("drop", []):
        (d["queue"] / f"{jid}.json").unlink(missing_ok=True)
    known = {p.stem for p in d["queue"].glob("*.json")}
    known |= {p.name for p in d["work"].iterdir()}
    known |= {p.stem for p in d["fold"].glob("*.json")}
    known |= {p.stem for p in d["done"].glob("*.json")}
    for job in req.get("jobs", []):
        if job["id"] not in known:
            write_atomic(d["queue"] / f"{job['id']}.json", json.dumps(job))
    if req.get("flush"):
        (root / "flush-wanted").touch()
    if req.get("wind_down"):
        write_atomic(root / "wind-down", str(float(req["wind_down"])))
    (root / "last_sync").touch()

    # Queued, then running, then folding, then done: a job moves the same
    # way (into work/ before it leaves queue/, into fold/ or done/ before it
    # leaves work/, into done/ before it leaves fold/), so it can't fall
    # between the listings.
    queued = sorted(p.stem for p in d["queue"].glob("*.json"))
    running = sorted(p.name for p in d["work"].iterdir())
    folding = sorted(p.stem for p in d["fold"].glob("*.json"))
    # When each running job started: moving its job.json into work/ set
    # the file's ctime.
    started = {}
    for jid in running:
        try:
            started[jid] = (d["work"] / jid / "job.json").stat().st_ctime
        except FileNotFoundError:
            pass
    done = []
    for p in d["done"].glob("*.json"):
        try:
            done.append(json.loads(p.read_text()))
        except (FileNotFoundError, json.JSONDecodeError):
            pass
    flushes = []
    for p in d["flushes"].glob("*.json"):
        try:
            flushes.append(json.loads(p.read_text()))
        except (FileNotFoundError, json.JSONDecodeError):
            pass
    try:
        store = json.loads((d["store"] / "status.json").read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        store = None
    try:
        s3 = json.loads((d["store"] / "ops.json").read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        s3 = None
    serving = False
    try:
        serving = pid_alive(int((root / "serve.pid").read_text()))
    except (FileNotFoundError, ValueError):
        pass
    mem = mem_available_mb()
    try:
        note = (root / "note").read_text()
    except FileNotFoundError:
        note = None
    try:
        interrupting = float((root / "interrupting").read_text())
    except FileNotFoundError:
        interrupting = None
    except ValueError:
        interrupting = time.time() + 120
    try:
        ending = float((root / "ending").read_text())
    except (FileNotFoundError, ValueError):
        ending = None
    reply = {
        "done": done,
        "queued": queued,
        "running": running,
        "folding": folding,
        "started": started,
        "flushes": sorted(flushes, key=lambda f: f["seq"]),
        "store": store,
        "s3": s3,
        "serving": serving,
        "interrupting": interrupting,
        "ending": ending,
        "load": os.getloadavg()[0],
        "mem_available_mb": mem,
        "note": note,
    }
    json.dump(reply, sys.stdout)


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    here = Path(__file__).resolve().parent
    s = sub.add_parser("serve")
    s.add_argument("--dir", type=Path, default=here)
    s.add_argument("--slots", type=int, required=True)
    s.add_argument("--job-timeout", type=float, default=60, help="minutes")
    s.add_argument("--idle-minutes", type=float, default=15)
    s.add_argument("--imds", action="store_true")
    s.add_argument("--poweroff", action="store_true")
    s.add_argument("--min-free-mb", type=int, default=0,
                   help="start no test job while the host has less memory available")
    s.add_argument("--imds-url", default=IMDS_URL, help="the instance metadata's base URL (tests)")
    s.add_argument("--run", help="the run's name: test jobs' tables go to its store (Store)")
    s.add_argument("--instance", default=socket.gethostname(), help="the host's name in the store's keys")
    s.add_argument("--policy", type=Path, help="the upload policies (Sink)")
    s.add_argument("--rotate-mb", type=float, default=512, help="close a generation this big")
    s.add_argument("--end-at", type=float, help="seconds since the epoch: stop and flush then")
    s.add_argument("--duckdb", type=Path, help="the DuckDB CLI (else simdb's)")
    y = sub.add_parser("sync")
    y.add_argument("--dir", type=Path, default=here)
    args = p.parse_args()
    {"serve": serve, "sync": sync}[args.cmd](args)


if __name__ == "__main__":
    main()
