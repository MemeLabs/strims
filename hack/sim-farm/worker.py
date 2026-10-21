#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sim farm's worker: runs farm.py's jobs on one host.

  worker.py serve [--dir D] --slots N [--job-timeout MIN] [--idle-minutes MIN]
                  [--imds] [--poweroff]
  worker.py sync [--dir D]

`serve` runs up to N jobs at once from D/queue/, oldest first. A job is a
test binary in D/bin/ run as one test (`--exact <test> --ignored
--nocapture`) with the job's environment, in D/work/<id>/, its output in
out.log there. A finished job leaves D/done/<id>.json: its exit, the
single `ROW {json}` it printed, its wall and CPU time, its peak RSS,
and the tail of its log if it failed. A job past --job-timeout minutes is
killed. With --imds it polls the instance metadata for a spot
interruption notice and, once one comes, starts nothing new. With nothing
running or queued and no sync for --idle-minutes it exits, and with
--poweroff it powers the host off first; farm.py launches instances that
terminate on shutdown, so a host whose controller went away stops costing
money.

`sync` is the controller's one round trip per tick. It reads one JSON
object from stdin, {"ack": [ids], "jobs": [jobs]}: it deletes the acked
results (the controller has stored them) and queues the jobs. It prints
one JSON object: the results not yet acked, the queued and running ids,
and whether serve is up and whether the host is being interrupted. A
result stays until it is acked, so a sync whose reply is lost loses
nothing.

D defaults to this file's directory. Every write is a rename, so serve
and sync can run at once.
"""

import argparse
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

LOG_TAIL = 40


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


class Running:
    def __init__(self, job: dict, proc: subprocess.Popen, started: float):
        self.job = job
        self.proc = proc
        self.started = started


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
    (root / "last_sync").touch()
    # A job a previous serve left running goes back to the queue.
    for w in d["work"].iterdir():
        job = w / "job.json"
        if job.exists():
            os.replace(job, d["queue"] / f"{w.name}.json")
        shutil.rmtree(w, ignore_errors=True)

    host = socket.gethostname()
    running: dict[str, Running] = {}
    interrupting = False
    next_notice = 0.0
    stop = False

    def on_term(*_):
        nonlocal stop
        stop = True

    signal.signal(signal.SIGTERM, on_term)
    signal.signal(signal.SIGINT, on_term)

    while not stop:
        now = time.monotonic()
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
            row, why = find_row(log)
            exit_code = r.proc.returncode
            ok = exit_code == 0 and row is not None and not timed_out
            result = {
                "id": jid,
                "exit": "timeout" if timed_out else exit_code,
                "row": row if ok else None,
                "error": None if ok else ("timeout" if timed_out else why or f"exit {exit_code}"),
                "wall_s": round(now - r.started, 3),
                "cpu_s": round(usage.ru_utime + usage.ru_stime, 3),
                "max_rss_mb": round(usage.ru_maxrss / 1024, 1),
                "host": host,
            }
            if not ok:
                result["log_tail"] = log_tail(log)
            write_atomic(d["done"] / f"{jid}.json", json.dumps(result))
            shutil.rmtree(work, ignore_errors=True)
            del running[jid]

        if args.imds and not interrupting and now >= next_notice:
            next_notice = now + 5
            if spot_notice():
                interrupting = True
                write_atomic(root / "interrupting", "1")

        queued = sorted(d["queue"].glob("*.json"), key=lambda p: (p.stat().st_mtime, p.name))
        while not interrupting and len(running) < args.slots and queued:
            q = queued.pop(0)
            jid = q.stem
            work = d["work"] / jid
            work.mkdir(parents=True, exist_ok=True)
            try:
                os.replace(q, work / "job.json")
            except FileNotFoundError:
                continue
            job = json.loads((work / "job.json").read_text())
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
                "--ignored",
                "--nocapture",
                "--test-threads",
                "1",
            ]
            try:
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
            except OSError as e:
                result = {"id": jid, "exit": None, "row": None, "error": str(e), "host": host}
                write_atomic(d["done"] / f"{jid}.json", json.dumps(result))
                shutil.rmtree(work, ignore_errors=True)
                continue
            running[jid] = Running(job, proc, now)

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
    pidfile.unlink(missing_ok=True)


def sync(args: argparse.Namespace) -> None:
    root = args.dir
    d = dirs(root)
    req = json.loads(sys.stdin.read() or "{}")
    for jid in req.get("ack", []):
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
    reply = {
        "done": done,
        "queued": queued,
        "running": running,
        "serving": serving,
        "interrupting": (root / "interrupting").exists(),
        "load": os.getloadavg()[0],
        "mem_available_mb": mem,
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
