#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sim farm's worker: runs farm.py's jobs on one host.

  worker.py serve [--dir D] --slots N [--job-timeout MIN] [--idle-minutes MIN]
                  [--imds] [--poweroff]
  worker.py sync [--dir D]

`serve` runs up to N jobs at once from D/queue/, by their `seq` if they
have one, then oldest first. A job is a test binary in D/bin/ run as one
test (`--exact <test> --include-ignored --nocapture`) with the job's
environment, in D/work/<id>/, its output in out.log there. A finished job
leaves D/done/<id>.json: its exit, the single `ROW {json}` it printed,
its wall and CPU time, its peak RSS, and the tail of its log if it
failed. A job past --job-timeout minutes is killed.

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

With --imds it polls the instance metadata for a spot
interruption notice and, once one comes, starts nothing new. With nothing
running or queued and no sync for --idle-minutes it exits, and with
--poweroff it powers the host off first; farm.py launches instances that
terminate on shutdown, so a host whose controller went away stops costing
money.

`sync` is the controller's one round trip per tick. It reads one JSON
object from stdin, {"ack": [ids], "jobs": [jobs]}: it deletes the acked
results (the controller has stored them) and queues the jobs. It prints
one JSON object: the results not yet acked, the queued and running ids,
whether serve is up and whether the host is being interrupted, and why
serve waits, if it does. A result stays until it is acked, so a sync
whose reply is lost loses nothing.

D defaults to this file's directory. Every write is a rename, so serve
and sync can run at once.
"""

import argparse
import datetime
import functools
import json
import os
import shutil
import signal
import socket
import statistics
import subprocess
import sys
import tarfile
import time
import urllib.request
from pathlib import Path

LOG_TAIL = 40
# Samples, a second apart: the stretch steal's peak is over.
STEAL_WINDOW = 10
# What runs as root where sudo wants a password.
ROOT_IMAGE = "ubuntu:26.04"


def dirs(root: Path) -> dict[str, Path]:
    d = {k: root / k for k in ("bin", "queue", "work", "done")}
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


def spot_notice() -> bool:
    """Whether EC2 has scheduled this spot instance's interruption."""
    base = "http://169.254.169.254/latest"
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
        urllib.request.urlopen(req, timeout=1).read()
        return True
    except Exception:
        return False


# --- testbed jobs


@functools.cache
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


def serve(args: argparse.Namespace) -> None:
    root = args.dir
    d = dirs(root)
    pidfile = root / "serve.pid"
    if pidfile.exists():
        try:
            if pid_alive(int(pidfile.read_text())):
                sys.exit("serve is already running")
        except ValueError:
            pass
    write_atomic(pidfile, str(os.getpid()))
    (root / "interrupting").unlink(missing_ok=True)
    (root / "note").unlink(missing_ok=True)
    (root / "last_sync").touch()
    # A job a previous serve left running goes back to the queue.
    for w in d["work"].iterdir():
        job = w / "job.json"
        if job.exists():
            os.replace(job, d["queue"] / f"{w.name}.json")
        remove(w, root)

    host = socket.gethostname()
    running: dict[str, Running] = {}
    interrupting = False
    next_notice = 0.0
    stop = False
    note = None

    def on_term(*_):
        nonlocal stop
        stop = True

    signal.signal(signal.SIGTERM, on_term)
    signal.signal(signal.SIGINT, on_term)

    while not stop:
        now = time.monotonic()
        for r in running.values():
            if r.watch is not None:
                r.watch.sample()
        for jid, r in list(running.items()):
            pid, status, usage = os.wait4(r.proc.pid, os.WNOHANG)
            timed_out = False
            if pid == 0:
                if now - r.started < args.job_timeout * 60:
                    continue
                os.killpg(r.proc.pid, signal.SIGKILL)
                pid, status, usage = os.wait4(r.proc.pid, 0)
                timed_out = True
            r.proc.returncode = os.waitstatus_to_exitcode(status)
            work = d["work"] / jid
            log = work / "out.log"
            exit_code = r.proc.returncode
            wall = round(now - r.started, 3)
            extra = {}
            if r.watch is None:
                row, why = find_row(log)
            else:
                if timed_out:
                    clear_testbed()
                extra = finish_testbed(root, r, work, exit_code == 0 and not timed_out)
                row, why = extra.pop("row", None), extra.pop("error", None)
                if row is not None:
                    row["wall_s"] = wall
            ok = exit_code == 0 and row is not None and not timed_out
            result = {
                "id": jid,
                "exit": "timeout" if timed_out else exit_code,
                "row": row if ok else None,
                "error": None if ok else ("timeout" if timed_out else why or f"exit {exit_code}"),
                "wall_s": wall,
                "cpu_s": round(usage.ru_utime + usage.ru_stime, 3),
                "max_rss_mb": round(usage.ru_maxrss / 1024, 1),
                "host": host,
                **extra,
            }
            if not ok:
                result["log_tail"] = log_tail(log)
            write_atomic(d["done"] / f"{jid}.json", json.dumps(result))
            remove(work, root)
            del running[jid]

        if args.imds and not interrupting and now >= next_notice:
            next_notice = now + 5
            if spot_notice():
                interrupting = True
                write_atomic(root / "interrupting", "1")

        queued = sorted(d["queue"].glob("*.json"), key=queue_order)
        waits = None
        while not interrupting and len(running) < args.slots and queued:
            q = queued[0]
            try:
                job = json.loads(q.read_text())
            except FileNotFoundError:
                queued.pop(0)
                continue
            testbed = job.get("kind") == "testbed"
            if testbed:
                others = testbed_names()
                if others:
                    waits = f"waiting for {others[0]}, another testbed's container, to end"
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
                    env = {
                        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
                        "HOME": os.environ.get("HOME", str(root)),
                        "RUST_BACKTRACE": "1",
                        **job["env"],
                    }
                    cmd = [
                        str(d["bin"] / job["bin"]),
                        "--exact",
                        job["test"],
                        "--include-ignored",
                        "--nocapture",
                        "--test-threads",
                        "1",
                    ]
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

        if not running and not queued:
            idle = time.time() - (root / "last_sync").stat().st_mtime
            if idle > args.idle_minutes * 60:
                if args.poweroff:
                    subprocess.run(["sudo", "poweroff"], check=False)
                break
        time.sleep(1)

    for r in running.values():
        try:
            os.killpg(r.proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    if any(r.watch is not None for r in running.values()):
        clear_testbed()
    pidfile.unlink(missing_ok=True)


def sync(args: argparse.Namespace) -> None:
    root = args.dir
    d = dirs(root)
    req = json.loads(sys.stdin.read() or "{}")
    for jid in req.get("ack", []):
        (d["done"] / f"{jid}.tar.gz").unlink(missing_ok=True)
        (d["done"] / f"{jid}.json").unlink(missing_ok=True)
    known = {p.stem for p in d["queue"].glob("*.json")}
    known |= {p.name for p in d["work"].iterdir()}
    known |= {p.stem for p in d["done"].glob("*.json")}
    for job in req.get("jobs", []):
        if job["id"] not in known:
            write_atomic(d["queue"] / f"{job['id']}.json", json.dumps(job))
    (root / "last_sync").touch()

    # Queued, then running, then done: a job moves the same way (into
    # work/ before it leaves queue/, into done/ before it leaves work/), so
    # it can't fall between the listings.
    queued = sorted(p.stem for p in d["queue"].glob("*.json"))
    running = sorted(p.name for p in d["work"].iterdir())
    done = []
    for p in d["done"].glob("*.json"):
        try:
            done.append(json.loads(p.read_text()))
        except (FileNotFoundError, json.JSONDecodeError):
            pass
    serving = False
    try:
        serving = pid_alive(int((root / "serve.pid").read_text()))
    except (FileNotFoundError, ValueError):
        pass
    try:
        mem = next(
            int(line.split()[1]) // 1024
            for line in Path("/proc/meminfo").read_text().splitlines()
            if line.startswith("MemAvailable:")
        )
    except (OSError, StopIteration):
        mem = None
    try:
        note = (root / "note").read_text()
    except FileNotFoundError:
        note = None
    reply = {
        "done": done,
        "queued": queued,
        "running": running,
        "serving": serving,
        "interrupting": (root / "interrupting").exists(),
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
    y = sub.add_parser("sync")
    y.add_argument("--dir", type=Path, default=here)
    args = p.parse_args()
    {"serve": serve, "sync": sync}[args.cmd](args)


if __name__ == "__main__":
    main()
